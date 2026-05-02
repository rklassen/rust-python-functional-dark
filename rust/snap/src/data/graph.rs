use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::data::edges::{Edge, Edges};
use crate::data::err::SemanticErr;
use crate::data::literals::LiteralEntry;
use crate::data::meta::GraphMeta;
use crate::data::nodes::Nodes;
use crate::data::registers::RegisterEntry;
use crate::data::streams::StreamEntry;
use crate::data::type_registry::TypeRegistry;
use crate::data::types::AttrValue;

#[derive(Debug)]
pub struct Graph {
    meta: GraphMeta,
    handle: Option<SmolStr>,
    nodes: Nodes,
    edges: Edges,
    extras: IndexMap<SmolStr, AttrValue>,
    layout: IndexMap<SmolStr, (f64, f64)>,
    literals: IndexMap<SmolStr, LiteralEntry>,
    registers: IndexMap<SmolStr, RegisterEntry>,
    streams: IndexMap<SmolStr, StreamEntry>,
    types: TypeRegistry,
}

impl Graph {
    /// Minimal constructor: nodes + edges only. Sets a default
    /// `GraphMeta`. Useful for in-code construction; the parser uses
    /// `with_sections`.
    pub fn new(
        nodes: Nodes,
        edge_results: Vec<Result<Edge, SemanticErr>>,
    ) -> Result<Self, Vec<SemanticErr>> {
        let meta = GraphMeta::minimal(
            SmolStr::new("a000"),
            SmolStr::new("0.6"),
        );
        Self::with_sections(
            meta,
            None,
            nodes,
            edge_results,
            IndexMap::new(),
            IndexMap::new(),
            IndexMap::new(),
            IndexMap::new(),
            IndexMap::new(),
            TypeRegistry::empty(),
        )
    }

    /// Full constructor: every section provided.
    #[allow(clippy::too_many_arguments)]
    pub fn with_sections(
        meta: GraphMeta,
        handle: Option<SmolStr>,
        nodes: Nodes,
        edge_results: Vec<Result<Edge, SemanticErr>>,
        extras: IndexMap<SmolStr, AttrValue>,
        layout: IndexMap<SmolStr, (f64, f64)>,
        literals: IndexMap<SmolStr, LiteralEntry>,
        registers: IndexMap<SmolStr, RegisterEntry>,
        streams: IndexMap<SmolStr, StreamEntry>,
        types: TypeRegistry,
    ) -> Result<Self, Vec<SemanticErr>> {
        let mut ok = Vec::with_capacity(edge_results.len());
        let mut err: Vec<SemanticErr> = Vec::new();
        for r in edge_results {
            match r {
                Ok(e) => ok.push(e),
                Err(e) => err.push(e),
            }
        }
        if !err.is_empty() {
            return Err(err);
        }
        let edges = Edges::from_sorted_edges(nodes.len(), ok)
            .map_err(|e| vec![e])?;
        Ok(Self {
            meta,
            handle,
            nodes,
            edges,
            extras,
            layout,
            literals,
            registers,
            streams,
            types,
        })
    }

    #[must_use] pub fn meta(&self) -> &GraphMeta { &self.meta }
    #[must_use] pub fn handle(&self) -> Option<&SmolStr> {
        self.handle.as_ref()
    }
    #[must_use] pub fn nodes(&self) -> &Nodes { &self.nodes }
    #[must_use] pub fn edges(&self) -> &Edges { &self.edges }
    #[must_use] pub fn extras(&self) -> &IndexMap<SmolStr, AttrValue> {
        &self.extras
    }

    #[must_use] pub fn layout(&self) -> &IndexMap<SmolStr, (f64, f64)> {
        &self.layout
    }

    #[must_use] pub fn literals(&self) -> &IndexMap<SmolStr, LiteralEntry> {
        &self.literals
    }

    #[must_use] pub fn registers(&self) -> &IndexMap<SmolStr, RegisterEntry> {
        &self.registers
    }

    #[must_use] pub fn streams(&self) -> &IndexMap<SmolStr, StreamEntry> {
        &self.streams
    }

    #[must_use] pub fn types(&self) -> &TypeRegistry { &self.types }

    /// Parse snap text into a Graph. Delegates to
    /// `crate::io::snap::Snap::parse`. Associated function (no `&self`)
    /// since this constructs a new Graph.
    pub fn from_snap(input: &str) -> Result<Self, Vec<SemanticErr>> {
        crate::io::snap::Snap::parse(input)
    }

    /// Emit canonical snap text. Delegates to
    /// `crate::io::snap::Snap::emit`.
    #[must_use] pub fn to_snap(&self) -> String {
        crate::io::snap::Snap::emit(self)
    }

    /// Ergonomic delegate to `crate::io::petgraph::Petgraph::export`.
    #[cfg(feature = "petgraph")]
    #[must_use] pub fn to_petgraph(
        &self,
    ) -> ::petgraph::graph::DiGraph<
        crate::data::nodes::NodeData,
        crate::data::weight::EdgeWeight,
    > {
        crate::io::petgraph::Petgraph::export(self)
    }

    /// Ergonomic delegate to `crate::io::daggy::Daggy::export`.
    #[cfg(feature = "daggy")]
    pub fn to_dag(
        &self,
    ) -> Result<
        ::daggy::Dag<
            crate::data::nodes::NodeData,
            crate::data::weight::EdgeWeight,
        >,
        Vec<crate::data::err::SemanticErr>,
    > {
        crate::io::daggy::Daggy::export(self)
    }

    /// Ergonomic delegate to `crate::io::dot::Dot::emit`.
    #[cfg(feature = "dot")]
    #[must_use] pub fn to_dot(&self) -> String {
        crate::io::dot::Dot::emit(self)
    }

    /// Ergonomic delegate to `crate::io::dot::Dot::parse`.
    #[cfg(feature = "dot")]
    pub fn from_dot(input: &str) -> Result<Self, Vec<SemanticErr>> {
        crate::io::dot::Dot::parse(input)
    }

    /// Ergonomic delegate to `crate::io::json::Json::emit`.
    #[cfg(feature = "json")]
    #[must_use] pub fn to_json(&self) -> String {
        crate::io::json::Json::emit(self)
    }

    /// Ergonomic delegate to `crate::io::json::Json::parse`.
    #[cfg(feature = "json")]
    pub fn from_json(input: &str) -> Result<Self, Vec<SemanticErr>> {
        crate::io::json::Json::parse(input)
    }

    /// Ergonomic delegate to `crate::io::xml::Xml::emit`.
    #[cfg(feature = "xml")]
    #[must_use] pub fn to_xml(&self) -> String {
        crate::io::xml::Xml::emit(self)
    }

    /// Ergonomic delegate to `crate::io::xml::Xml::parse`.
    #[cfg(feature = "xml")]
    pub fn from_xml(input: &str) -> Result<Self, Vec<SemanticErr>> {
        crate::io::xml::Xml::parse(input)
    }
}
