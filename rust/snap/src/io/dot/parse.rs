//! DOT parse entry + cursor + statement dispatch + assemble. Helpers
//! for `AttrValue` tag decoding, weight parsing, and small lexer
//! primitives live in `parse_body` as a continuation `impl DotReader`
//! block to honor the per-file 432-line ceiling.
//!
//! The single free fn `parse` is the documented bridge from
//! `Dot::parse` into the carrier — the recognized exception to the
//! no-free-fn rule.
//!
//! `DotReader` uses `&mut self` on its methods. This is the explicit
//! cursor exception: a reader IS a cursor; mutation is its
//! semantics. `DotReader` is private to this module and never escapes,
//! same pattern as `io/snap/lex.rs::Lexer`.
//!
//! Weight strings round-trip through
//! `crate::codec::weight_text::WeightText::parse` — codec layer is
//! the single source of truth.

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::data::edges::EdgeDef;
use crate::data::err::{NonEmpty, SemanticErr};
use crate::data::graph::Graph;
use crate::data::literals::LiteralEntry;
use crate::data::nodes::{NodeDef, Nodes};
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::TypeRegistry;
use crate::data::types::{AttrValue, NodeKind};

pub(crate) fn parse(input: &str) -> Result<Graph, Vec<SemanticErr>> {
    let mut r = DotReader::new(input);
    r.parse_root()
}

#[derive(Default)]
pub(crate) struct DotState {
    pub(crate) handle: Option<SmolStr>,
    pub(crate) meta_kv: IndexMap<SmolStr, AttrValue>,
    pub(crate) types_attr: Option<SmolStr>,
    pub(crate) nodes: Vec<NodeDef>,
    pub(crate) edges: Vec<EdgeDef>,
    pub(crate) extras: IndexMap<SmolStr, AttrValue>,
    pub(crate) layout: IndexMap<SmolStr, (f64, f64)>,
    pub(crate) literals: IndexMap<SmolStr, LiteralEntry>,
    pub(crate) registers: IndexMap<SmolStr, RegisterEntry>,
    pub(crate) streams: IndexMap<SmolStr, StreamEntry>,
}

pub(crate) struct DotReader<'a> {
    pub(crate) src: &'a str,
    pub(crate) pos: usize,
    pub(crate) errs: Vec<SemanticErr>,
}

impl<'a> DotReader<'a> {
    pub(crate) fn new(src: &'a str) -> Self {
        Self { src, pos: 0, errs: Vec::new() }
    }

