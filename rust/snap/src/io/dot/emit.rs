//! DOT emit. The single free fn `emit` is the documented bridge from
//! `Dot::emit` to the carrier methods — the recognized exception to
//! the no-free-fn rule for breaking up a single conceptual carrier
//! across multiple files.
//!
//! Format outline:
//! ```dot
//! digraph "snap_demo" {
//!   _snap_handle="demo";
//!   _snap_meta_id="g001";
//!   _snap_meta_version="0.6";
//!   _snap_types="T";
//!   n_a001 [_snap_kind="object", _snap_name="A",
//!           _snap_attr_type="@i:T"];
//!   subgraph cluster_flow {
//!     n_a001 -> n_b002 [_snap_family="flow"];
//!     n_a001 -> n_b002 [_snap_family="flow",
//!                       _snap_w="[0.1, 0.5, 0.9]:unorm"];
//!   }
//! }
//! ```
//!
//! `AttrValue` serialization (single-line, parser-trivial):
//!   Ident → "@i:VALUE", Str → "@s:VALUE",
//!   Bool → "@b:true|false", Int → "@n:NUM",
//!   Float → "@f:NUM", `DateTime` → "@d:VALUE",
//!   None → "@_". List/Dict use JSON via `serde_json`
//!   ("@j:..."). All values are quoted in dot per spec.

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::codec::weight_text::WeightText;
use crate::data::edges::Edges;
use crate::data::graph::Graph;
use crate::data::literals::LiteralEntry;
use crate::data::meta::GraphMeta;
use crate::data::nodes::Nodes;
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::{TypeEntry, TypeRegistry};
use crate::data::types::AttrValue;

pub(crate) fn emit(g: &Graph) -> String {
    let mut out = String::with_capacity(256);
    DotEmit::write_graph(&mut out, g);
    out
}

pub(crate) struct DotEmit;

impl DotEmit {
    fn write_graph(out: &mut String, g: &Graph) {
        let label = g.handle().map_or_else(
            || "snap".to_string(),
            |h| format!("snap_{h}"),
        );
        out.push_str("digraph \"");
        out.push_str(&Self::esc(&label));
        out.push_str("\" {\n");
        if let Some(h) = g.handle() {
            Self::w_kv(out, "  ", "_snap_handle", h);
        }
        Self::w_meta(out, g.meta());
        Self::w_types_attr(out, g.types());
        Self::w_extras(out, g.extras());
        Self::w_layout(out, g.layout());
        Self::w_literals(out, g.literals());
        Self::w_registers(out, g.registers());
        Self::w_streams(out, g.streams());
        Self::w_nodes(out, g.nodes());
        Self::w_edges(out, g.edges(), g.nodes());
        out.push_str("}\n");
    }

    fn w_meta(out: &mut String, m: &GraphMeta) {
        Self::w_kv(out, "  ", "_snap_meta_gen", &format!("{}", m.gen));
        Self::w_kv(out, "  ", "_snap_meta_id", &m.id);
        Self::w_kv(out, "  ", "_snap_meta_name", &m.name);
        Self::w_kv(out, "  ", "_snap_meta_operators", &m.operators);
        Self::w_kv(out, "  ", "_snap_meta_time", &m.time);
        match &m.types {
            None => {
                Self::w_kv(out, "  ", "_snap_meta_types", "@_");
            }
            Some(s) => Self::w_kv(out, "  ", "_snap_meta_types", s),
        }
        Self::w_kv(out, "  ", "_snap_meta_version", &m.version);
        Self::w_kv(out, "  ", "_snap_meta_workspace", &m.workspace);
        if let Some(s) = &m.code_path {
            Self::w_kv(out, "  ", "_snap_meta_code_path", s);
        }
        if let Some(s) = &m.data_path {
            Self::w_kv(out, "  ", "_snap_meta_data_path", s);
        }
        if let Some(s) = &m.date {
            Self::w_kv(out, "  ", "_snap_meta_date", s);
        }
    }

    fn w_types_attr(out: &mut String, t: &TypeRegistry) {
        if t.is_empty() {
            return;
        }
        let parts: Vec<String> = t
            .entries()
            .iter()
            .map(|e| match e {
                TypeEntry::Concrete(n) => n.to_string(),
                TypeEntry::Alias { alias, expr } => {
                    format!("'{alias}'->{expr}")
                }
            })
            .collect();
        Self::w_kv(out, "  ", "_snap_types", &parts.join("|"));
    }

