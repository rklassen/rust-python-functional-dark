//! `literals` section entries.

use smol_str::SmolStr;

use crate::data::types::AttrValue;

#[derive(Clone, Debug, PartialEq)]
pub struct LiteralEntry {
    /// Semantic handle.
    pub name: SmolStr,
    /// Stable element id.
    pub id: SmolStr,
    /// Declared type (e.g. `"int"`, `"bool"`).
    pub type_name: SmolStr,
    /// The assigned value.
    pub value: AttrValue,
}

impl LiteralEntry {
    #[must_use] pub fn new(
        name: SmolStr,
        id: SmolStr,
        type_name: SmolStr,
        value: AttrValue,
    ) -> Self {
        Self { name, id, type_name, value }
    }
}
