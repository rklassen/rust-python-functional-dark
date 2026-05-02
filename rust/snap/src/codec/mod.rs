//! Codec layer: pure encode/decode transforms between representations.
//!
//! Depends only on `crate::data`. Used by all `io` modules so the
//! edge-weight inner-text format, the base64 stream payload format,
//! and the hex byte-string format share one source of truth.
//!
//! Codecs are zero-state carrier structs. Methods are static (no
//! `&self`, no `&mut self`). Errors propagate as `Vec<SemanticErr>`
//! when one input may produce multiple problems.

pub mod weight_text;
mod weight_text_emit;
mod weight_text_errors;
mod weight_text_parse;
pub mod hex;
pub mod base64;

pub use weight_text::WeightText;
pub use hex::Hex;
pub use base64::Base64;
