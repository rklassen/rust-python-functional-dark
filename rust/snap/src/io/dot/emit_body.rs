//! Continuation of `impl DotEmit` for edge grouping, `AttrValue` tag
//! encoding, and small render helpers. Split out of `emit.rs` to
//! honor the per-file 432-line ceiling. This file is named after the
//! `DotEmit` carrier because it CONTINUES that impl block — file/
//! struct alignment preserved.

use indexmap::IndexMap;
use serde_json::{Map, Value};
use smol_str::SmolStr;

use crate::data::edges::Edges;
use crate::data::nodes::Nodes;
use crate::data::types::{AttrValue, NodeKind};
use crate::data::weight::EdgeWeight;
use crate::io::dot::emit::DotEmit;

impl DotEmit {
    pub(crate) fn group_edges(
        edges: &Edges,
        nodes: &Nodes,
    ) -> IndexMap<SmolStr, Vec<(SmolStr, SmolStr, EdgeWeight)>> {
        let mut groups: IndexMap<
            SmolStr,
            Vec<(SmolStr, SmolStr, EdgeWeight)>,
        > = IndexMap::new();
        let id_at = |i: usize| -> Option<SmolStr> {
            nodes.iter().nth(i).map(|n| n.data.id.clone())
        };
        for src_ix in 0..nodes.len() {
            let su = match u32::try_from(src_ix) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let outs = edges.out_edges(su);
            let ws = edges.out_weights(su);
            let fams = edges.out_families(su);
            let src_id = match id_at(src_ix) {
                Some(s) => s,
                None => continue,
            };
            for i in 0..outs.len() {
                let t = match outs.get(i).copied() {
                    Some(v) => v as usize,
                    None => continue,
                };
                let tgt_id = match id_at(t) {
                    Some(s) => s,
                    None => continue,
                };
                let fam = match fams.get(i) {
                    Some(f) => f.clone(),
                    None => continue,
                };
                let w = match ws.get(i) {
                    Some(w) => w.clone(),
                    None => continue,
                };
                groups
                    .entry(fam)
                    .or_default()
                    .push((src_id.clone(), tgt_id, w));
            }
        }
        for rows in groups.values_mut() {
            rows.sort_by(|a, b| {
                a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1))
            });
        }
        groups
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

    /// Tag an `AttrValue` into a single string. The leading `@<c>:`
    /// (or `@_`) marks the variant. List/Dict fall back to JSON.
    pub(crate) fn tag_attr(v: &AttrValue) -> String {
        match v {
            AttrValue::None => "@_".to_string(),
            AttrValue::Bool(b) => {
                format!("@b:{}", if *b { "true" } else { "false" })
            }
            AttrValue::Int(i) => format!("@n:{i}"),
            AttrValue::Float(f) => {
                format!("@f:{}", Self::render_float(*f))
            }
            AttrValue::Str(s) => format!("@s:{s}"),
            AttrValue::Ident(s) => format!("@i:{s}"),
            AttrValue::DateTime(s) => format!("@d:{s}"),
            AttrValue::List(_) | AttrValue::Dict(_) => {
                let val = Self::attr_to_value(v);
                let json = serde_json::to_string(&val)
                    .unwrap_or_default();
                format!("@j:{json}")
            }
        }
    }

    fn attr_to_value(v: &AttrValue) -> Value {
        match v {
            AttrValue::None => Value::Null,
            AttrValue::Bool(b) => Value::Bool(*b),
            AttrValue::Int(i) => Value::from(*i),
            AttrValue::Float(f) => Value::from(*f),
            AttrValue::Str(s) => {
                let mut m = Map::new();
                m.insert(
                    "$str".into(),
                    Value::String(s.to_string()),
                );
                Value::Object(m)
            }
            AttrValue::Ident(s) => Value::String(s.to_string()),
            AttrValue::DateTime(s) => {
                let mut m = Map::new();
                m.insert(
                    "$dt".into(),
                    Value::String(s.to_string()),
                );
                Value::Object(m)
            }
            AttrValue::List(items) => Value::Array(
                items.iter().map(Self::attr_to_value).collect(),
            ),
            AttrValue::Dict(d) => {
                let mut m = Map::new();
                for (k, v) in d {
                    m.insert(
                        k.to_string(),
                        Self::attr_to_value(v),
                    );
                }
                Value::Object(m)
            }
        }
    }

    // f.fract()==0 && finite ensures f is integer-valued and within
    // weight bounds (well inside i64 range).
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn render_float(f: f64) -> String {
        if f.fract() == 0.0 && f.is_finite() {
            format!("{}", f as i64)
        } else {
            format!("{f}")
        }
    }
}
