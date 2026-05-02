# Snap Specification

v0.7
2026-05-02T11:57Z

## Related Projects

- VS Code extension project: `./ts/snap-vsix/`
- Rust implementation summary: `../io/readme.md`

## Abstract

- Snap is a serialization language for typed, directed graph over object and
  operator nodes. It is intended to be human readable and editable, in a 
  worst case scenario, deterministic in formatting, maximally deterministic and
  maximally flat in final form.
- Graph connectivity is defined through directed edges and typed matchpoints
  such as `node.out.path` and `node.in.path`.
- `object` nodes represent endurants (continuants).
- `operator` nodes represent actions (occurrents or perdurants).
- `property` is a constrained operator with a unitary input and a non-fallible,
  typed value output.
- `v0.6` introduces numerical weight embeddings on edges via an arrow-suffix
  syntax with literal-format marks (`s`, `u`, `h`).
- `v0.7` adds two extensions: an edge weight may reference an `operator` node
  (`-(@op_id)->`) for runtime-evaluated dynamic weight; and node entries may
  carry a `weight:` field using the same value grammar as edge weights.

## Contents

Top-level sections are all required and appear in this order:
  - [header]
  - `.graph`
  - `edges`
  - `extras`
  - `layout`
  - `literals`
  - `nodes`
  - `registers`
  - `streams`
  - `types`
  - [trailer]

`types` remains last in canonical serialized output.

## Core Shape

```text
🪢snap <optional handle>
.graph { ... version: 0.7, ... }
edges { ... }
extras { ... }
layout { ... }
literals { ... }
nodes { ... }
registers { ... }
streams { ... }
types { ... }
end🪢
```

## Formatting and Determinism

