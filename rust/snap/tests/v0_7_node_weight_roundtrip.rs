//! v0.7 node-weight round-trip tests. The snap text emitter is
//! canonical; parse-then-emit must be byte-identical for every
//! fixture.

const NODE_WEIGHT_LIST: &str = "\
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
edges { }
extras { }
layout { }
literals { }
nodes {
 object { id: a001, name: 'A', type: T, weight: (0.5, 0.3)s },
}
registers { }
streams { }
types {
 T,
}
end🪢
";

const NODE_WEIGHT_OPREF: &str = "\
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
edges { }
extras { }
layout { }
literals { }
nodes {
 object { id: a001, name: 'A', type: T, weight: (@scorer) },
 operator { id: scorer, name: 'score' },
}
registers { }
streams { }
types {
 T,
}
end🪢
";

#[test]
fn node_weight_snorm_list_roundtrip() {
    let g = snap::Graph::from_snap(NODE_WEIGHT_LIST)
        .expect("parse must succeed on canonical input");
    let out = g.to_snap();
    assert_eq!(
        out, NODE_WEIGHT_LIST,
        "node weight (0.5, 0.3)s roundtrip drift",
    );
}

#[test]
fn node_weight_opref_roundtrip() {
    let g = snap::Graph::from_snap(NODE_WEIGHT_OPREF)
        .expect("parse must succeed on canonical input");
    let out = g.to_snap();
    assert_eq!(
        out, NODE_WEIGHT_OPREF,
        "node weight @scorer roundtrip drift",
    );
}
