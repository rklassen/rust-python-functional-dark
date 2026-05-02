//! `Snap::emit` entry + emitter carrier + header + `.graph` + edges
//! emitters. Other section emitters live in `emit_body` as a
//! continuation `impl Emitter` block.
//!
//! The single free fn `emit` is the documented bridge from the public
//! `Snap::emit` method into the emitter carrier — the recognized
//! exception to the no-free-fn rule for breaking up a single
//! conceptual carrier across multiple files.
//!
//! `Emitter` uses `&mut self` on its methods. This is the explicit
//! cursor exception: a writer IS a cursor; mutation is its semantics.
//! The doctrine forbids `&mut self` only on the public API of
//! immutable data types. `Emitter` is private to this module and
//! never escapes.
//!
//! Canonical formatting rules baked in here:
//! - 1-space indent
//! - single-quoted strings
//! - terminal commas on every list element
//! - alphabetic-by-key sort within sections
//! - edges sorted by (src, tgt) within each family
//! - empty section: `name { }` (one space between braces)
//! - `.graph` first; rest alphabetic
//! - magic open: `🪢snap [handle]\n`
//! - trailer: `end🪢\n`

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::codec::weight_text::WeightText;
use crate::data::edges::Edges;
use crate::data::graph::Graph;
use crate::data::meta::GraphMeta;
use crate::data::nodes::Nodes;
use crate::data::types::NumericEncoding;
use crate::data::weight::EdgeWeight;

/// Crate-private entry: bridge from `Snap::emit` to the carrier.
pub(crate) fn emit(g: &Graph) -> String {
    let mut out = String::with_capacity(Emitter::estimate(g));
    let mut e = Emitter { out: &mut out };
    e.emit_graph(g);
    out
}

pub(crate) struct Emitter<'a> {
    pub(crate) out: &'a mut String,
}

