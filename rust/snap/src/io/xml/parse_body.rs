//! Continuation of `impl XmlReader` for layout/literals/registers/
//! streams/types section parsers, plus the shared `Sections` accumulator.
//! Split out of `parse.rs` to honor the per-file 432-line ceiling.
//! Same `&mut self` cursor exception applies (documented in `parse.rs`).

use indexmap::IndexMap;
use quick_xml::events::Event;
use smol_str::SmolStr;

use crate::data::edges::EdgeDef;
use crate::data::literals::LiteralEntry;
use crate::data::meta::GraphMeta;
use crate::data::nodes::NodeDef;
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::TypeEntry;
use crate::data::types::AttrValue;
use crate::io::xml::parse::XmlReader;

#[derive(Default)]
pub(crate) struct Sections {
    // Captured from <snap version=".."> attr; reserved for future
    // version-gated parsing. Kept on the struct to round-trip intent.
    #[allow(dead_code)]
    pub(crate) version: SmolStr,
    pub(crate) meta: Option<GraphMeta>,
    pub(crate) edges: Vec<EdgeDef>,
    pub(crate) extras: IndexMap<SmolStr, AttrValue>,
    pub(crate) layout: IndexMap<SmolStr, (f64, f64)>,
    pub(crate) literals: IndexMap<SmolStr, LiteralEntry>,
    pub(crate) nodes: Vec<NodeDef>,
    pub(crate) registers: IndexMap<SmolStr, RegisterEntry>,
    pub(crate) streams: IndexMap<SmolStr, StreamEntry>,
    pub(crate) types: Vec<TypeEntry>,
}

impl XmlReader<'_> {
    pub(crate) fn r_layout(
        &mut self,
    ) -> IndexMap<SmolStr, (f64, f64)> {
        let mut out: IndexMap<SmolStr, (f64, f64)> = IndexMap::new();
        loop {
            match self.next_event() {
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "pos" =>
                {
                    let key = Self::attr_str(&b, "key")
                        .unwrap_or_default();
                    let x = Self::attr_str(&b, "x")
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let y = Self::attr_str(&b, "y")
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    out.insert(key, (x, y));
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs_mut()
                        .push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
        out
    }

    pub(crate) fn r_literals(
        &mut self,
    ) -> IndexMap<SmolStr, LiteralEntry> {
        let mut out: IndexMap<SmolStr, LiteralEntry> = IndexMap::new();
        loop {
            match self.next_event() {
                Ok(Event::Start(b))
                    if Self::tag_name(&b) == "literal" =>
                {
                    let name = Self::attr_str(&b, "name")
                        .unwrap_or_default();
                    let id = Self::attr_str(&b, "id")
                        .unwrap_or_default();
                    let type_name = Self::attr_str(&b, "type")
                        .unwrap_or_default();
                    let value =
                        self.r_literal_value();
                    out.insert(
                        name.clone(),
                        LiteralEntry::new(
                            name, id, type_name, value,
                        ),
                    );
                }
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "literal" =>
                {
                    let name = Self::attr_str(&b, "name")
                        .unwrap_or_default();
                    let id = Self::attr_str(&b, "id")
                        .unwrap_or_default();
                    let type_name = Self::attr_str(&b, "type")
                        .unwrap_or_default();
                    out.insert(
                        name.clone(),
                        LiteralEntry::new(
                            name, id, type_name, AttrValue::None,
                        ),
                    );
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs_mut()
                        .push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
        out
    }

    fn r_literal_value(&mut self) -> AttrValue {
        let mut value = AttrValue::None;
        loop {
            match self.next_event() {
                Ok(Event::Start(b))
                    if Self::tag_name(&b) == "attr" =>
                {
                    value = self.r_attr_body(&b, "attr");
                }
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "attr" =>
                {
                    value = self.r_attr_body(&b, "attr");
                }
                Ok(Event::End(b))
                    if Self::tag_name_end(&b) == "literal" =>
                {
                    return value;
                }
                Ok(Event::End(_)) => return value,
                Ok(Event::Eof) => return value,
                Ok(_) => {}
                Err(e) => {
                    self.errs_mut()
                        .push(Self::e_xml(&e.to_string()));
                    return value;
                }
            }
        }
    }

    pub(crate) fn r_registers(
        &mut self,
    ) -> IndexMap<SmolStr, RegisterEntry> {
        let mut out: IndexMap<SmolStr, RegisterEntry> = IndexMap::new();
        loop {
            match self.next_event() {
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "register" =>
                {
                    let name = Self::attr_str(&b, "name")
                        .unwrap_or_default();
                    let id = Self::attr_str(&b, "id")
                        .unwrap_or_default();
                    let type_name = Self::attr_str(&b, "type")
                        .unwrap_or_default();
                    out.insert(
                        name.clone(),
                        RegisterEntry::new(name, id, type_name),
                    );
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs_mut()
                        .push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
        out
    }

    pub(crate) fn r_streams(
        &mut self,
    ) -> IndexMap<SmolStr, StreamEntry> {
        let mut out: IndexMap<SmolStr, StreamEntry> = IndexMap::new();
        loop {
            match self.next_event() {
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "stream" =>
                {
                    let name = Self::attr_str(&b, "name")
                        .unwrap_or_default();
                    let id = Self::attr_str(&b, "id")
                        .unwrap_or_default();
                    let len = Self::attr_str(&b, "len")
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);
                    out.insert(
                        name,
                        StreamEntry::new(
                            id, None, vec![0u8; len],
                        ),
                    );
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs_mut()
                        .push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
        out
    }

    pub(crate) fn r_types(&mut self) -> Vec<TypeEntry> {
        let mut out: Vec<TypeEntry> = Vec::new();
        loop {
            match self.next_event() {
                Ok(Event::Empty(b)) => {
                    let n = Self::tag_name(&b);
                    if n == "type" {
                        let nm = Self::attr_str(&b, "name")
                            .unwrap_or_default();
                        out.push(TypeEntry::Concrete(nm));
                    } else if n == "alias" {
                        let alias = Self::attr_str(&b, "name")
                            .unwrap_or_default();
                        let expr = Self::attr_str(&b, "expr")
                            .unwrap_or_default();
                        out.push(TypeEntry::Alias { alias, expr });
                    }
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs_mut()
                        .push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
        out
    }

    pub(crate) fn errs_mut(
        &mut self,
    ) -> &mut Vec<crate::data::err::SemanticErr> {
        &mut self.errs
    }
}