- Parsing is whitespace-insensitive outside quoted strings.
- Comments begin with `#` and continue to end of line.
- Strings use single quotes when quoting is required.
- Backslash `\` is reserved for escaping inside strings.
- Terminal commas are required in canonical output.
- Canonical serialization uses deterministic section order and deterministic
  ordering within sections.
- Dictionaries, lists of named entries, and section members are alphabetically
  ordered in canonical output unless a section explicitly says otherwise.
- All characters remain significant. Lines longer than 80 will receive a 
  warning but not an error.
- Paths use forward slashes.
- Trailing slashes on path-like values are permitted but are not required unless
  a producer has a semantic reason to preserve them.

## Header

### Magic Number

1. Display: `🪢snap`
2. UTF-8 bytes: `F0 9F AA A2 73 6E 61 70`
3. Unicode sequence: `U+1FAA2 U+0073 U+006E U+0061 U+0070`
4. May be followed by a user-given handle for the file.
5. Must be followed by a newline.

```text
🪢snap projectname
```

### Information Table (`.graph` section)

Required keys:
1. `gen`: generation index. Incremented at every write. Not user-editable.
2. `id`: stable graph id.
3. `name`: display name.
4. `operators`: operator source root or logical operator namespace.
5. `time`: canonical timestamp for this file snapshot.
6. `types`: type registry source summary or `None`.
7. `version`: must be `0.7`.
8. `workspace`: workspace root for relative paths.

Optional keys:
1. `date`: auxiliary ISO 8601 creation or export date.
2. `data_path`: auxiliary data root.
3. `code_path`: auxiliary code root.

Rules:
- `time` and `date`, when present, use ISO 8601 UTC with terminal `Z`.
- Strings are single-quoted when quoting is required.
- Unknown keys are not part of canonical `v0.7`.

```snap
.graph {
 gen: 0,
 id: sb83,
 name: 'Project Name',
 operators: 'code_path/library_name/',
 time: 2026-04-20T00:00:00Z,
 types: None,
 version: 0.7,
 workspace: 'data_path/project_name/',
 date: 2026-04-20T00:00:00Z,
 data_path: 'data_path/',
 code_path: 'code_path/',
}
```

## Identifiers and Reference Syntax

- `@` denotes direct references to existing ids, for example `@a43d` or
  `@b6d3.out.alignments`.
- `$` is shorthand id creation in inline declarations, for example
  `write$32gp` or `alignment_index$p9a2`.
- Bare names such as `design_alignment` are semantic handles that normally come
  from `registers`.
- Ids use Crockford base32 lowercase and must be unique within one `.snap`.

### Identifier Generation

An element id must:
- be represented in Crockford base32 lowercase
- begin with an alpha character
- be unique within the file

Preferred range:
- minimum value: `a000`
- maximum value: `zzzz`

Eight-digit ids `a0000000` to `zzzzzzzz` are permitted but disfavored.

### Matchpoints

Matchpoints are half-edge endpoints:
- `node.out.path`
- `node.in.path`
- `@id.out.path`
- `@id.in.path`

Array indexing may be expressed as `name[index]`.

## Edges and Matchpoints

- `edges` contains directed graph connectivity.
- Untyped edges remain valid.
- Edge families are block-scoped.
- Edge weight is optional and may be a scalar, list, matrix, or bytestream
  reference.
- Edges may chain.
- Snap is intentionally flat: nested embedding is avoided in favor of `@id`
  references.
- Matchpoints are typed half-edge endpoints and are valid in source and target
  positions.
- Builtins such as `get(@id)` are not standardized in `v0.7`; explicit named
  references through `registers` are preferred.

Untyped edges:

```snap
edges {
 alignment_path.out -> read_alignments.in.path,
 read_alignments.out.alignments -> design_alignment,
 design_alignment -> generate_surface.in.from_alignment,
 generate_surface.out.surface -> write.in.surface,
 @ny5a -> write.in.path,
}
```

Typed family blocks:

```snap
edges {
 structure {
  root -> module,
  module -> file,
 }

 flow {
  checkpoint_file -> load_model.in.path,
  load_model.out.model -> model,
 }
}
```

### Edge weight embeddings (v0.6)

Numerical weights ride on the arrow itself. The arrow alphabet is:

- `->` unweighted
- `-(...)->` default (raw, no literal-format interpretation)
- `-(...)s->` snorm (signed normalized, range `[-1, 1]`)
- `-(...)u->` unorm (unsigned normalized, range `[0, 1]`)
- `-(...)h->` hex (raw bytes, uppercase hexadecimal)

Rules:

- The format mark is a single letter from `{s, u, h}`, placed between `)`
  and `->`.
- The format mark is a literal-format, not a type hint. The `:` operator
  remains reserved for type annotations on values; numerical encodings on
  weights live on the arrow itself.
- Brackets `[` and `]` are not used in edge weights. A length-1 vector is
  serialized as a scalar in the codec.
- Flat list weights use a bare comma list: `0.5, 0.875, 0.23`.
- List-of-lists rows use the pipe `|` separator:
  `1, 2, 3 | 4, 5 | 6, 7, 8, 9`.
- Bytestream references take the form `@id ..len` or `@id +offset..len`.
  The format mark on the arrow says how to decode the referenced bytes;
  for example `-(@emb_42 ..1024)s->` decodes the bytes as snorm. The
  default decoding (no mark) is Raw with no interpretation.

Validation rules:

- snorm values must lie in `[-1, 1]`.
- unorm values must lie in `[0, 1]`.
- Hex weights must be byte-aligned (even nibble count).
- Hex tokens use uppercase `A`-`F` and digits `0`-`9` only.
- Mixing integer and float in the same flat list is rejected.
- An empty weight input `-()->` is rejected.
- The `:0o` octal-prefix form is hard-banned: it collides with Rust and
  Python octal literal conventions and is never accepted.
- The legacy `:snorm`, `:unorm`, and `:0h` suffix-tag forms from earlier
  drafts are rejected; format marks live on the arrow only.

Internal data model mapping (for implementers):

- single value -> `Vec(len 1)`
- flat list -> `Vec`
- pipe-separated rows -> `Matrix`
- bytestream ref -> `ByteRef(_, encoding)`

Worked examples:

```snap
edges {
 embeddings {
  a -> b,
  a -(352)-> b,
  a -(0.5)s-> b,
  a -(0.875u)-> b,
  a -(FF12AABB)h-> b,
  a -(-0.25, .875, 0, 0.5)s-> b,
  a -(0.5, 0.875, 0.23)-> b,
  a -(1, 4, 2, 3)-> b,
  a -(00FFAA12BB34CC89DD, 119922DE23FF00FF)h-> b,
  a -(1, 2, 3 | 4, 5 | 6, 7, 8, 9)-> b,
  a -(0.1, 0.2 | 0.3, 0.4)s-> b,
  a -(@emb_42 ..1024)-> b,
  a -(@emb_42 ..1024)s-> b,
  a -(@emb_42 +512..1024)-> b,
 }
}
```

## Extras

There can be only one dictionary of extras.

Rules:
- `extras` is factual metadata for the graph as a whole.
- Keys are identifiers or dash-separated names.
- Values may be booleans, numbers, strings, lists, or nested dictionaries.
- View-only styling metadata should not be placed in core graph sections unless
  the graph itself depends on it semantically.

```snap
extras {
 boolean-key: true,
 key: 'str-value',
 numerical-key: 123.45,
 sub-dictionary: {
  boolean-key: false,
  numerical-key: 123,
  str-key: 'value1',
 },
}
```

## Layout

`layout` stores editor coordinates for stable graph rendering and interaction.

- Keys may be node names, register names, `@id`, or typed matchpoints such as
  `@b6d3.out.alignments`.
- Values are `(x, y)` tuples of float coordinates in editor space.
- Canonical output uses parentheses.

```snap
layout {
 alignment_path: (-1.0, 2.0),
 @b6d3: (2.0, 200.0),
 @a43d: (4.0, 6.0),
 surface_path: (26.0, 20.0),
 @b6d3.out.alignments: (20.0, 0.0),
}
```

## Literals

User-defined values are written into the file under `literals`.

Rules:
- Canonical form is `name$id: Type = value,`
- `name` is the semantic handle.
- `id` is the stable element id.
- `value` may be a string, number, or boolean.

```snap
literals {
 alignment_index$p9a2: int = 0,
 verbose$c7f1: bool = true,
}
```

## Nodes

- Nodes are defined only inside `nodes { ... }`.
- Standardized node kinds are:
  - `file`
  - `function`
  - `info`
  - `object`
  - `operator`
  - `property`
- Inline and verbose forms are equivalent:
  - `kind name$id { ... }`
  - `kind { id: ..., name: ... }`
- Node internals are alphabetical by key in canonical output.
- Additional lower-case custom kinds are permitted if a producer and consumer
  agree on their semantics; unknown custom kinds behave like generic nodes.

### File

A `file` node is a typed object used for read or write boundaries.

Rules:
- `op` must be `read` or `write`.
- `path` is relative to `workspace` unless explicitly absolute.
- `type` is required.
- `name` may be given either inline or as a property.

```snap
file {
 id: x4nw,
 name: alignment.15n.xml,
 op: read,
 path: './20.alignments.gen/',
 type: FileTypeLandXml,
},

