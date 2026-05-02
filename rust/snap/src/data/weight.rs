use smallvec::SmallVec;

use crate::data::types::{BytestreamRef, NodeId, NumericEncoding};

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeWeight {
    None,
    Vec(SmallVec<[f64; 8]>, NumericEncoding),
    Matrix(Vec<SmallVec<[f64; 8]>>, NumericEncoding),
    ByteRef(BytestreamRef, NumericEncoding),
    /// v0.7: dynamic weight evaluated at runtime by an `operator` node.
    /// The encoding is the format mark on the arrow (`s`/`u`/`h`); it
    /// tells the runtime how to coerce the operator's return value.
    /// The default (no mark) is `Raw`, meaning the runtime returns the
    /// value unchanged.
    OpRef(NodeId, NumericEncoding),
}

impl EdgeWeight {
    #[must_use] pub fn is_none(&self) -> bool {
        matches!(self, EdgeWeight::None)
    }

    #[must_use] pub fn encoding(&self) -> Option<NumericEncoding> {
        match self {
            EdgeWeight::None => None,
            EdgeWeight::Vec(_, e)
            | EdgeWeight::Matrix(_, e)
            | EdgeWeight::ByteRef(_, e)
            | EdgeWeight::OpRef(_, e) => Some(*e),
        }
    }
}
