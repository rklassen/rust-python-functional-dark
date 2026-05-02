//! Continuation of `impl Graph` for cycle enumeration. Iterative
//! Tarjan SCC over the CSR. Method-on-struct per Power-Six S2.
//!
//! This file is named after `Graph` because it CONTINUES the `impl Graph`
//! block — the file/struct alignment rule is preserved by treating these
//! as part of the same logical unit, split only to honor the per-file
//! line ceiling.

use crate::data::err::{NonEmpty, SemanticErr};
use crate::data::graph::Graph;
use crate::data::types::NodeIx;

impl Graph {
    /// Iterative Tarjan SCC over our CSR. Returns one `SemanticErr` per
    /// cycle (nontrivial SCC OR self-loop). Order is deterministic: by
    /// lowest `NodeIx` in the SCC.
    ///
    /// Why iterative? `snap` is a serialization format that can be very
    /// wide; recursive Tarjan blows stacks at ~10k nodes. We use two
    /// `Vec`s (work stack + result stack) plus per-node arrays for
    /// `index`, `lowlink`, `on_stack`. All array access goes through
    /// `.get` / `.get_mut` to honor the no-panic rule.
    // Tarjan's SCC requires this many lines; const moved here from
    // mid-fn to satisfy items_after_statements.
    #[allow(clippy::too_many_lines, clippy::items_after_statements)]
    #[must_use] pub fn cycles(&self) -> Vec<SemanticErr> {
        let nodes = self.nodes();
        let edges = self.edges();
        let n = nodes.len();

        if n == 0 {
            return Vec::new();
        }

        // Sentinel: u32::MAX means "unvisited".
        const UNVISITED: u32 = u32::MAX;

        let mut index_of: Vec<u32> = vec![UNVISITED; n];
        let mut lowlink: Vec<u32> = vec![0; n];
        let mut on_stack: Vec<bool> = vec![false; n];
        let mut tarjan_stack: Vec<NodeIx> = Vec::new();

        let mut next_index: u32 = 0;
        // Work stack entry: (current node, next child cursor).
        let mut work: Vec<(NodeIx, usize)> = Vec::new();

        let mut sccs: Vec<Vec<NodeIx>> = Vec::new();
        let mut self_loops: Vec<NodeIx> = Vec::new();

        for start_usize in 0..n {
            let start: NodeIx = match u32::try_from(start_usize) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match index_of.get(start_usize) {
                Some(&v) if v != UNVISITED => continue,
                _ => {}
            }

            if let Some(slot) = index_of.get_mut(start_usize) {
                *slot = next_index;
            }
            if let Some(slot) = lowlink.get_mut(start_usize) {
                *slot = next_index;
            }
            next_index = next_index.saturating_add(1);
            if let Some(slot) = on_stack.get_mut(start_usize) {
                *slot = true;
            }
            tarjan_stack.push(start);
            work.push((start, 0));

            while let Some(&(v, cursor)) = work.last() {
                let v_usize = v as usize;
                let outs = edges.out_edges(v);

                if let Some(w_ix) = outs.get(cursor).copied() {
                    if let Some(top) = work.last_mut() {
                        top.1 = cursor + 1;
                    }

                    if w_ix == v {
                        self_loops.push(v);
                        continue;
                    }

                    let w_usize = w_ix as usize;
                    let w_index = match index_of.get(w_usize) {
                        Some(&x) => x,
                        None => continue,
                    };

                    if w_index == UNVISITED {
                        if let Some(slot) = index_of.get_mut(w_usize) {
                            *slot = next_index;
                        }
                        if let Some(slot) = lowlink.get_mut(w_usize) {
                            *slot = next_index;
                        }
                        next_index = next_index.saturating_add(1);
                        if let Some(slot) = on_stack.get_mut(w_usize) {
                            *slot = true;
                        }
                        tarjan_stack.push(w_ix);
                        work.push((w_ix, 0));
                    } else {
                        let w_on_stack = matches!(
                            on_stack.get(w_usize),
                            Some(&true)
                        );
                        if w_on_stack {
                            let v_low = match lowlink.get(v_usize) {
                                Some(&x) => x,
                                None => continue,
                            };
                            if w_index < v_low {
                                if let Some(slot) =
                                    lowlink.get_mut(v_usize)
                                {
                                    *slot = w_index;
                                }
                            }
                        }
                    }
                } else {
                    work.pop();

                    let v_low = match lowlink.get(v_usize) {
                        Some(&x) => x,
                        None => continue,
                    };
                    let v_index = match index_of.get(v_usize) {
                        Some(&x) => x,
                        None => continue,
                    };

                    if let Some(&(parent, _)) = work.last() {
                        let p_usize = parent as usize;
                        let p_low = match lowlink.get(p_usize) {
                            Some(&x) => x,
                            None => continue,
                        };
                        if v_low < p_low {
                            if let Some(slot) =
                                lowlink.get_mut(p_usize)
                            {
                                *slot = v_low;
                            }
                        }
                    }

                    if v_low == v_index {
                        let mut component: Vec<NodeIx> = Vec::new();
                        // Two-condition break (empty stack OR popped==v);
                        // not a clean while-let.
                        #[allow(clippy::while_let_loop)]
                        loop {
                            let popped = match tarjan_stack.pop() {
                                Some(x) => x,
                                None => break,
                            };
                            let popped_usize = popped as usize;
                            if let Some(slot) =
                                on_stack.get_mut(popped_usize)
                            {
                                *slot = false;
                            }
                            component.push(popped);
                            if popped == v {
                                break;
                            }
                        }
                        if component.len() > 1 {
                            sccs.push(component);
                        }
                    }
                }
            }
        }

