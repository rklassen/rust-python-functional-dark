//! v0.7 kind-check validation tests for `Edges::new`. Sibling test
//! module attached to `edges.rs` via `#[path]` to keep `edges.rs`
//! under the per-file 432-line ceiling.

use indexmap::IndexMap;

use super::{Edges, EdgeDef};
use crate::data::nodes::{NodeDef, Nodes};
use crate::data::types::{BytestreamRef, NodeKind, NumericEncoding};
use crate::data::weight::EdgeWeight;

fn nd(id: &str, kind: NodeKind) -> NodeDef {
    NodeDef {
        id: id.into(),
        kind,
        name: None,
        attrs: IndexMap::new(),
        weight: None,
    }
}

#[test]
fn opref_against_object_target_rejected() {
    let nodes = Nodes::new(vec![
        nd("a001", NodeKind::Object),
        nd("b002", NodeKind::Object),
        // `@op01` is an OBJECT, not an operator — must reject.
        nd("op01", NodeKind::Object),
    ])
    .expect("nodes ok");
    let defs = vec![EdgeDef {
        family: "flow".into(),
        src: "a001".into(),
        tgt: "b002".into(),
        weight: EdgeWeight::OpRef(
            "op01".into(),
            NumericEncoding::Raw,
        ),
    }];
    let results = Edges::new(&nodes, defs);
    let err = results
        .into_iter()
        .next()
        .and_then(Result::err)
        .expect("expected SemanticErr");
    assert!(
        err.found.contains("operator-ref"),
        "wrong error: {err:?}",
    );
}

#[test]
fn byteref_against_non_stream_rejected() {
    // tgt of the slice is an `object`, not a `stream` — must reject.
    // `Custom("stream")` is the canonical stream-node kind.
    let nodes = Nodes::new(vec![
        nd("a001", NodeKind::Object),
        nd("b002", NodeKind::Object),
        nd("emb_42", NodeKind::Object),
    ])
    .expect("nodes ok");
    let defs = vec![EdgeDef {
        family: "flow".into(),
        src: "a001".into(),
        tgt: "b002".into(),
        weight: EdgeWeight::ByteRef(
            BytestreamRef {
                stream: "emb_42".into(),
                offset: 0,
                len: 1024,
            },
            NumericEncoding::Raw,
        ),
    }];
    let results = Edges::new(&nodes, defs);
    let err = results
        .into_iter()
        .next()
        .and_then(Result::err)
        .expect("expected SemanticErr");
    assert!(err.found.contains("slice"), "wrong error: {err:?}");
}

#[test]
fn opref_against_operator_target_accepted() {
    let nodes = Nodes::new(vec![
        nd("a001", NodeKind::Object),
        nd("b002", NodeKind::Object),
        nd("op01", NodeKind::Operator),
    ])
    .expect("nodes ok");
    let defs = vec![EdgeDef {
        family: "flow".into(),
        src: "a001".into(),
        tgt: "b002".into(),
        weight: EdgeWeight::OpRef(
            "op01".into(),
            NumericEncoding::Raw,
        ),
    }];
    let results = Edges::new(&nodes, defs);
    assert!(results.into_iter().all(|r| r.is_ok()));
}
