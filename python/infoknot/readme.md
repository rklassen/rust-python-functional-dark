# infoknot

Python export surface for Snap canonical text. This package mirrors the
Rust implementation under `rust/snap`: it carries the same data shapes
and emits text the Rust parser accepts byte-for-byte.

`infoknot` is **emit-only**. There is no parser in this package. If you
need to read `.snap`, use the Rust crate.

## Requirements

Python **3.12+** (PEP 695 syntax: `class Foo[T]:`, `type Result[T] = ...`).

## Result-typed emit

The emitter is a pure function that returns a `Result[T]` rather than
raising on validation failures. This lets agent consumers route errors
without `try/except`.

```python
from infoknot import (
    Edge, Graph, GraphMeta, Node, NumericEncoding,
    StandardKind, WeightVec, emit, ident,
)
from infoknot.result import Ok, Err

graph = Graph(
    handle="demo",
    meta=GraphMeta(
        id="g001", name="demo", operators="op/",
        time="2026-05-01T00:00:00Z", version="0.7", workspace="ws/",
    ),
    nodes=[
        Node(
            StandardKind.OBJECT, "a001", "A",
            {"type": ident("T")},
            WeightVec((0.5,), NumericEncoding.FLOAT),
        ),
        Node(StandardKind.OBJECT, "b002", "B", {"type": ident("T")}),
    ],
    edges=[Edge("flow", "a001", "b002")],
    types=["T"],
)

match emit(graph):
    case Ok(value=text):
        print(text)
    case Err(err=e):
        print(e.pretty())
```

`Graph.to_snap()` and `export_to_snap(graph)` are convenience wrappers
that unwrap and raise `SemanticErr` on failure.

## Sum-type `EdgeWeight`

```python
from infoknot import (
    BytestreamRef, NumericEncoding,
    WeightByteRef, WeightMatrix, WeightNone, WeightOpRef, WeightVec,
)

def render(w):
    match w:
        case WeightNone():
            return "<unweighted>"
        case WeightVec(values=vs, encoding=enc):
            return f"vec({len(vs)} values, {enc.value})"
        case WeightMatrix(rows=rs):
            return f"matrix({len(rs)} rows)"
        case WeightByteRef(ref=r):
            return f"byteref(@{r.stream})"
        case WeightOpRef(node_id=nid):
            return f"opref(@{nid})"
```

The five variants mirror `EdgeWeight` in the Rust crate exactly. Nothing
in the public API uses a stringly-typed `kind` field.

## Parity with Rust

The canonical canary fixture is duplicated in
`tests/test_emit.py::CanaryParityTests`. It is byte-identical to
`rust/snap/tests/canary_roundtrip.rs::FIXTURE`. Any drift fails CI.

The hex range, snorm/unorm range, and "no mixed int/float" rules are
enforced on emit; out-of-range hex values return
`Err(SemanticErr)` rather than silently clamping (see
`test_hex_out_of_range_returns_err`).

## Verification

```sh
cd python
python3.12 -m unittest discover -s infoknot/tests -t .
```
