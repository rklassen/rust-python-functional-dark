//! snap — reference serializer for the snap typed-graph DSL (v0.6).
//!
//! The crate is hyper-performance + immutable-post-build. Construction is
//! two-stage (`Nodes::new`, then `Edges::new`) and never mutates after
//! assembly. Every fallible operation returns Result<T, `SemanticErr`>.

// Stylistic pedantic lints allowed crate-wide. These are non-substantive
// and would harm readability of the carrier-vs-cursor architecture if
// enforced (e.g. let-else hides the cursor's three-way match arms,
// match_same_arms collapses semantically distinct token cases).
#![allow(clippy::needless_late_init)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::format_push_string)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::single_match_else)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::manual_let_else)]

pub mod data;
pub mod codec;

pub use data::{
    AttrValue, BytestreamRef, Edge, EdgeDef, EdgeIx, EdgeWeight, Edges,
    Graph, GraphMeta, LiteralEntry, NodeData, NodeDef, NodeId, NodeIx,
    NodeKind, NodeRef, Nodes, NonEmpty, NumericEncoding, RegisterEntry,
    SemanticErr, StreamEntry, TypeEntry, TypeRegistry,
};
pub use codec::{Base64, Hex, WeightText};

pub mod io;
pub use io::snap::Snap;
#[cfg(feature = "petgraph")]
pub use io::petgraph::Petgraph;
#[cfg(feature = "daggy")]
pub use io::daggy::Daggy;
#[cfg(feature = "dot")]
pub use io::dot::Dot;
#[cfg(feature = "json")]
pub use io::json::Json;
#[cfg(feature = "xml")]
pub use io::xml::Xml;
