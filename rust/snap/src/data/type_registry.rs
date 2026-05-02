//! `types` section entries. Aliases vs concrete types.

use smol_str::SmolStr;

#[derive(Clone, Debug, PartialEq)]
pub enum TypeEntry {
    /// Concrete type: `TypeName,`.
    Concrete(SmolStr),
    /// Alias: `'AliasName' -> TypeExpr,` — `TypeExpr` stored verbatim.
    Alias { alias: SmolStr, expr: SmolStr },
}

/// Sorted view of a `types` section. Aliases first (by alias name), then
/// concrete (by name) — matches the canonical output rule from spec.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TypeRegistry {
    entries: Vec<TypeEntry>,
}

impl TypeRegistry {
    #[must_use] pub fn new(entries: Vec<TypeEntry>) -> Self {
        Self { entries }
    }

    #[must_use] pub fn empty() -> Self {
        Self { entries: Vec::new() }
    }

    #[must_use] pub fn entries(&self) -> &[TypeEntry] {
        &self.entries
    }

    #[must_use] pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use] pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
