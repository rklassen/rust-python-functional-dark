//! Emit helpers for `WeightText`. Split from `weight_text.rs` only to
//! honor the per-file line ceiling. The format mark (`s`/`u`/`h`) is
//! NOT emitted here — the caller wraps the inner string with the right
//! arrow form. Emit produces only the inner body.
//!
//! Layout: comma joins values within a row; ` | ` joins rows. For Hex
//! a row is a single concatenated uppercase blob (no inner spaces).
//! Single-row Matrix collapses to Vec form (no pipe).

use smallvec::SmallVec;

use crate::codec::hex::Hex;
use crate::codec::weight_text::WeightText;
use crate::data::types::{BytestreamRef, NumericEncoding};

type Enc = NumericEncoding;

impl WeightText {
    pub(super) fn e_vec_one(v: f64, e: Enc) -> String {
        Self::e_num(v, e)
    }

    pub(super) fn e_vec_many(
        vs: &SmallVec<[f64; 8]>,
        e: Enc,
    ) -> String {
        if matches!(e, Enc::Hex) {
            return Self::e_hex_blob(vs);
        }
        let parts: Vec<String> =
            vs.iter().map(|v| Self::e_num(*v, e)).collect();
        parts.join(", ")
    }

    pub(super) fn e_matrix(
        rows: &[SmallVec<[f64; 8]>],
        e: Enc,
    ) -> String {
        if matches!(e, Enc::Hex) {
            let parts: Vec<String> =
                rows.iter().map(Self::e_hex_blob).collect();
            return parts.join(" | ");
        }
        let parts: Vec<String> = rows
            .iter()
            .map(|r| {
                let inner: Vec<String> =
                    r.iter().map(|v| Self::e_num(*v, e)).collect();
                inner.join(", ")
            })
            .collect();
        parts.join(" | ")
    }

    pub(super) fn e_byteref(r: &BytestreamRef) -> String {
        if r.offset == 0 {
            format!("@{} ..{}", r.stream, r.len)
        } else {
            format!("@{} +{}..{}", r.stream, r.offset, r.len)
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
    )]
    fn e_num(v: f64, e: Enc) -> String {
        match e {
            Enc::Int => format!("{}", v as i64),
            Enc::Hex => Hex::encode(&[v as u8]),
            _ => Self::e_float(v),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn e_float(v: f64) -> String {
        if v.fract() == 0.0 && v.is_finite() {
            format!("{}", v as i64)
        } else {
            format!("{v}")
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
    )]
    fn e_hex_blob(vs: &SmallVec<[f64; 8]>) -> String {
        let bs: Vec<u8> = vs.iter().map(|x| *x as u8).collect();
        Hex::encode(&bs)
    }
}
