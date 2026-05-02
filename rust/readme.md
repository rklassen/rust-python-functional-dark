# snap — reference serializer

Rust implementation of the snap typed-graph DSL (v0.7). Hyper-performance,
immutable-post-build, agent-consumed. The canonical text format and four
adapter formats (graphviz dot, json, xml, plus petgraph and daggy in-memory
exports) all round-trip the same `Graph` value.

The text spec lives one directory up at
[`../snap_specification.md`](../snap_specification.md).

## Architecture

Four layers, one-way dependencies (`view → data`, `io → codec → data`,
`data` depends on nothing else):

```
src/
├── data/          domain — types and pure algorithms
│   ├── err.rs              SemanticErr + NonEmpty<consider>
│   ├── types.rs            NodeId, NumericEncoding, BytestreamRef, NodeKind, AttrValue
│   ├── weight.rs           EdgeWeight enum (None / Vec / Matrix / ByteRef / OpRef)
│   ├── nodes.rs            NodeDef, NodeData, Nodes (sealed CSR)
│   ├── edges.rs            EdgeDef, Edge, Edges (out-only CSR, parallel edges allowed)
│   ├── graph.rs            Graph + section accessors
│   ├── graph_cycles.rs     Iterative Tarjan SCC (impl Graph continuation)
│   ├── meta.rs             GraphMeta (.graph section)
│   ├── literals.rs streams.rs registers.rs type_registry.rs
│   └── ...
├── codec/         pure encode/decode transforms
│   ├── weight_text.rs      WeightText carrier (parse/emit edge weight inner)
│   ├── hex.rs              Hex byte string codec
│   └── base64.rs           Base64 (for stream payloads)
└── io/            file/stream/external-format adapters
    ├── snap/               canonical snap text — Snap::parse / Snap::emit
    ├── dot/                graphviz — Dot::emit / Dot::parse
    ├── json/               serde_json — Json::emit / Json::parse
    ├── xml/                quick-xml (snap-native dialect)
    ├── petgraph.rs         Petgraph::export → DiGraph
    └── daggy.rs            Daggy::export → Dag (with cycle pre-check)
```

## Two-stage construction

No `add_node` exists. Build nodes first as a batch, then edges with a
read-only `&Nodes`. Per-edge results so partial success is observable.

```rust
let nodes = snap::Nodes::new(node_defs)?;                // sorted, dedup-checked
let edges = snap::Edges::new(&nodes, edge_defs);         // Vec<Result<Edge, SemanticErr>>
let graph = snap::Graph::new(nodes, edges)?;             // sealed; Vec<SemanticErr> on partial fail

println!("{}", graph.to_snap());                          // canonical text
let copy = snap::Graph::from_snap(&graph.to_snap())?;     // round-trip
```

`Graph` is immutable post-build. Mutation = rebuild via
`graph.into_builder()` returning the owned `(Vec<NodeDef>, Vec<EdgeDef>)`.

## Error model

Every fallible operation returns `Result<T, SemanticErr>` (or
`Vec<Result<_, _>>` for batch). `SemanticErr` is structurally:

```rust
pub struct SemanticErr {
    pub found: String,            // compact description of what we got
    pub expected: Option<String>, // the rule that was broken
    pub consider: NonEmpty<String>, // ≥1 actionable suggestion (type-enforced)
}
```

`NonEmpty<String>` is a `(String, Vec<String>)` newtype so the empty
`consider:` case is unconstructable — agentic consumers always get at
least one concrete fix to try. Pretty-prints multi-line via
`SemanticErr::pretty()`.

`Option<T>` is permitted only for single-obvious-failure lookups
(e.g. `Nodes::id_to_index(id) -> Option<NodeIx>`). Anything with multiple
failure modes returns `Result`.

## v0.7 edge weight grammar

Numerical weights ride on the arrow itself:

| Form | Meaning |
|---|---|
| `a -> b` | unweighted |
| `a -(0.5)-> b` | default float scalar |
| `a -(0.5)s-> b` | snorm |
| `a -(0.875)u-> b` | unorm |
| `a -(FF12AABB)h-> b` | hex bytes |
| `a -(0.1, 0.2, 0.3)-> b` | flat vector |
| `a -(1,2,3 \| 4,5 \| 6,7,8)-> b` | list-of-lists, pipe-separated rows |
| `a -(@emb_42 ..1024)s-> b` | bytestream slice, decode as snorm |
| `a -(@scorer)-> b` | operator ref (v0.7) — runtime-evaluated |

Nodes carry the same value grammar via an optional `weight:` field
(v0.7). The internal `EdgeWeight` enum is reused for both.

## Features

Default features: `serde`. All other adapters are opt-in.

```toml
[dependencies]
snap = { version = "0.1", features = ["petgraph", "daggy", "dot", "json", "xml"] }
```

Each feature pulls in its respective adapter and the corresponding
`Graph::to_X` / `Graph::from_X` delegate methods.

## Storage shape

Single CSR (out-only adjacency), parallel edges allowed
(`col_indices` is edge-indexed, not target-unique). Sorted at finalize
into `Box<[T]>` so canonical output is deterministic without hash-seed
leakage. `IndexMap` during build, frozen after. `SmolStr` for IDs
(4/8-char Crockford b32 lowercase fits inline). `SmallVec<[f64; 8]>` for
edge weight vectors.

## Doctrine

The crate is held to a NASA-Power-10-derived rule set the user calls
**Power Six**:

- **Reliability:** simple control flow (Tarjan iterative, no recursion);
  bounded loops; allocation only at constructors and API boundaries;
  every error branch propagates (via `?`, no terminal truncation);
  no `panic!`/`unwrap`/`expect`/`unreachable!`/`todo!`/`unimplemented!`
  in non-test code; pedantic-clean (`-D warnings -W clippy::pedantic`).
- **Structural:** 432-line ceiling per file, hard-wrap at 80 cols;
  functions attach to structs (no free-function sprawl — bridge fns
  between sibling continuation `impl` files are documented exceptions);
  file/struct name alignment (`types.rs` is the multi-type exception);
  layered decomposition (`data/codec/io`).
- **No builders.** All constructors are
  `::new(args) -> Result<Self, SemanticErr>` or `-> Self` if infallible.
- **No `&mut self` on public APIs of immutable data types.** Cursor types
  in lex/parse/emit (Lexer, Parser, Emitter, XmlReader, DotReader) take
  the documented cursor exception.

## Tests

```sh
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings -W clippy::pedantic
```

Current coverage: 56 tests across codec, data, and the four io formats,
including a byte-identical canary round-trip in
`tests/canary_roundtrip.rs` and a v0.7 node-weight round-trip in
`tests/v0_7_node_weight_roundtrip.rs`.
