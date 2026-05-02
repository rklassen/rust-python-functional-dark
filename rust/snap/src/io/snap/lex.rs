//! snap text tokenizer. Cursor-style — `Lexer::next` mutates state
//! (`&mut self`). This is the explicit doctrine exception: the rule
//! against `&mut self` on public APIs of immutable data types does NOT
//! apply to lexers/parsers/emitters. A `Lexer` IS a cursor; mutation is
//! its semantics. The `Lexer` type is `pub(crate)` and never escapes the
//! `io::snap` module, so it is not a public API surface anyway.
//!
//! Recognizes the v0.6 snap surface syntax: the magic header `🪢snap`
//! and trailer `end🪢`, section keywords, node-kind keywords,
//! identifiers (incl. dotted), numbers (int/float), single-quoted
//! strings, ISO 8601 datetimes, and the punctuation set used by the
//! grammar. Whitespace, newlines, and `# ... \n` comments are skipped.
//!
//! Escape policy: in v0.6, single-quoted strings are simple — backslash
//! has no meaning. Reserved as the escape introducer in v0.7.
//!
//! Dash lexing rule: a `-` byte IMMEDIATELY followed by `>` is emitted
//! as `Arrow`. Any other `-` (including `-(` for the weighted-edge
//! arrow, `-x` ident-adjacent forms, etc.) is emitted as a bare `Dash`
//! token. The parser composes weighted-arrow shapes
//! (`-(content)X->`) by combining `Dash`, `LParen`, raw paren body,
//! `RParen`, optional format-mark ident, and `Arrow`.

use crate::data::err::SemanticErr;

