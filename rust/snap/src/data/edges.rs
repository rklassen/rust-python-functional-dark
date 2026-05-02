use smol_str::SmolStr;

use crate::data::err::{NonEmpty, SemanticErr};
use crate::data::nodes::Nodes;
use crate::data::types::{EdgeIx, NodeId, NodeIx};
use crate::data::weight::EdgeWeight;

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeDef {
    pub family: SmolStr,
    pub src: NodeId,
    pub tgt: NodeId,
    pub weight: EdgeWeight,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub family: SmolStr,
    pub src: NodeIx,
    pub tgt: NodeIx,
    pub weight: EdgeWeight,
}

#[derive(Debug)]
pub struct Edges {
    pub(crate) row_offsets: Box<[EdgeIx]>, // len = nodes.len() + 1
    pub(crate) col_indices: Box<[NodeIx]>, // len = m, parallel-edges allowed
    pub(crate) weights: Box<[EdgeWeight]>, // len = m
    pub(crate) families: Box<[SmolStr]>,   // len = m
}

impl Edges {
    /// Per-edge resolution. Returns one Result per input edge — never bails.
    /// Errors: src or tgt id not in `nodes` -> `SemanticErr`.
    /// Does NOT detect cycles (graph-global, handled at `to_dag`).
    // Returns Vec of Results rather than Self; this is the locked
    // public API of two-stage construction.
    #[allow(clippy::new_ret_no_self)]
    #[must_use] pub fn new(
        nodes: &Nodes,
        defs: Vec<EdgeDef>,
    ) -> Vec<Result<Edge, SemanticErr>> {
        defs.into_iter()
            .map(|d| {
                let src = nodes.id_to_index(&d.src).ok_or_else(|| {
                    SemanticErr::new(
                        format!("unknown source id `{}`", d.src),
                        Some(
                            "edge source must reference a node declared \
                             in `nodes`"
                                .into(),
                        ),
                        NonEmpty::with_tail(
                            format!("declare node `{}` first", d.src),
                            vec![
                                "check spelling".into(),
                                "use a registered semantic handle".into(),
                            ],
                        ),
                    )
                })?;
                let tgt = nodes.id_to_index(&d.tgt).ok_or_else(|| {
                    SemanticErr::new(
                        format!("unknown target id `{}`", d.tgt),
                        Some(
                            "edge target must reference a node declared \
                             in `nodes`"
                                .into(),
                        ),
                        NonEmpty::with_tail(
                            format!("declare node `{}` first", d.tgt),
                            vec![
                                "check spelling".into(),
                                "use a registered semantic handle".into(),
                            ],
                        ),
                    )
                })?;
                Ok(Edge {
                    family: d.family,
                    src,
                    tgt,
                    weight: d.weight,
                })
            })
            .collect()
    }

    #[must_use] pub fn len(&self) -> usize {
        self.col_indices.len()
    }

    #[must_use] pub fn is_empty(&self) -> bool {
        self.col_indices.is_empty()
    }

    /// Targets of all out-edges from `src`, in canonical edge order.
    /// Returns &[] if `src` is out of range. (Single obvious failure
    /// mode; Option would be noise here — bounded by `nodes.len()`.)
    #[must_use] pub fn out_edges(&self, src: NodeIx) -> &[NodeIx] {
        self.csr_slice(src, &self.col_indices)
    }

    /// Weights parallel to `out_edges(src)`, same length and order.
    #[must_use] pub fn out_weights(&self, src: NodeIx) -> &[EdgeWeight] {
        self.csr_slice(src, &self.weights)
    }

    /// Families parallel to `out_edges(src)`, same length and order.
    #[must_use] pub fn out_families(&self, src: NodeIx) -> &[SmolStr] {
        self.csr_slice(src, &self.families)
    }

    /// Slice helper: bound-checks both ends with `.get`. No unchecked
    /// indexing. Returns &[] when `src` is out of range or the slice
    /// would be empty.
    fn csr_slice<'a, T>(
        &'a self,
        src: NodeIx,
        parallel: &'a [T],
    ) -> &'a [T] {
        let i = src as usize;
        let lo = match self.row_offsets.get(i) {
            Some(&v) => v as usize,
            None => return &[],
        };
        let hi = match self.row_offsets.get(i + 1) {
            Some(&v) => v as usize,
            None => return &[],
        };
        parallel.get(lo..hi).unwrap_or(&[])
    }

    /// Internal CSR builder used by `Graph::new` — sort then bucket.
    /// All slice access is bounds-checked; never panics.
    pub(crate) fn from_sorted_edges(
        node_count: usize,
        mut edges: Vec<Edge>,
    ) -> Result<Self, SemanticErr> {
        // Sort by (src, family, tgt) for canonical determinism. Family
        // sort is stable so within-family order is (src, tgt).
        edges.sort_by(|a, b| {
            a.src
                .cmp(&b.src)
                .then_with(|| a.family.cmp(&b.family))
                .then_with(|| a.tgt.cmp(&b.tgt))
        });
        let m = edges.len();

        // Validate every edge.src fits within node_count before bucketing.
        for e in &edges {
            let i = e.src as usize;
            if i >= node_count {
                return Err(SemanticErr::new(
                    format!(
                        "edge source index {i} out of bounds for \
                         {node_count} nodes"
                    ),
                    Some(
                        "internal CSR builder invariant: src must index \
                         into `nodes`"
                            .into(),
                    ),
                    NonEmpty::with_tail(
                        "rebuild the node table to include this src".into(),
                        vec!["report this as an internal bug".into()],
                    ),
                ));
            }
        }

        // row_offsets has length node_count + 1; each `i+1` for
        // i in 0..node_count is therefore valid.
        let mut row_offsets = vec![0u32; node_count + 1];
        for e in &edges {
            let i = e.src as usize;
            let slot = row_offsets
                .get_mut(i + 1)
                .ok_or_else(Self::csr_invariant_violation)?;
            *slot = slot.saturating_add(1);
        }
        for i in 1..row_offsets.len() {
            let prev = row_offsets
                .get(i - 1)
                .copied()
                .ok_or_else(Self::csr_invariant_violation)?;
            let cur = row_offsets
                .get_mut(i)
                .ok_or_else(Self::csr_invariant_violation)?;
            *cur = cur.saturating_add(prev);
        }

        let mut col_indices = vec![0u32; m];
        let mut weights: Vec<EdgeWeight> = Vec::with_capacity(m);
        let mut families: Vec<SmolStr> = Vec::with_capacity(m);
        let mut cursor = vec![0u32; node_count];
        for e in edges {
            let i = e.src as usize;
            let base = row_offsets
                .get(i)
                .copied()
                .ok_or_else(Self::csr_invariant_violation)?;
            let off = cursor
                .get(i)
                .copied()
                .ok_or_else(Self::csr_invariant_violation)?;
            let pos = base.saturating_add(off) as usize;
            let cur = cursor
                .get_mut(i)
                .ok_or_else(Self::csr_invariant_violation)?;
            *cur = cur.saturating_add(1);
            let cell = col_indices
                .get_mut(pos)
                .ok_or_else(Self::csr_invariant_violation)?;
            *cell = e.tgt;
            weights.push(e.weight);
            families.push(e.family);
        }

        Ok(Edges {
            row_offsets: row_offsets.into_boxed_slice(),
            col_indices: col_indices.into_boxed_slice(),
            weights: weights.into_boxed_slice(),
            families: families.into_boxed_slice(),
        })
    }

    fn csr_invariant_violation() -> SemanticErr {
        SemanticErr::new(
            "internal CSR builder index out of bounds".into(),
            Some("CSR builder bookkeeping desynced from input".into()),
            NonEmpty::with_tail(
                "report this as an internal bug".into(),
                vec!["rebuild the input edge list".into()],
            ),
        )
    }
}
