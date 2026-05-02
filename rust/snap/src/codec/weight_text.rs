//! Textual encoding of the *inner content* of an edge weight.
//!
//! The format mark (`s`/`u`/`h`) lives on the arrow itself in v0.6
//! (`-(value)s->`, `-(value)u->`, `-(value)h->`); io/snap parses the
//! mark, picks the matching `NumericEncoding`, and passes the inner
//! body here. The codec NEVER sees a `:tag` suffix in v0.6 input and
//! NEVER emits one in output.
//!
//! Accepted forms:
//!   - bare scalar:        `0.5`, `352`, `-0.25`
//!   - bare flat list:     `0.5, 0.875, 0.23`
//!   - hex tokens:         `FF12AABB` or `FF12, AABB, DEAD`
//!   - list-of-lists:      `1,2,3 | 4,5 | 6,7,8,9` (pipes
//!     between rows, commas within)
//!   - byteref (Raw):      `@id ..len` or `@id +offset..len`
//!
//! Length-1 vec is the canonical scalar carrier (collapsed in data).
//! Brackets `[` and `]` are hard-rejected anywhere in the input — they
//! were the legacy list-of-lists form. Error builders live in
//! `weight_text_errors`. Parse and emit helpers live in sibling files
//! for the per-file line ceiling.

use crate::codec::weight_text_errors::WErrs;
use crate::data::err::SemanticErr;
use crate::data::types::NumericEncoding;
use crate::data::weight::EdgeWeight;

/// Carrier for `EdgeWeight` text encoding/decoding. Stateless.
pub struct WeightText;

type WErr = Vec<SemanticErr>;

impl WeightText {
    /// Parse the inner content of an edge weight (the stuff between
    /// `(` and `)` on a `-(...)X->` arrow). The encoding is supplied
    /// by the caller from the arrow's format mark.
    ///
    /// # Errors
    ///
    /// Returns `Vec<SemanticErr>` for empty input, legacy `:tag`
    /// suffixes, any `[`/`]` characters, mixed numeric kinds, range
    /// violations on snorm/unorm, malformed hex, leading/trailing
    /// pipes, empty rows, or malformed byterefs. Multiple problems
    /// may be reported at once.
    pub fn parse(
        input: &str,
        encoding: NumericEncoding,
    ) -> Result<EdgeWeight, WErr> {
        let s = input.trim();
        if s.is_empty() {
            return Err(vec![WErrs::empty_weight()]);
        }
        Self::p_legacy_check(s)?;
        if s.contains('[') || s.contains(']') {
            return Err(vec![WErrs::brackets_rejected()]);
        }
        if let Some(rest) = s.strip_prefix('@') {
            return Self::p_byteref(rest, encoding);
        }
        Self::p_top(s, encoding)
    }