file report.md$23ff {
 op: write,
 path: './25.roadway.pdf/',
 type: FileTypeMarkdown,
},
```

### Function

A subtype of operator. Nested names such as `Class.method`,
`Module.Class.method`, or free functions are permitted.

Rules:
- `source` identifies the implementation source path or symbol origin.
- `in` is a dictionary of input ports.
- `out` is either a dictionary of output ports or `None` when no explicit
  result ports are declared.

```snap
function {
 id: a43d,
 in: {
  from_alignment: AlignmentCollection,
  index: int,
  verbose: bool,
 },
 name: RoadwayGenerator.generate_surface,
 out: {
  surface: FileTypeLandXml,
 },
 source: Roadway/RoadwayGenerator.py,
},

function LandXmlWriter.write$32gp {
 in: {
  path: FileTypeLandXml,
  surface: FileTypeLandXml,
 },
 out: None,
 source: Writers/LandXmlWriter.py,
},
```

### Info

Generic dictionary with arbitrary keys in addition to `id` and `name`.

```snap
info {
 id: c7f1,
 name: 'LAZ Source Info',
 key_b: true,
 key_f: 123.45,
 key_i: 123,
 key_s: 'value',
},
```

### Object

A typed reference to a cognizable enduring entity.

```snap
object {
 id: o1a2,
 name: 'Object Name',
 type: ExampleObjectType,
},
```

### Operator

Generic action node.

Rules:
- `in` is a dictionary of input ports.
- `out` is either a dictionary of output ports or `None`.
- `logic` may reference a bytestream defined in `streams`.

```snap
operator {
 id: p9a2,
 in: {
  input: type,
 },
 logic: @a11t,
 name: 'merge',
 out: {None},
},

