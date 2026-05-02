# Snap Specification

v0.5
2026-04-21T02:28Z

## Related Projects

- VS Code extension project: `./ts/snap-vsix/`
- Rust implementation summary: `../io/readme.md`

## Abstract

- Snap is a typed directed graph over objects, operators, and related node
  kinds.
- Graph connectivity is defined through directed edges and typed matchpoints
  such as `node.out.path` and `node.in.path`.
- `object` nodes represent endurants (continuants).
- `operator` nodes represent actions (occurrents or perdurants).
- `property` is a constrained operator with a unitary input and a non-fallible,
  typed value output.
- `v0.4` introduced typed edge families and optional edge weights.
- `v0.5` retains that edge model while preserving the hand-editable `v0.3`
  file layout.

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
.graph { ... version: 0.5, ... }
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
- All characters remain significant. The old `v0.3` rule that ignored columns
  `81+` is removed in `v0.5`.
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

`v0.5` keeps the `v0.3` graph metadata shape and incorporates the additional
fields that appeared in the `v0.4` in-memory model.

Required keys:
1. `gen`: generation index. Incremented at every write. Not user-editable.
2. `id`: stable graph id.
3. `name`: display name.
4. `operators`: operator source root or logical operator namespace.
5. `time`: canonical timestamp for this file snapshot.
6. `types`: type registry source summary or `None`.
7. `version`: must be `0.5`.
8. `workspace`: workspace root for relative paths.

Optional keys:
1. `date`: auxiliary ISO 8601 creation or export date.
2. `data_path`: auxiliary data root.
3. `code_path`: auxiliary code root.

Rules:
- `time` and `date`, when present, use ISO 8601 UTC with terminal `Z`.
- Strings are single-quoted when quoting is required.
- Unknown keys are not part of canonical `v0.5`.

```snap
.graph {
 gen: 0,
 id: sb83,
 name: 'Project Name',
 operators: 'code_path/library_name/',
 time: 2026-04-20T00:00:00Z,
 types: None,
 version: 0.5,
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
- Typed grouped edges from `v0.4` are part of `v0.5`.
- Edge families are block-scoped.
- Edge weight is optional and numeric.
- Unweighted edges may chain.
- Weighted edges must be written as single edges, not chained paths.
- Matchpoints are typed half-edge endpoints and are valid in source and target
  positions.
- Builtins such as `get(@id)` are not standardized in `v0.5`; explicit named
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

Weighted edges:

```snap
edges {
 import {
  Design -> Civil : 352,
  Design -> Pdf : 388,
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
  +afDvn69q1hzHJjKCRHWUmTj8OvTyBqquR9WAmDt6IDLjGYwH/HmoRnOUdg44V42C7oCa9bq3HoJG5R
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

## Time and Scalars

- Timestamps use ISO 8601 UTC with terminal `Z`.
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
