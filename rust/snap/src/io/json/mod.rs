//! JSON emit + parse. Sections as alphabetic-keyed object (`BTreeMap`),
//! edges within a family as ordered array sorted by (src, tgt) on emit.
//!
//! Weight type tags ("snorm"/"unorm"/"hex") preserved verbatim — never
//! quantized to f32 in transit. Emit defers to
//! `crate::codec::weight_text::WeightText` for canonical weight strings;
//! parse delegates back to it. f64 numbers throughout.
//!
//! The carrier `Json` is stateless. Its `parse` / `emit` methods bridge
//! to the per-file modules `emit` and `parse` which exist only to
//! honor the per-file 432-line ceiling. The crate-private free fns
//! `emit::emit` and `parse::parse` are the recognized exception to the
//! "no free fn outside impl" rule for breaking up a single conceptual
//! carrier across multiple files; they are documented at the head of
//! each of those files.

mod emit;
mod parse;
mod parse_body;

use crate::data::err::SemanticErr;
use crate::data::graph::Graph;

/// Stateless carrier for JSON encoding/decoding.
pub struct Json;

impl Json {
    /// Infallible canonical JSON emit.
    #[must_use] pub fn emit(g: &Graph) -> String {
        emit::emit(g)
    }

    /// Multi-error parse: surfaces every problem, doesn't bail on first.
    pub fn parse(input: &str) -> Result<Graph, Vec<SemanticErr>> {
        parse::parse(input)
    }
}

#[cfg(test)]
mod tests {
    use crate::data::graph::Graph;

    const FIXTURE: &str = "\
\u{1FAA2}snap demo
.graph {
 gen: 0,
 id: g001,
 name: 'demo',
 operators: 'op/',
 time: 2026-05-01T00:00:00Z,
 types: None,
 version: 0.6,
 workspace: 'ws/',
}
edges {
 flow {
  a001 -> b002,
  a001 -(0.5)-> b002,
 }
}
extras { }
layout { }
literals { }
nodes {
 object { id: a001, name: 'A', type: T },
 object { id: b002, name: 'B', type: T },
}
registers { }
streams { }
types {
 T,
}
end\u{1FAA2}
";

    #[test]
    fn roundtrip_via_json() {
        let g1 = Graph::from_snap(FIXTURE)
            .unwrap_or_else(|e| panic!("snap parse: {e:?}"));
        let serialized = g1.to_json();
        let g2 = Graph::from_json(&serialized)
            .unwrap_or_else(|e| panic!("json parse: {e:?}"));
        let snap_again = g2.to_snap();
        assert_eq!(
            FIXTURE, snap_again,
            "round-trip via json drift",
        );
    }
}
