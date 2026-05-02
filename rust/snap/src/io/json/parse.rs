//! JSON parse entry + top-level dispatch + meta/nodes/edges parsers.
//! Section parsers for extras/layout/literals/registers/streams/types
//! and the `AttrValue` mapper live in `parse_body` as a continuation
//! `impl JsonParse` block to honor the per-file 432-line ceiling.
//!
//! The single free fn `parse` is the documented bridge from
//! `Json::parse` into the carrier — the recognized exception to the
//! no-free-fn rule.
//!
//! Weight strings are parsed via
//! `crate::codec::weight_text::WeightText::parse` — codec layer is
//! the single source of truth for weight format.

use indexmap::IndexMap;
use serde_json::Value;
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

pub(crate) fn parse(input: &str) -> Result<Graph, Vec<SemanticErr>> {
    JsonParse::parse(input)
}

pub(crate) struct JsonParse;

pub(crate) type Errs = Vec<SemanticErr>;

impl JsonParse {
    pub(crate) fn parse(input: &str) -> Result<Graph, Errs> {
        let root: Value = serde_json::from_str(input)
            .map_err(|e| vec![Self::err_root(&e.to_string())])?;
        let snap = match root.get("snap") {
            Some(v) => v,
            None => {
                return Err(vec![Self::e(
                    "missing top-level `snap` key",
                    "an object with a `snap` member",
                )]);
            }
        };
        let snap_obj = match snap.as_object() {
            Some(o) => o,
            None => {
                return Err(vec![Self::e(
                    "`snap` is not an object",
                    "an object",
                )]);
            }
        };

        let mut errs: Errs = Vec::new();

        let handle = snap_obj
            .get("handle")
            .and_then(|v| v.as_str())
            .map(SmolStr::new);

        let meta = snap_obj.get("graph").map_or_else(
            || {
                GraphMeta::minimal(
                    SmolStr::new("a000"),
                    SmolStr::new("0.6"),
                )
            },
            |v| Self::p_meta(v, &mut errs),
        );

        let node_defs = snap_obj
            .get("nodes")
            .map(|v| Self::p_nodes(v, &mut errs))
            .unwrap_or_default();
        let edge_defs = snap_obj
            .get("edges")
            .map(|v| Self::p_edges(v, &mut errs))
            .unwrap_or_default();
        let extras = snap_obj
            .get("extras")
            .map(|v| Self::p_kv(v, &mut errs))
            .unwrap_or_default();
        let layout = snap_obj
            .get("layout")
            .map(|v| Self::p_layout(v, &mut errs))
            .unwrap_or_default();
        let literals = snap_obj
            .get("literals")
            .map(|v| Self::p_literals(v, &mut errs))
            .unwrap_or_default();
        let registers = snap_obj
            .get("registers")
            .map(|v| Self::p_registers(v, &mut errs))
            .unwrap_or_default();
        let streams = snap_obj
            .get("streams")
            .map(|v| Self::p_streams(v, &mut errs))
            .unwrap_or_default();
        let types = snap_obj
            .get("types")
            .map(|v| Self::p_types(v, &mut errs))
            .unwrap_or_default();

        if !errs.is_empty() {
            return Err(errs);
        }
        let nodes = Nodes::new(node_defs).map_err(|e| vec![e])?;
        let edge_results = crate::data::edges::Edges::new(
            &nodes, edge_defs,
        );
        Graph::with_sections(
            meta,
            handle,
            nodes,
            edge_results,
            extras,
            layout,
            literals,
            registers,
            streams,
            TypeRegistry::new(types),
        )
    }

