//! CANARY: snap text -> Graph -> snap text MUST be byte-identical.
//!
//! This test guards the determinism contract for the snap text format.
//! v0.7 fixture exercises the new features:
//!   - operator node (target of operator-ref edge weight)
//!   - dynamic edge weight `-(@op)->`
//!   - node `weight:` field

const FIXTURE: &str = "\
🪢snap demo
.graph {
 gen: 0,
 id: g001,
 name: 'demo',
 operators: 'op/',
 time: 2026-05-01T00:00:00Z,
 types: None,
 version: 0.7,
 workspace: 'ws/',
}
edges {
 flow {
  a001 -> b002,
  a001 -(0.1, 0.5, 0.9)u-> b002,
  a001 -(@op01)-> b002,
 }
}
extras { }
layout { }
literals { }
nodes {
 object { id: a001, name: 'A', type: T, weight: (0.5) },
 object { id: b002, name: 'B', type: T },
 operator { id: op01, name: 'merge' },
}
registers { }
streams { }
types {
 T,
}
end🪢
";

#[test]
fn canary_byte_identical_roundtrip() {
    let g = snap::Graph::from_snap(FIXTURE)
        .expect("parse must succeed on canonical input");
    let out = g.to_snap();
    assert_eq!(out, FIXTURE, "roundtrip must be byte-identical");
}
