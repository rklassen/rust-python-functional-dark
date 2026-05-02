use smallvec::SmallVec;

use crate::data::types::{BytestreamRef, NumericEncoding};

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeWeight {
    None,
    Vec(SmallVec<[f64; 8]>, NumericEncoding),
    Matrix(Vec<SmallVec<[f64; 8]>>, NumericEncoding),
    ByteRef(BytestreamRef, NumericEncoding),
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
            | EdgeWeight::ByteRef(_, e) => Some(*e),
        }
    }
}
