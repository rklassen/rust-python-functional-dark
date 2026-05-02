//! XML emit. The single free fn `emit` is the documented bridge from
//! `Xml::emit` to the carrier methods — the recognized exception to
//! the no-free-fn rule for breaking up a single conceptual carrier
//! across multiple files.
//!
//! Schema:
//! ```xml
//! <snap version="0.6" handle="demo">
//!   <graph><gen>0</gen><id>g001</id>...</graph>
//!   <edges>
//!     <family name="flow">
//!       <edge src="a001" tgt="b002" w="[0.1, 0.5, 0.9]:unorm"/>
//!     </family>
//!   </edges>
//!   <extras/><layout/><literals/>
//!   <nodes>
//!     <object id="a001" name="A">
//!       <attr key="type">T</attr>
//!     </object>
//!   </nodes>
//!   <registers/><streams/>
//!   <types><type name="T"/></types>
//! </snap>
//! ```
//!
//! `<attr>` content distinguishes `AttrValue::Str` via a `quoted="true"`
//! boolean attribute; bare-text content is `Ident`. `<dt>...</dt>`
//! marks `DateTime`. `<list>` / `<dict>` mark composites.

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::codec::weight_text::WeightText;
use crate::data::edges::Edges;
use crate::data::graph::Graph;
use crate::data::meta::GraphMeta;
use crate::data::nodes::Nodes;
use crate::data::types::AttrValue;
use crate::data::weight::EdgeWeight;

pub(crate) fn emit(g: &Graph) -> String {
    let mut out = String::with_capacity(256);
    XmlEmit::write_graph(&mut out, g);
    out
}

pub(crate) struct XmlEmit;

impl XmlEmit {
    fn write_graph(out: &mut String, g: &Graph) {
        out.push_str("<snap version=\"");
        out.push_str(&Self::esc_attr(&g.meta().version));
        out.push('"');
        if let Some(h) = g.handle() {
            out.push_str(" handle=\"");
            out.push_str(&Self::esc_attr(h));
            out.push('"');
        }
        out.push_str(">\n");
        Self::w_meta(out, g.meta());
        Self::w_edges(out, g.edges(), g.nodes());
        Self::w_kv_section(out, "extras", g.extras());
        Self::w_layout(out, g.layout());
        Self::w_literals(out, g.literals());
        Self::w_nodes(out, g.nodes());
        Self::w_registers(out, g.registers());
        Self::w_streams(out, g.streams());
        Self::w_types(out, g.types());
        out.push_str("</snap>\n");
    }

    fn w_meta(out: &mut String, m: &GraphMeta) {
        out.push_str("  <graph>\n");
        Self::tag_text(out, "    ", "gen", &format!("{}", m.gen));
        Self::tag_text(out, "    ", "id", &m.id);
        Self::tag_text(out, "    ", "name", &m.name);
        Self::tag_text(out, "    ", "operators", &m.operators);
        Self::tag_text(out, "    ", "time", &m.time);
        match &m.types {
            None => out.push_str("    <types xsi:nil=\"true\"/>\n"),
            Some(s) => Self::tag_text(out, "    ", "types", s),
        }
        Self::tag_text(out, "    ", "version", &m.version);
        Self::tag_text(out, "    ", "workspace", &m.workspace);
        if let Some(s) = &m.code_path {
            Self::tag_text(out, "    ", "code_path", s);
        }
        if let Some(s) = &m.data_path {
            Self::tag_text(out, "    ", "data_path", s);
        }
        if let Some(s) = &m.date {
            Self::tag_text(out, "    ", "date", s);
        }
        out.push_str("  </graph>\n");
    }

    fn tag_text(out: &mut String, ind: &str, tag: &str, v: &str) {
        out.push_str(ind);
        out.push('<');
        out.push_str(tag);
        out.push('>');
        out.push_str(&Self::esc_text(v));
        out.push_str("</");
        out.push_str(tag);
        out.push_str(">\n");
    }

