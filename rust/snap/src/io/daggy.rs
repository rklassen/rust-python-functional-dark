//! `daggy::Dag` export with full cycle enumeration.
//!
//! Cycle detection delegated to `data::graph::Graph::cycles` (iterative
//! Tarjan SCC over our CSR). Hand-off to daggy only if acyclic.
//!
//! `NumericEncoding` / `BytestreamRef` preserved by clone — never quantized.

use ::daggy::Dag;

use crate::data::err::{NonEmpty, SemanticErr};
use crate::data::graph::Graph;
use crate::data::nodes::NodeData;
use crate::data::types::NodeIx;
use crate::data::weight::EdgeWeight;

/// Stateless carrier for `daggy::Dag` export.
pub struct Daggy;

impl Daggy {
    /// Convert a snap `Graph` into a `daggy::Dag`.
    ///
    /// Returns `Err(Vec<SemanticErr>)` listing every cycle (every
    /// nontrivial SCC and every self-loop) when the graph is not
    /// acyclic. The error vec is ordered deterministically by lowest
    /// `NodeIx` in each SCC.
    pub fn export(
        g: &Graph,
    ) -> Result<Dag<NodeData, EdgeWeight>, Vec<SemanticErr>> {
        let cycles = g.cycles();
        if !cycles.is_empty() {
            return Err(cycles);
        }

        let nodes = g.nodes();
        let edges = g.edges();
        let mut dag: Dag<NodeData, EdgeWeight> =
            Dag::with_capacity(nodes.len(), edges.len());

        // Stage 1: push all nodes in canonical order; record the daggy
        // NodeIndex for each snap NodeIx.
        let dag_ix: Vec<_> = nodes
            .iter()
            .map(|n| dag.add_node(n.data.clone()))
            .collect();

        // Stage 2: walk CSR; for each (src, tgt) push the edge with its
        // cloned EdgeWeight. Cycles already ruled out, so add_edge
        // cannot fail under daggy's invariant.
        // NodeIx is u32 by design; node count bounded by construction.
        #[allow(clippy::cast_possible_truncation)]
        for src in 0..nodes.len() as NodeIx {
            let tgts = edges.out_edges(src);
            let weights = edges.out_weights(src);
            for (i, &tgt) in tgts.iter().enumerate() {
                let from = match dag_ix.get(src as usize) {
                    Some(&v) => v,
                    None => continue,
                };
                let to = match dag_ix.get(tgt as usize) {
                    Some(&v) => v,
                    None => continue,
                };
                let w = match weights.get(i) {
                    Some(v) => v.clone(),
                    None => EdgeWeight::None,
                };
                // Cycle pre-check guarantees Ok; if daggy disagrees we
                // surface a single SemanticErr rather than panic.
                if dag.add_edge(from, to, w).is_err() {
                    return Err(vec![SemanticErr::new(
                        format!(
                            "daggy rejected edge {src} -> {tgt} after \
                             acyclic pre-check passed",
                        ),
                        Some(
                            "this indicates a Tarjan SCC bug; please \
                             report"
                                .into(),
                        ),
                        NonEmpty::with_tail(
                            "fall back to Petgraph::export".into(),
                            vec![
                                "open issue snap#daggy-disagreement"
                                    .into(),
                                "include the input snap text".into(),
                            ],
                        ),
                    )]);
                }
            }
        }

        Ok(dag)
    }
}
