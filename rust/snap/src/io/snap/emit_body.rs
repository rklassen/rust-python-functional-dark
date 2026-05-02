//! Continuation of `impl Emitter` for the per-section emitters and
//! attribute-rendering helpers. Split out of `emit.rs` to honor the
//! per-file 432-line ceiling. This file is named after the `Emitter`
//! carrier because it CONTINUES that impl block — file/struct
//! alignment preserved.
//!
//! Same `&mut self` cursor exception applies (documented in
//! `emit.rs`).

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::data::literals::LiteralEntry;
use crate::data::nodes::Nodes;
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::{TypeEntry, TypeRegistry};
use crate::data::types::{AttrValue, NodeKind};
use crate::io::snap::emit::Emitter;

impl Emitter<'_> {
    pub(crate) fn emit_extras(
        &mut self,
        extras: &IndexMap<SmolStr, AttrValue>,
    ) {
        if extras.is_empty() {
            self.out.push_str("extras { }\n");
            return;
        }
        self.out.push_str("extras {\n");
        let mut keys: Vec<&SmolStr> = extras.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = extras.get(k) {
                self.out.push(' ');
                self.out.push_str(k);
                self.out.push_str(": ");
                self.out.push_str(&Self::render_attr(v));
                self.out.push_str(",\n");
            }
        }
        self.out.push_str("}\n");
    }

    pub(crate) fn emit_layout(
        &mut self,
        layout: &IndexMap<SmolStr, (f64, f64)>,
    ) {
        if layout.is_empty() {
            self.out.push_str("layout { }\n");
            return;
        }
        self.out.push_str("layout {\n");
        let mut keys: Vec<&SmolStr> = layout.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some((x, y)) = layout.get(k) {
                self.out.push(' ');
                self.out.push_str(k);
                self.out.push_str(": (");
                self.out.push_str(&Self::render_float(*x));
                self.out.push_str(", ");
                self.out.push_str(&Self::render_float(*y));
                self.out.push_str("),\n");
            }
        }
        self.out.push_str("}\n");
    }

    pub(crate) fn emit_literals(
        &mut self,
        literals: &IndexMap<SmolStr, LiteralEntry>,
    ) {
        if literals.is_empty() {
            self.out.push_str("literals { }\n");
            return;
        }
        self.out.push_str("literals {\n");
        let mut keys: Vec<&SmolStr> = literals.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = literals.get(k) {
                self.out.push(' ');
                self.out.push_str(k);
                self.out.push_str(": { id: ");
                self.out.push_str(&v.id);
                self.out.push_str(", type: ");
                self.out.push_str(&v.type_name);
                self.out.push_str(", value: ");
                self.out.push_str(&Self::render_attr(&v.value));
                self.out.push_str(" },\n");
            }
        }
        self.out.push_str("}\n");
    }

    pub(crate) fn emit_nodes(&mut self, nodes: &Nodes) {
        if nodes.is_empty() {
            self.out.push_str("nodes { }\n");
            return;
        }
        self.out.push_str("nodes {\n");
        for nref in nodes.iter() {
            let nd = nref.data;
            self.out.push(' ');
            self.out.push_str(Self::node_kind_str(&nd.kind));
            self.out.push_str(" { id: ");
            self.out.push_str(&nd.id);
            if let Some(name) = &nd.name {
                self.out.push_str(", name: '");
                self.out.push_str(name);
                self.out.push('\'');
            }
            let mut keys: Vec<&SmolStr> = nd.attrs.keys().collect();
            keys.sort_unstable();
            for k in keys {
                if let Some(v) = nd.attrs.get(k) {
                    self.out.push_str(", ");
                    self.out.push_str(k);
                    self.out.push_str(": ");
                    self.out.push_str(&Self::render_attr(v));
                }
            }
            self.out.push_str(" },\n");
        }
        self.out.push_str("}\n");
    }

    pub(crate) fn emit_registers(
        &mut self,
        regs: &IndexMap<SmolStr, RegisterEntry>,
    ) {
        if regs.is_empty() {
            self.out.push_str("registers { }\n");
            return;
        }
        self.out.push_str("registers {\n");
        let mut keys: Vec<&SmolStr> = regs.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = regs.get(k) {
                self.out.push(' ');
                self.out.push_str(k);
                self.out.push_str(": { id: ");
                self.out.push_str(&v.id);
                self.out.push_str(", type: ");
                self.out.push_str(&v.type_name);
                self.out.push_str(" },\n");
            }
        }
        self.out.push_str("}\n");
    }

    pub(crate) fn emit_streams(
        &mut self,
        streams: &IndexMap<SmolStr, StreamEntry>,
    ) {
        if streams.is_empty() {
            self.out.push_str("streams { }\n");
            return;
        }
        self.out.push_str("streams {\n");
        let mut keys: Vec<&SmolStr> = streams.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = streams.get(k) {
                self.out.push(' ');
                self.out.push_str(k);
                self.out.push_str(": { id: ");
                self.out.push_str(&v.id);
                self.out.push_str(", len: ");
                self.out.push_str(&format!("{}", v.data.len()));
                self.out.push_str(" },\n");
            }
        }
        self.out.push_str("}\n");
    }

    pub(crate) fn emit_types(&mut self, types: &TypeRegistry) {
        if types.is_empty() {
            self.out.push_str("types { }\n");
            return;
        }
        self.out.push_str("types {\n");
        let mut aliases: Vec<(&SmolStr, &SmolStr)> = Vec::new();
        let mut concrete: Vec<&SmolStr> = Vec::new();
        for entry in types.entries() {
            match entry {
                TypeEntry::Alias { alias, expr } => {
                    aliases.push((alias, expr));
                }
                TypeEntry::Concrete(n) => concrete.push(n),
            }
        }
        aliases.sort_by(|a, b| a.0.cmp(b.0));
        concrete.sort_unstable();
        for (alias, expr) in aliases {
            self.out.push_str(" '");
            self.out.push_str(alias);
            self.out.push_str("' -> ");
            self.out.push_str(expr);
            self.out.push_str(",\n");
        }
        for c in concrete {
            self.out.push(' ');
            self.out.push_str(c);
            self.out.push_str(",\n");
        }
        self.out.push_str("}\n");
    }

    fn render_attr(v: &AttrValue) -> String {
        match v {
            AttrValue::None => "None".into(),
            AttrValue::Bool(b) => {
                if *b { "true".into() } else { "false".into() }
            }
            AttrValue::Int(i) => format!("{i}"),
            AttrValue::Float(f) => Self::render_float(*f),
            AttrValue::Str(s) => format!("'{s}'"),
            AttrValue::Ident(s) => s.to_string(),
            AttrValue::DateTime(s) => s.to_string(),
            AttrValue::List(items) => {
                let parts: Vec<String> =
                    items.iter().map(Self::render_attr).collect();
                format!("[{}]", parts.join(", "))
            }
            AttrValue::Dict(d) => Self::render_dict(d),
        }
    }

    fn render_dict(
        d: &IndexMap<SmolStr, AttrValue>,
    ) -> String {
        let mut keys: Vec<&SmolStr> = d.keys().collect();
        keys.sort_unstable();
        let mut s = String::from("{ ");
        let mut first = true;
        for k in keys {
            if let Some(v) = d.get(k) {
                if !first { s.push_str(", "); }
                first = false;
                s.push_str(k);
                s.push_str(": ");
                s.push_str(&Self::render_attr(v));
            }
        }
        s.push_str(" }");
        s
    }

    fn render_float(f: f64) -> String {
        if f.fract() == 0.0 && f.is_finite() {
            format!("{f:.1}")
        } else {
            format!("{f}")
        }
    }

    fn node_kind_str(k: &NodeKind) -> &str {
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
}