        self_loops.sort_unstable();
        self_loops.dedup();

        for scc in &mut sccs {
            scc.sort_unstable();
        }
        sccs.sort_by_key(|s| s.first().copied().unwrap_or(u32::MAX));

        let mut errs: Vec<SemanticErr> =
            Vec::with_capacity(sccs.len() + self_loops.len());

        for scc in sccs {
            errs.push(self.scc_to_err(&scc));
        }
        for v in self_loops {
            errs.push(self.self_loop_to_err(v));
        }

        errs
    }

    /// Render a nontrivial SCC into a `SemanticErr`.
    fn scc_to_err(&self, scc: &[NodeIx]) -> SemanticErr {
        let mut path = String::from("cycle: ");
        let mut first_id: Option<String> = None;
        for (i, ix) in scc.iter().enumerate() {
            let id_str = self.render_node_id(*ix);
            if i == 0 {
                first_id = Some(id_str.clone());
            }
            if i > 0 {
                path.push_str(" -> ");
            }
            path.push_str(&id_str);
        }
        if let Some(first) = first_id {
            path.push_str(" -> ");
            path.push_str(&first);
        }

        SemanticErr::new(
            path,
            Some(String::from("acyclic edge set for daggy::Dag")),
            Self::consider_options(),
        )
    }

    /// Render a self-loop into a `SemanticErr`.
    fn self_loop_to_err(&self, v: NodeIx) -> SemanticErr {
        let id_str = self.render_node_id(v);
        let found =
            format!("cycle: {id_str} -> {id_str} (self-loop)");
        SemanticErr::new(
            found,
            Some(String::from("acyclic edge set for daggy::Dag")),
            Self::consider_options(),
        )
    }

    /// Build the standard `consider` list. Always non-empty.
    fn consider_options() -> NonEmpty<String> {
        NonEmpty::with_tail(
            String::from("drop one edge in the cycle"),
            vec![
                String::from(
                    "use Graph::to_petgraph (cycle-tolerant) instead",
                ),
                String::from("promote the family to a feedback family"),
            ],
        )
    }

    /// Render a `NodeIx` into a human-readable id. Falls back to the
    /// numeric index when the node lookup fails (cannot panic).
    fn render_node_id(&self, ix: NodeIx) -> String {
        let i = ix as usize;
        match self.nodes().iter().nth(i) {
            Some(nref) => format!("{}", nref.data.id),
            None => format!("#{ix}"),
        }
    }
}
