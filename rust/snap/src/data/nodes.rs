use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::data::err::{NonEmpty, SemanticErr};
use crate::data::types::{AttrValue, NodeId, NodeIx, NodeKind};

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDef {
    pub id: NodeId,
    pub kind: NodeKind,
    /// Optional display name.
    pub name: Option<SmolStr>,
    /// Kind-specific fields, insertion-ordered. Canonical emit re-sorts
    /// alphabetically; the parser accepts any order.
    pub attrs: IndexMap<SmolStr, AttrValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeData {
    pub id: NodeId,
    pub kind: NodeKind,
    /// Optional display name.
    pub name: Option<SmolStr>,
    /// Kind-specific fields, insertion-ordered. Canonical emit re-sorts
    /// alphabetically; the parser accepts any order.
    pub attrs: IndexMap<SmolStr, AttrValue>,
}

#[derive(Debug)]
pub struct Nodes {
    weights: Box<[NodeData]>, // sorted by id
    ids: Box<[NodeId]>,       // parallel to weights, for binary_search
}

#[derive(Copy, Clone, Debug)]
pub struct NodeRef<'a> {
    pub ix: NodeIx,
    pub data: &'a NodeData,
}

impl Nodes {
    /// Sort, dedupe-check, and seal a batch of node defs.
    /// Errors: duplicate id (`SemanticErr` with
    /// `consider: ["rename one", "merge defs", "use $id suffix"]`).
    pub fn new(mut defs: Vec<NodeDef>) -> Result<Self, SemanticErr> {
        defs.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        // Detect duplicate ids by scanning the sorted Vec; the first
        // duplicate becomes a SemanticErr.
        for win in defs.windows(2) {
            if let [a, b] = win {
                if a.id == b.id {
                    return Err(SemanticErr::new(
                        format!("duplicate node id `{}`", a.id),
                        Some(
                            "each node id must be unique within the file"
                                .into(),
                        ),
                        NonEmpty::with_tail(
                            format!("rename one of `{}`", a.id),
                            vec![
                                "merge the duplicate defs".into(),
                                "use a `$id` suffix to disambiguate".into(),
                            ],
                        ),
                    ));
                }
            }
        }
        let ids: Box<[NodeId]> = defs.iter().map(|d| d.id.clone()).collect();
        let weights: Box<[NodeData]> = defs
            .into_iter()
            .map(|d| NodeData {
                id: d.id,
                kind: d.kind,
                name: d.name,
                attrs: d.attrs,
            })
            .collect();
        Ok(Self { weights, ids })
    }

    #[must_use] pub fn len(&self) -> usize {
        self.weights.len()
    }

    #[must_use] pub fn is_empty(&self) -> bool {
        self.weights.is_empty()
    }

    /// Single obvious failure: id not present. Option is the right shape here.
    #[must_use] pub fn id_to_index(&self, id: &NodeId) -> Option<NodeIx> {
        self.ids
            .binary_search(id)
            .ok()
            .and_then(|i| u32::try_from(i).ok())
    }

    #[must_use] pub fn get(&self, id: &NodeId) -> Option<NodeRef<'_>> {
        let ix = self.id_to_index(id)?;
        let data = self.weights.get(ix as usize)?;
        Some(NodeRef { ix, data })
    }

    pub fn iter(&self) -> impl Iterator<Item = NodeRef<'_>> {
        self.weights.iter().enumerate().filter_map(|(i, data)| {
            u32::try_from(i).ok().map(|ix| NodeRef { ix, data })
        })
    }
}