    fn w_edges(out: &mut String, edges: &Edges, nodes: &Nodes) {
        if edges.is_empty() {
            out.push_str("  <edges/>\n");
            return;
        }
        out.push_str("  <edges>\n");
        let groups = Self::group_edges(edges, nodes);
        let mut fams: Vec<&SmolStr> = groups.keys().collect();
        fams.sort_unstable();
        for fam in fams {
            out.push_str("    <family name=\"");
            out.push_str(&Self::esc_attr(fam));
            out.push_str("\">\n");
            let rows = match groups.get(fam) {
                Some(r) => r,
                None => continue,
            };
            for (src, tgt, w) in rows {
                out.push_str("      <edge src=\"");
                out.push_str(&Self::esc_attr(src));
                out.push_str("\" tgt=\"");
                out.push_str(&Self::esc_attr(tgt));
                out.push('"');
                if !w.is_none() {
                    out.push_str(" w=\"");
                    out.push_str(&Self::esc_attr(&WeightText::emit(w)));
                    out.push('"');
                }
                out.push_str("/>\n");
            }
            out.push_str("    </family>\n");
        }
        out.push_str("  </edges>\n");
    }

    fn group_edges(
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

    fn w_kv_section(
        out: &mut String,
        name: &str,
        m: &IndexMap<SmolStr, AttrValue>,
    ) {
        if m.is_empty() {
            out.push_str("  <");
            out.push_str(name);
            out.push_str("/>\n");
            return;
        }
        out.push_str("  <");
        out.push_str(name);
        out.push_str(">\n");
        let mut keys: Vec<&SmolStr> = m.keys().collect();
        keys.sort_unstable();
        for k in keys {
            if let Some(v) = m.get(k) {
                Self::w_attr(out, "    ", k, v);
            }
        }
        out.push_str("  </");
        out.push_str(name);
        out.push_str(">\n");
    }

    pub(crate) fn w_attr(
        out: &mut String,
        ind: &str,
        key: &str,
        v: &AttrValue,
    ) {
        out.push_str(ind);
        out.push_str("<attr key=\"");
        out.push_str(&Self::esc_attr(key));
        out.push('"');
        Self::w_attr_body(out, ind, v, "attr");
    }

    /// `tag` selects the closing tag (`attr` for attribute slots,
    /// `item` for list elements). Recurses into nested
    /// list/dict bodies.
    pub(crate) fn w_attr_body(
        out: &mut String,
        ind: &str,
        v: &AttrValue,
        tag: &str,
    ) {
        let close = |o: &mut String| {
            o.push_str("</");
            o.push_str(tag);
            o.push_str(">\n");
        };
        match v {
            AttrValue::None => out.push_str(" nil=\"true\"/>\n"),
            AttrValue::Bool(b) => {
                out.push_str(" type=\"bool\">");
                out.push_str(if *b { "true" } else { "false" });
                close(out);
            }
            AttrValue::Int(i) => {
                out.push_str(" type=\"int\">");
                out.push_str(&format!("{i}"));
                close(out);
            }
            AttrValue::Float(f) => {
                out.push_str(" type=\"float\">");
                out.push_str(&Self::render_float(*f));
                close(out);
            }
            AttrValue::Str(s) => {
                out.push_str(" type=\"str\">");
                out.push_str(&Self::esc_text(s));
                close(out);
            }
            AttrValue::Ident(s) => {
                out.push('>');
                out.push_str(&Self::esc_text(s));
                close(out);
            }
            AttrValue::DateTime(s) => {
                out.push_str(" type=\"dt\">");
                out.push_str(&Self::esc_text(s));
                close(out);
            }
            AttrValue::List(items) => {
                out.push_str(" type=\"list\">\n");
                let inner = format!("{ind}  ");
                for it in items {
                    out.push_str(&inner);
                    out.push_str("<item");
                    Self::w_attr_body(out, &inner, it, "item");
                }
                out.push_str(ind);
                close(out);
            }
            AttrValue::Dict(d) => {
                out.push_str(" type=\"dict\">\n");
                let inner = format!("{ind}  ");
                let mut ks: Vec<&SmolStr> = d.keys().collect();
                ks.sort_unstable();
                for k in ks {
                    if let Some(v) = d.get(k) {
                        Self::w_attr(out, &inner, k, v);
                    }
                }
                out.push_str(ind);
                close(out);
            }
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn render_float(f: f64) -> String {
        if f.fract() == 0.0 && f.is_finite() {
            format!("{}", f as i64)
        } else {
            format!("{f}")
        }
    }

    pub(crate) fn esc_attr(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&apos;"),
                _ => out.push(c),
            }
        }
        out
    }

    pub(crate) fn esc_text(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                _ => out.push(c),
            }
        }
        out
    }
}
