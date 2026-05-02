//! snap text format io. Parser + canonical emitter.
//!
//! Round-trip property: snap text -> Graph -> snap text is byte-identical
//! when the input is in canonical form. The canary test guards this.
//!
//! The carrier `Snap` is stateless. Its `parse` / `emit` methods bridge
//! to the per-file modules `lex`, `parse`, `emit` which exist only to
//! honor the per-file 432-line ceiling. The crate-private free fns
//! `parse::parse` and `emit::emit` are the recognized exception to the
//! "no free fn outside impl" rule for breaking up a single conceptual
//! carrier across multiple files; they are documented at the head of
//! each of those files.

mod lex;
mod lex_errs;
mod parse;
mod parse_body;
mod parse_weight;
mod emit;
mod emit_body;

use crate::data::err::SemanticErr;
use crate::data::graph::Graph;

/// Stateless carrier for snap text encoding/decoding.
pub struct Snap;

impl Snap {
    /// Parse snap text into a Graph. Multi-error: surfaces every
    /// problem rather than bailing on the first.
    pub fn parse(input: &str) -> Result<Graph, Vec<SemanticErr>> {
        parse::parse(input)
    }

    /// Emit canonical snap text. Infallible: every Graph has a defined
    /// canonical form.
    #[must_use] pub fn emit(g: &Graph) -> String {
        emit::emit(g)
    }
}
