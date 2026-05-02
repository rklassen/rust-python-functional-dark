//! Continuation of `impl Parser` for v0.7 node `weight:` value
//! reading. Split out of `parse_body.rs` to honor the per-file
//! 432-line ceiling. Same `&mut self` cursor exception applies
//! (documented in `parse.rs`).

use crate::codec::weight_text::WeightText;
use crate::data::weight::EdgeWeight;
use crate::io::snap::lex::Tok;
use crate::io::snap::parse::Parser;

impl Parser<'_> {
    /// v0.7 node `weight:` value reader. Two forms:
    /// - parens form: `(...)X` (X optional `s`/`u`/`h`); reuses the
    ///   weighted-arrow body grammar.
    /// - bare scalar: a single Number token. Shorthand for `(N)`.
    ///   Bare `@id` and bare lists require parens like edge weights.
    pub(crate) fn read_node_weight(&mut self) -> Option<EdgeWeight> {
        if matches!(self.cur.value, Tok::LParen) {
            return self.read_node_weight_parens();
        }
        if let Tok::Number(s) = &self.cur.value {
            let body = (*s).to_string();
            self.bump_or_record();
            let enc = Self::infer_encoding(&body);
            return match WeightText::parse(&body, enc) {
                Ok(w) => Some(w),
                Err(es) => {
                    self.errs.extend(es);
                    None
                }
            };
        }
        self.errs.push(self.expected(
            "a node weight: bare number or `(...)` form",
        ));
        None
    }

    fn read_node_weight_parens(&mut self) -> Option<EdgeWeight> {
        let body = match self.lex.take_paren_body() {
            Ok(s) => s.to_string(),
            Err(e) => {
                self.errs.push(e);
                return None;
            }
        };
        self.bump_or_record();
        let mark = Self::format_mark(&self.cur.value);
        if mark.is_some() {
            self.bump_or_record();
        }
        let enc = mark.unwrap_or_else(|| Self::infer_encoding(&body));
        match WeightText::parse(&body, enc) {
            Ok(w) => Some(w),
            Err(es) => {
                self.errs.extend(es);
                None
            }
        }
    }
}
