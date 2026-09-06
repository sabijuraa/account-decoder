//! # Account Decoder Anchor Generator
//!
//! Generate decoder implementations from Anchor IDL JSON files.
//!
//! This crate provides tools for automatically generating type-safe decoders
//! from Anchor program IDLs, eliminating the need to manually write parsing code.
//!
//! ## Pipeline Overview
//!
//! ```text
//! IDL JSON File
//!       |
//!       v
//! [Parse IDL] ─────> IdlProgram struct
//!       |
//!       v
//! [Generate Types] ─> Rust struct definitions
//!       |
//!       v
//! [Generate Decoders] ─> AccountDecoder/InstructionDecoder impls
//!       |
//!       v
//! Rust source code (TokenStream)
//! ```
//!
//! ## Example
//!
//! ```rust
//! use account_decoder_anchor_gen::{CodeGenerator, GeneratorConfig, IdlParser};
//!
//! // A minimal IDL: one account type with two fields.
//! let idl_json = r#"{
//!   "version": "0.1.0",
//!   "name": "counter",
//!   "instructions": [],
//!   "accounts": [{
//!     "name": "Counter",
//!     "type": { "kind": "struct", "fields": [
//!       { "name": "authority", "type": "publicKey" },
//!       { "name": "count", "type": "u64" }
//!     ]}
//!   }]
//! }"#;
//!
//! let idl = IdlParser::parse(idl_json)?;
//! assert_eq!(idl.name, "counter");
//!
//! let code = CodeGenerator::new(GeneratorConfig::default())
//!     .generate(&idl)?
//!     .to_string();
//!
//! // The output is real Rust: a struct for the account and a decoder that
//! // dispatches on Anchor's discriminator.
//! assert!(code.contains("Counter"));
//! assert!(code.contains("AccountDecoder"));
//!
//! // And it parses as Rust, which is the check that matters -- a generator
//! // that emitted something almost-valid would pass a string comparison.
//! syn::parse_file(&code).expect("generated code is valid Rust");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod codegen;
mod idl;
mod types;

pub use codegen::{CodeGenerator, GeneratorConfig, GeneratorError};
pub use idl::{IdlAccount, IdlField, IdlInstruction, IdlParser, IdlProgram, IdlType};
pub use types::{RustType, TypeMapper};
