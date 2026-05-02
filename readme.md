# Rust Python Functional Dark

A dark color theme for VS Code optimized for Rust and Python development.

## Features

- Dark theme optimized for long coding sessions
- Special syntax highlighting for Rust and Python*
- Now with markdown and typesecript support also.
- Preattentively highlights functions, and distinguishes control
- Reduced eye strain through careful color selection
- Functional programming-inspired design philosophy
- Related semantics (code, comments, literals) share colorband.

## Screenview

[![Screenview](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/rpfd-screenview.gif)](https://github.com/rklassen/rust-python-functional-dark/blob/main/media/rpfd-screenview.gif)

## Design Philosophy

[![Colorwheel Study](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/rpfd-colorwheels.png)](https://github.com/rklassen/rust-python-functional-dark/blob/main/media/rpfd-colorwheels.png)

- Core mapping: functions/actions use the cool accent; verbs/operations use the warm accent.
- Visual emphasis is limited to 3 ranks (0–2) to keep contrast reliable and reduce false salience.
- Thank you for caring about accessibility with us; this palette was explicitly tuned against contrast-ratio checks for deuteranomaly/deutan-deficient viewing conditions.

### Emphasis Ranks (0 = strongest)

Rank | Intent | Scalar | Visual Intensity |
--- | --- | --- | ---
0 | Primary focus (actions, errors, key edges) | 1.0 | Highest chroma, highest contrast |
1 | Secondary focus (types, structure, navigation) | 0.618 | Mid chroma, mid contrast |
2 | Tertiary context (hints, punctuation, low-salience UI) | 0.382 | Low chroma, lower contrast |

### Palette Definition

Swatch | Hex | Color Name | Category | Example usage | Description
--- | --- | --- | --- | --- | ---
![Functional swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_functional.png) | `#e7e8b9` | Gracelynn     | Verbs               | Functions and document headings    | Action-oriented semantics
![Executive swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_executive.png)   | `#c898b5` | Mauve Mist    | Executive           | Control flow and active borders    | Syntactic-ontological control tokens
![Common swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_common.png)         | `#7f9fa1` | Granny Smith  | Common              | Default text and neutral structure | Semantic, unmarked
![Editor bg swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_editor_bg.png)   | `#131315` | Business Black | Editor bg           | Primary editor background          | Main editing surface tone
![Background swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_base_bg.png)    | `#0c0c10` | Woodsmoke     | Base BG             | Editor and panel surfaces          | Foundational canvas tone
![Literal swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_literal.png)       | `#a9cdd9` | Sinbad        | Literal             | Literals and atomics               | Ephemera, atomics
![Paratext swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_paratext.png)     | `#49595b` | Feldgrau      | Paratext            | Comments and low-salience hints    | Meta-discourse
![Warning swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_warning.png)       | `#c8bfa8` | Coral Bright  | Warning             | Warning states and caution UI      | Elevated non-fatal attention
![Error swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_error.png)           | `#d0667f` | Charm         | Error               | Errors and failures                | Critical attention channel
![Info swatch](https://raw.githubusercontent.com/rklassen/rust-python-functional-dark/main/media/swatch_info.png)             | `#87b2d1` | Cimarron      | Info                | Informational states               | Contextual guidance channel

Color names from [color-name.com](https://www.color-name.com/) and [chir.ag/Name that Color](https://chir.ag/projects/name-that-color/)

## Installation

1. Launch VS Code
2. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "Rust Python Functional Dark"
4. Click Install
5. Select the theme from Code > Preferences > Color Theme

## Feedback

If you have any suggestions or issues, please open an issue on the GitHub repository.

## License

MIT

**Enjoy!**
