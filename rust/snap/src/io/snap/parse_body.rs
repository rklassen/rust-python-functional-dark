//! Continuation of `impl Parser` for the per-section parsers. Split
//! out of `parse.rs` to honor the per-file 432-line ceiling. This file
//! is named after the `Parser` carrier because it CONTINUES that impl
//! block — file/struct alignment preserved.
//!
//! Same `&mut self` cursor exception applies (documented in `parse.rs`
//! and `lex.rs`).

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::data::edges::EdgeDef;
use crate::data::literals::LiteralEntry;
use crate::data::meta::GraphMeta;
use crate::data::nodes::NodeDef;
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::TypeEntry;
use crate::data::types::{AttrValue, NodeKind};
use crate::data::weight::EdgeWeight;
use crate::io::snap::lex::Tok;
use crate::io::snap::parse::Parser;

impl Parser<'_> {
    pub(crate) fn p_dot_graph(&mut self) -> Option<GraphMeta> {
        if !self.expect_lbrace("`{` after `.graph`") {
            return None;
        }
        let mut kv: IndexMap<SmolStr, AttrValue> = IndexMap::new();
        loop {
            if matches!(self.cur.value, Tok::RBrace) {
                self.bump_or_record();
                break;
            }
            if matches!(self.cur.value, Tok::Eof) {
                self.errs.push(self.expected(
                    "`}` to close `.graph`",
                ));
                return None;
            }
            let key = match &self.cur.value {
                Tok::Ident(s) | Tok::Section(s) => SmolStr::new(*s),
                _ => {
                    self.errs.push(self.expected(
                        "a `.graph` field name",
                    ));
                    self.bump_or_record();
                    continue;
                }
            };
            self.bump_or_record();
            if !self.expect_colon() { continue; }
            let v = self.parse_attr_value();
            self.expect_comma_after_kv();
            kv.insert(key, v);
        }
        Some(Self::assemble_meta(&kv))
    }


    pub(crate) fn p_edges(&mut self) -> Vec<EdgeDef> {
        let mut out: Vec<EdgeDef> = Vec::new();
        if !self.expect_lbrace("`{` after `edges`") { return out; }
        loop {
            match self.cur.value.clone() {
                Tok::RBrace => { self.bump_or_record(); break; }
                Tok::Eof => {
                    self.errs.push(self.expected("`}` after `edges`"));
                    return out;
                }
                Tok::Ident(fam) => {
                    let family = SmolStr::new(fam);
                    self.bump_or_record();
                    self.p_edge_family(&family, &mut out);
                }
                _ => {
                    self.errs.push(self.expected(
                        "an edge family name",
                    ));
                    self.bump_or_record();
                }
            }
        }
        out
    }

    fn p_edge_family(
        &mut self,
        family: &SmolStr,
        out: &mut Vec<EdgeDef>,
    ) {
        if !self.expect_lbrace("`{` after edge family name") {
            return;
        }
        loop {
            match self.cur.value.clone() {
                Tok::RBrace => { self.bump_or_record(); break; }
                Tok::Eof => {
                    self.errs.push(self.expected(
                        "`}` to close edge family",
                    ));
                    return;
                }
                Tok::Ident(src) => {
                    let s = SmolStr::new(src);
                    self.bump_or_record();
                    self.p_edge_after_src(family, &s, out);
                }
                _ => {
                    self.errs.push(self.expected("an edge src id"));
                    self.bump_or_record();
                }
            }
        }
    }

    fn p_edge_after_src(
        &mut self,
        family: &SmolStr,
        src: &SmolStr,
        out: &mut Vec<EdgeDef>,
    ) {
        let weight = match self.cur.value {
            Tok::Arrow => {
                self.bump_or_record();
                EdgeWeight::None
            }
            Tok::Dash => match self.read_weighted_arrow() {
                Some(w) => w,
                None => return,
            },
            _ => {
                self.errs.push(self.expected(
                    "`->` or `-(...)->` after edge src",
                ));
                return;
            }
        };
        let tgt = if let Tok::Ident(s) = &self.cur.value {
            SmolStr::new(*s)
        } else {
            self.errs.push(self.expected("an edge tgt id"));
            return;
        };
        self.bump_or_record();
        self.expect_comma_after_kv();
        out.push(EdgeDef {
            family: family.clone(),
            src: src.clone(),
            tgt,
            weight,
        });
    }

    pub(crate) fn p_nodes(&mut self) -> Vec<NodeDef> {
        let mut out: Vec<NodeDef> = Vec::new();
        if !self.expect_lbrace("`{` after `nodes`") { return out; }
        loop {
            match self.cur.value.clone() {
                Tok::RBrace => { self.bump_or_record(); break; }
                Tok::Eof => {
                    self.errs.push(self.expected(
                        "`}` to close `nodes`",
                    ));
                    return out;
                }
                Tok::NodeKind(k) => {
                    let kind = Self::node_kind_for(k);
                    self.bump_or_record();
                    self.p_node_body(kind, &mut out);
                }
                _ => {
                    self.errs.push(self.expected(
                        "a node-kind keyword",
                    ));
                    self.bump_or_record();
                }
            }
        }
        out
    }

    fn p_node_body(
        &mut self,
        kind: NodeKind,
        out: &mut Vec<NodeDef>,
    ) {
        if !self.expect_lbrace("`{` after node-kind") { return; }
        let mut id: SmolStr = SmolStr::default();
        let mut name: Option<SmolStr> = None;
        let mut attrs: IndexMap<SmolStr, AttrValue> = IndexMap::new();
        let mut weight: Option<EdgeWeight> = None;
        loop {
            match self.cur.value.clone() {
                Tok::RBrace => { self.bump_or_record(); break; }
                Tok::Eof => {
                    self.errs.push(self.expected(
                        "`}` after node body",
                    ));
                    return;
                }
                Tok::Ident(k) => {
                    let key = SmolStr::new(k);
                    self.bump_or_record();
                    if !self.expect_colon() { continue; }
                    if key == "weight" {
                        if let Some(w) = self.read_node_weight() {
                            weight = Some(w);
                        }
                        self.expect_comma_after_kv();
                        continue;
                    }
                    let v = self.parse_attr_value();
                    if key == "id" {
                        if let AttrValue::Ident(s) = &v {
                            id = s.clone();
                        } else if let AttrValue::Str(s) = &v {
                            id = s.clone();
                        }
                    } else if key == "name" {
                        if let AttrValue::Str(s) = &v {
                            name = Some(s.clone());
                        }
                    } else {
                        attrs.insert(key, v);
                    }
                    self.expect_comma_after_kv();
                }
                _ => {
                    self.errs.push(self.expected(
                        "a node attribute key",
                    ));
                    self.bump_or_record();
                }
            }
        }
        self.expect_comma_after_kv();
        out.push(NodeDef { id, kind, name, attrs, weight });
    }

    pub(crate) fn p_extras(
        &mut self,
    ) -> IndexMap<SmolStr, AttrValue> {
        let mut out = IndexMap::new();
        if !self.expect_lbrace("`{` after `extras`") { return out; }
        loop {
            match self.cur.value.clone() {
                Tok::RBrace => { self.bump_or_record(); break; }
                Tok::Eof => {
                    self.errs.push(self.expected(
                        "`}` after `extras`",
                    ));
                    return out;
                }
                Tok::Ident(k) => {
                    let key = SmolStr::new(k);
                    self.bump_or_record();
                    if !self.expect_colon() { continue; }
                    let v = self.parse_attr_value();
                    self.expect_comma_after_kv();
                    out.insert(key, v);
                }
                _ => {
                    self.errs.push(self.expected("an extras key"));
                    self.bump_or_record();
                }
            }
        }
        out
    }

    pub(crate) fn p_layout(
        &mut self,
    ) -> IndexMap<SmolStr, (f64, f64)> {
        let out = IndexMap::new();
        if !self.expect_lbrace("`{` after `layout`") { return out; }
        loop {
            match self.cur.value.clone() {
                Tok::RBrace => { self.bump_or_record(); break; }
                Tok::Eof => {
                    self.errs.push(self.expected(
                        "`}` after `layout`",
                    ));
                    return out;
                }
                _ => {
                    self.errs.push(self.expected(
                        "an empty `layout` body for v0.6",
                    ));
                    self.bump_or_record();
                }
            }
        }
        out
    }

    pub(crate) fn p_literals(
        &mut self,
    ) -> IndexMap<SmolStr, LiteralEntry> {
        self.consume_empty_section("literals")
    }

    pub(crate) fn p_registers(
        &mut self,
    ) -> IndexMap<SmolStr, RegisterEntry> {
        self.consume_empty_section("registers")
    }

    pub(crate) fn p_streams(
        &mut self,
    ) -> IndexMap<SmolStr, StreamEntry> {
        self.consume_empty_section("streams")
    }

    fn consume_empty_section<V>(
        &mut self,
        name: &str,
    ) -> IndexMap<SmolStr, V> {
        let out = IndexMap::new();
        if !self.expect_lbrace_named(name) { return out; }
        if matches!(self.cur.value, Tok::RBrace) {
            self.bump_or_record();
        } else {
            self.errs.push(self.expected(
                "an empty body or content for this section",
            ));
            let mut depth = 1;
            while depth > 0 {
                match self.cur.value {
                    Tok::LBrace => depth += 1,
                    Tok::RBrace => depth -= 1,
                    Tok::Eof => break,
                    _ => {}
                }
                self.bump_or_record();
            }
        }
        out
    }

    pub(crate) fn p_types(&mut self) -> Vec<TypeEntry> {
        let mut out: Vec<TypeEntry> = Vec::new();
        if !self.expect_lbrace("`{` after `types`") { return out; }
        loop {
            match self.cur.value.clone() {
                Tok::RBrace => { self.bump_or_record(); break; }
                Tok::Eof => {
                    self.errs.push(self.expected(
                        "`}` after `types`",
                    ));
                    return out;
                }
                Tok::Ident(s) => {
                    let name = SmolStr::new(s);
                    self.bump_or_record();
                    if matches!(self.cur.value, Tok::Arrow) {
                        self.bump_or_record();
                        let expr = self.tok_text();
                        self.bump_or_record();
                        out.push(TypeEntry::Alias {
                            alias: name,
                            expr: SmolStr::new(expr),
                        });
                    } else {
                        out.push(TypeEntry::Concrete(name));
                    }
                    self.expect_comma_after_kv();
                }
                Tok::Str(s) => {
                    let alias = SmolStr::new(s);
                    self.bump_or_record();
                    if !matches!(self.cur.value, Tok::Arrow) {
                        self.errs.push(self.expected(
                            "`->` after alias name",
                        ));
                        continue;
                    }
                    self.bump_or_record();
                    let expr = self.tok_text();
                    self.bump_or_record();
                    out.push(TypeEntry::Alias {
                        alias,
                        expr: SmolStr::new(expr),
                    });
                    self.expect_comma_after_kv();
                }
                _ => {
                    self.errs.push(self.expected("a type entry"));
                    self.bump_or_record();
                }
            }
        }
        out
    }
}

