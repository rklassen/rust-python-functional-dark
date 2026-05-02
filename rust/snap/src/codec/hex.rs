//! Hex byte-string codec for the `:0h` weight type.
//!
//! Even nibble count required (byte-aligned). Uppercase letters in the
//! canonical emitted form. Decoder accepts mixed case input but rejects
//! whitespace and any non-hex character.

use crate::data::err::{NonEmpty, SemanticErr};

/// Carrier for hex byte-string encoding/decoding. Stateless.
pub struct Hex;

impl Hex {
    /// Decode a hex string into bytes. Errors on odd nibble count or
    /// non-hex character. Whitespace is treated as a non-hex character.
    pub fn decode(input: &str) -> Result<Vec<u8>, SemanticErr> {
        let bytes = input.as_bytes();
        // Find any non-hex character (whitespace counts as non-hex).
        for (idx, b) in bytes.iter().enumerate() {
            if Self::nibble(*b).is_none() {
                let ch = *b as char;
                return Err(SemanticErr::new(
                    format!(
                        "non-hex char '{ch}' at position {idx}",
                    ),
                    Some("[0-9a-fA-F]".into()),
                    NonEmpty::with_tail(
                        "fix the typo".into(),
                        vec![
                            "use :0h only for hex bytes".into(),
                            "switch to :int suffix".into(),
                        ],
                    ),
                ));
            }
        }
        if bytes.len() % 2 != 0 {
            return Err(SemanticErr::new(
                format!(
                    "hex string of {} nibbles",
                    bytes.len(),
                ),
                Some(
                    ":0h requires even nibble count (byte-aligned)"
                        .into(),
                ),
                NonEmpty::with_tail(
                    "pad to even count".into(),
                    vec![
                        "truncate one nibble".into(),
                        "use :unorm or :snorm".into(),
                    ],
                ),
            ));
        }
        let mut out = Vec::with_capacity(bytes.len() / 2);
        let mut i = 0;
        while i < bytes.len() {
            let hi = Self::nibble(bytes[i]).unwrap_or(0);
            let lo = Self::nibble(bytes[i + 1]).unwrap_or(0);
            out.push((hi << 4) | lo);
            i += 2;
        }
        Ok(out)
    }

    /// Encode bytes into the canonical uppercase hex string. Infallible.
    #[must_use] pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push(Self::hi_char(*b));
            s.push(Self::lo_char(*b));
        }
        s
    }

    fn nibble(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    fn hi_char(b: u8) -> char {
        Self::nib_char(b >> 4)
    }

    fn lo_char(b: u8) -> char {
        Self::nib_char(b & 0x0F)
    }

    fn nib_char(n: u8) -> char {
        match n {
            0..=9 => (b'0' + n) as char,
            10..=15 => (b'A' + (n - 10)) as char,
            _ => '0',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let s = Hex::encode(&[]);
        assert_eq!(s, "");
        let r = Hex::decode(&s);
        assert!(r.is_ok(), "decode empty: {r:?}");
        if let Ok(v) = r {
            assert_eq!(v, Vec::<u8>::new());
        }
    }

    #[test]
    fn roundtrip_one_byte() {
        let bytes = [0xABu8];
        let s = Hex::encode(&bytes);
        assert_eq!(s, "AB");
        let r = Hex::decode(&s);
        assert!(r.is_ok(), "decode 1: {r:?}");
        if let Ok(v) = r {
            assert_eq!(v, bytes.to_vec());
        }
    }

    #[test]
    fn roundtrip_thirty_two_bytes() {
        let mut bytes = Vec::with_capacity(32);
        for i in 0..32u8 {
            bytes.push(i.wrapping_mul(7));
        }
        let s = Hex::encode(&bytes);
        assert_eq!(s.len(), 64);
        let r = Hex::decode(&s);
        assert!(r.is_ok(), "decode 32: {r:?}");
        if let Ok(v) = r {
            assert_eq!(v, bytes);
        }
    }

    #[test]
    fn roundtrip_two_fifty_six_bytes() {
        let bytes: Vec<u8> = (0..=255u8).collect();
        let s = Hex::encode(&bytes);
        assert_eq!(s.len(), 512);
        let r = Hex::decode(&s);
        assert!(r.is_ok(), "decode 256: {r:?}");
        if let Ok(v) = r {
            assert_eq!(v, bytes);
        }
    }

    #[test]
    fn reject_odd_nibbles() {
        let r = Hex::decode("ABC");
        assert!(r.is_err());
        if let Err(e) = r {
            assert!(
                e.found.contains("3 nibbles"),
                "found was {:?}",
                e.found,
            );
        }
    }

    #[test]
    fn reject_non_hex_char() {
        let r = Hex::decode("AG");
        assert!(r.is_err());
        if let Err(e) = r {
            assert!(e.found.contains("'G'"));
            assert!(e.found.contains("position 1"));
        }
    }

    #[test]
    fn lowercase_input_works() {
        let r = Hex::decode("abcd");
        assert!(r.is_ok(), "lowercase: {r:?}");
        if let Ok(v) = r {
            assert_eq!(v, vec![0xAB, 0xCD]);
        }
    }

    #[test]
    fn whitespace_rejected() {
        let r = Hex::decode("AB CD");
        assert!(r.is_err());
    }
}