    fn w_kv(
        out: &mut String,
        ind: &str,
        key: &str,
        val: &str,
    ) {
        out.push_str(ind);
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(&Self::esc(val));
        out.push_str("\";\n");
    }

    fn w_extras(
        out: &mut String,
        extras: &IndexMap<SmolStr, AttrValue>,
    ) {
        let mut keys: Vec<&SmolStr> = extras.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = extras.get(k) {
                let key = format!("_snap_extra_{k}");
                let val = Self::tag_attr(v);
                Self::w_kv(out, "  ", &key, &val);
            }
        }
    }

    fn w_layout(
        out: &mut String,
        layout: &IndexMap<SmolStr, (f64, f64)>,
    ) {
        let mut keys: Vec<&SmolStr> = layout.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some((x, y)) = layout.get(k) {
                let key = format!("_snap_layout_{k}");
                let val = format!("{x},{y}");
                Self::w_kv(out, "  ", &key, &val);
            }
        }
    }

    fn w_literals(
        out: &mut String,
        m: &IndexMap<SmolStr, LiteralEntry>,
    ) {
        let mut keys: Vec<&SmolStr> = m.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = m.get(k) {
                let key = format!("_snap_literal_{k}");
                let val = format!(
                    "{}|{}|{}",
                    v.id,
                    v.type_name,
                    Self::tag_attr(&v.value),
                );
                Self::w_kv(out, "  ", &key, &val);
            }
        }
    }

    fn w_registers(
        out: &mut String,
        m: &IndexMap<SmolStr, RegisterEntry>,
    ) {
        let mut keys: Vec<&SmolStr> = m.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = m.get(k) {
                let key = format!("_snap_register_{k}");
                let val = format!("{}|{}", v.id, v.type_name);
                Self::w_kv(out, "  ", &key, &val);
            }
        }
    }

    fn w_streams(
        out: &mut String,
        m: &IndexMap<SmolStr, StreamEntry>,
    ) {
        let mut keys: Vec<&SmolStr> = m.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = m.get(k) {
                let key = format!("_snap_stream_{k}");
                let val = format!("{}|{}", v.id, v.data.len());
                Self::w_kv(out, "  ", &key, &val);
            }
        }
    }

    fn w_nodes(out: &mut String, nodes: &Nodes) {
        for nref in nodes.iter() {
            let nd = nref.data;
            out.push_str("  n_");
            out.push_str(&nd.id);
            out.push_str(" [");
            let mut first = true;
            Self::push_pair(
                out,
                &mut first,
                "_snap_kind",
                Self::kind_str(&nd.kind),
            );
            if let Some(name) = &nd.name {
                Self::push_pair(
                    out, &mut first, "_snap_name", name,
                );
            }
            let mut ks: Vec<&SmolStr> = nd.attrs.keys().collect();
            ks.sort_unstable();
            for k in ks {
                if let Some(v) = nd.attrs.get(k) {
                    let key = format!("_snap_attr_{k}");
                    let val = Self::tag_attr(v);
                    Self::push_pair(out, &mut first, &key, &val);
                }
            }
            out.push_str("];\n");
        }
    }

    fn push_pair(
        out: &mut String,
        first: &mut bool,
        key: &str,
        val: &str,
    ) {
        if !*first {
            out.push_str(", ");
        }
        *first = false;
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(&Self::esc(val));
        out.push('"');
    }

    fn w_edges(out: &mut String, edges: &Edges, nodes: &Nodes) {
        let groups = Self::group_edges(edges, nodes);
        let mut fams: Vec<&SmolStr> = groups.keys().collect();
        fams.sort_unstable();
        for fam in fams {
            out.push_str("  subgraph cluster_");
            out.push_str(fam);
            out.push_str(" {\n");
            let rows = match groups.get(fam) {
                Some(r) => r,
                None => continue,
            };
            for (src, tgt, w) in rows {
                out.push_str("    n_");
                out.push_str(src);
                out.push_str(" -> n_");
                out.push_str(tgt);
                out.push_str(" [_snap_family=\"");
                out.push_str(&Self::esc(fam));
                out.push('"');
                if !w.is_none() {
                    out.push_str(", _snap_w=\"");
                    out.push_str(&Self::esc(&WeightText::emit(w)));
                    out.push('"');
                }
                out.push_str("];\n");
            }
            out.push_str("  }\n");
        }
    }

    /// DOT string escape: backslash and double-quote.
    pub(crate) fn esc(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                _ => out.push(c),
            }
        }
        out
    }
}
