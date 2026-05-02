//! Continuation of `impl XmlReader` for the `AttrValue` tree readers
//! (`r_attrs`, `r_attr_body`, scalar/list/dict bodies, `r_kv`) plus
//! the low-level text/skip helpers shared with the other parser files.
//! Split out of `parse.rs` to honor the per-file 432-line ceiling.
//! Same `&mut self` cursor exception applies (documented in
//! `parse.rs`).

use indexmap::IndexMap;
use quick_xml::events::{BytesStart, Event};
use smol_str::SmolStr;

use crate::codec::weight_text::WeightText;
use crate::data::types::{AttrValue, NumericEncoding};
use crate::data::weight::EdgeWeight;
use crate::io::xml::parse::XmlReader;

impl XmlReader<'_> {
    pub(crate) fn r_attrs(
        &mut self,
        end_tag: &str,
    ) -> IndexMap<SmolStr, AttrValue> {
        let mut out: IndexMap<SmolStr, AttrValue> = IndexMap::new();
        loop {
            match self.next_event() {
                Ok(Event::Start(b))
                    if Self::tag_name(&b) == "attr" =>
                {
                    let key = Self::attr_str(&b, "key")
                        .unwrap_or_default();
                    let v = self.r_attr_body(&b, "attr");
                    out.insert(key, v);
                }
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "attr" =>
                {
                    let key = Self::attr_str(&b, "key")
                        .unwrap_or_default();
                    let v = self.r_attr_body(&b, "attr");
                    out.insert(key, v);
                }
                Ok(Event::End(b))
                    if Self::tag_name_end(&b) == end_tag =>
                {
                    break;
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

    /// Parses one attr-or-item element body. The opening tag is
    /// already consumed; `tag_name` is the name we're inside ("attr"
    /// or "item") so we know which `</tag>` to match for nested
    /// list/dict bodies.
    pub(crate) fn r_attr_body(
        &mut self,
        b: &BytesStart<'_>,
        tag_name: &str,
    ) -> AttrValue {
        if Self::attr_str(b, "nil").as_deref() == Some("true") {
            return AttrValue::None;
        }
        let ty = Self::attr_str(b, "type");
        match ty.as_deref() {
            Some("list") => self.r_list_body(tag_name),
            Some("dict") => self.r_dict_body(tag_name),
            _ => self.r_scalar_body(ty.as_deref(), tag_name),
        }
    }

    pub(crate) fn r_scalar_body(
        &mut self,
        ty: Option<&str>,
        tag_name: &str,
    ) -> AttrValue {
        let raw = self.read_text_until(tag_name);
        let txt = raw.trim().to_string();
        match ty {
            Some("bool") => AttrValue::Bool(txt == "true"),
            Some("int") => txt
                .parse::<i64>()
                .map(AttrValue::Int)
                .unwrap_or(AttrValue::None),
            Some("float") => txt
                .parse::<f64>()
                .map(AttrValue::Float)
                .unwrap_or(AttrValue::None),
            Some("str") => AttrValue::Str(SmolStr::new(&txt)),
            Some("dt") => AttrValue::DateTime(SmolStr::new(&txt)),
            _ => AttrValue::Ident(SmolStr::new(&txt)),
        }
    }

    pub(crate) fn r_list_body(
        &mut self,
        end_tag: &str,
    ) -> AttrValue {
        let mut items: Vec<AttrValue> = Vec::new();
        loop {
            match self.next_event() {
                Ok(Event::Start(b))
                    if Self::tag_name(&b) == "item" =>
                {
                    items.push(self.r_attr_body(&b, "item"));
                }
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "item" =>
                {
                    items.push(self.r_attr_body(&b, "item"));
                }
                Ok(Event::End(b))
                    if Self::tag_name_end(&b) == end_tag =>
                {
                    break;
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
        AttrValue::List(items)
    }

    pub(crate) fn r_dict_body(
        &mut self,
        end_tag: &str,
    ) -> AttrValue {
        let attrs = self.r_attrs(end_tag);
        AttrValue::Dict(attrs)
    }

    pub(crate) fn r_kv(
        &mut self,
        end_tag: &str,
    ) -> IndexMap<SmolStr, AttrValue> {
        self.r_attrs(end_tag)
    }

    pub(crate) fn read_text_until(
        &mut self,
        _end: &str,
    ) -> String {
        let mut buf = String::new();
        loop {
            match self.next_event() {
                Ok(Event::Text(t)) => {
                    if let Ok(s) = t.unescape() {
                        buf.push_str(&s);
                    }
                }
                Ok(Event::CData(c)) => {
                    if let Ok(s) =
                        std::str::from_utf8(c.as_ref())
                    {
                        buf.push_str(s);
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
        buf
    }

    /// v0.7 node `weight=` XML attribute reader. Float-encoding
    /// fallback matches the edge `w=` convention; see `r_edge`.
    pub(crate) fn node_weight_from(
        b: &BytesStart<'_>,
    ) -> Option<EdgeWeight> {
        Self::attr_str(b, "weight").and_then(|s| {
            WeightText::parse(&s, NumericEncoding::Float).ok()
        })
    }

    pub(crate) fn skip_to_end(&mut self, name: &str) {
        let mut depth = 1i32;
        while depth > 0 {
            match self.next_event() {
                Ok(Event::Start(b))
                    if Self::tag_name(&b) == name =>
                {
                    depth += 1;
                }
                Ok(Event::End(b))
                    if Self::tag_name_end(&b) == name =>
                {
                    depth -= 1;
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs_mut()
                        .push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
    }
}
