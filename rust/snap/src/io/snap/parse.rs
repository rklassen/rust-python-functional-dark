//! `Snap::parse` entry + parser carrier + top-level dispatch + common
//! helpers. Section parsers live in `parse_body` as a continuation
//! `impl Parser` block.
//!
//! The single free fn `parse` is the documented bridge from the public
//! `Snap::parse` method into the parser carrier; it is the recognized
//! exception to the no-free-fn rule for breaking up a single conceptual
//! carrier across multiple files.
//!
//! `Parser` uses `&mut self` on its methods. This is the explicit
//! cursor exception: the doctrine forbids `&mut self` on the public
//! API of immutable data types like `Graph`/`Nodes`/`Edges`, but a
//! parser IS a cursor; mutation is its semantics. `Parser` is
//! `pub(crate)` and never escapes the `io::snap` module.

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::codec::weight_text::WeightText;
use crate::data::edges::EdgeDef;
use crate::data::err::{NonEmpty, SemanticErr};
use crate::data::graph::Graph;
use crate::data::literals::LiteralEntry;
use crate::data::meta::GraphMeta;
use crate::data::nodes::{NodeDef, Nodes};
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::{TypeEntry, TypeRegistry};
use crate::data::types::{AttrValue, NodeKind, NumericEncoding};
use crate::data::weight::EdgeWeight;
use crate::io::snap::lex::{Lexer, Spanned, Tok};

/// Crate-private entry: bridge from `Snap::parse` to the carrier.
pub(crate) fn parse(input: &str) -> Result<Graph, Vec<SemanticErr>> {
    let mut p = match Parser::new(input) {
        Ok(p) => p,
        Err(e) => return Err(vec![e]),
    };
    p.parse_graph()
}

pub(crate) struct Parser<'a> {
    pub(crate) lex: Lexer<'a>,
    pub(crate) cur: Spanned<Tok<'a>>,
    pub(crate) errs: Vec<SemanticErr>,
}

#[derive(Default)]
pub(crate) struct Sections {
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

impl<'a> Parser<'a> {
    pub(crate) fn new(
        input: &'a str,
    ) -> Result<Self, SemanticErr> {
        let mut lex = Lexer::new(input);
        let cur = lex.next()?;
        Ok(Self { lex, cur, errs: Vec::new() })
    }

