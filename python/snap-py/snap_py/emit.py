"""Canonical Snap text emitter reconciled to ``rust/snap/src/io/snap``.

The emitter is a pure function: ``emit(graph) -> Result[str]``. Validation
failures are returned as ``Err(SemanticErr)`` rather than raised, so
agent consumers can route them. ``Graph.to_snap()`` is the convenience
wrapper that unwraps and raises on ``Err``.

Float-vs-int rendering rule (mirrors Rust ``weight_text_emit::e_float``):

    if value is finite and value.fract() == 0.0:
        emit as integer (no decimal point)
    else:
        emit as a plain float

The same rule governs ``render_float`` for layout coordinates EXCEPT
that ``layout`` writes a trailing ``.0`` for whole numbers — the
Rust ``render_float`` keeps the ``.1`` precision form for layout,
weight emission collapses to bare integer.
"""

from __future__ import annotations

import base64
import math
from collections import defaultdict
from typing import Any, Iterable, Mapping

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
    NumericEncoding,
    RegisterEntry,
    StandardKind,
    StreamEntry,
    WeightByteRef,
    WeightMatrix,
    WeightNone,
    WeightOpRef,
    WeightVec,
    kind_str,
)
from .result import Err, Ok, Result, SemanticErr


def emit(graph: Graph) -> Result[str]:
    """Emit canonical Snap text. Returns ``Ok(text)`` or ``Err(SemanticErr)``."""
    return Emitter().emit_graph(graph)


def export_to_snap(graph: Graph) -> str:
    """Convenience: emit canonical Snap text, raising on validation error."""
    match emit(graph):
        case Ok(value=text):
            return text
        case Err(err=err):
            raise err
    raise AssertionError("unreachable: Result is exhaustive")


def _weight_encoding(weight: EdgeWeight) -> NumericEncoding | None:
    """Project the encoding tag from any non-None weight variant."""
    match weight:
        case WeightNone():
            return None
        case WeightVec(encoding=enc):
            return enc
        case WeightMatrix(encoding=enc):
            return enc
        case WeightByteRef(encoding=enc):
            return enc
        case WeightOpRef(encoding=enc):
            return enc
    raise AssertionError(f"unsupported EdgeWeight variant: {weight!r}")