operator backfill$c8d3 {
 in: {
  input: type,
 },
 out: {None},
},
```

### Property

Computed properties have a unitary input and cannot fail.

Rules:
- `in` is a single direct reference or semantic handle.
- `out` is a single type, not a result envelope.

```snap
property {
 id: p78p,
 name: 'Example Property',
 in: @o1a2,
 out: ExamplePropertyType,
},

property length$0pla {
 in: @v8e5,
 out: float,
},
```

## Registers

`registers` defines stable named handles for selected typed references.

Rules:
1. Form: `name$id: Type,`
2. `name` is the semantic handle used in edges, layout, and UI binding.
3. `id` is the stable element id.
4. Entries are sorted alphabetically by `name` in canonical output.

```snap
registers {
 cnn_model$m2c1: ExampleObjectType,
 design_alignment$v8e5: Civil.Alignment,
 dim128$d128: int,
 points$p01n: ExampleObjectType,
}
```

## Streams

Bytestreams store executable or opaque binary content.

Rules:
1. Encoding is base64.
2. Long values may be split across multiple lines and joined with `+`.
3. `==` padding is permitted as usual for base64.
4. Preferred inline form uses `:` to imply the `data` key.

```snap
streams {
 stream {
  id: a11t,
  data: p78pTR/UdclA0BcO9mqCc0f9JH0p0iQyuUHUj4LaIcFrMA1uaxXcQHt3QkWrWvfULhMx3==,
  name: 'binary data',
 },
 stream additional$c8d3 {
  :ZU7YGMxmNLTokjYIcPGS6zB6Ymb316WdvEnlicNAOrf60ShcYVOYdIiAAoYxI15VV30OBAqHn4hWU
  +afDvn69q1hzHJjKCRHWUmTj8OvTyBqquR9WAmDt6IDLjGYwH/HmoRnOUdg44V42C7oCa9bq3HoJG5
  +7bd6N8Ncs9GjYilu9Tx3fzDbPrpAYiWRWLBK8+Sog==,
 },
}
```

## Types

`types` is the closed type registry for the file.

Rules:
1. Canonical output sorts aliases first, then concrete types alphabetically.
2. Type aliases use `'Alias' -> TypeExpr,`
3. Concrete type entries use `TypeName,`
4. The section is always present, even if empty.

```snap
types {
 'Alignments' -> list[Civil.Alignment],
 AlignmentCollection,
 bool,
 Civil.Alignment,
 FileTypeLandXml,
 float,
 int,
}
```

## Keywords and Syntactical Operators

Reserved syntax and section words:

```text
.graph
@
$
->
-(
)->
)s->
)u->
)h->
|
:
=
snap
edges
extras
layout
literals
nodes
registers
stream
streams
types
end
file
function
info
object
operator
property
in
out
op
path
type
name
id
version
```

Notes:

- The format marks `s`, `u`, and `h` are only meaningful in arrow-suffix
  position (between `)` and `->` on a weighted edge). Outside that
  position they are ordinary identifiers.
- `|` is the list-of-lists row separator inside edge weight parentheses.
- `[` and `]` are not used in edge weights; they still appear in the
  `types` section list-bracket syntax such as `list[T]`.

## Time and Scalars

- Timestamps use ISO 8601 UTC with terminal `Z`, seconds omitted.
- Strings use single quotes when quoting is required.
- Numbers may be integers or floats.
- Booleans are `true` and `false`.
- `None` is a reserved scalar token for absent optional values and empty
  operator results.

## Export

- `.snap` may be transpiled to executable graph runtime code.
- `.snap` may be exported to graphviz or dot while preserving canonical graph
  semantics.
- View policy such as styling belongs outside the core graph unless it is
  semantically required by the graph itself.

## Trailer

1. End line followed immediately by a single newline.
2. Unicode sequence `U+0065 U+006E U+0064 U+1FAA2`
3. Canonical text is exactly `end🪢`

```text
end🪢
```
