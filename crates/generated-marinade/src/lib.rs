//! Decoder generated from the Marinade Finance Anchor IDL.
//!
//! The contents of [`generated`] are produced by `anchor-gen` from
//! `crates/anchor-gen/tests/fixtures/marinade_idl.json`, which was fetched from
//! the IDL account of `MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD` on mainnet.
//!
//! This crate exists to keep the code generator honest. It is a normal
//! workspace member, so the generated code has to compile on every build, and
//! its tests decode a real account fetched from mainnet. A generator that
//! emitted invalid Rust, or a decoder whose discriminators did not match the
//! chain, would fail here rather than passing a string comparison.
//!
//! Regenerate with:
//!
//! ```sh
//! cargo run -p anchor-gen-cli -- \
//!     crates/anchor-gen/tests/fixtures/marinade_idl.json \
//!     -o crates/generated-marinade/src
//! mv crates/generated-marinade/src/marinade_finance.rs \
//!    crates/generated-marinade/src/generated.rs
//! ```

#![allow(missing_docs)]

pub mod generated;

pub use generated::*;