    pub(crate) fn parse_graph(
        &mut self,
    ) -> Result<Graph, Vec<SemanticErr>> {
        if !matches!(self.cur.value, Tok::MagicOpen) {
            return Err(vec![self.expected("magic header `🪢snap`")]);
        }
        self.bump_or_record();

        let handle = match &self.cur.value {
            Tok::Ident(s) => {
                let h = SmolStr::new(*s);
                self.bump_or_record();
                Some(h)
            }
            _ => None,
        };

        let mut sx = Sections::default();
        loop {
            match self.cur.value.clone() {
                Tok::MagicClose => {
                    self.bump_or_record();
                    break;
                }
                Tok::Eof => {
                    self.errs.push(self.expected("`end🪢` trailer"));
                    break;
                }
                Tok::Section(name) => {
                    let n = name;
                    self.bump_or_record();
                    self.dispatch_section(n, &mut sx);
                }
                _ => {
                    self.errs.push(self.expected("a section keyword"));
                    self.bump_or_record();
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
        let nodes = match Nodes::new(sx.nodes) {
            Ok(n) => n,
            Err(e) => return Err(vec![e]),
        };
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
        sx: &mut Sections,
    ) {
        match name {
            ".graph" => sx.meta = self.p_dot_graph(),
            "edges" => sx.edges = self.p_edges(),
            "extras" => sx.extras = self.p_extras(),
            "layout" => sx.layout = self.p_layout(),
            "literals" => sx.literals = self.p_literals(),
            "nodes" => sx.nodes = self.p_nodes(),
            "registers" => sx.registers = self.p_registers(),
            "streams" => sx.streams = self.p_streams(),
            "types" => sx.types = self.p_types(),
            _ => {
                self.errs.push(self.expected(
                    "a known section name",
                ));
            }
        }
    }

    pub(crate) fn parse_attr_value(&mut self) -> AttrValue {
        let v = match self.cur.value.clone() {
            Tok::None_ => AttrValue::None,
            Tok::Str(s) => AttrValue::Str(SmolStr::new(s)),
            Tok::DateTime(s) => AttrValue::DateTime(SmolStr::new(s)),
            Tok::Ident(s) => {
                if s == "true" { AttrValue::Bool(true) }
                else if s == "false" { AttrValue::Bool(false) }
                else { AttrValue::Ident(SmolStr::new(s)) }
            }
            Tok::Number(s) => self.parse_number_value(s),
            _ => {
                self.errs.push(self.expected("a value"));
                AttrValue::None
            }
        };
        self.bump_or_record();
        v
    }

    fn parse_number_value(&mut self, s: &str) -> AttrValue {
        if s.contains('.') {
            if let Ok(f) = s.parse::<f64>() { AttrValue::Float(f) } else {
                self.errs.push(self.expected("a valid float"));
                AttrValue::None
            }
        } else if let Ok(i) = s.parse::<i64>() { AttrValue::Int(i) } else {
            self.errs.push(self.expected("a valid integer"));
            AttrValue::None
        }
    }

    pub(crate) fn tok_text(&self) -> String {
        match &self.cur.value {
            Tok::Ident(s) => (*s).to_string(),
            Tok::Number(s) => (*s).to_string(),
            Tok::Str(s) => format!("'{s}'"),
            Tok::DateTime(s) => (*s).to_string(),
            Tok::None_ => "None".into(),
            _ => String::new(),
        }
    }

    pub(crate) fn bump_or_record(&mut self) {
        match self.lex.next() {
            Ok(t) => self.cur = t,
            Err(e) => {
                self.errs.push(e);
                self.cur = Spanned {
                    value: Tok::Eof,
                    line: self.cur.line,
                    col: self.cur.col,
                };
            }
        }
    }

    pub(crate) fn expect_lbrace(&mut self, msg: &'static str) -> bool {
        if matches!(self.cur.value, Tok::LBrace) {
            self.bump_or_record();
            true
        } else {
            self.errs.push(self.expected(msg));
            false
        }
    }

    pub(crate) fn expect_lbrace_named(&mut self, name: &str) -> bool {
        if matches!(self.cur.value, Tok::LBrace) {
            self.bump_or_record();
            true
        } else {
            self.errs.push(SemanticErr::new(
                format!("missing `{{` after section `{name}`"),
                Some("a `{` to open the section body".into()),
                NonEmpty::with_tail(
                    "insert `{` after the section name".into(),
                    vec!["check the section is well-formed".into()],
                ),
            ));
            false
        }
    }

    pub(crate) fn expect_colon(&mut self) -> bool {
        if matches!(self.cur.value, Tok::Colon) {
            self.bump_or_record();
            true
        } else {
            self.errs.push(self.expected("`:` after key"));
            false
        }
    }

    pub(crate) fn expect_comma_after_kv(&mut self) {
        if matches!(self.cur.value, Tok::Comma) {
            self.bump_or_record();
        }
    }

    pub(crate) fn expected(&self, msg: &str) -> SemanticErr {
        SemanticErr::new(
            format!(
                "unexpected token at {}:{}",
                self.cur.line, self.cur.col,
            ),
            Some(msg.into()),
            NonEmpty::with_tail(
                "fix the syntax to match the spec".into(),
                vec!["see the snap v0.6 grammar".into()],
            ),
        )
    }


    // Pure helpers used by the section parsers in `parse_body.rs`.
    // Kept here to keep `parse_body.rs` under the line ceiling.

    pub(crate) fn assemble_meta(
        kv: &IndexMap<SmolStr, AttrValue>,
    ) -> GraphMeta {
        let id = Self::take_str(kv, "id");
        let name = Self::take_str(kv, "name");
        let operators = Self::take_str(kv, "operators");
        let time = Self::take_dt(kv, "time");
        let workspace = Self::take_str(kv, "workspace");
        let gen = match kv.get("gen") {
            // gen field is u64 in the data model; negative parsed
            // ints land in the _ branch below in practice.
            #[allow(clippy::cast_sign_loss)]
            Some(AttrValue::Int(i)) => *i as u64,
            _ => 0,
        };
        let types = match kv.get("types") {
            Some(AttrValue::None) | None => None,
            Some(AttrValue::Str(s) | AttrValue::Ident(s)) => Some(s.clone()),
            _ => None,
        };
        let version = match kv.get("version") {
            Some(AttrValue::Float(f)) => {
                SmolStr::new(Self::format_float(*f))
            }
            Some(AttrValue::Int(i)) => SmolStr::new(format!("{i}")),
            Some(AttrValue::Str(s)) => s.clone(),
            _ => SmolStr::new("0.6"),
        };
        let date = Self::opt_str(kv, "date");
        let data_path = Self::opt_str(kv, "data_path");
        let code_path = Self::opt_str(kv, "code_path");
        GraphMeta::new(
            gen, id, name, operators, time, types, version,
            workspace, date, data_path, code_path,
        )
    }

    fn take_str(
        kv: &IndexMap<SmolStr, AttrValue>,
        k: &str,
    ) -> SmolStr {
        match kv.get(k) {
            Some(AttrValue::Str(s) | AttrValue::Ident(s)) => s.clone(),
            _ => SmolStr::default(),
        }
    }

    fn take_dt(
        kv: &IndexMap<SmolStr, AttrValue>,
        k: &str,
    ) -> SmolStr {
        match kv.get(k) {
            Some(AttrValue::DateTime(s)) => s.clone(),
            _ => SmolStr::default(),
        }
    }

    fn opt_str(
        kv: &IndexMap<SmolStr, AttrValue>,
        k: &str,
    ) -> Option<SmolStr> {
        match kv.get(k) {
            Some(AttrValue::Str(s)) => Some(s.clone()),
            _ => None,
        }
    }

    pub(crate) fn node_kind_for(s: &str) -> NodeKind {
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

    fn format_float(f: f64) -> String {
        if f.fract() == 0.0 && f.is_finite() {
            format!("{f:.1}")
        } else {
            format!("{f}")
        }
    }

    /// Parse `-(content)X-> tgt`. `cur` must be `Dash`. Leaves `cur`
    /// at the target ident on success; returns `None` and pushes to
    /// `self.errs` on malformed shapes.
    pub(crate) fn read_weighted_arrow(
        &mut self,
    ) -> Option<EdgeWeight> {
        // cur == Dash. Consume; expect LParen.
        self.bump_or_record();
        if !matches!(self.cur.value, Tok::LParen) {
            self.errs.push(self.expected("`(` after `-`"));
            return None;
        }
        // Take raw paren body via the lexer (single-line).
        let body = match self.lex.take_paren_body() {
            Ok(s) => s.to_string(),
            Err(e) => {
                self.errs.push(e);
                return None;
            }
        };
        // Re-prime cur (was the stale token after the open paren).
        self.bump_or_record();
        // Optional one-letter format mark.
        let mark = Self::format_mark(&self.cur.value);
        if mark.is_some() {
            self.bump_or_record();
        }
        if !matches!(self.cur.value, Tok::Arrow) {
            self.errs.push(self.expected("`->` after `(...)`"));
            return None;
        }
        self.bump_or_record();
        let enc = mark.unwrap_or_else(|| Self::infer_encoding(&body));
        match WeightText::parse(&body, enc) {
            Ok(w) => Some(w),
            Err(es) => {
                self.errs.extend(es);
                None
            }
        }
    }

    fn format_mark(t: &Tok<'_>) -> Option<NumericEncoding> {
        match t {
            Tok::Ident(s) => match *s {
                "s" => Some(NumericEncoding::Snorm),
                "u" => Some(NumericEncoding::Unorm),
                "h" => Some(NumericEncoding::Hex),
                _ => None,
            },
            _ => None,
        }
    }

    /// Default encoding when the arrow has no `s|u|h` mark.
    /// Heuristic: `@`-ref → `Raw`; contains `.` → `Float`; else
    /// `Int`. The detailed validation lives inside `WeightText`.
    fn infer_encoding(s: &str) -> NumericEncoding {
        let t = s.trim();
        if t.starts_with('@') { NumericEncoding::Raw }
        else if t.contains('.') { NumericEncoding::Float }
        else { NumericEncoding::Int }
    }
}
