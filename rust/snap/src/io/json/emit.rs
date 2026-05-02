//! JSON emit. The single free fn `emit` is the documented bridge from
//! `Json::emit` to the carrier methods — the recognized exception to
//! the no-free-fn rule for breaking up a single conceptual carrier
//! across multiple files.
//!
//! Schema (`BTreeMap` keys = alphabetic ordering on emit):
//! ```json
//! {
//!   "snap": {
//!     "version": "0.6", "handle": "demo",
//!     "graph": { "gen": 0, "id": "g001", ... },
//!     "edges": { "flow": [ {"src":"a","tgt":"b"}, ... ] },
//!     "extras": {}, "layout": {}, "literals": {},
//!     "nodes": { "object": [ ... ] },
//!     "registers": {}, "streams": {}, "types": ["T"]
//!   }
//! }
//! ```

use std::collections::BTreeMap;

use indexmap::IndexMap;
use serde_json::{Map, Value};
use smol_str::SmolStr;

use crate::codec::weight_text::WeightText;
use crate::data::edges::Edges;
use crate::data::graph::Graph;
use crate::data::literals::LiteralEntry;
use crate::data::meta::GraphMeta;
use crate::data::nodes::Nodes;
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::{TypeEntry, TypeRegistry};
use crate::data::types::{AttrValue, NodeKind};
use crate::data::weight::EdgeWeight;

/// Crate-private entry: bridge from `Json::emit` to the carrier.
pub(crate) fn emit(g: &Graph) -> String {
    JsonEmit::emit(g)
}

pub(crate) struct JsonEmit;

impl JsonEmit {
    pub(crate) fn emit(g: &Graph) -> String {
        let snap = Self::build_snap(g);
        let mut root = Map::new();
        root.insert("snap".into(), snap);
        serde_json::to_string_pretty(&Value::Object(root))
            .unwrap_or_default()
    }

    fn build_snap(g: &Graph) -> Value {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(
            "version".into(),
            Value::String(g.meta().version.to_string()),
        );
        if let Some(h) = g.handle() {
            map.insert("handle".into(), Value::String(h.to_string()));
        }
        map.insert("graph".into(), Self::meta_v(g.meta()));
        map.insert(
            "edges".into(),
            Self::edges_v(g.edges(), g.nodes()),
        );
        map.insert("extras".into(), Self::extras_v(g.extras()));
        map.insert("layout".into(), Self::layout_v(g.layout()));
        map.insert(
            "literals".into(),
            Self::literals_v(g.literals()),
        );
        map.insert("nodes".into(), Self::nodes_v(g.nodes()));
        map.insert(
            "registers".into(),
            Self::registers_v(g.registers()),
        );
        map.insert("streams".into(), Self::streams_v(g.streams()));
        map.insert("types".into(), Self::types_v(g.types()));
        Self::btree_to_value(map)
    }

    fn btree_to_value(map: BTreeMap<String, Value>) -> Value {
        let mut out = Map::new();
        for (k, v) in map {
            out.insert(k, v);
        }
        Value::Object(out)
    }

    fn meta_v(m: &GraphMeta) -> Value {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert("gen".into(), Value::from(m.gen));
        map.insert("id".into(), Value::String(m.id.to_string()));
        map.insert("name".into(), Value::String(m.name.to_string()));
        map.insert(
            "operators".into(),
            Value::String(m.operators.to_string()),
        );
        map.insert("time".into(), Value::String(m.time.to_string()));
        match &m.types {
            None => map.insert("types".into(), Value::Null),
            Some(s) => map.insert(
                "types".into(),
                Value::String(s.to_string()),
            ),
        };
        map.insert(
            "version".into(),
            Value::String(m.version.to_string()),
        );
        map.insert(
            "workspace".into(),
            Value::String(m.workspace.to_string()),
        );
        if let Some(s) = &m.code_path {
            map.insert(
                "code_path".into(),
                Value::String(s.to_string()),
            );
        }
        if let Some(s) = &m.data_path {
            map.insert(
                "data_path".into(),
                Value::String(s.to_string()),
            );
        }
        if let Some(s) = &m.date {
            map.insert("date".into(), Value::String(s.to_string()));
        }
        Self::btree_to_value(map)
    }

    fn edges_v(edges: &Edges, nodes: &Nodes) -> Value {
        let groups = Self::group_edges(edges, nodes);
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        for (fam, rows) in groups {
            let arr: Vec<Value> = rows
                .into_iter()
                .map(|(src, tgt, w)| {
                    let mut e: BTreeMap<String, Value> =
                        BTreeMap::new();
                    e.insert("src".into(), Value::String(src.into()));
                    e.insert("tgt".into(), Value::String(tgt.into()));
                    if !w.is_none() {
                        e.insert(
                            "w".into(),
                            Value::String(WeightText::emit(&w)),
                        );
                    }
                    Self::btree_to_value(e)
                })
                .collect();
            map.insert(fam.to_string(), Value::Array(arr));
        }
        Self::btree_to_value(map)
    }

