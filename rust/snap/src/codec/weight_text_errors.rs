//! Error builders for `WeightText`. Split from `weight_text.rs` only
//! to honor the per-file line ceiling. All `pub(super)` so the codec's
//! parent module never re-exports these helpers. Messages refer users
//! to the v0.6 arrow-suffix form `-(value)X->` where X is `s`, `u`, or
//! `h` for snorm/unorm/hex; the legacy `:tag` form is rejected.

use crate::data::err::{NonEmpty, SemanticErr};

pub(super) struct WErrs;

impl WErrs {
    pub(super) fn e(
        f: &str,
        ex: &str,
        c: &[&str],
    ) -> SemanticErr {
        let head = c.first().copied().unwrap_or("").to_string();
        let tail: Vec<String> =
            c.iter().skip(1).map(|s| (*s).to_string()).collect();
        SemanticErr::new(
            f.into(),
            Some(ex.into()),
            NonEmpty::with_tail(head, tail),
        )
    }

    pub(super) fn ef(
        f: String,
        ex: &str,
        c: &[&str],
    ) -> SemanticErr {
        let head = c.first().copied().unwrap_or("").to_string();
        let tail: Vec<String> =
            c.iter().skip(1).map(|s| (*s).to_string()).collect();
        SemanticErr::new(
            f,
            Some(ex.into()),
            NonEmpty::with_tail(head, tail),
        )
    }

    pub(super) fn empty_weight() -> SemanticErr {
        Self::e(
            "empty weight",
            "a non-empty value, list, or @byteref",
            &[
                "supply >=1 token",
                "drop the parens",
                "use scalar 0",
            ],
        )
    }

    pub(super) fn legacy_tag(tag: &str) -> SemanticErr {
        Self::ef(
            format!("legacy `:{tag}` suffix is rejected in v0.6"),
            "format mark on the arrow: `-(value)s->` etc.",
            &[
                "drop `:tag` and use `-(value)s->` for snorm",
                "use `-(value)u->` for unorm",
                "use `-(value)h->` for hex",
            ],
        )
    }

    pub(super) fn brackets_rejected() -> SemanticErr {
        Self::e(
            "brackets not used in v0.6 edge weights",
            "rows are pipe-separated, e.g. `1,2,3 | 4,5`",
            &[
                "drop the brackets: `0.5, 0.875`",
                "for length 1, use the bare value: `0.5`",
                "list-of-lists: `1,2,3 | 4,5 | 6,7,8,9`",
            ],
        )
    }

    pub(super) fn empty_row() -> SemanticErr {
        Self::e(
            "empty row in list-of-lists",
            "non-empty comma list between pipes",
            &[
                "remove the stray `|`",
                "supply >=1 element per row",
            ],
        )
    }

    pub(super) fn leading_pipe() -> SemanticErr {
        Self::e(
            "leading pipe in list-of-lists",
            "first row before any `|` separator",
            &["drop the leading `|`"],
        )
    }

    pub(super) fn trailing_pipe() -> SemanticErr {
        Self::e(
            "trailing pipe in list-of-lists",
            "last row not terminated by `|`",
            &["drop the trailing `|`"],
        )
    }

    pub(super) fn mixed() -> SemanticErr {
        Self::e(
            "mixed int and float in flat list",
            "uniform element type within a flat list",
            &[
                "cast all to float (use `-(...)>` Float arrow)",
                "cast all to int",
                "use `-(value)s->` for snorm normalization",
            ],
        )
    }

    pub(super) fn hex_wrong_enc() -> SemanticErr {
        Self::e(
            "hex tokens with non-hex encoding",
            "the `-(value)h->` arrow for hex blobs",
            &[
                "switch the arrow to `-(...)h->`",
                "use decimal numbers instead",
            ],
        )
    }

    pub(super) fn raw_value_list() -> SemanticErr {
        Self::e(
            "Raw encoding on a value list",
            "a `@byteref` (Raw is only valid for byterefs)",
            &[
                "use `@id ..len` form",
                "switch to a numeric arrow",
            ],
        )
    }

    pub(super) fn bad_number(s: &str) -> SemanticErr {
        Self::ef(
            format!("not a number: {s:?}"),
            "decimal int or float",
            &["check the literal"],
        )
    }

    pub(super) fn snorm_oor(v: f64) -> SemanticErr {
        Self::ef(
            format!("snorm value {v} out of [-1, 1]"),
            "value within [-1, 1] under `-(value)s->`",
            &[
                "rescale into range",
                "switch arrow to plain `-(...)>`",
                "clamp at the source",
            ],
        )
    }

    pub(super) fn unorm_oor(v: f64) -> SemanticErr {
        Self::ef(
            format!("unorm value {v} out of [0, 1]"),
            "value within [0, 1] under `-(value)u->`",
            &[
                "rescale into range",
                "switch arrow to plain `-(...)>`",
                "clamp at the source",
            ],
        )
    }

    pub(super) fn bad_byteref() -> SemanticErr {
        Self::e(
            "malformed bytestream ref",
            "`@id ..len` or `@id +offset..len`",
            &["supply id and slice form"],
        )
    }

    pub(super) fn bad_opref() -> SemanticErr {
        Self::e(
            "malformed operator ref",
            "`@id` (a single bare id, no slice tail)",
            &[
                "drop extra whitespace or punctuation",
                "use `@id ..len` for a bytestream slice instead",
            ],
        )
    }

    pub(super) fn bad_offset(s: &str) -> SemanticErr {
        Self::ef(
            format!("bad offset {s:?}"),
            "non-negative integer",
            &["use a u32 (no sign, no decimals)"],
        )
    }

    pub(super) fn bad_len(s: &str) -> SemanticErr {
        Self::ef(
            format!("bad length {s:?}"),
            "positive integer",
            &["use a u32"],
        )
    }

    pub(super) fn zero_len() -> SemanticErr {
        Self::e(
            "..0 zero-length slice",
            "..len with len >= 1",
            &["use ..1", "omit the edge", "use scalar 0"],
        )
    }
}
