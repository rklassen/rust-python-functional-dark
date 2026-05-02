# Markdown Color Mapping Chart

This chart shows how Markdown tokens are mapped in the theme and which existing Rust/Python color families they borrow from.

| Markdown sample | Token scope(s) | Swatch | Applied style | Draws from |
|---|---|---|---|---|
| `# Heading` text | `markup.heading.1.markdown`, `markup.heading.setext.1.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#a9cdd9;border:1px solid #333;"></span> `#a9cdd9` | bold | string/constant blue |
| `##` to `######` heading text | `markup.heading.markdown`, `markup.heading.setext.markdown`, `markup.heading.2.markdown` ... `markup.heading.6.markdown`, `markup.heading.setext.2.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#a9cdd9;border:1px solid #333;"></span> `#a9cdd9` | bold | string/constant blue |
| `#` marker | `punctuation.definition.heading.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#ce9193;border:1px solid #333;"></span> `#ce9193` | normal | control-keyword pink |
| `` `code` `` text | `markup.inline.raw.string.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#a9cdd9;border:1px solid #333;"></span> `#a9cdd9` | normal | string/constant blue |
| inline backticks | `punctuation.definition.raw.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c455;border:1px solid #333;"></span> `#9bc2c455` | normal | soft punctuation |
| fenced block text (` ``` ` block body) | `markup.fenced_code.block.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c466;border:1px solid #333;"></span> `#9bc2c466` | normal | comment green |
| code fence markers (` ``` `) | `punctuation.section.code.begin.markdown`, `punctuation.section.code.end.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c455;border:1px solid #333;"></span> `#9bc2c455` | normal | soft punctuation |
| `_italic_` | `markup.italic.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#d5bfa4;border:1px solid #333;"></span> `#d5bfa4` | italic | emphasis/result tone |
| `**bold**` | `markup.bold.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#d5bfa4;border:1px solid #333;"></span> `#d5bfa4` | bold | emphasis/result tone |
| `~~strike~~` | `markup.strikethrough.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c499;border:1px solid #333;"></span> `#9bc2c499` | strikethrough | muted body tone |
| `> quote` text | `markup.quote.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c488;border:1px solid #333;"></span> `#9bc2c488` | italic | namespace-muted tone |
| `>` marker | `punctuation.definition.quote.begin.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c455;border:1px solid #333;"></span> `#9bc2c455` | normal | soft punctuation |
| list text (`- item`, `1. item`) | `markup.list.unnumbered.markdown`, `markup.list.numbered.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c4cc;border:1px solid #333;"></span> `#9bc2c4cc` | normal | variable/body text |
| list marker (`-`, `1.`) | `punctuation.definition.list.begin.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#ce9193;border:1px solid #333;"></span> `#ce9193` | normal | control-keyword pink |
| link URL (`[text](url)`) | `markup.underline.link.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#ce9193;border:1px solid #333;"></span> `#ce9193` | underline | control-keyword pink |
| link text/reference (`[text](url)`) | `string.other.link.description.markdown`, `string.other.link.title.markdown`, `constant.other.reference.link.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c4cc;border:1px solid #333;"></span> `#9bc2c4cc` | underline | variable/body text |
| link punctuation (`[]()`) | `punctuation.definition.link.markdown`, `punctuation.definition.metadata.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c455;border:1px solid #333;"></span> `#9bc2c455` | normal | soft punctuation |
| horizontal rule (`---`) | `meta.separator.markdown` | <span style="display:inline-block;width:1.25em;height:1.25em;background:#6e7881;border:1px solid #333;"></span> `#6e7881` | normal | UI border gray |
| HTML comments (`<!-- note -->`) | `comment` (generic) | <span style="display:inline-block;width:1.25em;height:1.25em;background:#9bc2c466;border:1px solid #333;"></span> `#9bc2c466` | normal | comment muted green-blue |

## Note On Fenced Language Blocks

When a fenced code block includes a language tag (for example, `rust` or `python`), VS Code injects that language grammar inside the block, so Rust/Python token scopes take over for code content.
