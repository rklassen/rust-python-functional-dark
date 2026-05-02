//! Parse helpers for `WeightText`. Split from `weight_text.rs` only to
//! honor the per-file line ceiling. The continuation `impl WeightText`
//! block holds private parse routines; the public entry lives in
//! `weight_text.rs`.

use smallvec::SmallVec;

use crate::codec::hex::Hex;
use crate::codec::weight_text::WeightText;
use crate::codec::weight_text_errors::WErrs;
use crate::data::err::SemanticErr;
use crate::data::types::{BytestreamRef, NumericEncoding};
use crate::data::weight::EdgeWeight;

type WErr = Vec<SemanticErr>;
type Enc = NumericEncoding;

impl WeightText {
    pub(super) fn p_legacy_check(s: &str) -> Result<(), WErr> {
        for tag in ["snorm", "unorm", "0h", "0o", "int", "float"] {
            let needle = format!(":{tag}");
            if s.ends_with(&needle) {
                return Err(vec![WErrs::legacy_tag(tag)]);
            }
        }
        Ok(())
    }

    pub(super) fn p_byteref(
        rest: &str,
        enc: Enc,
    ) -> Result<EdgeWeight, WErr> {
        // v0.7 disambiguation: bare `@id` (no `..` or `+` after the id)
        // is an operator-ref dynamic weight; `@id ..len` /
        // `@id +offset..len` is a bytestream slice. Kind validation
        // (operator vs stream) lives in `Edges::new` where the node
        // table is in scope.
        let s = rest.trim();
        if !Self::has_slice_form(s) {
            return Self::p_opref(s, enc);
        }
        if !matches!(enc, Enc::Raw) {
            // Caller passed a non-Raw arrow with `@id ..len` content;
            // that's the caller's bug, but we still emit a clear
            // diagnostic.
            return Err(vec![WErrs::e(
                "@byteref under non-Raw encoding",
                "the plain `-(@id ..len)>` arrow (Raw)",
                &["use `-(@id ..len)>` (no format mark)"],
            )]);
        }
        let bad = || vec![WErrs::bad_byteref()];
        let id_end = s.find([' ', '+', '.']).ok_or_else(bad)?;
        let id = s[..id_end].trim();
        if id.is_empty() {
            return Err(bad());
        }
        let tail = s[id_end..].trim_start();
        let (offset, len_part) = Self::p_byteref_slice(tail)?;
        let len_str = len_part.trim();
        let len: u32 = len_str
            .parse()
            .map_err(|_| vec![WErrs::bad_len(len_str)])?;
        if len == 0 {
            return Err(vec![WErrs::zero_len()]);
        }
        let bref = BytestreamRef {
            stream: id.into(),
            offset,
            len,
        };
        Ok(EdgeWeight::ByteRef(bref, Enc::Raw))
    }

    /// True when an `@id` body has a slice tail (`..` or `+...`).
    /// Bare `@id` (no slice) is an operator-ref.
    fn has_slice_form(s: &str) -> bool {
        s.contains("..") || s.contains('+')
    }

    pub(super) fn p_opref(
        s: &str,
        enc: Enc,
    ) -> Result<EdgeWeight, WErr> {
        let id = s.trim();
        if id.is_empty() {
            return Err(vec![WErrs::bad_opref()]);
        }
        // The id must be a single identifier — no whitespace, no
        // commas, no parens. Any of those means a malformed weight.
        if id
            .bytes()
            .any(|b| matches!(b, b' ' | b'\t' | b',' | b'|'))
        {
            return Err(vec![WErrs::bad_opref()]);
        }
        Ok(EdgeWeight::OpRef(id.into(), enc))
    }

    fn p_byteref_slice(tail: &str) -> Result<(u32, &str), WErr> {
        let bad = || vec![WErrs::bad_byteref()];
        if let Some(a) = tail.strip_prefix('+') {
            let dd = a.find("..").ok_or_else(bad)?;
            let os = a[..dd].trim();
            let off: u32 = os
                .parse()
                .map_err(|_| vec![WErrs::bad_offset(os)])?;
            Ok((off, &a[dd + 2..]))
        } else if let Some(a) = tail.strip_prefix("..") {
            Ok((0u32, a))
        } else {
            Err(bad())
        }
    }

