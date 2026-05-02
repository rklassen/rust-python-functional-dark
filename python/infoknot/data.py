"""Data carriers for the Python Snap exporter.

This module is the canonical Python data layer. It is intentionally
emit-only: there is no parser here. The shapes mirror the Rust crate
under ``rust/snap`` so that an equivalent ``Graph`` value emits text
the Rust parser accepts.

Sum types use PEP 695 ``type`` aliases (``EdgeWeight``, ``NodeKind``,
``TypeEntry``). Pattern-match on the variant classes for narrowing.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Mapping, Sequence


class NumericEncoding(str, Enum):
    """Encoding tag carried by an ``EdgeWeight``.

    The Rust crate calls this ``NumericEncoding`` and uses it both as
    arrow-suffix format mark on emit (``s``/``u``/``h``) and as the
    parse-time decoder hint.
    """

    INT = "int"
    FLOAT = "float"
    SNORM = "snorm"
    UNORM = "unorm"
    HEX = "hex"
    RAW = "raw"


@dataclass(frozen=True, slots=True)
class Ident:
    """Bare identifier (unquoted) value used in attribute slots."""

    value: str


@dataclass(frozen=True, slots=True)
class DateTime:
    """ISO-8601 timestamp emitted as a bare token (no quotes)."""

    value: str


def ident(value: str) -> Ident:
    """Construct an ``Ident`` value."""
    return Ident(value)


def snap_datetime(value: str) -> DateTime:
    """Construct a ``DateTime`` value."""
    return DateTime(value)


@dataclass(frozen=True, slots=True)
class BytestreamRef:
    """A reference into a ``streams`` entry: ``@id +offset..len`` form."""

    stream: str
    offset: int
    len: int


# --- EdgeWeight (sum type) --------------------------------------------------


@dataclass(frozen=True, slots=True)
class WeightNone:
    """No weight present on the edge."""


@dataclass(frozen=True, slots=True)
class WeightVec:
    """A flat list of numeric values (or single value when len == 1)."""

    values: tuple[float, ...]
    encoding: NumericEncoding


@dataclass(frozen=True, slots=True)
class WeightMatrix:
    """A list-of-lists of numeric values; emitted with ``|`` row sep."""

    rows: tuple[tuple[float, ...], ...]
    encoding: NumericEncoding


@dataclass(frozen=True, slots=True)
class WeightByteRef:
    """Reference into a stream; encoding tells how to decode the bytes."""

    ref: BytestreamRef
    encoding: NumericEncoding


@dataclass(frozen=True, slots=True)
class WeightOpRef:
    """Dynamic weight: a reference to an ``operator`` node id."""

    node_id: str
    encoding: NumericEncoding


type EdgeWeight = (
    WeightNone | WeightVec | WeightMatrix | WeightByteRef | WeightOpRef
)


# --- NodeKind (closed enum + custom variant) -------------------------------


class StandardKind(str, Enum):
    """The six kinds standardized by the v0.7 spec."""

    FILE = "file"
    FUNCTION = "function"
    INFO = "info"
    OBJECT = "object"
    OPERATOR = "operator"
    PROPERTY = "property"


@dataclass(frozen=True, slots=True)
class CustomKind:
    """User-defined node kind.

    Validated lowercase, no whitespace, non-empty. The constructor
    rejects malformed input with ``ValueError`` to match the Rust
    crate's invariant on ``NodeKind::Custom(SmolStr)``.
    """

    name: str

    def __post_init__(self) -> None:
        if not self.name:
            raise ValueError("CustomKind.name must be non-empty")
        if any(ch.isspace() for ch in self.name):
            raise ValueError(
                "CustomKind.name must not contain whitespace",
            )
        if self.name != self.name.lower():
            raise ValueError("CustomKind.name must be lowercase")


type NodeKind = StandardKind | CustomKind


def kind_str(kind: NodeKind | str) -> str:
    """Render a ``NodeKind`` (or back-compat str) to its on-wire token."""
    match kind:
        case StandardKind():
            return kind.value
        case CustomKind(name=n):
            return n
        case str():
            return kind
    raise TypeError(f"unsupported NodeKind value: {kind!r}")


# --- Graph metadata ---------------------------------------------------------


@dataclass(frozen=True, slots=True)
class GraphMeta:
    """The ``.graph`` info table. Required keys first."""

    gen: int = 0
    id: str = "a000"
    name: str = ""
    operators: str = ""
    time: str = ""
    types: str | None = None
    version: str = "0.7"
    workspace: str = ""
    date: str | None = None
    data_path: str | None = None
    code_path: str | None = None


# --- Node / Edge ------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Node:
    """A graph node. ``kind`` is a closed enum with ``CustomKind`` escape."""

    kind: NodeKind | str
    id: str
    name: str | None = None
    attrs: Mapping[str, Any] = field(default_factory=dict)
    weight: EdgeWeight | None = None


@dataclass(frozen=True, slots=True)
class Edge:
    """A directed graph edge in family ``family``."""

    family: str
    src: str
    tgt: str
    weight: EdgeWeight = field(default_factory=WeightNone)


# --- Section-entry types ----------------------------------------------------


@dataclass(frozen=True, slots=True)
class LiteralEntry:
    """One entry in the ``literals`` section."""

    id: str
    type_name: str
    value: Any


@dataclass(frozen=True, slots=True)
class RegisterEntry:
    """One entry in the ``registers`` section."""

    id: str
    type_name: str


@dataclass(frozen=True, slots=True)
class StreamEntry:
    """One entry in the ``streams`` section.

    ``data`` is the RAW byte payload; the emitter base64-encodes it.
    """

    id: str
    data: bytes = b""
    name: str | None = None


@dataclass(frozen=True, slots=True)
class AliasType:
    """A type alias entry: ``'Alias' -> TypeExpr,``"""

    alias: str
    expr: str


@dataclass(frozen=True, slots=True)
class ConcreteType:
    """A concrete type registry entry."""

    name: str


type TypeEntry = AliasType | ConcreteType | str


# --- Graph ------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Graph:
    """The top-level Snap graph carrier."""

    meta: GraphMeta = field(default_factory=GraphMeta)
    nodes: Sequence[Node] = field(default_factory=tuple)
    edges: Sequence[Edge] = field(default_factory=tuple)
    handle: str | None = None
    extras: Mapping[str, Any] = field(default_factory=dict)
    layout: Mapping[str, tuple[float, float]] = field(default_factory=dict)
    literals: Mapping[str, LiteralEntry] = field(default_factory=dict)
    registers: Mapping[str, RegisterEntry] = field(default_factory=dict)
    streams: Mapping[str, StreamEntry] = field(default_factory=dict)
    types: Sequence[TypeEntry] = field(default_factory=tuple)

    def to_snap(self) -> str:
        """Emit the canonical Snap text for this graph.

        Convenience wrapper that unwraps a successful ``Result``; on
        validation failure it raises ``SemanticErr``. Callers wanting
        the structured error should use ``infoknot.emit.emit`` directly.
        """
        from .emit import emit
        from .result import Err, Ok

        match emit(self):
            case Ok(value=text):
                return text
            case Err(err=err):
                raise err
        raise AssertionError("unreachable: Result is exhaustive")
