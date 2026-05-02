//! Base64 codec for `streams` section payloads.
//!
//! Uses the standard alphabet with `=` padding. Multi-line continuation
//! via leading `+` is handled by the snap text io layer; this codec
//! takes a contiguous string only.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::data::err::{NonEmpty, SemanticErr};

/// Carrier for base64 encoding/decoding. Stateless.
pub struct Base64;

impl Base64 {
    /// Decode a standard-alphabet base64 string with `=` padding into
    /// raw bytes. Wraps `DecodeError` into a `SemanticErr`.
    pub fn decode(input: &str) -> Result<Vec<u8>, SemanticErr> {
        STANDARD.decode(input).map_err(|e| {
            SemanticErr::new(
                format!("invalid base64: {e}"),
                Some(
                    "standard alphabet base64 with = padding".into(),
                ),
                NonEmpty::with_tail(
                    "check character set".into(),
                    vec![
                        "verify padding".into(),
                        "remove non-alphabet whitespace".into(),
                    ],
                ),
            )
        })
    }

    /// Encode bytes to standard-alphabet base64 with `=` padding.
    /// Infallible.
    #[must_use] pub fn encode(bytes: &[u8]) -> String {
        STANDARD.encode(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let s = Base64::encode(&[]);
        assert_eq!(s, "");
        let r = Base64::decode(&s);
        assert!(r.is_ok(), "decode empty: {r:?}");
        if let Ok(v) = r {
            assert_eq!(v, Vec::<u8>::new());
        }
    }

    #[test]
    fn roundtrip_short() {
        let bytes = b"hello";
        let s = Base64::encode(bytes);
        assert_eq!(s, "aGVsbG8=");
        let r = Base64::decode(&s);
        assert!(r.is_ok(), "decode short: {r:?}");
        if let Ok(v) = r {
            assert_eq!(v, bytes.to_vec());
        }
    }

    #[test]
    fn roundtrip_long() {
        let bytes: Vec<u8> = (0..=255u8).collect();
        let s = Base64::encode(&bytes);
        let r = Base64::decode(&s);
        assert!(r.is_ok(), "decode long: {r:?}");
        if let Ok(v) = r {
            assert_eq!(v, bytes);
        }
    }

    #[test]
    fn reject_non_alphabet() {
        let r = Base64::decode("aGVs!bG8=");
        assert!(r.is_err());
        if let Err(e) = r {
            assert!(e.found.starts_with("invalid base64:"));
        }
    }

    #[test]
    fn reject_bad_padding() {
        let r = Base64::decode("aGVsbG8");
        assert!(r.is_err());
    }
}
