//! `petgraph::DiGraph` export. `AoS` rebuild from our CSR; O(n+m).
//!
//! Cycle detection NOT performed here — petgraph allows cycles. For the
//! daggy adapter (which requires acyclicity) cycle detection lives in
//! `data::graph::Graph::cycles`.
//!
//! `NumericEncoding` / `BytestreamRef` preserved by clone — never quantized.

use ::petgraph::graph::DiGraph;

use crate::data::graph::Graph;
use crate::data::nodes::NodeData;
use crate::data::types::NodeIx;
use crate::data::weight::EdgeWeight;

/// Stateless carrier for `petgraph::DiGraph` export.
pub struct Petgraph;

impl Petgraph {
    /// Build a petgraph `DiGraph` from a snap `Graph`.
    ///
    /// Walks the CSR row-by-row and re-issues each edge into petgraph's
    /// adjacency-of-structs storage. Order of `add_edge` calls is
    /// deterministic: ascending source `NodeIx`, then ascending column
    /// index within the row.
    ///
    /// # Why no `Result`?
    /// We never fabricate or drop information; we only clone what is
    /// already well-typed. A petgraph build cannot fail short of OOM,
    /// which Rust surfaces via allocator panics outside our control.
    #[must_use] pub fn export(g: &Graph) -> DiGraph<NodeData, EdgeWeight> {
        let nodes = g.nodes();
        let edges = g.edges();
        let mut out: DiGraph<NodeData, EdgeWeight> =
            DiGraph::with_capacity(nodes.len(), edges.len());
        let pg_ix: Vec<_> = nodes
            .iter()
            .map(|n| out.add_node(n.data.clone()))
            .collect();
        // NodeIx is u32 by design; node count is bounded by the
        // construction API (nodes.len() <= u32::MAX is invariant).
        #[allow(clippy::cast_possible_truncation)]
        for src in 0..nodes.len() as NodeIx {
            let tgts = edges.out_edges(src);
            let weights = edges.out_weights(src);
            for (i, &tgt) in tgts.iter().enumerate() {
                let from = match pg_ix.get(src as usize) {
                    Some(&v) => v,
                    None => continue,
                };
                let to = match pg_ix.get(tgt as usize) {
                    Some(&v) => v,
                    None => continue,
                };
                let w = match weights.get(i) {
                    Some(v) => v.clone(),
                    None => EdgeWeight::None,
                };
                out.add_edge(from, to, w);
            }
        }
        out
    }
}
