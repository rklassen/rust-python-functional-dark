//! XML parse entry + cursor + meta/edges/nodes parsers. Section
//! parsers for layout/literals/registers/streams/types and the
//! `AttrValue` mapper live in `parse_body` as a continuation
//! `impl XmlReader` block to honor the per-file 432-line ceiling.
//!
//! The single free fn `parse` is the documented bridge from
//! `Xml::parse` into the carrier — the recognized exception to the
//! no-free-fn rule.
//!
//! `XmlReader` uses `&mut self` on its methods. This is the explicit
//! cursor exception: a reader IS a cursor; mutation is its
//! semantics. `XmlReader` is private to this module and never escapes,
//! same pattern as `io/snap/lex.rs::Lexer`.
//!
//! Weight strings round-trip through
//! `crate::codec::weight_text::WeightText::parse` — codec layer is
//! the single source of truth.

use indexmap::IndexMap;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use smol_str::SmolStr;

use crate::codec::weight_text::WeightText;
use crate::data::edges::EdgeDef;
use crate::data::err::{NonEmpty, SemanticErr};
use crate::data::graph::Graph;
use crate::data::meta::GraphMeta;
use crate::data::nodes::{NodeDef, Nodes};
use crate::data::type_registry::TypeRegistry;
use crate::data::types::{AttrValue, NodeKind};
use crate::data::weight::EdgeWeight;
use crate::io::xml::parse_body::Sections;

pub(crate) fn parse(input: &str) -> Result<Graph, Vec<SemanticErr>> {
    let mut r = XmlReader::new(input);
    r.parse_root()
}

pub(crate) struct XmlReader<'a> {
    pub(crate) rd: Reader<&'a [u8]>,
    pub(crate) errs: Vec<SemanticErr>,
}

