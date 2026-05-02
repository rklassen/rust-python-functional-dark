"""Tests for the Python Snap emitter.

The canary fixture is byte-identical to the Rust crate's
``rust/snap/tests/canary_roundtrip.rs::FIXTURE``. Any drift fails
``test_canary_matches_rust_fixture``.
"""

from __future__ import annotations

import unittest

from snap_py import (
    BytestreamRef,
    CustomKind,
    Edge,
    Graph,
    GraphMeta,
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
    emit,
    ident,
)
from snap_py.result import Err, Ok, SemanticErr


META = GraphMeta(
    gen=0,
    id="g001",
    name="demo",
    operators="op/",
    time="2026-05-01T00:00:00Z",
    types=None,
    version="0.7",
    workspace="ws/",
)


CANARY = """\
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
"""


class CanaryParityTests(unittest.TestCase):
    """The canonical fixture must come out byte-identical."""

    def _build(self) -> Graph:
        return Graph(
            handle="demo",
            meta=META,
            edges=[
                Edge("flow", "a001", "b002"),
                Edge(
                    "flow",
                    "a001",
                    "b002",
                    WeightVec((0.1, 0.5, 0.9), NumericEncoding.UNORM),
                ),
                Edge(
                    "flow",
                    "a001",
                    "b002",
                    WeightOpRef("op01", NumericEncoding.RAW),
                ),
            ],
            nodes=[
                Node(
                    StandardKind.OBJECT,
                    "a001",
                    "A",
                    {"type": ident("T")},
                    WeightVec((0.5,), NumericEncoding.FLOAT),
                ),
                Node(
                    StandardKind.OBJECT,
                    "b002",
                    "B",
                    {"type": ident("T")},
                ),
                Node(StandardKind.OPERATOR, "op01", "merge"),
            ],
            types=["T"],
        )

    def test_canary_matches_rust_fixture(self) -> None:
        graph = self._build()
        self.assertEqual(graph.to_snap(), CANARY)

    def test_canary_emit_returns_ok(self) -> None:
        match emit(self._build()):
            case Ok(value=text):
                self.assertEqual(text, CANARY)
            case Err():
                self.fail("canary emit must succeed")


class WeightEmitTests(unittest.TestCase):
    def test_hex_scalar_emit(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightVec((255.0,), NumericEncoding.HEX),
                ),
            ],
        )
        text = graph.to_snap()
        self.assertIn(" -(FF)h-> ", text)

    def test_hex_matrix_emit(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightMatrix(
                        ((0xFF, 0x12), (0xAA, 0xBB)),
                        NumericEncoding.HEX,
                    ),
                ),
            ],
        )
        self.assertIn(" -(FF12 | AABB)h-> ", graph.to_snap())

    def test_hex_out_of_range_returns_err(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightVec((300.0,), NumericEncoding.HEX),
                ),
            ],
        )
        result = emit(graph)
        self.assertIsInstance(result, Err)
        assert isinstance(result, Err)
        self.assertIn("300", result.err.found)
        # Range hint must surface in the consider list.
        joined = " ".join(result.err.consider)
        self.assertIn("0, 255", joined)

    def test_hex_negative_returns_err(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightVec((-1.0,), NumericEncoding.HEX),
                ),
            ],
        )
        self.assertIsInstance(emit(graph), Err)

    def test_hex_non_integer_returns_err(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightVec((1.5,), NumericEncoding.HEX),
                ),
            ],
        )
        self.assertIsInstance(emit(graph), Err)

    def test_matrix_int_emit(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightMatrix(
                        ((1.0, 2.0, 3.0), (4.0, 5.0), (6.0, 7.0, 8.0, 9.0)),
                        NumericEncoding.INT,
                    ),
                ),
            ],
        )
        self.assertIn(
            " -(1, 2, 3 | 4, 5 | 6, 7, 8, 9)-> ",
            graph.to_snap(),
        )

    def test_bytestream_ref_zero_offset(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightByteRef(
                        BytestreamRef("emb_42", 0, 1024),
                        NumericEncoding.RAW,
                    ),
                ),
            ],
        )
        self.assertIn(" -(@emb_42 ..1024)-> ", graph.to_snap())

    def test_bytestream_ref_with_offset(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightByteRef(
                        BytestreamRef("emb_42", 512, 1024),
                        NumericEncoding.RAW,
                    ),
                ),
            ],
        )
        self.assertIn(" -(@emb_42 +512..1024)-> ", graph.to_snap())

    def test_bytestream_ref_snorm_decoding(self) -> None:
        graph = Graph(
            meta=META,
            edges=[
                Edge(
                    "embeddings",
                    "a",
                    "b",
                    WeightByteRef(
                        BytestreamRef("emb_42", 0, 1024),
                        NumericEncoding.SNORM,
                    ),
                ),
            ],
        )
        self.assertIn(" -(@emb_42 ..1024)s-> ", graph.to_snap())

    def test_edge_weight_match_exhaustive(self) -> None:
        weights: list = [
            WeightNone(),
            WeightVec((1.0,), NumericEncoding.INT),
            WeightMatrix(((1.0, 2.0), (3.0, 4.0)), NumericEncoding.INT),
            WeightByteRef(
                BytestreamRef("s", 0, 8), NumericEncoding.RAW
            ),
            WeightOpRef("op", NumericEncoding.RAW),
        ]
        # Exhaustive match must cover every variant; this test
        # documents the variant set.
        seen: set[str] = set()
        for w in weights:
            match w:
                case WeightNone():
                    seen.add("none")
                case WeightVec():
                    seen.add("vec")
                case WeightMatrix():
                    seen.add("matrix")
                case WeightByteRef():
                    seen.add("byte_ref")
                case WeightOpRef():
                    seen.add("op_ref")
        self.assertEqual(
            seen, {"none", "vec", "matrix", "byte_ref", "op_ref"},
        )


