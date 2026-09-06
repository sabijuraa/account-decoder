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
//! ```rust
//! use account_decoder_core::{
//!     AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderIdentity,
//!     DecoderMetadata, DecoderRegistry, EventKind, TypedEvent,
//! };
//! use solana_sdk::pubkey::Pubkey;
//! use std::any::Any;
//!
//! const PROGRAM: Pubkey = Pubkey::new_from_array([1u8; 32]);
//!
//! #[derive(Debug)]
//! struct Balance(u64);
//!
//! impl DecodedEvent for Balance {
//!     fn event_kind(&self) -> EventKind { EventKind::Account }
//!     fn event_type(&self) -> &'static str { "Balance" }
//!     fn program_name(&self) -> &'static str { "example" }
//!     fn as_any(&self) -> &dyn Any { self }
//! }
//!
//! #[derive(Debug)]
//! struct Example;
//!
//! impl DecoderIdentity for Example {
//!     fn metadata(&self) -> DecoderMetadata { DecoderMetadata::new("example", PROGRAM) }
//!     fn as_any(&self) -> &dyn Any { self }
//! }
//!
//! impl AccountDecoder for Example {
//!     fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
//!         let bytes: [u8; 8] = data.get(..8)
//!             .and_then(|b| b.try_into().ok())
//!             .ok_or_else(|| DecodeError::insufficient_data(8, data.len()))?;
//!         Ok(Box::new(Balance(u64::from_le_bytes(bytes))))
//!     }
//!     fn can_decode(&self, data: &[u8]) -> bool { data.len() == 8 }
//! }
//!
//! let mut registry = DecoderRegistry::new();
//! registry.register_account(Box::new(Example));
//!
//! let event = registry.decode_account(&PROGRAM, &99u64.to_le_bytes())?;
//! assert_eq!(event.downcast_ref::<Balance>().unwrap().0, 99);
//!
//! // A program nobody registered is a clean error, not a panic.
//! let unknown = Pubkey::new_from_array([2u8; 32]);
//! assert!(matches!(
//!     registry.decode_account(&unknown, &[0u8; 8]),
//!     Err(DecodeError::UnknownProgram(_))
//! ));
//! # Ok::<(), Box<dyn std::error::Error>>(())
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
    pub use crate::decoder::{
        AccountDecoder, DecoderIdentity, DecoderMetadata, InstructionDecoder,
    };
    pub use crate::error::{DecodeError, DecodeResult};
    pub use crate::event::{DecodedEvent, EventKind, TypedEvent};
    pub use crate::registry::DecoderRegistry;
}