    pub(crate) fn parse_root(
        &mut self,
    ) -> Result<Graph, Vec<SemanticErr>> {
        self.skip_ws();
        if !self.eat_kw("digraph") {
            return Err(vec![Self::e(
                "missing `digraph` keyword",
                "digraph \"name\" { ... }",
            )]);
        }
        self.skip_ws();
        // Optional graph name (string or id) — discard.
        if self.peek() == Some('"') {
            let _ = self.read_string();
        } else if let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                let _ = self.read_ident();
            }
        }
        self.skip_ws();
        if !self.eat_char('{') {
            return Err(vec![Self::e(
                "missing `{` after digraph header",
                "{",
            )]);
        }
        let mut st = DotState::default();
        self.parse_body_block(&mut st);
        if !self.errs.is_empty() {
            return Err(std::mem::take(&mut self.errs));
        }
        self.assemble(st)
    }

    fn parse_body_block(&mut self, st: &mut DotState) {
        loop {
            self.skip_ws();
            match self.peek() {
                Some('}') => {
                    let _ = self.eat_char('}');
                    return;
                }
                None => return,
                _ => {}
            }
            self.parse_stmt(st);
        }
    }

    fn parse_stmt(&mut self, st: &mut DotState) {
        if self.eat_kw("subgraph") {
            self.parse_subgraph(st);
            return;
        }
        // It's either a node-stmt, edge-stmt, or graph attr.
        // Read a token: identifier OR a string-quoted form.
        let head = self.read_ident_or_str();
        if head.is_empty() {
            // Skip stray punctuation.
            self.bump();
            return;
        }
        self.skip_ws();
        match self.peek() {
            Some('=') => {
                // graph-level k=v;
                let _ = self.eat_char('=');
                self.skip_ws();
                let val = self.read_value_string();
                self.skip_ws();
                let _ = self.eat_char(';');
                self.handle_top_attr(st, &head, &val);
            }
            Some('-') => {
                // edge: head -> tgt [...]
                let _ = self.eat_char('-');
                let _ = self.eat_char('>');
                self.skip_ws();
                let tgt = self.read_ident_or_str();
                let attrs = self.read_attr_list();
                self.skip_ws();
                let _ = self.eat_char(';');
                self.handle_edge(st, &head, &tgt, &attrs);
            }
            Some('[') => {
                // node-stmt
                let attrs = self.read_attr_list();
                self.skip_ws();
                let _ = self.eat_char(';');
                self.handle_node(st, &head, &attrs);
            }
            Some(';') => {
                let _ = self.eat_char(';');
            }
            _ => {
                // Skip unknown — be lenient.
            }
        }
    }

    fn parse_subgraph(&mut self, st: &mut DotState) {
        self.skip_ws();
        // Optional name (we don't use it for parsing — _snap_family
        // attribute on each edge is authoritative).
        if let Some(c) = self.peek() {
            if c != '{' {
                let _ = self.read_ident_or_str();
            }
        }
        self.skip_ws();
        if !self.eat_char('{') {
            return;
        }
        self.parse_body_block(st);
    }

    // Cursor method: kept &mut self for uniform dispatch with sibling
    // handlers that mutate the cursor.
    #[allow(clippy::unused_self)]
    fn handle_top_attr(
        &mut self,
        st: &mut DotState,
        key: &str,
        val: &str,
    ) {
        if key == "_snap_handle" {
            st.handle = Some(SmolStr::new(val));
            return;
        }
        if let Some(rest) = key.strip_prefix("_snap_meta_") {
            st.meta_kv.insert(
                SmolStr::new(rest),
                AttrValue::Str(SmolStr::new(val)),
            );
            return;
        }
        if key == "_snap_types" {
            st.types_attr = Some(SmolStr::new(val));
            return;
        }
        if let Some(rest) = key.strip_prefix("_snap_extra_") {
            let v = Self::untag_attr(val);
            st.extras.insert(SmolStr::new(rest), v);
            return;
        }
        if let Some(rest) = key.strip_prefix("_snap_layout_") {
            if let Some((xs, ys)) = val.split_once(',') {
                let x = xs.trim().parse::<f64>().unwrap_or(0.0);
                let y = ys.trim().parse::<f64>().unwrap_or(0.0);
                st.layout.insert(SmolStr::new(rest), (x, y));
            }
            return;
        }
        if let Some(rest) = key.strip_prefix("_snap_literal_") {
            let parts: Vec<&str> = val.splitn(3, '|').collect();
            if let [id, ty, vs] = parts.as_slice() {
                let v = Self::untag_attr(vs);
                st.literals.insert(
                    SmolStr::new(rest),
                    LiteralEntry::new(
                        SmolStr::new(rest),
                        SmolStr::new(*id),
                        SmolStr::new(*ty),
                        v,
                    ),
                );
            }
            return;
        }
        if let Some(rest) = key.strip_prefix("_snap_register_") {
            let parts: Vec<&str> = val.splitn(2, '|').collect();
            if let [id, ty] = parts.as_slice() {
                st.registers.insert(
                    SmolStr::new(rest),
                    RegisterEntry::new(
                        SmolStr::new(rest),
                        SmolStr::new(*id),
                        SmolStr::new(*ty),
                    ),
                );
            }
            return;
        }
        if let Some(rest) = key.strip_prefix("_snap_stream_") {
            let parts: Vec<&str> = val.splitn(2, '|').collect();
            if let [id, lens] = parts.as_slice() {
                let len = lens.parse::<usize>().unwrap_or(0);
                st.streams.insert(
                    SmolStr::new(rest),
                    StreamEntry::new(
                        SmolStr::new(*id),
                        None,
                        vec![0u8; len],
                    ),
                );
            }
        }
    }

    // Cursor method: see handle_top_attr.
    #[allow(clippy::unused_self)]
    fn handle_node(
        &mut self,
        st: &mut DotState,
        head: &str,
        attrs: &IndexMap<String, String>,
    ) {
        let id = match head.strip_prefix("n_") {
            Some(s) => SmolStr::new(s),
            None => SmolStr::new(head),
        };
        let kind_s = attrs
            .get("_snap_kind")
            .map_or("object", String::as_str);
        let kind = Self::node_kind(kind_s);
        let name = attrs
            .get("_snap_name")
            .map(SmolStr::new);
        let mut node_attrs: IndexMap<SmolStr, AttrValue> =
            IndexMap::new();
        for (k, v) in attrs {
            if let Some(rest) = k.strip_prefix("_snap_attr_") {
                node_attrs.insert(
                    SmolStr::new(rest),
                    Self::untag_attr(v),
                );
            }
        }
        let weight = attrs
            .get("_snap_weight")
            .and_then(|s| Self::parse_node_weight_attr(s));
        st.nodes.push(NodeDef {
            id,
            kind,
            name,
            attrs: node_attrs,
            weight,
        });
    }

    fn handle_edge(
        &mut self,
        st: &mut DotState,
        src: &str,
        tgt: &str,
        attrs: &IndexMap<String, String>,
    ) {
        let s = src.strip_prefix("n_").unwrap_or(src);
        let t = tgt.strip_prefix("n_").unwrap_or(tgt);
        let family = attrs.get("_snap_family").map_or_else(
            || SmolStr::new("default"),
            SmolStr::new,
        );
        let weight = match attrs.get("_snap_w") {
            None => crate::data::weight::EdgeWeight::None,
            Some(ws) => {
                match crate::codec::weight_text::WeightText::parse(
                    ws,
                    crate::data::types::NumericEncoding::Float,
                ) {
                    Ok(w) => w,
                    Err(es) => {
                        self.errs.extend(es);
                        crate::data::weight::EdgeWeight::None
                    }
                }
            }
        };
        st.edges.push(EdgeDef {
            family,
            src: SmolStr::new(s),
            tgt: SmolStr::new(t),
            weight,
        });
    }

    // Cursor method: see handle_top_attr.
    #[allow(clippy::unused_self)]
    fn assemble(
        &mut self,
        st: DotState,
    ) -> Result<Graph, Vec<SemanticErr>> {
        let meta = Self::assemble_meta(&st.meta_kv);
        let types =
            Self::assemble_types(st.types_attr.as_deref());
        let nodes = Nodes::new(st.nodes).map_err(|e| vec![e])?;
        let edge_results = crate::data::edges::Edges::new(
            &nodes, st.edges,
        );
        Graph::with_sections(
            meta,
            st.handle,
            nodes,
            edge_results,
            st.extras,
            st.layout,
            st.literals,
            st.registers,
            st.streams,
            TypeRegistry::new(types),
        )
    }

    /// Parse the `_snap_weight` DOT attribute (stringly-typed
    /// `WeightText` form, default Float encoding — same convention as
    /// `_snap_w` on edges). Returns None on parse failure to avoid
    /// failing the whole graph for one bad attr.
    pub(crate) fn parse_node_weight_attr(
        s: &str,
    ) -> Option<crate::data::weight::EdgeWeight> {
        crate::codec::weight_text::WeightText::parse(
            s,
            crate::data::types::NumericEncoding::Float,
        )
        .ok()
    }

    pub(crate) fn node_kind(s: &str) -> NodeKind {
        match s {
            "file" => NodeKind::File,
            "function" => NodeKind::Function,
            "info" => NodeKind::Info,
            "object" => NodeKind::Object,
            "operator" => NodeKind::Operator,
            "property" => NodeKind::Property,
            other => NodeKind::Custom(SmolStr::new(other)),
        }
    }

    pub(crate) fn e(
        found: &str,
        expected: &str,
    ) -> SemanticErr {
        SemanticErr::new(
            found.into(),
            Some(expected.into()),
            NonEmpty::with_tail(
                "match the snap DOT schema".into(),
                vec!["see io::dot::emit for the schema".into()],
            ),
        )
    }
}