    fn group_edges(
        edges: &Edges,
        nodes: &Nodes,
    ) -> BTreeMap<SmolStr, Vec<(SmolStr, SmolStr, EdgeWeight)>> {
        let mut groups: BTreeMap<
            SmolStr,
            Vec<(SmolStr, SmolStr, EdgeWeight)>,
        > = BTreeMap::new();
        let n = nodes.len();
        let id_at = |i: usize| -> Option<SmolStr> {
            nodes.iter().nth(i).map(|n| n.data.id.clone())
        };
        for src_ix in 0..n {
            let src_u = match u32::try_from(src_ix) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let outs = edges.out_edges(src_u);
            let ws = edges.out_weights(src_u);
            let fams = edges.out_families(src_u);
            let src_id = match id_at(src_ix) {
                Some(s) => s,
                None => continue,
            };
            for i in 0..outs.len() {
                let t_ix = match outs.get(i).copied() {
                    Some(v) => v as usize,
                    None => continue,
                };
                let tgt_id = match id_at(t_ix) {
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

    fn extras_v(extras: &IndexMap<SmolStr, AttrValue>) -> Value {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        for (k, v) in extras {
            map.insert(k.to_string(), Self::attr_v(v));
        }
        Self::btree_to_value(map)
    }

    fn layout_v(layout: &IndexMap<SmolStr, (f64, f64)>) -> Value {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        for (k, (x, y)) in layout {
            let arr = vec![
                Value::from(*x),
                Value::from(*y),
            ];
            map.insert(k.to_string(), Value::Array(arr));
        }
        Self::btree_to_value(map)
    }

    fn literals_v(
        literals: &IndexMap<SmolStr, LiteralEntry>,
    ) -> Value {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        for (k, v) in literals {
            let mut item: BTreeMap<String, Value> = BTreeMap::new();
            item.insert("id".into(), Value::String(v.id.to_string()));
            item.insert(
                "type".into(),
                Value::String(v.type_name.to_string()),
            );
            item.insert("value".into(), Self::attr_v(&v.value));
            map.insert(k.to_string(), Self::btree_to_value(item));
        }
        Self::btree_to_value(map)
    }

    fn nodes_v(nodes: &Nodes) -> Value {
        let mut by_kind: BTreeMap<String, Vec<Value>> =
            BTreeMap::new();
        for nref in nodes.iter() {
            let nd = nref.data;
            let kind_s = Self::kind_str(&nd.kind).to_string();
            let mut item: BTreeMap<String, Value> = BTreeMap::new();
            item.insert("id".into(), Value::String(nd.id.to_string()));
            if let Some(name) = &nd.name {
                item.insert(
                    "name".into(),
                    Value::String(name.to_string()),
                );
            }
            if let Some(w) = &nd.weight {
                item.insert(
                    "weight".into(),
                    Value::String(WeightText::emit(w)),
                );
            }
            let mut attrs: BTreeMap<String, Value> = BTreeMap::new();
            for (k, v) in &nd.attrs {
                attrs.insert(k.to_string(), Self::attr_v(v));
            }
            if !attrs.is_empty() {
                item.insert(
                    "attrs".into(),
                    Self::btree_to_value(attrs),
                );
            }
            by_kind
                .entry(kind_s)
                .or_default()
                .push(Self::btree_to_value(item));
        }
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        for (k, v) in by_kind {
            map.insert(k, Value::Array(v));
        }
        Self::btree_to_value(map)
    }

    fn kind_str(k: &NodeKind) -> &str {
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

    fn registers_v(
        regs: &IndexMap<SmolStr, RegisterEntry>,
    ) -> Value {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        for (k, v) in regs {
            let mut item: BTreeMap<String, Value> = BTreeMap::new();
            item.insert("id".into(), Value::String(v.id.to_string()));
            item.insert(
                "type".into(),
                Value::String(v.type_name.to_string()),
            );
            map.insert(k.to_string(), Self::btree_to_value(item));
        }
        Self::btree_to_value(map)
    }

    fn streams_v(
        streams: &IndexMap<SmolStr, StreamEntry>,
    ) -> Value {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        for (k, v) in streams {
            let mut item: BTreeMap<String, Value> = BTreeMap::new();
            item.insert("id".into(), Value::String(v.id.to_string()));
            item.insert("len".into(), Value::from(v.data.len()));
            map.insert(k.to_string(), Self::btree_to_value(item));
        }
        Self::btree_to_value(map)
    }

    fn types_v(types: &TypeRegistry) -> Value {
        let mut arr: Vec<Value> = Vec::new();
        for entry in types.entries() {
            match entry {
                TypeEntry::Concrete(n) => {
                    arr.push(Value::String(n.to_string()));
                }
                TypeEntry::Alias { alias, expr } => {
                    let mut item: BTreeMap<String, Value> =
                        BTreeMap::new();
                    item.insert(
                        "alias".into(),
                        Value::String(alias.to_string()),
                    );
                    item.insert(
                        "expr".into(),
                        Value::String(expr.to_string()),
                    );
                    arr.push(Self::btree_to_value(item));
                }
            }
        }
        Value::Array(arr)
    }

    fn attr_v(v: &AttrValue) -> Value {
        match v {
            AttrValue::None => Value::Null,
            AttrValue::Bool(b) => Value::Bool(*b),
            AttrValue::Int(i) => Value::from(*i),
            AttrValue::Float(f) => Value::from(*f),
            AttrValue::Str(s) => {
                let mut m = Map::new();
                m.insert("$str".into(), Value::String(s.to_string()));
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
            AttrValue::List(items) => {
                let arr: Vec<Value> =
                    items.iter().map(Self::attr_v).collect();
                Value::Array(arr)
            }
            AttrValue::Dict(d) => {
                let mut map: BTreeMap<String, Value> =
                    BTreeMap::new();
                for (k, v) in d {
                    map.insert(k.to_string(), Self::attr_v(v));
                }
                Self::btree_to_value(map)
            }
        }
    }
}
