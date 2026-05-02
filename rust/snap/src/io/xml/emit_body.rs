//! Continuation of `impl XmlEmit` for layout/literals/nodes/registers/
//! streams/types section emitters. Split out of `emit.rs` to honor the
//! per-file 432-line ceiling. This file is named after the `XmlEmit`
//! carrier because it CONTINUES that impl block — file/struct
//! alignment preserved.

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::data::literals::LiteralEntry;
use crate::data::nodes::Nodes;
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::{TypeEntry, TypeRegistry};
use crate::data::types::NodeKind;
use crate::io::xml::emit::XmlEmit;

impl XmlEmit {
    pub(crate) fn w_layout(
        out: &mut String,
        m: &IndexMap<SmolStr, (f64, f64)>,
    ) {
        if m.is_empty() {
            out.push_str("  <layout/>\n");
            return;
        }
        out.push_str("  <layout>\n");
        let mut keys: Vec<&SmolStr> = m.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some((x, y)) = m.get(k) {
                out.push_str("    <pos key=\"");
                out.push_str(&Self::esc_attr(k));
                out.push_str("\" x=\"");
                out.push_str(&Self::render_float(*x));
                out.push_str("\" y=\"");
                out.push_str(&Self::render_float(*y));
                out.push_str("\"/>\n");
            }
        }
        out.push_str("  </layout>\n");
    }

    pub(crate) fn w_literals(
        out: &mut String,
        m: &IndexMap<SmolStr, LiteralEntry>,
    ) {
        if m.is_empty() {
            out.push_str("  <literals/>\n");
            return;
        }
        out.push_str("  <literals>\n");
        let mut keys: Vec<&SmolStr> = m.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = m.get(k) {
                out.push_str("    <literal name=\"");
                out.push_str(&Self::esc_attr(k));
                out.push_str("\" id=\"");
                out.push_str(&Self::esc_attr(&v.id));
                out.push_str("\" type=\"");
                out.push_str(&Self::esc_attr(&v.type_name));
                out.push_str("\">\n");
                Self::w_attr(out, "      ", "value", &v.value);
                out.push_str("    </literal>\n");
            }
        }
        out.push_str("  </literals>\n");
    }

    pub(crate) fn w_nodes(out: &mut String, nodes: &Nodes) {
        if nodes.is_empty() {
            out.push_str("  <nodes/>\n");
            return;
        }
        out.push_str("  <nodes>\n");
        for nref in nodes.iter() {
            let nd = nref.data;
            let kind = Self::kind_str(&nd.kind);
            out.push_str("    <");
            out.push_str(kind);
            out.push_str(" id=\"");
            out.push_str(&Self::esc_attr(&nd.id));
            out.push('"');
            if let Some(name) = &nd.name {
                out.push_str(" name=\"");
                out.push_str(&Self::esc_attr(name));
                out.push('"');
            }
            if nd.attrs.is_empty() {
                out.push_str("/>\n");
            } else {
                out.push_str(">\n");
                let mut ks: Vec<&SmolStr> =
                    nd.attrs.keys().collect();
                ks.sort_unstable();
                for k in ks {
                    if let Some(v) = nd.attrs.get(k) {
                        Self::w_attr(out, "      ", k, v);
                    }
                }
                out.push_str("    </");
                out.push_str(kind);
                out.push_str(">\n");
            }
        }
        out.push_str("  </nodes>\n");
    }

    pub(crate) fn kind_str(k: &NodeKind) -> &str {
        match k {
            NodeKind::File => "file",
            NodeKind::Function => "function",
            NodeKind::Info => "info",
            NodeKind::Object => "object",
            NodeKind::Operator => "operator",
            NodeKind::Property => "property",
            NodeKind::Custom(s) => s.as_str(),
        }
    }

    pub(crate) fn w_registers(
        out: &mut String,
        m: &IndexMap<SmolStr, RegisterEntry>,
    ) {
        if m.is_empty() {
            out.push_str("  <registers/>\n");
            return;
        }
        out.push_str("  <registers>\n");
        let mut keys: Vec<&SmolStr> = m.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = m.get(k) {
                out.push_str("    <register name=\"");
                out.push_str(&Self::esc_attr(k));
                out.push_str("\" id=\"");
                out.push_str(&Self::esc_attr(&v.id));
                out.push_str("\" type=\"");
                out.push_str(&Self::esc_attr(&v.type_name));
                out.push_str("\"/>\n");
            }
        }
        out.push_str("  </registers>\n");
    }

    pub(crate) fn w_streams(
        out: &mut String,
        m: &IndexMap<SmolStr, StreamEntry>,
    ) {
        if m.is_empty() {
            out.push_str("  <streams/>\n");
            return;
        }
        out.push_str("  <streams>\n");
        let mut keys: Vec<&SmolStr> = m.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = m.get(k) {
                out.push_str("    <stream name=\"");
                out.push_str(&Self::esc_attr(k));
                out.push_str("\" id=\"");
                out.push_str(&Self::esc_attr(&v.id));
                out.push_str("\" len=\"");
                out.push_str(&format!("{}", v.data.len()));
                out.push_str("\"/>\n");
            }
        }
        out.push_str("  </streams>\n");
    }

    pub(crate) fn w_types(out: &mut String, t: &TypeRegistry) {
        if t.is_empty() {
            out.push_str("  <types/>\n");
            return;
        }
        out.push_str("  <types>\n");
        for entry in t.entries() {
            match entry {
                TypeEntry::Concrete(n) => {
                    out.push_str("    <type name=\"");
                    out.push_str(&Self::esc_attr(n));
                    out.push_str("\"/>\n");
                }
                TypeEntry::Alias { alias, expr } => {
                    out.push_str("    <alias name=\"");
                    out.push_str(&Self::esc_attr(alias));
                    out.push_str("\" expr=\"");
                    out.push_str(&Self::esc_attr(expr));
                    out.push_str("\"/>\n");
                }
            }
        }
        out.push_str("  </types>\n");
    }
}
