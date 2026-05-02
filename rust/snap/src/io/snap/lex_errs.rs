//! Continuation of `impl Lexer` for the error builders. Split out of
//! `lex.rs` to honor the per-file 432-line ceiling. This file is named
//! after its purpose (lex errors) and CONTINUES the `Lexer` carrier's
//! impl block — the file/struct alignment is preserved.

use crate::data::err::{NonEmpty, SemanticErr};
use crate::io::snap::lex::Lexer;

impl Lexer<'_> {
    pub(crate) fn unterminated_str(
        line: u32,
        col: u32,
    ) -> SemanticErr {
        SemanticErr::new(
            format!(
                "unterminated single-quoted string at {line}:{col}"
            ),
            Some("a closing `'` on the same line".into()),
            NonEmpty::with_tail(
                "add the closing quote".into(),
                vec![
                    "check for a stray newline in the literal".into(),
                ],
            ),
        )
    }

    pub(crate) fn unterminated_paren(
        line: u32,
        col: u32,
    ) -> SemanticErr {
        SemanticErr::new(
            format!(
                "unterminated `(...)` weight body at {line}:{col}"
            ),
            Some("a closing `)` on the same line".into()),
            NonEmpty::with_tail(
                "add the closing `)`".into(),
                vec![
                    "weighted-edge bodies are single-line".into(),
                ],
            ),
        )
    }

    pub(crate) fn unexpected_byte(
        b: u8,
        line: u32,
        col: u32,
    ) -> SemanticErr {
        SemanticErr::new(
            format!("unexpected byte 0x{b:02X} at {line}:{col}"),
            Some(
                "a token start: ident, number, string, or punct".into(),
            ),
            NonEmpty::with_tail(
                "remove the offending byte".into(),
                vec![
                    "check for non-ASCII outside string literals".into(),
                ],
            ),
        )
    }
}
