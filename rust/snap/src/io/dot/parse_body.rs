//! Continuation of `impl DotReader` for the lexer primitives,
//! attribute-list reader, and `AttrValue` tag decoder. Split out of
//! `parse.rs` to honor the per-file 432-line ceiling. Same `&mut
//! self` cursor exception applies (documented in `parse.rs`).

use indexmap::IndexMap;
use serde_json::Value;
use smol_str::SmolStr;

use crate::data::meta::GraphMeta;
use crate::data::type_registry::TypeEntry;
use crate::data::types::AttrValue;
use crate::io::dot::parse::DotReader;

impl DotReader<'_> {
    pub(crate) fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    pub(crate) fn bump(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    pub(crate) fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => self.bump(),
                Some('/') => {
                    if self.starts_with("//") {
                        while let Some(c) = self.peek() {
                            self.bump();
                            if c == '\n' { break; }
                        }
                    } else if self.starts_with("/*") {
                        self.pos += 2;
                        while !self.starts_with("*/")
                            && self.peek().is_some()
                        {
                            self.bump();
                        }
                        if self.starts_with("*/") {
                            self.pos += 2;
                        }
                    } else {
                        return;
                    }
                }
                Some('#') => {
                    while let Some(c) = self.peek() {
                        self.bump();
                        if c == '\n' { break; }
                    }
                }
                _ => return,
            }
        }
    }

    pub(crate) fn starts_with(&self, prefix: &str) -> bool {
        self.src[self.pos..].starts_with(prefix)
    }

    pub(crate) fn eat_kw(&mut self, kw: &str) -> bool {
        if !self.starts_with(kw) {
            return false;
        }
        let after = self.pos + kw.len();
        let next = self.src[after..].chars().next();
        match next {
            Some(c) if c.is_alphanumeric() || c == '_' => false,
            _ => {
                self.pos = after;
                true
            }
        }
    }

    pub(crate) fn eat_char(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(crate) fn read_ident(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                out.push(c);
                self.bump();
            } else {
                break;
            }
        }
        out
    }

    pub(crate) fn read_string(&mut self) -> String {
        let mut out = String::new();
        if self.peek() != Some('"') { return out; }
        self.bump();
        while let Some(c) = self.peek() {
            if c == '\\' {
                self.bump();
                if let Some(n) = self.peek() {
                    out.push(n);
                    self.bump();
                }
            } else if c == '"' {
                self.bump();
                break;
            } else {
                out.push(c);
                self.bump();
            }
        }
        out
    }

    pub(crate) fn read_ident_or_str(&mut self) -> String {
        self.skip_ws();
        match self.peek() {
            Some('"') => self.read_string(),
            _ => self.read_ident(),
        }
    }

    /// Reads either a quoted string value or a numeric/identifier
    /// value; used for graph-level `key=value;` statements.
    pub(crate) fn read_value_string(&mut self) -> String {
        self.skip_ws();
        if let Some('"') = self.peek() { self.read_string() } else {
            let mut out = String::new();
            while let Some(c) = self.peek() {
                if c == ';'
                    || c == ','
                    || c == ']'
                    || c.is_whitespace()
                {
                    break;
                }
                out.push(c);
                self.bump();
            }
            out
        }
    }

    pub(crate) fn read_attr_list(
        &mut self,
    ) -> IndexMap<String, String> {
        let mut out: IndexMap<String, String> = IndexMap::new();
        self.skip_ws();
        if !self.eat_char('[') {
            return out;
        }
        loop {
            self.skip_ws();
            if self.eat_char(']') { break; }
            let k = self.read_ident_or_str();
            self.skip_ws();
            if !self.eat_char('=') {
                // Tolerate stray punct.
                self.bump();
                continue;
            }
            let v = self.read_value_string();
            out.insert(k, v);
            self.skip_ws();
            if self.eat_char(',') { continue; }
            // Optional whitespace separator.
            if self.eat_char(']') { break; }
        }
        out
    }

    /// Decode an `@<c>:` tagged `AttrValue` string. Inverse of
    /// `DotEmit::tag_attr`.
    pub(crate) fn untag_attr(s: &str) -> AttrValue {
        if s == "@_" {
            return AttrValue::None;
        }
        let rest = match s.strip_prefix('@') {
            Some(r) => r,
            None => return AttrValue::Ident(SmolStr::new(s)),
        };
        let (tag, body) = match rest.split_once(':') {
            Some(p) => p,
            None => return AttrValue::Ident(SmolStr::new(s)),
        };
        match tag {
            "b" => AttrValue::Bool(body == "true"),
            "n" => body
                .parse::<i64>()
                .map(AttrValue::Int)
                .unwrap_or(AttrValue::None),
            "f" => body
                .parse::<f64>()
                .map(AttrValue::Float)
                .unwrap_or(AttrValue::None),
            "s" => AttrValue::Str(SmolStr::new(body)),
            "i" => AttrValue::Ident(SmolStr::new(body)),
            "d" => AttrValue::DateTime(SmolStr::new(body)),
            "j" => Self::value_to_attr(
                serde_json::from_str::<Value>(body)
                    .unwrap_or(Value::Null),
            ),
            _ => AttrValue::Ident(SmolStr::new(s)),
        }
    }

    pub(crate) fn assemble_meta(
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
                Some(AttrValue::Str(s)) if s.as_str() != "@_" => {
                    Some(s.clone())
                }
                _ => None,
            }
        };
        let gen = match kv.get("gen") {
            Some(AttrValue::Str(s)) => s.parse::<u64>().unwrap_or(0),
            _ => 0,
        };
        let types = match kv.get("types") {
            Some(AttrValue::Str(s)) if s.as_str() == "@_" => None,
            Some(AttrValue::Str(s)) => Some(s.clone()),
            _ => None,
        };
        GraphMeta::new(
            gen, s("id"), s("name"), s("operators"),
            s("time"), types, s("version"), s("workspace"),
            opt("date"), opt("data_path"), opt("code_path"),
        )
    }

    pub(crate) fn assemble_types(
        s: Option<&str>,
    ) -> Vec<TypeEntry> {
        let s = match s {
            Some(s) if !s.is_empty() => s,
            _ => return Vec::new(),
        };
        let mut out: Vec<TypeEntry> = Vec::new();
        for part in s.split('|') {
            if let Some(rest) = part.strip_prefix('\'') {
                if let Some(end) = rest.find('\'') {
                    let alias = &rest[..end];
                    let after = &rest[end + 1..];
                    if let Some(expr) =
                        after.strip_prefix("->")
                    {
                        out.push(TypeEntry::Alias {
                            alias: SmolStr::new(alias),
                            expr: SmolStr::new(expr),
                        });
                        continue;
                    }
                }
            }
            out.push(TypeEntry::Concrete(SmolStr::new(part)));
        }
        out
    }

    fn value_to_attr(v: Value) -> AttrValue {
        match v {
            Value::Null => AttrValue::None,
            Value::Bool(b) => AttrValue::Bool(b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    AttrValue::Int(i)
                } else if let Some(f) = n.as_f64() {
                    AttrValue::Float(f)
                } else {
                    AttrValue::None
                }
            }
            Value::String(s) => AttrValue::Ident(SmolStr::new(s)),
            Value::Array(a) => AttrValue::List(
                a.into_iter().map(Self::value_to_attr).collect(),
            ),
            Value::Object(m) => {
                if let Some(Value::String(s)) = m.get("$str") {
                    if m.len() == 1 {
                        return AttrValue::Str(SmolStr::new(s));
                    }
                }
                if let Some(Value::String(s)) = m.get("$dt") {
                    if m.len() == 1 {
                        return AttrValue::DateTime(
                            SmolStr::new(s),
                        );
                    }
                }
                let mut d: IndexMap<SmolStr, AttrValue> =
                    IndexMap::new();
                for (k, v) in m {
                    d.insert(SmolStr::new(&k), Self::value_to_attr(v));
                }
                AttrValue::Dict(d)
            }
        }
    }
}
