//! `registers` section entries.

use smol_str::SmolStr;

#[derive(Clone, Debug, PartialEq)]
pub struct RegisterEntry {
    /// Semantic handle (the section key).
    pub name: SmolStr,
    /// Stable element id.
    pub id: SmolStr,
    /// Typed reference target type.
    pub type_name: SmolStr,
}

impl RegisterEntry {
    #[must_use] pub fn new(
        name: SmolStr,
        id: SmolStr,
        type_name: SmolStr,
    ) -> Self {
        Self { name, id, type_name }
    }
}
