//! IO layer: file/stream/external-format adapters.
//!
//! Each external representation is one carrier struct here. Implementation
//! detail lives in this layer; data types in `crate::data` may carry thin
//! one-line delegate methods (e.g. `Graph::to_petgraph`) for ergonomic API.
//!
//! Depends on `crate::data` and `crate::codec`. NEVER on `crate::view`.

pub mod snap;

#[cfg(feature = "petgraph")]
pub mod petgraph;
#[cfg(feature = "daggy")]
pub mod daggy;
#[cfg(feature = "dot")]
pub mod dot;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "xml")]
pub mod xml;