    /// Emit the inner content: no parens, no format mark, no leading
    /// or trailing whitespace. Caller supplies the wrapping arrow.
    #[must_use]
    pub fn emit(weight: &EdgeWeight) -> String {
        match weight {
            EdgeWeight::None => String::new(),
            EdgeWeight::Vec(vs, e) => match vs.split_first() {
                Some((v, [])) => Self::e_vec_one(*v, *e),
                _ => Self::e_vec_many(vs, *e),
            },
            EdgeWeight::Matrix(rs, e) => match rs.as_slice() {
                [only] => Self::e_vec_many(only, *e),
                _ => Self::e_matrix(rs, *e),
            },
            EdgeWeight::ByteRef(r, _) => Self::e_byteref(r),
            EdgeWeight::OpRef(id, _) => Self::e_opref(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;

    use crate::data::types::BytestreamRef;

    type Enc = NumericEncoding;

    fn p(s: &str, e: Enc) -> EdgeWeight {
        WeightText::parse(s, e)
            .expect("parse should succeed for accept-case fixture")
    }

    fn rt(s: &str, e: Enc, expect: &str) {
        let w = p(s, e);
        assert_eq!(WeightText::emit(&w), expect, "input={s:?}");
    }

    fn rej(s: &str, e: Enc, needle: &str) {
        let r = WeightText::parse(s, e);
        assert!(r.is_err(), "{s:?} should reject");
        if let Err(es) = r {
            assert!(
                es.iter().any(|x| x.found.contains(needle)
                    || x.consider.iter().any(|c| c.contains(needle))),
                "{s:?}: needle {needle:?} not in {es:?}",
            );
        }
    }

    #[test]
    fn scalar_float() {
        let w = p("0.5", Enc::Float);
        assert_eq!(w, EdgeWeight::Vec(smallvec![0.5], Enc::Float));
    }

    #[test]
    fn scalar_int() {
        let w = p("352", Enc::Int);
        assert_eq!(w, EdgeWeight::Vec(smallvec![352.0], Enc::Int));
    }

    #[test]
    fn flat_float_list() {
        let w = p("0.5, 0.875, 0.23", Enc::Float);
        assert_eq!(
            w,
            EdgeWeight::Vec(
                smallvec![0.5, 0.875, 0.23],
                Enc::Float,
            ),
        );
    }

    #[test]
    fn flat_int_list() {
        let w = p("1, 4, 2, 3", Enc::Int);
        assert_eq!(
            w,
            EdgeWeight::Vec(smallvec![1.0, 4.0, 2.0, 3.0], Enc::Int),
        );
    }

    #[test]
    fn snorm_scalar() {
        let w = p("0.5", Enc::Snorm);
        assert_eq!(w, EdgeWeight::Vec(smallvec![0.5], Enc::Snorm));
    }

    #[test]
    fn hex_single_blob() {
        let w = p("FF12AABB", Enc::Hex);
        assert_eq!(
            w,
            EdgeWeight::Vec(
                smallvec![255.0, 18.0, 170.0, 187.0],
                Enc::Hex,
            ),
        );
    }

    #[test]
    fn hex_multi_blob() {
        let w = p("FF12 | AABB", Enc::Hex);
        assert_eq!(
            w,
            EdgeWeight::Matrix(
                vec![
                    smallvec![255.0, 18.0],
                    smallvec![170.0, 187.0],
                ],
                Enc::Hex,
            ),
        );
    }

    #[test]
    fn list_of_lists() {
        let w = p("1, 2, 3 | 4, 5 | 6, 7, 8, 9", Enc::Int);
        assert_eq!(
            w,
            EdgeWeight::Matrix(
                vec![
                    smallvec![1.0, 2.0, 3.0],
                    smallvec![4.0, 5.0],
                    smallvec![6.0, 7.0, 8.0, 9.0],
                ],
                Enc::Int,
            ),
        );
    }

    #[test]
    fn list_of_lists_snorm() {
        let w = p("0.1, 0.2 | 0.3, 0.4", Enc::Snorm);
        assert_eq!(
            w,
            EdgeWeight::Matrix(
                vec![smallvec![0.1, 0.2], smallvec![0.3, 0.4]],
                Enc::Snorm,
            ),
        );
    }

    #[test]
    fn list_of_lists_tight_pipe() {
        let w = p("1,2 |3,4", Enc::Int);
        assert_eq!(
            w,
            EdgeWeight::Matrix(
                vec![smallvec![1.0, 2.0], smallvec![3.0, 4.0]],
                Enc::Int,
            ),
        );
    }

    #[test]
    fn hex_multi_row() {
        let w = p("FF12 | AABB | DEAD", Enc::Hex);
        assert_eq!(
            w,
            EdgeWeight::Matrix(
                vec![
                    smallvec![255.0, 18.0],
                    smallvec![170.0, 187.0],
                    smallvec![222.0, 173.0],
                ],
                Enc::Hex,
            ),
        );
    }

    #[test]
    fn byteref_no_offset() {
        let w = p("@emb_42 ..1024", Enc::Raw);
        assert_eq!(
            w,
            EdgeWeight::ByteRef(
                BytestreamRef {
                    stream: "emb_42".into(),
                    offset: 0,
                    len: 1024,
                },
                Enc::Raw,
            ),
        );
    }

    #[test]
    fn byteref_with_offset() {
        let w = p("@emb_42 +512..1024", Enc::Raw);
        assert_eq!(
            w,
            EdgeWeight::ByteRef(
                BytestreamRef {
                    stream: "emb_42".into(),
                    offset: 512,
                    len: 1024,
                },
                Enc::Raw,
            ),
        );
    }

    #[test]
    fn roundtrip_table() {
        let cases: &[(&str, Enc)] = &[
            ("0.5", Enc::Float),
            ("0.5, 0.875, 0.23", Enc::Float),
            ("1, 2, 3 | 4, 5 | 6, 7, 8, 9", Enc::Int),
            ("0.1, 0.2 | 0.3, 0.4", Enc::Snorm),
            ("FF12AABB", Enc::Hex),
            ("FF12 | AABB | DEAD", Enc::Hex),
            ("@emb_42 ..1024", Enc::Raw),
            ("@emb_42 +512..1024", Enc::Raw),
        ];
        for (text, enc) in cases {
            let parsed = WeightText::parse(text, *enc)
                .expect("table case must parse");
            let emitted = WeightText::emit(&parsed);
            assert_eq!(
                *text, emitted,
                "roundtrip drift for {text:?}",
            );
        }
    }

    #[test]
    fn roundtrip_emits_legacy() {
        rt("0.5", Enc::Float, "0.5");
        rt("0.5, 0.875, 0.23", Enc::Float, "0.5, 0.875, 0.23");
        rt("1, 4, 2, 3", Enc::Int, "1, 4, 2, 3");
        rt("FF12AABB", Enc::Hex, "FF12AABB");
        rt("FF12 | AABB", Enc::Hex, "FF12 | AABB");
        rt("@s7k ..256", Enc::Raw, "@s7k ..256");
        rt("@abc +4..16", Enc::Raw, "@abc +4..16");
    }

    #[test]
    fn rej_legacy_snorm_suffix() {
        rej("0.5:snorm", Enc::Float, "legacy");
    }

    #[test]
    fn rej_brackets_lol() {
        rej("[1,2,3], [4,5]", Enc::Int, "brackets not used");
    }

    #[test]
    fn rej_brackets_flat() {
        rej("[0.5, 0.875]", Enc::Float, "brackets not used");
    }

    #[test]
    fn rej_brackets_len1() {
        rej("[0.5]", Enc::Float, "brackets not used");
    }

    #[test]
    fn rej_brackets_legacy_unorm() {
        rej("[0.5, 0.875]:unorm", Enc::Float, "legacy");
    }

    #[test]
    fn rej_empty_input() {
        rej("", Enc::Float, "empty weight");
    }

    #[test]
    fn rej_empty_row_middle() {
        rej("1,2 | | 3,4", Enc::Int, "empty row");
    }

    #[test]
    fn rej_leading_pipe() {
        rej("| 1,2", Enc::Int, "leading pipe");
    }

    #[test]
    fn rej_trailing_pipe() {
        rej("1,2 |", Enc::Int, "trailing pipe");
    }

    #[test]
    fn rej_mixed_int_float() {
        rej("1, 0.5", Enc::Float, "mixed int and float");
    }

    #[test]
    fn rej_snorm_oor() {
        rej("-1.5", Enc::Snorm, "snorm value");
    }

    #[test]
    fn rej_unorm_oor() {
        rej("1.0001", Enc::Unorm, "unorm value");
    }

    #[test]
    fn rej_hex_odd_nibble() {
        rej("FFF", Enc::Hex, "nibble");
    }

    #[test]
    fn rej_byteref_zero_len() {
        rej("@emb_42 ..0", Enc::Raw, "zero-length");
    }

    #[test]
    fn rej_hex_under_float_enc() {
        rej("FF12, AABB", Enc::Float, "hex");
    }

    // v0.7: bare `@id` is an operator-ref weight (no slice tail).
    #[test]
    fn opref_bare_under_raw() {
        let w = p("@scorer", Enc::Raw);
        assert_eq!(
            w,
            EdgeWeight::OpRef("scorer".into(), Enc::Raw),
        );
    }

    #[test]
    fn opref_distinguished_from_byteref() {
        // Bare `@emb_42` → OpRef.
        let op = p("@emb_42", Enc::Raw);
        assert!(matches!(op, EdgeWeight::OpRef(_, _)));
        // `@emb_42 ..1024` → ByteRef.
        let br = p("@emb_42 ..1024", Enc::Raw);
        assert!(matches!(br, EdgeWeight::ByteRef(_, _)));
    }

    #[test]
    fn opref_carries_format_mark_encoding() {
        // The format mark on the arrow is preserved on OpRef so the
        // runtime knows how to coerce the operator's return value.
        let w = p("@op", Enc::Snorm);
        assert_eq!(w, EdgeWeight::OpRef("op".into(), Enc::Snorm));
    }

    #[test]
    fn opref_roundtrip_emits_bare_at() {
        let w = p("@scorer", Enc::Raw);
        let out = WeightText::emit(&w);
        assert_eq!(out, "@scorer");
        // And under a different encoding still emits no slice tail.
        let w2 = p("@op", Enc::Snorm);
        assert_eq!(WeightText::emit(&w2), "@op");
    }
}