/// Magic-open emoji + literal `snap`.
pub(crate) const MAGIC_OPEN: &str = "\u{1FAA2}snap";
/// Magic-close: literal `end` + emoji.
pub(crate) const MAGIC_CLOSE: &str = "end\u{1FAA2}";

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Tok<'a> {
    MagicOpen,
    MagicClose,
    /// `.graph`, `edges`, ... `types`.
    Section(&'a str),
    /// `file`, `function`, `info`, `object`, `operator`, `property`.
    NodeKind(&'a str),
    /// Identifier (incl. dotted like `Civil.Alignment`).
    Ident(&'a str),
    /// Raw numeric text — codec interprets.
    Number(&'a str),
    /// Single-quoted content (no quotes).
    Str(&'a str),
    /// ISO 8601 datetime.
    DateTime(&'a str),
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Colon,
    Comma,
    /// `->`
    Arrow,
    /// Bare `-` (used in `-(...)->` weighted-edge arrow forms).
    Dash,
    At,
    Dollar,
    Plus,
    /// `..` for slice form.
    DotDot,
    /// Bare `None` keyword.
    None_,
    Eof,
}

#[derive(Clone, Debug)]
pub(crate) struct Spanned<T> {
    pub value: T,
    pub line: u32,
    pub col: u32,
}

pub(crate) struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self { src: input, pos: 0, line: 1, col: 1 }
    }

    pub(crate) fn next(
        &mut self,
    ) -> Result<Spanned<Tok<'a>>, SemanticErr> {
        self.skip_ws_and_comments();
        let line = self.line;
        let col = self.col;
        if self.pos >= self.src.len() {
            return Ok(Spanned { value: Tok::Eof, line, col });
        }

        // Magic open / close (multi-byte emoji prefix).
        if self.starts_with(MAGIC_OPEN) {
            self.advance_bytes(MAGIC_OPEN.len());
            return Ok(Spanned { value: Tok::MagicOpen, line, col });
        }
        if self.starts_with(MAGIC_CLOSE) {
            self.advance_bytes(MAGIC_CLOSE.len());
            return Ok(Spanned { value: Tok::MagicClose, line, col });
        }

        let b = match self.src.as_bytes().get(self.pos).copied() {
            Some(v) => v,
            None => {
                return Ok(Spanned { value: Tok::Eof, line, col });
            }
        };

        match b {
            b'{' => { self.advance_bytes(1); Self::ok(Tok::LBrace, line, col) }
            b'}' => { self.advance_bytes(1); Self::ok(Tok::RBrace, line, col) }
            b'[' => {
                self.advance_bytes(1);
                Self::ok(Tok::LBracket, line, col)
            }
            b']' => {
                self.advance_bytes(1);
                Self::ok(Tok::RBracket, line, col)
            }
            b'(' => {
                self.advance_bytes(1);
                Self::ok(Tok::LParen, line, col)
            }
            b')' => {
                self.advance_bytes(1);
                Self::ok(Tok::RParen, line, col)
            }
            b':' => { self.advance_bytes(1); Self::ok(Tok::Colon, line, col) }
            b',' => { self.advance_bytes(1); Self::ok(Tok::Comma, line, col) }
            b'@' => { self.advance_bytes(1); Self::ok(Tok::At, line, col) }
            b'$' => { self.advance_bytes(1); Self::ok(Tok::Dollar, line, col) }
            b'+' => { self.advance_bytes(1); Self::ok(Tok::Plus, line, col) }
            b'.' => self.lex_dot(line, col),
            b'-' => self.lex_dash(line, col),
            b'\'' => self.lex_str(line, col),
            b'0'..=b'9' => self.lex_number_or_dt(line, col),
            _ => self.lex_word(line, col),
        }
    }

    fn lex_dot(
        &mut self,
        line: u32,
        col: u32,
    ) -> Result<Spanned<Tok<'a>>, SemanticErr> {
        let bs = self.src.as_bytes();
        if bs.get(self.pos + 1).copied() == Some(b'.') {
            self.advance_bytes(2);
            return Self::ok(Tok::DotDot, line, col);
        }
        // `.section` keyword.
        let start = self.pos;
        self.advance_bytes(1);
        while let Some(&c) =
            self.src.as_bytes().get(self.pos)
        {
            if Self::is_ident_byte(c) {
                self.advance_bytes(1);
            } else {
                break;
            }
        }
        let slice = self.src.get(start..self.pos).unwrap_or("");
        Self::ok(Tok::Section(slice), line, col)
    }

    fn lex_dash(
        &mut self,
        line: u32,
        col: u32,
    ) -> Result<Spanned<Tok<'a>>, SemanticErr> {
        let bs = self.src.as_bytes();
        if bs.get(self.pos + 1).copied() == Some(b'>') {
            self.advance_bytes(2);
            return Self::ok(Tok::Arrow, line, col);
        }
        // Bare `-`: weighted-edge arrow opener `-(...)->` or other.
        // The parser composes the full arrow shape; the lexer only
        // reports the dash here.
        self.advance_bytes(1);
        Self::ok(Tok::Dash, line, col)
    }

    /// Take raw bytes up to (and consuming) the next `)`. Single-
    /// line: a `\n` before `)` is an error. Call only after `LParen`
    /// has been emitted (position sits just past `(`).
    ///
    /// # Errors
    /// `SemanticErr` if `)` does not precede EOF or `\n`.
    pub(crate) fn take_paren_body(
        &mut self,
    ) -> Result<&'a str, SemanticErr> {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        loop {
            let b = match self.src.as_bytes().get(self.pos).copied() {
                Some(v) => v,
                None => return Err(Self::unterminated_paren(line, col)),
            };
            if b == b')' {
                let slice = self.src.get(start..self.pos).unwrap_or("");
                self.advance_bytes(1);
                return Ok(slice);
            }
            if b == b'\n' {
                return Err(Self::unterminated_paren(line, col));
            }
            self.advance_one_char();
        }
    }

    fn lex_str(
        &mut self,
        line: u32,
        col: u32,
    ) -> Result<Spanned<Tok<'a>>, SemanticErr> {
        // Skip opening quote.
        self.advance_bytes(1);
        let start = self.pos;
        loop {
            let bs = self.src.as_bytes();
            match bs.get(self.pos).copied() {
                Some(b'\'') => {
                    let slice =
                        self.src.get(start..self.pos).unwrap_or("");
                    self.advance_bytes(1);
                    return Self::ok(Tok::Str(slice), line, col);
                }
                Some(b'\n') | None => {
                    return Err(Self::unterminated_str(line, col));
                }
                Some(_) => self.advance_one_char(),
            }
        }
    }

    fn lex_number_or_dt(
        &mut self,
        line: u32,
        col: u32,
    ) -> Result<Spanned<Tok<'a>>, SemanticErr> {
        let start = self.pos;
        // Walk digits; might extend to ISO datetime if we see `T`.
        while let Some(&c) = self.src.as_bytes().get(self.pos) {
            if c.is_ascii_digit() {
                self.advance_bytes(1);
            } else {
                break;
            }
        }
        // ISO 8601 datetime?  YYYY-MM-DDTHH:MM:SSZ
        if self.peek_is(b'-') && self.is_iso_after() {
            return self.consume_dt(start, line, col);
        }
        if self.peek_is(b'.') {
            // float
            self.advance_bytes(1);
            while let Some(&c) = self.src.as_bytes().get(self.pos) {
                if c.is_ascii_digit() {
                    self.advance_bytes(1);
                } else {
                    break;
                }
            }
        }
        let slice = self.src.get(start..self.pos).unwrap_or("");
        Self::ok(Tok::Number(slice), line, col)
    }

    fn is_iso_after(&self) -> bool {
        // Need at least: -DD-DDTHH:MM:SSZ. We cheaply look ahead for
        // a `T` within the next 12 bytes.
        let bs = self.src.as_bytes();
        let mut i = self.pos;
        let end = (self.pos + 14).min(bs.len());
        while i < end {
            match bs.get(i).copied() {
                Some(b'T') => return true,
                Some(b' ' | b'\n' | b',' | b'}' | b']') | None => return false,
                _ => i += 1,
            }
        }
        false
    }

    fn consume_dt(
        &mut self,
        start: usize,
        line: u32,
        col: u32,
    ) -> Result<Spanned<Tok<'a>>, SemanticErr> {
        // Consume until `Z` inclusive.
        while let Some(&c) = self.src.as_bytes().get(self.pos) {
            self.advance_bytes(1);
            if c == b'Z' {
                break;
            }
        }
        let slice = self.src.get(start..self.pos).unwrap_or("");
        Self::ok(Tok::DateTime(slice), line, col)
    }

    fn lex_word(
        &mut self,
        line: u32,
        col: u32,
    ) -> Result<Spanned<Tok<'a>>, SemanticErr> {
        let start = self.pos;
        while let Some(&c) = self.src.as_bytes().get(self.pos) {
            if Self::is_ident_byte(c) {
                self.advance_bytes(1);
            } else {
                break;
            }
        }
        let slice = self.src.get(start..self.pos).unwrap_or("");
        if slice.is_empty() {
            return Err(Self::unexpected_byte(
                self.src.as_bytes().get(start).copied().unwrap_or(0),
                line,
                col,
            ));
        }
        let tok = match slice {
            "None" => Tok::None_,
            "file" | "function" | "info" | "object"
            | "operator" | "property" => Tok::NodeKind(slice),
            "edges" | "extras" | "layout" | "literals"
            | "nodes" | "registers" | "streams" | "types" => {
                Tok::Section(slice)
            }
            _ => Tok::Ident(slice),
        };
        Self::ok(tok, line, col)
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            let b = match self.src.as_bytes().get(self.pos).copied() {
                Some(v) => v,
                None => return,
            };
            match b {
                b' ' | b'\t' | b'\r' => self.advance_bytes(1),
                b'\n' => {
                    self.pos += 1;
                    self.line = self.line.saturating_add(1);
                    self.col = 1;
                }
                b'#' => {
                    while let Some(&c) =
                        self.src.as_bytes().get(self.pos)
                    {
                        if c == b'\n' {
                            break;
                        }
                        self.advance_bytes(1);
                    }
                }
                _ => return,
            }
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src
            .get(self.pos..)
            .is_some_and(|t| t.starts_with(s))
    }

    fn peek_is(&self, b: u8) -> bool {
        self.src.as_bytes().get(self.pos).copied() == Some(b)
    }

    fn advance_bytes(&mut self, n: usize) {
        let end = (self.pos + n).min(self.src.len());
        // Walk char-by-char to keep line/col honest.
        while self.pos < end {
            self.advance_one_char();
        }
    }

    fn advance_one_char(&mut self) {
        let bs = self.src.as_bytes();
        let b = match bs.get(self.pos).copied() {
            Some(v) => v,
            None => return,
        };
        let step = Self::utf8_step(b);
        if b == b'\n' {
            self.line = self.line.saturating_add(1);
            self.col = 1;
        } else {
            self.col = self.col.saturating_add(1);
        }
        self.pos = (self.pos + step).min(self.src.len());
    }

    fn utf8_step(b: u8) -> usize {
        if b < 0xC0 { 1 }
        else if b < 0xE0 { 2 }
        else if b < 0xF0 { 3 }
        else { 4 }
    }

    fn is_ident_byte(b: u8) -> bool {
        matches!(
            b,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'
            | b'_' | b'/' | b'.'
        )
    }

    // Wraps in Result for uniform call sites: peer helpers
    // (unterminated_str, etc.) return Err(SemanticErr).
    #[allow(clippy::unnecessary_wraps)]
    fn ok(
        t: Tok<'a>,
        line: u32,
        col: u32,
    ) -> Result<Spanned<Tok<'a>>, SemanticErr> {
        Ok(Spanned { value: t, line, col })
    }

}
