//! `NonEmpty` + `SemanticErr`. The error type carried by every fallible op in
//! the crate. `NonEmpty` enforces >=1 `consider` string at the type level.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonEmpty<T> {
    head: T,
    tail: Vec<T>,
}

impl<T> NonEmpty<T> {
    pub fn new(head: T) -> Self {
        Self { head, tail: Vec::new() }
    }

    pub fn with_tail(head: T, tail: Vec<T>) -> Self {
        Self { head, tail }
    }

    pub fn head(&self) -> &T {
        &self.head
    }

    pub fn tail(&self) -> &[T] {
        &self.tail
    }

    // is_empty would be a constant `false` for NonEmpty by
    // construction; the lint is asking for misleading API.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        1 + self.tail.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(self.tail.iter())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticErr {
    pub found: String,
    pub expected: Option<String>,
    pub consider: NonEmpty<String>,
}

impl SemanticErr {
    #[must_use] pub fn new(
        found: String,
        expected: Option<String>,
        consider: NonEmpty<String>,
    ) -> Self {
        Self { found, expected, consider }
    }

    /// Multi-line, human-friendly rendering for diagnostic output.
    #[must_use] pub fn pretty(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("found: {}\n", self.found));
        match &self.expected {
            Some(e) => out.push_str(&format!("expected: {e}\n")),
            None => out.push_str("expected: <unspecified>\n"),
        }
        out.push_str("consider:\n");
        for item in self.consider.iter() {
            out.push_str(&format!("  - {item}\n"));
        }
        // Trim the trailing newline so callers can append cleanly.
        if out.ends_with('\n') {
            out.pop();
        }
        out
    }
}

impl std::fmt::Display for SemanticErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expected = match &self.expected {
            Some(e) => e.as_str(),
            None => "<unspecified>",
        };
        let mut considers = String::new();
        considers.push('[');
        let mut first = true;
        for item in self.consider.iter() {
            if !first {
                considers.push_str(", ");
            }
            considers.push_str(item);
            first = false;
        }
        considers.push(']');
        write!(
            f,
            "found `{}`; expected `{}`; consider {}",
            self.found, expected, considers
        )
    }
}

impl std::error::Error for SemanticErr {}