    fn p_meta(v: &Value, errs: &mut Errs) -> GraphMeta {
        let m = if let Some(m) = v.as_object() { m } else {
            errs.push(Self::e(
                "`graph` is not an object",
                "an object of meta fields",
            ));
            return GraphMeta::minimal(
                SmolStr::new("a000"),
                SmolStr::new("0.6"),
            );
        };
        let s = |k: &str| -> SmolStr {
            m.get(k)
                .and_then(|v| v.as_str())
                .map(SmolStr::new)
                .unwrap_or_default()
        };
        let opt = |k: &str| -> Option<SmolStr> {
            m.get(k).and_then(|v| v.as_str()).map(SmolStr::new)
        };
        let gen = m
            .get("gen")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let types = match m.get("types") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(SmolStr::new(s)),
            _ => None,
        };
        GraphMeta::new(
            gen, s("id"), s("name"), s("operators"),
            s("time"), types, s("version"), s("workspace"),
            opt("date"), opt("data_path"), opt("code_path"),
        )
    }

    fn p_nodes(v: &Value, errs: &mut Errs) -> Vec<NodeDef> {
        let mut out: Vec<NodeDef> = Vec::new();
        let obj = if let Some(o) = v.as_object() { o } else {
            errs.push(Self::e(
                "`nodes` is not an object",
                "an object keyed by node-kind",
            ));
            return out;
        };
        for (kind_s, arr_v) in obj {
            let arr = if let Some(a) = arr_v.as_array() { a } else {
                errs.push(Self::e(
                    "node-kind value is not an array",
                    "an array of node objects",
                ));
                continue;
            };
            let kind = Self::node_kind(kind_s);
            for item in arr {
                if let Some(d) = Self::p_node_item(&kind, item, errs) {
                    out.push(d);
                }
            }
        }
        out
    }

    fn p_node_item(
        kind: &NodeKind,
        item: &Value,
        errs: &mut Errs,
    ) -> Option<NodeDef> {
        let m = if let Some(m) = item.as_object() { m } else {
            errs.push(Self::e(
                "node entry is not an object",
                "an object with id/name/attrs",
            ));
            return None;
        };
        let id = if let Some(s) =
            m.get("id").and_then(|v| v.as_str())
        {
            SmolStr::new(s)
        } else {
            errs.push(Self::e(
                "missing node `id`",
                "a string node id",
            ));
            return None;
        };
        let name = m
            .get("name")
            .and_then(|v| v.as_str())
            .map(SmolStr::new);
        let mut attrs: IndexMap<SmolStr, AttrValue> = IndexMap::new();
        if let Some(a) = m.get("attrs").and_then(|v| v.as_object()) {
            for (k, v) in a {
                attrs.insert(SmolStr::new(k), Self::attr(v));
            }
        }
        Some(NodeDef {
            id,
            kind: kind.clone(),
            name,
            attrs,
        })
    }

    fn p_edges(v: &Value, errs: &mut Errs) -> Vec<EdgeDef> {
        let mut out: Vec<EdgeDef> = Vec::new();
        let obj = if let Some(o) = v.as_object() { o } else {
            errs.push(Self::e(
                "`edges` is not an object",
                "an object keyed by family",
            ));
            return out;
        };
        for (fam, arr_v) in obj {
            let arr = if let Some(a) = arr_v.as_array() { a } else {
                errs.push(Self::e(
                    "edge family value is not an array",
                    "an array of edge objects",
                ));
                continue;
            };
            let family = SmolStr::new(fam);
            for item in arr {
                if let Some(d) =
                    Self::p_edge_item(&family, item, errs)
                {
                    out.push(d);
                }
            }
        }
        out
    }

    fn p_edge_item(
        family: &SmolStr,
        item: &Value,
        errs: &mut Errs,
    ) -> Option<EdgeDef> {
        let m = if let Some(m) = item.as_object() { m } else {
            errs.push(Self::e(
                "edge entry is not an object",
                "an object with src/tgt[/w]",
            ));
            return None;
        };
        let src = if let Some(s) =
            m.get("src").and_then(|v| v.as_str())
        {
            SmolStr::new(s)
        } else {
            errs.push(Self::e(
                "missing edge `src`",
                "a string source id",
            ));
            return None;
        };
        let tgt = if let Some(s) =
            m.get("tgt").and_then(|v| v.as_str())
        {
            SmolStr::new(s)
        } else {
            errs.push(Self::e(
                "missing edge `tgt`",
                "a string target id",
            ));
            return None;
        };
        let weight = match m.get("w").and_then(|v| v.as_str()) {
            None => EdgeWeight::None,
            Some(s) => match WeightText::parse(
                s,
                crate::data::types::NumericEncoding::Float,
            ) {
                Ok(w) => w,
                Err(es) => {
                    errs.extend(es);
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

    fn err_root(reason: &str) -> SemanticErr {
        SemanticErr::new(
            format!("invalid JSON: {reason}"),
            Some("syntactically valid JSON".into()),
            NonEmpty::with_tail(
                "fix the JSON syntax".into(),
                vec!["validate against the snap JSON schema".into()],
            ),
        )
    }

    pub(crate) fn e(found: &str, expected: &str) -> SemanticErr {
        SemanticErr::new(
            found.into(),
            Some(expected.into()),
            NonEmpty::with_tail(
                "match the snap JSON schema".into(),
                vec!["see io::json::emit for the schema".into()],
            ),
        )
    }
}
