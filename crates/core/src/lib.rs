//! # Account Decoder Core
//!
//! Core traits, types, and registry for the pluggable Solana account decoder framework.
//!
//! This crate provides the foundational abstractions for building extensible decoders
//! that can parse raw Solana account and instruction data into structured, typed events.
//!
//! ## Key Concepts
//!
//! - **Decoder Traits**: [`AccountDecoder`] and [`InstructionDecoder`] define the interface
//!   for parsing account state and instruction data respectively.
//! - **Registry**: [`DecoderRegistry`] maps program IDs to their corresponding decoders,
//!   enabling dynamic dispatch based on the program that owns an account.
//! - **Events**: Decoded data is represented as [`DecodedEvent`], which can be downcast
//!   to program-specific types.
//!
//! ## Architecture
//!
//! ```text
//! Raw Account Data ──> Registry.get(program_id) ──> Decoder ──> DecodedEvent
//!                              │
//!                              └── Unknown program? ──> UnknownProgram error
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use account_decoder_core::{DecoderRegistry, AccountDecoder, DecodedEvent};
//! use solana_sdk::pubkey::Pubkey;
//!
//! // Create a registry and register decoders
//! let mut registry = DecoderRegistry::new();
//! registry.register(TokenDecoder::new());
//!
//! // Decode an account
//! let program_id = spl_token::id();
//! let account_data: &[u8] = /* ... */;
//!
//! if let Some(decoder) = registry.get(&program_id) {
//!     let event = decoder.decode_account(account_data)?;
//!     // Handle the decoded event
//! }
//! ```

mod decoder;
mod error;
mod event;
mod registry;

pub use decoder::{
    AccountDecoder, DecoderCapabilities, DecoderIdentity, DecoderInfo, DecoderMetadata,
    InstructionDecoder, ProgramDecoder,
};
pub use error::{DecodeError, DecodeResult};
pub use event::{ContextualEvent, DecodedEvent, EventKind, PartialEvent, TypedEvent};
pub use registry::{DecoderRegistry, RegistryBuilder};

/// Re-export commonly used types from solana-sdk for convenience.
pub mod solana {
    pub use solana_sdk::pubkey::Pubkey;
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::decoder::{AccountDecoder, DecoderIdentity, DecoderMetadata, InstructionDecoder};
    pub use crate::error::{DecodeError, DecodeResult};
    pub use crate::event::{DecodedEvent, EventKind, TypedEvent};
    pub use crate::registry::DecoderRegistry;
}