impl<'a> XmlReader<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        let mut rd = Reader::from_str(input);
        rd.config_mut().trim_text(true);
        Self { rd, errs: Vec::new() }
    }

    pub(crate) fn parse_root(
        &mut self,
    ) -> Result<Graph, Vec<SemanticErr>> {
        let snap_start = match self.find_start(b"snap") {
            Some(b) => b,
            None => {
                return Err(vec![Self::e(
                    "missing root <snap> element",
                    "<snap version=\"...\">",
                )]);
            }
        };
        let handle = Self::attr_str(&snap_start, "handle");
        let version = Self::attr_str(&snap_start, "version")
            .unwrap_or_else(|| SmolStr::new("0.6"));

        let mut sx = Sections { version, ..Sections::default() };

        loop {
            match self.next_event() {
                Ok(Event::Start(b)) => {
                    let name = Self::tag_name(&b);
                    self.dispatch_section(&name, &b, &mut sx);
                }
                Ok(Event::Empty(b)) => {
                    let name = Self::tag_name(&b);
                    if name == "snap" { break; }
                    // Empty top-level section: nothing to do.
                    let _ = name;
                }
                Ok(Event::End(b)) => {
                    let name = Self::tag_name_end(&b);
                    if name == "snap" { break; }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs.push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }

        if !self.errs.is_empty() {
            return Err(std::mem::take(&mut self.errs));
        }

        let meta = sx.meta.unwrap_or_else(|| {
            GraphMeta::minimal(
                SmolStr::new("a000"),
                SmolStr::new("0.6"),
            )
        });
        let nodes = Nodes::new(sx.nodes).map_err(|e| vec![e])?;
        let edge_results = crate::data::edges::Edges::new(
            &nodes, sx.edges,
        );
        Graph::with_sections(
            meta,
            handle,
            nodes,
            edge_results,
            sx.extras,
            sx.layout,
            sx.literals,
            sx.registers,
            sx.streams,
            TypeRegistry::new(sx.types),
        )
    }

    fn dispatch_section(
        &mut self,
        name: &str,
        _b: &BytesStart<'_>,
        sx: &mut Sections,
    ) {
        match name {
            "graph" => sx.meta = Some(self.r_meta()),
            "edges" => sx.edges = self.r_edges(),
            "extras" => sx.extras = self.r_kv("extras"),
            "layout" => sx.layout = self.r_layout(),
            "literals" => sx.literals = self.r_literals(),
            "nodes" => sx.nodes = self.r_nodes(),
            "registers" => sx.registers = self.r_registers(),
            "streams" => sx.streams = self.r_streams(),
            "types" => sx.types = self.r_types(),
            _ => {
                // Unknown top-level — skip to its end.
                self.skip_to_end(name);
            }
        }
    }

    fn r_meta(&mut self) -> GraphMeta {
        let mut kv: IndexMap<SmolStr, AttrValue> = IndexMap::new();
        loop {
            match self.next_event() {
                Ok(Event::Start(b)) => {
                    let name = Self::tag_name(&b);
                    let text = self.read_text_until(&name);
                    kv.insert(
                        SmolStr::new(&name),
                        AttrValue::Str(SmolStr::new(text.trim())),
                    );
                }
                Ok(Event::Empty(b)) => {
                    let name = Self::tag_name(&b);
                    kv.insert(
                        SmolStr::new(name),
                        AttrValue::None,
                    );
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs.push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
        Self::assemble_meta(&kv)
    }

    fn assemble_meta(
        kv: &IndexMap<SmolStr, AttrValue>,
    ) -> GraphMeta {
        let s = |k: &str| -> SmolStr {
            match kv.get(k) {
                Some(AttrValue::Str(s)) => s.clone(),
                _ => SmolStr::default(),
            }
        };
        let opt = |k: &str| -> Option<SmolStr> {
            match kv.get(k) {
                Some(AttrValue::Str(s)) => Some(s.clone()),
                _ => None,
            }
        };
        let gen = match kv.get("gen") {
            Some(AttrValue::Str(s)) => s.parse::<u64>().unwrap_or(0),
            _ => 0,
        };
        let types = match kv.get("types") {
            Some(AttrValue::None) | None => None,
            Some(AttrValue::Str(s)) => Some(s.clone()),
            _ => None,
        };
        GraphMeta::new(
            gen, s("id"), s("name"), s("operators"),
            s("time"), types, s("version"), s("workspace"),
            opt("date"), opt("data_path"), opt("code_path"),
        )
    }

    fn r_edges(&mut self) -> Vec<EdgeDef> {
        let mut out: Vec<EdgeDef> = Vec::new();
        loop {
            match self.next_event() {
                Ok(Event::Start(b))
                    if Self::tag_name(&b) == "family" =>
                {
                    let fam = Self::attr_str(&b, "name")
                        .unwrap_or_default();
                    self.r_edge_family(&fam, &mut out);
                }
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "family" =>
                {
                    // empty family — nothing to do
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs.push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
        out
    }

    fn r_edge_family(
        &mut self,
        family: &SmolStr,
        out: &mut Vec<EdgeDef>,
    ) {
        loop {
            match self.next_event() {
                Ok(Event::Empty(b))
                    if Self::tag_name(&b) == "edge" =>
                {
                    if let Some(d) = self.r_edge(&b, family) {
                        out.push(d);
                    }
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs.push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
    }

    fn r_edge(
        &mut self,
        b: &BytesStart<'_>,
        family: &SmolStr,
    ) -> Option<EdgeDef> {
        let src = Self::attr_str(b, "src")?;
        let tgt = Self::attr_str(b, "tgt")?;
        let weight = match Self::attr_str(b, "w") {
            None => EdgeWeight::None,
            Some(s) => match WeightText::parse(
                &s,
                crate::data::types::NumericEncoding::Float,
            ) {
                Ok(w) => w,
                Err(es) => {
                    self.errs.extend(es);
                    EdgeWeight::None
                }
            },
        };
        Some(EdgeDef {
            family: family.clone(),
            src,
            tgt,
            weight,
        })
    }

    fn r_nodes(&mut self) -> Vec<NodeDef> {
        let mut out: Vec<NodeDef> = Vec::new();
        loop {
            match self.next_event() {
                Ok(Event::Start(b)) => {
                    let kind_s = Self::tag_name(&b);
                    let kind = Self::node_kind(&kind_s);
                    let id = Self::attr_str(&b, "id")
                        .unwrap_or_default();
                    let name = Self::attr_str(&b, "name");
                    let attrs = self.r_attrs(&kind_s);
                    out.push(NodeDef { id, kind, name, attrs });
                }
                Ok(Event::Empty(b)) => {
                    let kind_s = Self::tag_name(&b);
                    let kind = Self::node_kind(&kind_s);
                    let id = Self::attr_str(&b, "id")
                        .unwrap_or_default();
                    let name = Self::attr_str(&b, "name");
                    out.push(NodeDef {
                        id,
                        kind,
                        name,
                        attrs: IndexMap::new(),
                    });
                }
                Ok(Event::End(_)) => break,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    self.errs.push(Self::e_xml(&e.to_string()));
                    break;
                }
            }
        }
        out
    }

    pub(crate) fn next_event(
        &mut self,
    ) -> quick_xml::Result<Event<'a>> {
        self.rd.read_event()
    }

    fn find_start(
        &mut self,
        wanted: &[u8],
    ) -> Option<BytesStart<'a>> {
        loop {
            match self.next_event() {
                Ok(Event::Start(b)) => {
                    if b.name().as_ref() == wanted {
                        return Some(b.into_owned());
                    }
                }
                Ok(Event::Empty(b)) => {
                    if b.name().as_ref() == wanted {
                        return Some(b.into_owned());
                    }
                }
                Ok(Event::Eof) => return None,
                Ok(_) => {}
                Err(e) => {
                    self.errs.push(Self::e_xml(&e.to_string()));
                    return None;
                }
            }
        }
    }

    pub(crate) fn tag_name(b: &BytesStart<'_>) -> String {
        std::str::from_utf8(b.name().as_ref())
            .map(std::string::ToString::to_string)
            .unwrap_or_default()
    }

    pub(crate) fn tag_name_end(
        b: &quick_xml::events::BytesEnd<'_>,
    ) -> String {
        std::str::from_utf8(b.name().as_ref())
            .map(std::string::ToString::to_string)
            .unwrap_or_default()
    }

    pub(crate) fn attr_str(
        b: &BytesStart<'_>,
        key: &str,
    ) -> Option<SmolStr> {
        for a in b.attributes().flatten() {
            if a.key.as_ref() == key.as_bytes() {
                if let Ok(v) = a.unescape_value() {
                    return Some(SmolStr::new(v.as_ref()));
                }
            }
        }
        None
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
                "match the snap XML schema".into(),
                vec!["see io::xml::emit for the schema".into()],
            ),
        )
    }

    pub(crate) fn e_xml(reason: &str) -> SemanticErr {
        SemanticErr::new(
            format!("invalid XML: {reason}"),
            Some("syntactically valid XML".into()),
            NonEmpty::with_tail(
                "fix the XML syntax".into(),
                vec!["validate against the snap XML schema".into()],
            ),
        )
    }
}
