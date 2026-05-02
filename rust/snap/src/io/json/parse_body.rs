//! Continuation of `impl JsonParse` for the per-section parsers and
//! the `AttrValue` mapper. Split out of `parse.rs` to honor the
//! per-file 432-line ceiling. This file is named after the `JsonParse`
//! carrier because it CONTINUES that impl block — file/struct
//! alignment preserved.

use indexmap::IndexMap;
use serde_json::Value;
use smol_str::SmolStr;

use crate::data::literals::LiteralEntry;
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::TypeEntry;
use crate::data::types::AttrValue;
use crate::io::json::parse::{Errs, JsonParse};

impl JsonParse {
    pub(crate) fn p_kv(
        v: &Value,
        errs: &mut Errs,
    ) -> IndexMap<SmolStr, AttrValue> {
        let mut out: IndexMap<SmolStr, AttrValue> = IndexMap::new();
        let obj = if let Some(o) = v.as_object() { o } else {
            errs.push(Self::e(
                "expected an object",
                "an object of attributes",
            ));
            return out;
        };
        for (k, v) in obj {
            out.insert(SmolStr::new(k), Self::attr(v));
        }
        out
    }

    pub(crate) fn p_layout(
        v: &Value,
        errs: &mut Errs,
    ) -> IndexMap<SmolStr, (f64, f64)> {
        let mut out: IndexMap<SmolStr, (f64, f64)> = IndexMap::new();
        let obj = if let Some(o) = v.as_object() { o } else {
            errs.push(Self::e(
                "`layout` is not an object",
                "an object of [x,y] arrays",
            ));
            return out;
        };
        for (k, v) in obj {
            let arr = match v.as_array() {
                Some(a) if a.len() == 2 => a,
                _ => {
                    errs.push(Self::e(
                        "layout entry not a 2-array",
                        "[x, y]",
                    ));
                    continue;
                }
            };
            let x = arr.first()
                .and_then(serde_json::Value::as_f64).unwrap_or(0.0);
            let y = arr.get(1)
                .and_then(serde_json::Value::as_f64).unwrap_or(0.0);
            out.insert(SmolStr::new(k), (x, y));
        }
        out
    }

    pub(crate) fn p_literals(
        v: &Value,
        errs: &mut Errs,
    ) -> IndexMap<SmolStr, LiteralEntry> {
        let mut out: IndexMap<SmolStr, LiteralEntry> = IndexMap::new();
        let obj = if let Some(o) = v.as_object() { o } else {
            errs.push(Self::e(
                "`literals` is not an object",
                "an object of literal entries",
            ));
            return out;
        };
        for (k, v) in obj {
            let m = match v.as_object() {
                Some(m) => m,
                None => continue,
            };
            let id = m
                .get("id")
                .and_then(|v| v.as_str())
                .map(SmolStr::new)
                .unwrap_or_default();
            let type_name = m
                .get("type")
                .and_then(|v| v.as_str())
                .map(SmolStr::new)
                .unwrap_or_default();
            let value = m
                .get("value")
                .map_or(AttrValue::None, Self::attr);
            out.insert(
                SmolStr::new(k),
                LiteralEntry::new(
                    SmolStr::new(k),
                    id,
                    type_name,
                    value,
                ),
            );
        }
        out
    }

    pub(crate) fn p_registers(
        v: &Value,
        errs: &mut Errs,
    ) -> IndexMap<SmolStr, RegisterEntry> {
        let mut out: IndexMap<SmolStr, RegisterEntry> = IndexMap::new();
        let obj = if let Some(o) = v.as_object() { o } else {
            errs.push(Self::e(
                "`registers` is not an object",
                "an object of register entries",
            ));
            return out;
        };
        for (k, v) in obj {
            let m = match v.as_object() {
                Some(m) => m,
                None => continue,
            };
            let id = m
                .get("id")
                .and_then(|v| v.as_str())
                .map(SmolStr::new)
                .unwrap_or_default();
            let type_name = m
                .get("type")
                .and_then(|v| v.as_str())
                .map(SmolStr::new)
                .unwrap_or_default();
            out.insert(
                SmolStr::new(k),
                RegisterEntry::new(SmolStr::new(k), id, type_name),
            );
        }
        out
    }

    pub(crate) fn p_streams(
        v: &Value,
        errs: &mut Errs,
    ) -> IndexMap<SmolStr, StreamEntry> {
        let mut out: IndexMap<SmolStr, StreamEntry> = IndexMap::new();
        let obj = if let Some(o) = v.as_object() { o } else {
            errs.push(Self::e(
                "`streams` is not an object",
                "an object of stream entries",
            ));
            return out;
        };
        for (k, v) in obj {
            let m = match v.as_object() {
                Some(m) => m,
                None => continue,
            };
            let id = m
                .get("id")
                .and_then(|v| v.as_str())
                .map(SmolStr::new)
                .unwrap_or_default();
            #[allow(clippy::cast_possible_truncation)]
            let len = m
                .get("len")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;
            out.insert(
                SmolStr::new(k),
                StreamEntry::new(id, None, vec![0u8; len]),
            );
        }
        out
    }

    pub(crate) fn p_types(
        v: &Value,
        errs: &mut Errs,
    ) -> Vec<TypeEntry> {
        let mut out: Vec<TypeEntry> = Vec::new();
        let arr = if let Some(a) = v.as_array() { a } else {
            errs.push(Self::e(
                "`types` is not an array",
                "an array of strings or alias objects",
            ));
            return out;
        };
        for item in arr {
            if let Some(s) = item.as_str() {
                out.push(TypeEntry::Concrete(SmolStr::new(s)));
            } else if let Some(m) = item.as_object() {
                let alias = m
                    .get("alias")
                    .and_then(|v| v.as_str())
                    .map(SmolStr::new)
                    .unwrap_or_default();
                let expr = m
                    .get("expr")
                    .and_then(|v| v.as_str())
                    .map(SmolStr::new)
                    .unwrap_or_default();
                out.push(TypeEntry::Alias { alias, expr });
            }
        }
        out
    }

    pub(crate) fn attr(v: &Value) -> AttrValue {
        match v {
            Value::Null => AttrValue::None,
            Value::Bool(b) => AttrValue::Bool(*b),
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
            Value::Array(a) => {
                AttrValue::List(a.iter().map(Self::attr).collect())
            }
            Value::Object(m) => {
                if let Some(s) =
                    m.get("$str").and_then(|v| v.as_str())
                {
                    if m.len() == 1 {
                        return AttrValue::Str(SmolStr::new(s));
                    }
                }
                if let Some(s) =
                    m.get("$dt").and_then(|v| v.as_str())
                {
                    if m.len() == 1 {
                        return AttrValue::DateTime(SmolStr::new(s));
                    }
                }
                let mut d: IndexMap<SmolStr, AttrValue> =
                    IndexMap::new();
                for (k, v) in m {
                    d.insert(SmolStr::new(k), Self::attr(v));
                }
                AttrValue::Dict(d)
            }
        }
    }
}