impl Emitter<'_> {
    pub(crate) fn estimate(g: &Graph) -> usize {
        128 + g.nodes().len() * 64 + g.edges().len() * 48
    }

    pub(crate) fn emit_graph(&mut self, g: &Graph) {
        self.out.push_str("\u{1FAA2}snap");
        if let Some(h) = g.handle() {
            self.out.push(' ');
            self.out.push_str(h);
        }
        self.out.push('\n');

        self.emit_dot_graph(g.meta());
        self.emit_edges(g.edges(), g.nodes());
        self.emit_extras(g.extras());
        self.emit_layout(g.layout());
        self.emit_literals(g.literals());
        self.emit_nodes(g.nodes());
        self.emit_registers(g.registers());
        self.emit_streams(g.streams());
        self.emit_types(g.types());

        self.out.push_str("end\u{1FAA2}\n");
    }

    fn emit_dot_graph(&mut self, m: &GraphMeta) {
        self.out.push_str(".graph {\n");
        // gen is a u64 generation counter; representable in i64
        // for our scale (well below 2^63).
        #[allow(clippy::cast_possible_wrap)]
        self.write_kv_int(" gen", m.gen as i64);
        self.write_kv_ident(" id", &m.id);
        self.write_kv_str(" name", &m.name);
        self.write_kv_str(" operators", &m.operators);
        self.write_kv_dt(" time", &m.time);
        match &m.types {
            None => self.write_kv_none(" types"),
            Some(s) => self.write_kv_str(" types", s),
        }
        self.write_kv_version(" version", &m.version);
        self.write_kv_str(" workspace", &m.workspace);
        if let Some(s) = &m.code_path {
            self.write_kv_str(" code_path", s);
        }
        if let Some(s) = &m.data_path {
            self.write_kv_str(" data_path", s);
        }
        if let Some(s) = &m.date {
            self.write_kv_str(" date", s);
        }
        self.out.push_str("}\n");
    }

    fn write_kv_int(&mut self, k: &str, v: i64) {
        self.out.push_str(k);
        self.out.push_str(": ");
        self.out.push_str(&format!("{v}"));
        self.out.push_str(",\n");
    }

    fn write_kv_ident(&mut self, k: &str, v: &SmolStr) {
        self.out.push_str(k);
        self.out.push_str(": ");
        self.out.push_str(v);
        self.out.push_str(",\n");
    }

    pub(crate) fn write_kv_str(&mut self, k: &str, v: &SmolStr) {
        self.out.push_str(k);
        self.out.push_str(": '");
        self.out.push_str(v);
        self.out.push_str("',\n");
    }

    fn write_kv_dt(&mut self, k: &str, v: &SmolStr) {
        self.out.push_str(k);
        self.out.push_str(": ");
        self.out.push_str(v);
        self.out.push_str(",\n");
    }

    fn write_kv_none(&mut self, k: &str) {
        self.out.push_str(k);
        self.out.push_str(": None,\n");
    }

    fn write_kv_version(&mut self, k: &str, v: &SmolStr) {
        self.out.push_str(k);
        self.out.push_str(": ");
        self.out.push_str(v);
        self.out.push_str(",\n");
    }

    fn emit_edges(&mut self, edges: &Edges, nodes: &Nodes) {
        if edges.is_empty() {
            self.out.push_str("edges { }\n");
            return;
        }
        self.out.push_str("edges {\n");
        let groups = Self::group_edges_by_family(edges, nodes);
        let mut families: Vec<&SmolStr> = groups.keys().collect();
        families.sort_unstable();
        for fam in families {
            self.out.push(' ');
            self.out.push_str(fam);
            self.out.push_str(" {\n");
            let rows = match groups.get(fam) {
                Some(r) => r,
                None => continue,
            };
            for (src, tgt, w) in rows {
                self.out.push_str("  ");
                self.out.push_str(src);
                if matches!(w, EdgeWeight::None) {
                    self.out.push_str(" -> ");
                } else {
                    self.out.push_str(" -(");
                    self.out.push_str(&WeightText::emit(w));
                    self.out.push(')');
                    self.out.push_str(Self::format_mark(w));
                    self.out.push_str("-> ");
                }
                self.out.push_str(tgt);
                self.out.push_str(",\n");
            }
            self.out.push_str(" }\n");
        }
        self.out.push_str("}\n");
    }

    fn format_mark(w: &EdgeWeight) -> &'static str {
        match w.encoding() {
            Some(NumericEncoding::Snorm) => "s",
            Some(NumericEncoding::Unorm) => "u",
            Some(NumericEncoding::Hex) => "h",
            _ => "",
        }
    }

    // tgt_ix / tgt_id naming is the natural pair (index + id);
    // similar_names lint flags them but they read clearly.
    #[allow(clippy::similar_names)]
    fn group_edges_by_family(
        edges: &Edges,
        nodes: &Nodes,
    ) -> IndexMap<SmolStr, Vec<(SmolStr, SmolStr, EdgeWeight)>> {
        let mut groups: IndexMap<
            SmolStr,
            Vec<(SmolStr, SmolStr, EdgeWeight)>,
        > = IndexMap::new();
        let n = nodes.len();
        for src_ix in 0..n {
            let src_u = match u32::try_from(src_ix) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let outs = edges.out_edges(src_u);
            let ws = edges.out_weights(src_u);
            let fams = edges.out_families(src_u);
            let src_id = match nodes.iter().nth(src_ix) {
                Some(n) => n.data.id.clone(),
                None => continue,
            };
            for i in 0..outs.len() {
                let tgt_ix = match outs.get(i).copied() {
                    Some(v) => v as usize,
                    None => continue,
                };
                let tgt_id = match nodes.iter().nth(tgt_ix) {
                    Some(n) => n.data.id.clone(),
                    None => continue,
                };
                let fam = match fams.get(i) {
                    Some(f) => f.clone(),
                    None => continue,
                };
                let w = match ws.get(i) {
                    Some(w) => w.clone(),
                    None => continue,
                };
                groups
                    .entry(fam)
                    .or_default()
                    .push((src_id.clone(), tgt_id, w));
            }
        }
        for rows in groups.values_mut() {
            rows.sort_by(|a, b| {
                a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1))
            });
        }
        groups
    }
}
