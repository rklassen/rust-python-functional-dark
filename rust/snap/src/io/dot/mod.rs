//! DOT (graphviz) emit + parse.
//!
//! Edge families map to `subgraph cluster_FAMILY` AND a redundant
//! `_snap_family="FAMILY"` attribute. The attribute is authoritative
//! on parse; the cluster is layout-only.
//!
//! Numerical edge weights are encoded as a stringly-typed `_snap_w`
//! attribute, e.g. `_snap_w="[0.1,0.2]:snorm"`. The codec layer
//! (`crate::codec::weight_text::WeightText`) is the single source
//! of truth for that string format — both emit and parse delegate.
//!
//! Node IDs are prefixed with `n_` to avoid graphviz keyword
//! collisions (e.g. `node`, `graph`, `edge`).
//!
//! Snap-only metadata (handle, meta, types, kind, attrs) rides as
//! `_snap_<...>` attributes on the appropriate scope.
//!
//! The carrier `Dot` is stateless. Its `parse` / `emit` methods bridge
//! to the per-file modules `emit` and `parse` which exist only to
//! honor the per-file 432-line ceiling. The crate-private free fns
//! `emit::emit` and `parse::parse` are the recognized exception to
//! the "no free fn outside impl" rule for breaking up a single
//! conceptual carrier across multiple files.

mod emit;
mod emit_body;
mod parse;
mod parse_body;

use crate::data::err::SemanticErr;
use crate::data::graph::Graph;

/// Stateless carrier for DOT encoding/decoding.
pub struct Dot;

impl Dot {
    /// Infallible canonical DOT emit.
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
    fn roundtrip_via_dot() {
        let g1 = Graph::from_snap(FIXTURE)
            .unwrap_or_else(|e| panic!("snap parse: {e:?}"));
        let serialized = g1.to_dot();
        let g2 = Graph::from_dot(&serialized)
            .unwrap_or_else(|e| panic!("dot parse: {e:?}"));
        let snap_again = g2.to_snap();
        assert_eq!(
            FIXTURE, snap_again,
            "round-trip via dot drift",
        );
    }
}
