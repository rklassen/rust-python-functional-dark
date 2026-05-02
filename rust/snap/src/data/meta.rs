//! `.graph` section metadata. Required + optional fields per v0.6 spec.

use smol_str::SmolStr;

/// Metadata from the `.graph` section. Required fields are non-Option;
/// optional fields are Option<...> per spec.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphMeta {
    /// Generation index.
    pub gen: u64,
    /// Stable graph id.
    pub id: SmolStr,
    /// Display name (may be empty `SmolStr`).
    pub name: SmolStr,
    /// Operator source root.
    pub operators: SmolStr,
    /// ISO 8601 UTC, terminal `Z`, verbatim.
    pub time: SmolStr,
    /// Type registry summary, or None when absent.
    pub types: Option<SmolStr>,
    /// Spec version (e.g. `"0.6"`).
    pub version: SmolStr,
    /// Workspace root.
    pub workspace: SmolStr,
    /// Optional auxiliary date.
    pub date: Option<SmolStr>,
    /// Optional auxiliary data root.
    pub data_path: Option<SmolStr>,
    /// Optional auxiliary code root.
    pub code_path: Option<SmolStr>,
}

impl GraphMeta {
    /// Construct from raw fields. No validation here; parser-side
    /// validates.
    #[allow(clippy::too_many_arguments)]
    #[must_use] pub fn new(
        gen: u64,
        id: SmolStr,
        name: SmolStr,
        operators: SmolStr,
        time: SmolStr,
        types: Option<SmolStr>,
        version: SmolStr,
        workspace: SmolStr,
        date: Option<SmolStr>,
        data_path: Option<SmolStr>,
        code_path: Option<SmolStr>,
    ) -> Self {
        Self {
            gen,
            id,
            name,
            operators,
            time,
            types,
            version,
            workspace,
            date,
            data_path,
            code_path,
        }
    }

    /// Minimal default suitable for in-code construction with no parser.
    #[must_use] pub fn minimal(id: SmolStr, version: SmolStr) -> Self {
        Self {
            gen: 0,
            id,
            name: SmolStr::default(),
            operators: SmolStr::default(),
            time: SmolStr::default(),
            types: None,
            version,
            workspace: SmolStr::default(),
            date: None,
            data_path: None,
            code_path: None,
        }
    }
}