class Emitter:
    """Stateful builder. The user-facing entry is ``emit_graph``."""

    def emit_graph(self, graph: Graph) -> Result[str]:
        out: list[str] = ["\U0001faa2snap"]
        if graph.handle is not None:
            out.extend([" ", graph.handle])
        out.append("\n")

        self._emit_dot_graph(out, graph.meta)
        edges_res = self._emit_edges(out, graph.edges, graph.nodes)
        if isinstance(edges_res, Err):
            return edges_res
        extras_res = self._emit_extras(out, graph.extras)
        if isinstance(extras_res, Err):
            return extras_res
        self._emit_layout(out, graph.layout)
        self._emit_literals(out, graph.literals)
        nodes_res = self._emit_nodes(out, graph.nodes)
        if isinstance(nodes_res, Err):
            return nodes_res
        self._emit_registers(out, graph.registers)
        self._emit_streams(out, graph.streams)
        self._emit_types(out, graph.types)
        out.append("end\U0001faa2\n")
        return Ok("".join(out))

    # --- header -------------------------------------------------------------

    def _emit_dot_graph(self, out: list[str], meta: GraphMeta) -> None:
        out.append(".graph {\n")
        out.extend([" gen: ", str(meta.gen), ",\n"])
        out.extend([" id: ", meta.id, ",\n"])
        out.extend([" name: '", meta.name, "',\n"])
        out.extend([" operators: '", meta.operators, "',\n"])
        out.extend([" time: ", meta.time, ",\n"])
        if meta.types is None:
            out.append(" types: None,\n")
        else:
            out.extend([" types: '", meta.types, "',\n"])
        out.extend([" version: ", meta.version, ",\n"])
        out.extend([" workspace: '", meta.workspace, "',\n"])
        if meta.code_path is not None:
            out.extend([" code_path: '", meta.code_path, "',\n"])
        if meta.data_path is not None:
            out.extend([" data_path: '", meta.data_path, "',\n"])
        if meta.date is not None:
            out.extend([" date: '", meta.date, "',\n"])
        out.append("}\n")

    # --- edges --------------------------------------------------------------

    def _emit_edges(
        self,
        out: list[str],
        edges: Iterable[Edge],
        nodes: Iterable[Node],
    ) -> Result[None]:
        edge_list = list(edges)
        if not edge_list:
            out.append("edges { }\n")
            return Ok(None)

        out.append("edges {\n")
        groups: dict[str, list[Edge]] = defaultdict(list)
        for edge in edge_list:
            groups[edge.family].append(edge)

        for family in sorted(groups):
            out.extend([" ", family, " {\n"])
            rows = sorted(groups[family], key=lambda e: (e.src, e.tgt))
            for edge in rows:
                inner_res = self._weight_inner(edge.weight)
                if isinstance(inner_res, Err):
                    return inner_res
                inner = inner_res.value
                out.extend(["  ", edge.src])
                if isinstance(edge.weight, WeightNone):
                    out.append(" -> ")
                else:
                    out.extend(
                        [
                            " -(",
                            inner,
                            ")",
                            self._format_mark(edge.weight),
                            "-> ",
                        ]
                    )
                out.extend([edge.tgt, ",\n"])
            out.append(" }\n")
        out.append("}\n")
        return Ok(None)

    # --- extras / layout ----------------------------------------------------

    def _emit_extras(
        self,
        out: list[str],
        extras: Mapping[str, Any],
    ) -> Result[None]:
        if not extras:
            out.append("extras { }\n")
            return Ok(None)
        out.append("extras {\n")
        for key in sorted(extras):
            rendered = self._render_attr(extras[key], indent=" ")
            if isinstance(rendered, Err):
                return rendered
            out.extend([" ", key, ": ", rendered.value, ",\n"])
        out.append("}\n")
        return Ok(None)

    def _emit_layout(
        self,
        out: list[str],
        layout: Mapping[str, tuple[float, float]],
    ) -> None:
        if not layout:
            out.append("layout { }\n")
            return
        out.append("layout {\n")
        for key in sorted(layout):
            x, y = layout[key]
            out.extend(
                [
                    " ",
                    key,
                    ": (",
                    self._render_layout_float(x),
                    ", ",
                    self._render_layout_float(y),
                    "),\n",
                ]
            )
        out.append("}\n")

    # --- literals -----------------------------------------------------------

    def _emit_literals(
        self,
        out: list[str],
        literals: Mapping[str, LiteralEntry],
    ) -> None:
        """Emit ``name$id: Type = value,`` per spec v0.7 §Literals."""
        if not literals:
            out.append("literals { }\n")
            return
        out.append("literals {\n")
        for key in sorted(literals):
            value = literals[key]
            rendered = self._render_attr(value.value, indent=" ")
            # rendered is always Ok for primitive literal values; unwrap
            # via match so a stray Err surfaces deterministically.
            text = (
                rendered.value if isinstance(rendered, Ok) else "<err>"
            )
            out.extend(
                [
                    " ",
                    key,
                    "$",
                    value.id,
                    ": ",
                    value.type_name,
                    " = ",
                    text,
                    ",\n",
                ]
            )
        out.append("}\n")

    # --- nodes --------------------------------------------------------------

    def _emit_nodes(
        self,
        out: list[str],
        nodes: Iterable[Node],
    ) -> Result[None]:
        node_list = sorted(nodes, key=lambda n: n.id)
        if not node_list:
            out.append("nodes { }\n")
            return Ok(None)
        out.append("nodes {\n")
        for node in node_list:
            out.extend([" ", kind_str(node.kind), " { id: ", node.id])
            if node.name is not None:
                out.extend([", name: '", node.name, "'"])
            keys = list(node.attrs.keys())
            if node.weight is not None:
                keys.append("weight")
            for key in sorted(keys):
                out.extend([", ", key, ": "])
                if key == "weight":
                    nw_res = self._render_node_weight(node.weight)
                    if isinstance(nw_res, Err):
                        return nw_res
                    out.append(nw_res.value)
                else:
                    av = self._render_attr(node.attrs[key], indent=" ")
                    if isinstance(av, Err):
                        return av
                    out.append(av.value)
            out.append(" },\n")
        out.append("}\n")
        return Ok(None)

    # --- registers / streams / types ---------------------------------------

    def _emit_registers(
        self,
        out: list[str],
        registers: Mapping[str, RegisterEntry],
    ) -> None:
        """Emit ``name$id: Type,`` per spec v0.7 §Registers."""
        if not registers:
            out.append("registers { }\n")
            return
        out.append("registers {\n")
        for key in sorted(registers):
            value = registers[key]
            out.extend(
                [
                    " ",
                    key,
                    "$",
                    value.id,
                    ": ",
                    value.type_name,
                    ",\n",
                ]
            )
        out.append("}\n")

    def _emit_streams(
        self,
        out: list[str],
        streams: Mapping[str, StreamEntry],
    ) -> None:
        """Emit ``stream { id, data: <base64>, name: '...' }`` per spec.

        The base64 payload is single-line. The spec also allows a
        multi-line ``+``-continued form; this emitter uses single-line
        as the canonical default since it is unambiguous and keeps
        output deterministic. The Rust parser accepts either.
        """
        if not streams:
            out.append("streams { }\n")
            return
        out.append("streams {\n")
        for key in sorted(streams):
            value = streams[key]
            encoded = base64.b64encode(value.data).decode("ascii")
            out.extend(
                [
                    " stream { id: ",
                    value.id,
                    ", data: ",
                    encoded,
                ]
            )
            if value.name is not None:
                out.extend([", name: '", value.name, "'"])
            out.append(" },\n")
        out.append("}\n")

    def _emit_types(self, out: list[str], types: Iterable[Any]) -> None:
        entries = list(types)
        if not entries:
            out.append("types { }\n")
            return
        aliases: list[AliasType] = []
        concrete: list[str] = []
        for entry in entries:
            if isinstance(entry, AliasType):
                aliases.append(entry)
            elif isinstance(entry, ConcreteType):
                concrete.append(entry.name)
            else:
                concrete.append(str(entry))

        out.append("types {\n")
        for entry in sorted(aliases, key=lambda e: e.alias):
            out.extend([" '", entry.alias, "' -> ", entry.expr, ",\n"])
        for name in sorted(concrete):
            out.extend([" ", name, ",\n"])
        out.append("}\n")

    # --- weight rendering --------------------------------------------------

    def _render_node_weight(
        self,
        weight: EdgeWeight | None,
    ) -> Result[str]:
        if weight is None:
            return Ok("()")
        inner = self._weight_inner(weight)
        if isinstance(inner, Err):
            return inner
        return Ok(f"({inner.value}){self._format_mark(weight)}")

    def _weight_inner(self, weight: EdgeWeight) -> Result[str]:
        """Inner content of a weight, no parens, no format mark.

        Mirrors ``rust/snap/src/codec/weight_text_emit.rs``. Hex-encoded
        Vec/Matrix values are validated against the byte range; out-of-
        range values return ``Err(SemanticErr)``.
        """
        match weight:
            case WeightNone():
                return Ok("")
            case WeightVec(values=vs, encoding=enc):
                if len(vs) == 1:
                    return self._render_num(vs[0], enc)
                return self._render_vec(vs, enc)
            case WeightMatrix(rows=rs, encoding=enc):
                if len(rs) == 1:
                    return self._render_vec(rs[0], enc)
                return self._render_matrix(rs, enc)
            case WeightByteRef(ref=r):
                return Ok(self._render_byteref(r))
            case WeightOpRef(node_id=nid):
                return Ok(f"@{nid}")
        return Err(
            SemanticErr(
                found=f"unknown EdgeWeight variant: {type(weight).__name__}",
                expected="one of: WeightNone, WeightVec, WeightMatrix, WeightByteRef, WeightOpRef",
                consider=(
                    "construct weight via the documented variant classes",
                    "ensure no third-party subclasses pretend to be EdgeWeight",
                ),
            )
        )

    def _render_vec(
        self,
        values: Iterable[float],
        encoding: NumericEncoding,
    ) -> Result[str]:
        vals = tuple(values)
        if encoding == NumericEncoding.HEX:
            return self._render_hex_blob(vals)
        parts: list[str] = []
        for v in vals:
            r = self._render_num(v, encoding)
            if isinstance(r, Err):
                return r
            parts.append(r.value)
        return Ok(", ".join(parts))

    def _render_matrix(
        self,
        rows: Iterable[Iterable[float]],
        encoding: NumericEncoding,
    ) -> Result[str]:
        if encoding == NumericEncoding.HEX:
            blobs: list[str] = []
            for row in rows:
                blob = self._render_hex_blob(tuple(row))
                if isinstance(blob, Err):
                    return blob
                blobs.append(blob.value)
            return Ok(" | ".join(blobs))
        out: list[str] = []
        for row in rows:
            r = self._render_vec(row, encoding)
            if isinstance(r, Err):
                return r
            out.append(r.value)
        return Ok(" | ".join(out))

    def _render_byteref(self, ref: BytestreamRef) -> str:
        if ref.offset == 0:
            return f"@{ref.stream} ..{ref.len}"
        return f"@{ref.stream} +{ref.offset}..{ref.len}"

    def _render_num(
        self,
        value: float,
        encoding: NumericEncoding,
    ) -> Result[str]:
        # Mirrors Rust ``e_num``: INT casts to i64, HEX encodes a single
        # byte, otherwise apply the float-vs-int collapse rule.
        if encoding == NumericEncoding.INT:
            return Ok(str(int(value)))
        if encoding == NumericEncoding.HEX:
            return self._render_hex_blob((value,))
        return Ok(self._render_weight_float(value))

    def _render_weight_float(self, value: float) -> str:
        """Mirror Rust ``e_float``: integer-valued finite floats lose
        their decimal point in weight emission."""
        v = float(value)
        if math.isfinite(v) and v.is_integer():
            return str(int(v))
        return str(v)

    def _render_hex_blob(self, values: tuple[float, ...]) -> Result[str]:
        bs = bytearray()
        for v in values:
            r = self._validate_byte(v)
            if isinstance(r, Err):
                return r
            bs.append(r.value)
        return Ok(bs.hex().upper())

    def _validate_byte(self, value: float) -> Result[int]:
        """Hex bytes must be integer-valued in [0, 255]. No silent clamp."""
        v = float(value)
        if math.isnan(v):
            return Err(
                SemanticErr(
                    found="NaN value in HEX-encoded weight",
                    expected="an integer in range [0, 255]",
                    consider=(
                        "remove NaN entries before emit",
                        "switch encoding to FLOAT or SNORM if non-integer values are required",
                    ),
                )
            )
        if not math.isfinite(v):
            return Err(
                SemanticErr(
                    found=f"non-finite value {v!r} in HEX-encoded weight",
                    expected="an integer in range [0, 255]",
                    consider=(
                        "filter out infinities before emit",
                        "switch encoding to FLOAT for unbounded values",
                    ),
                )
            )
        if not v.is_integer():
            return Err(
                SemanticErr(
                    found=f"non-integer value {v!r} in HEX-encoded weight",
                    expected="an integer in range [0, 255]",
                    consider=(
                        "round to integer before emit",
                        "switch encoding to FLOAT/SNORM/UNORM for fractional data",
                    ),
                )
            )
        i = int(v)
        if i < 0 or i > 255:
            return Err(
                SemanticErr(
                    found=f"value {i} out of byte range in HEX-encoded weight",
                    expected="an integer in range [0, 255]",
                    consider=(
                        "scale values into the byte range [0, 255] before emit",
                        "switch encoding to UNORM for [0, 1] floats",
                    ),
                )
            )
        return Ok(i)

    def _format_mark(self, weight: EdgeWeight) -> str:
        enc = _weight_encoding(weight)
        if enc == NumericEncoding.SNORM:
            return "s"
        if enc == NumericEncoding.UNORM:
            return "u"
        if enc == NumericEncoding.HEX:
            return "h"
        return ""

    # --- attr rendering ----------------------------------------------------

    def _render_attr(
        self,
        value: Any,
        indent: str,
    ) -> Result[str]:
        if value is None:
            return Ok("None")
        if isinstance(value, bool):
            return Ok("true" if value else "false")
        if isinstance(value, int):
            return Ok(str(value))
        if isinstance(value, float):
            return Ok(self._render_layout_float(value))
        if isinstance(value, Ident):
            return Ok(value.value)
        if isinstance(value, DateTime):
            return Ok(value.value)
        if isinstance(value, str):
            return Ok(f"'{value}'")
        if isinstance(value, list | tuple):
            parts: list[str] = []
            for v in value:
                r = self._render_attr(v, indent + " ")
                if isinstance(r, Err):
                    return r
                parts.append(r.value)
            return Ok("[" + ", ".join(parts) + "]")
        if isinstance(value, dict):
            return self._render_dict(value, indent)
        return Err(
            SemanticErr(
                found=f"unsupported attribute type {type(value).__name__}: {value!r}",
                expected="None | bool | int | float | str | Ident | DateTime | list | tuple | dict",
                consider=(
                    "wrap bare identifiers with ident(...)",
                    "wrap timestamps with snap_datetime(...)",
                    "convert custom objects to one of the supported scalar/aggregate types",
                ),
            )
        )

    def _render_dict(
        self,
        value: Mapping[str, Any],
        indent: str,
    ) -> Result[str]:
        """Multi-line nested dict rendering.

        Spec example:

            extras {
             sub-dictionary: {
              boolean-key: false,
              numerical-key: 123,
              str-key: 'value1',
             },
            }

        The opening ``{`` lives on the parent value line; entries are
        indented one space deeper than the parent key, alphabetical, with
        terminal commas; the closing ``}`` aligns with the parent indent.
        """
        if not value:
            return Ok("{ }")
        inner_indent = indent + " "
        lines = ["{"]
        for k in sorted(value):
            r = self._render_attr(value[k], inner_indent)
            if isinstance(r, Err):
                return r
            lines.append(f"{inner_indent}{k}: {r.value},")
        lines.append(f"{indent}}}")
        return Ok("\n".join(lines))

    def _render_layout_float(self, value: float) -> str:
        v = float(value)
        if math.isfinite(v) and v.is_integer():
            return f"{v:.1f}"
        return str(v)
