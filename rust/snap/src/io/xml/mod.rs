//! XML (snap-native dialect, NOT `GraphML`) emit + parse.
//!
//! No xmlns required for now (project not public). `Xml::parse` is lax —
//! absence of xmlns is accepted; presence is not validated.
//!
//! Weight strings round-trip through
//! `crate::codec::weight_text::WeightText` — codec layer is the
//! single source of truth for weight format. The XML carrier never
//! decomposes weights itself.
//!
//! The carrier `Xml` is stateless. Its `parse` / `emit` methods bridge
//! to the per-file modules `emit` and `parse` which exist only to
//! honor the per-file 432-line ceiling. The crate-private free fns
//! `emit::emit` and `parse::parse` are the recognized exception to
//! the "no free fn outside impl" rule for breaking up a single
//! conceptual carrier across multiple files.

mod emit;
mod emit_body;
mod parse;
mod parse_attr;
mod parse_body;

use crate::data::err::SemanticErr;
use crate::data::graph::Graph;

/// Stateless carrier for XML encoding/decoding.
pub struct Xml;

impl Xml {
    /// Infallible canonical XML emit.
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
    fn roundtrip_via_xml() {
        let g1 = Graph::from_snap(FIXTURE)
            .unwrap_or_else(|e| panic!("snap parse: {e:?}"));
        let serialized = g1.to_xml();
        let g2 = Graph::from_xml(&serialized)
            .unwrap_or_else(|e| panic!("xml parse: {e:?}"));
        let snap_again = g2.to_snap();
        assert_eq!(
            FIXTURE, snap_again,
            "round-trip via xml drift",
        );
    }
}
