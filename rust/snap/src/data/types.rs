//! Identifier + scalar types shared across the crate.
//!
//! Doctrine exception: this is the single multi-type file in the data
//! layer. The one-type-per-file rule is relaxed here because these
//! primitives are tiny, mutually referential, and used together by every
//! downstream module.

use smol_str::SmolStr;

/// Stable per-graph identifier. Crockford b32 lowercase, 4 or 8 chars.
pub type NodeId = SmolStr;

/// Dense node index assigned at build time.
pub type NodeIx = u32;

/// Dense edge index assigned at build time.
pub type EdgeIx = u32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NumericEncoding {
    Int,
    Float,
    Snorm,
    Unorm,
    Hex,
    Raw,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BytestreamRef {
    pub stream: NodeId,
    pub offset: u32,
    pub len: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Function,
    Info,
    Object,
    Operator,
    Property,
    Custom(SmolStr),
}

/// Generic attribute/literal value carried by sections that hold open
/// key-value content (extras, literals, info-node bodies, the `.graph`
/// metadata, etc.). Closed under the v0.6 spec.
#[derive(Clone, Debug, PartialEq)]
pub enum AttrValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    /// Single-quoted string content; the surrounding quotes are not
    /// stored.
    Str(SmolStr),
    /// Bare identifier or dotted ident (e.g. `Civil.Alignment`).
    Ident(SmolStr),
    /// ISO 8601 with terminal `Z`, stored verbatim.
    DateTime(SmolStr),
    /// Nested list value as used in extras.
    List(Vec<AttrValue>),
    /// Nested sub-dict value as used in extras.
    Dict(indexmap::IndexMap<SmolStr, AttrValue>),
}
