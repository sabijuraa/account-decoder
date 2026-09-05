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
//! ```rust,ignore
//! use account_decoder_anchor_gen::{IdlParser, CodeGenerator, GeneratorConfig};
//!
//! // Parse an IDL file
//! let idl_json = std::fs::read_to_string("target/idl/my_program.json")?;
//! let idl = IdlParser::parse(&idl_json)?;
//!
//! // Generate decoder code
//! let config = GeneratorConfig::default();
//! let generator = CodeGenerator::new(config);
//! let code = generator.generate(&idl)?;
//!
//! // Write to a file
//! std::fs::write("src/generated/my_program.rs", code.to_string())?;
//! ```

mod codegen;
mod idl;
mod types;

pub use codegen::{CodeGenerator, GeneratorConfig, GeneratorError};
pub use idl::{IdlParser, IdlProgram, IdlAccount, IdlInstruction, IdlType, IdlField};
pub use types::{TypeMapper, RustType};
