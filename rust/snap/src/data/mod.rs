//! Domain layer: types, structs, and pure algorithms.
//!
//! This layer depends on no other crate-internal layer. Codec, io, and
//! view all depend on it; the dependency direction is one-way.

pub mod err;
pub mod types;
pub mod weight;
pub mod nodes;
pub mod edges;
pub mod meta;
pub mod literals;
pub mod registers;
pub mod streams;
pub mod type_registry;
pub mod graph;
pub mod graph_cycles;

pub use err::{NonEmpty, SemanticErr};
pub use types::{
    AttrValue, BytestreamRef, EdgeIx, NodeId, NodeIx, NodeKind,
    NumericEncoding,
};
pub use weight::EdgeWeight;
pub use nodes::{NodeData, NodeDef, NodeRef, Nodes};
pub use edges::{Edge, EdgeDef, Edges};
pub use meta::GraphMeta;
pub use literals::LiteralEntry;
pub use registers::RegisterEntry;
pub use streams::StreamEntry;
pub use type_registry::{TypeEntry, TypeRegistry};
pub use graph::Graph;
