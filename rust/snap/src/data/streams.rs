//! `streams` section entries (raw byte payloads).

use smol_str::SmolStr;

#[derive(Clone, Debug, PartialEq)]
pub struct StreamEntry {
    /// Stable element id.
    pub id: SmolStr,
    /// Optional display name.
    pub name: Option<SmolStr>,
    /// Decoded bytes; the codec layer handles base64.
    pub data: Vec<u8>,
}

impl StreamEntry {
    #[must_use] pub fn new(
        id: SmolStr,
        name: Option<SmolStr>,
        data: Vec<u8>,
    ) -> Self {
        Self { id, name, data }
    }
}
