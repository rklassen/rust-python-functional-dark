"""Python export surface for Snap canonical text.

This package mirrors the Rust emitter in ``rust/snap``. It is an
export/canonicalization layer, not a parser.

Public surface
--------------

Data carriers:
    ``AliasType``, ``BytestreamRef``, ``ConcreteType``, ``CustomKind``,
    ``DateTime``, ``Edge``, ``EdgeWeight``, ``Graph``, ``GraphMeta``,
    ``Ident``, ``LiteralEntry``, ``Node``, ``NodeKind``,
    ``NumericEncoding``, ``RegisterEntry``, ``StandardKind``,
    ``StreamEntry``, ``WeightByteRef``, ``WeightMatrix``,
    ``WeightNone``, ``WeightOpRef``, ``WeightVec``.

Emit:
    ``emit(graph) -> Result[str]`` — pure, no exceptions on validation.
    ``export_to_snap(graph) -> str`` — convenience wrapper, raises on Err.
    ``Graph.to_snap()`` — same convenience, dispatched off the carrier.

Result type:
    ``Result[T] = Ok[T] | Err``. ``Err`` carries a ``SemanticErr`` with
    ``found``, ``expected``, ``consider``.
"""

from .data import (
    AliasType,
    BytestreamRef,
    ConcreteType,
    CustomKind,
    DateTime,
    Edge,
    EdgeWeight,
    Graph,
    GraphMeta,
    Ident,
    LiteralEntry,
    Node,
    NodeKind,
    NumericEncoding,
    RegisterEntry,
    StandardKind,
    StreamEntry,
    WeightByteRef,
    WeightMatrix,
    WeightNone,
    WeightOpRef,
    WeightVec,
    ident,
    snap_datetime,
)
from .emit import Emitter, emit, export_to_snap
from .result import Err, Ok, Result, SemanticErr

__all__ = [
    "AliasType",
    "BytestreamRef",
    "ConcreteType",
    "CustomKind",
    "DateTime",
    "Edge",
    "EdgeWeight",
    "Emitter",
    "Err",
    "Graph",
    "GraphMeta",
    "Ident",
    "LiteralEntry",
    "Node",
    "NodeKind",
    "NumericEncoding",
    "Ok",
    "RegisterEntry",
    "Result",
    "SemanticErr",
    "StandardKind",
    "StreamEntry",
    "WeightByteRef",
    "WeightMatrix",
    "WeightNone",
    "WeightOpRef",
    "WeightVec",
    "emit",
    "export_to_snap",
    "ident",
    "snap_datetime",
]