    pub(super) fn p_top(s: &str, enc: Enc) -> Result<EdgeWeight, WErr> {
        // Caller has already rejected `[` and `]` characters.
        // Pipes split rows; commas split values within a row.
        if s.starts_with('|') {
            return Err(vec![WErrs::leading_pipe()]);
        }
        if s.ends_with('|') {
            return Err(vec![WErrs::trailing_pipe()]);
        }
        if s.contains('|') {
            return Self::p_pipe_rows(s, enc);
        }
        let parts: Vec<&str> =
            s.split(',').map(str::trim).collect();
        Self::p_flat(&parts, enc)
    }

    fn p_pipe_rows(s: &str, enc: Enc) -> Result<EdgeWeight, WErr> {
        if matches!(enc, Enc::Raw) {
            return Err(vec![WErrs::raw_value_list()]);
        }
        let mut errs: WErr = Vec::new();
        let mut rows: Vec<SmallVec<[f64; 8]>> = Vec::new();
        for row in s.split('|') {
            let trimmed = row.trim();
            if trimmed.is_empty() {
                errs.push(WErrs::empty_row());
                continue;
            }
            let parts: Vec<&str> =
                trimmed.split(',').map(str::trim).collect();
            if parts.iter().any(|p| p.is_empty()) {
                errs.push(WErrs::empty_row());
                continue;
            }
            match Self::p_flat(&parts, enc) {
                Ok(EdgeWeight::Vec(vs, _)) => rows.push(vs),
                Ok(_) => errs.push(WErrs::empty_row()),
                Err(es) => errs.extend(es),
            }
        }
        if !errs.is_empty() {
            return Err(errs);
        }
        Ok(EdgeWeight::Matrix(rows, enc))
    }

    fn p_flat(parts: &[&str], enc: Enc) -> Result<EdgeWeight, WErr> {
        if matches!(enc, Enc::Raw) {
            return Err(vec![WErrs::raw_value_list()]);
        }
        if matches!(enc, Enc::Hex) {
            return Self::p_hex_tokens(parts);
        }
        if Self::any_looks_hex(parts) {
            return Err(vec![WErrs::hex_wrong_enc()]);
        }
        let mut errs: WErr = Vec::new();
        Self::check_mixed(parts, &mut errs);
        let mut vs: SmallVec<[f64; 8]> = SmallVec::new();
        for p in parts {
            match p.parse::<f64>() {
                Ok(v) => {
                    if let Err(e) = Self::range(v, enc) {
                        errs.push(e);
                    }
                    vs.push(v);
                }
                Err(_) => errs.push(WErrs::bad_number(p)),
            }
        }
        if !errs.is_empty() {
            return Err(errs);
        }
        Ok(EdgeWeight::Vec(vs, enc))
    }

    fn p_hex_tokens(parts: &[&str]) -> Result<EdgeWeight, WErr> {
        let mut errs: WErr = Vec::new();
        let mut rows: Vec<SmallVec<[f64; 8]>> = Vec::new();
        for p in parts {
            match Hex::decode(p) {
                Ok(bs) => {
                    let row: SmallVec<[f64; 8]> =
                        bs.iter().copied().map(f64::from).collect();
                    rows.push(row);
                }
                Err(e) => errs.push(e),
            }
        }
        if !errs.is_empty() {
            return Err(errs);
        }
        if let [only] = rows.as_slice() {
            return Ok(EdgeWeight::Vec(only.clone(), Enc::Hex));
        }
        Ok(EdgeWeight::Matrix(rows, Enc::Hex))
    }

    fn check_mixed(parts: &[&str], errs: &mut WErr) {
        let mut f = false;
        let mut i = false;
        for p in parts {
            if Self::looks_float(p) {
                f = true;
            } else if !p.is_empty() {
                i = true;
            }
        }
        if f && i {
            errs.push(WErrs::mixed());
        }
    }

    fn any_looks_hex(parts: &[&str]) -> bool {
        parts
            .iter()
            .any(|p| Hex::decode(p).is_ok() && Self::looks_hex(p))
    }

    fn looks_hex(s: &str) -> bool {
        !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_hexdigit())
            && s.bytes().any(|b| !b.is_ascii_digit())
    }

    fn looks_float(s: &str) -> bool {
        s.contains('.') || s.contains('e') || s.contains('E')
    }

    fn range(v: f64, e: Enc) -> Result<(), SemanticErr> {
        match e {
            Enc::Snorm if !(-1.0..=1.0).contains(&v) => {
                Err(WErrs::snorm_oor(v))
            }
            Enc::Unorm if !(0.0..=1.0).contains(&v) => {
                Err(WErrs::unorm_oor(v))
            }
            _ => Ok(()),
        }
    }
}