class SectionShapeTests(unittest.TestCase):
    def test_streams_section_with_payload(self) -> None:
        # b'hello' base64-encodes to 'aGVsbG8='.
        graph = Graph(
            meta=META,
            streams={
                "blob": StreamEntry(
                    id="a11t", data=b"hello", name="binary data",
                ),
            },
        )
        text = graph.to_snap()
        self.assertIn(" stream { id: a11t, data: aGVsbG8=", text)
        self.assertIn(", name: 'binary data'", text)

    def test_literals_canonical_form(self) -> None:
        graph = Graph(
            meta=META,
            literals={
                "alignment_index": LiteralEntry(
                    id="p9a2", type_name="int", value=0,
                ),
                "verbose": LiteralEntry(
                    id="c7f1", type_name="bool", value=True,
                ),
            },
        )
        text = graph.to_snap()
        self.assertIn(" alignment_index$p9a2: int = 0,\n", text)
        self.assertIn(" verbose$c7f1: bool = true,\n", text)

    def test_registers_canonical_form(self) -> None:
        graph = Graph(
            meta=META,
            registers={
                "cnn_model": RegisterEntry(
                    id="m2c1", type_name="ExampleObjectType",
                ),
                "design_alignment": RegisterEntry(
                    id="v8e5", type_name="Civil.Alignment",
                ),
            },
        )
        text = graph.to_snap()
        self.assertIn(
            " cnn_model$m2c1: ExampleObjectType,\n", text,
        )
        self.assertIn(
            " design_alignment$v8e5: Civil.Alignment,\n", text,
        )

    def test_extras_nested_dict_multiline(self) -> None:
        graph = Graph(
            meta=META,
            extras={
                "boolean-key": True,
                "sub-dictionary": {
                    "boolean-key": False,
                    "numerical-key": 123,
                    "str-key": "value1",
                },
            },
        )
        text = graph.to_snap()
        # The nested dict must span multiple lines, alphabetical order,
        # one-space-deeper indent than the parent key, terminal commas.
        self.assertIn(" sub-dictionary: {\n", text)
        self.assertIn("  boolean-key: false,\n", text)
        self.assertIn("  numerical-key: 123,\n", text)
        self.assertIn("  str-key: 'value1',\n", text)
        self.assertIn(" },\n", text)


class NodeKindTests(unittest.TestCase):
    def test_node_kind_custom(self) -> None:
        graph = Graph(
            meta=META,
            nodes=[
                Node(StandardKind.OBJECT, "a001", "A"),
                Node(CustomKind("xnvtq"), "z001", "Z"),
            ],
            types=["T"],
        )
        text = graph.to_snap()
        self.assertIn(" object { id: a001", text)
        self.assertIn(" xnvtq { id: z001", text)


class ResultTests(unittest.TestCase):
    def test_semantic_err_empty_consider_rejected(self) -> None:
        with self.assertRaises(ValueError):
            SemanticErr(found="x", expected="y", consider=())

    def test_result_ok_err_match(self) -> None:
        good: object = Ok(42)
        bad: object = Err(
            SemanticErr(
                found="bad",
                expected="good",
                consider=("try harder",),
            )
        )
        match good:
            case Ok(value=v):
                self.assertEqual(v, 42)
            case Err():
                self.fail("good must match Ok")
        match bad:
            case Ok():
                self.fail("bad must match Err")
            case Err(err=e):
                self.assertEqual(e.found, "bad")

    def test_semantic_err_pretty_includes_consider(self) -> None:
        e = SemanticErr(
            found="x", expected=None, consider=("a", "b"),
        )
        text = e.pretty()
        self.assertIn("found: x", text)
        self.assertIn("  - a", text)
        self.assertIn("  - b", text)


if __name__ == "__main__":
    unittest.main()
