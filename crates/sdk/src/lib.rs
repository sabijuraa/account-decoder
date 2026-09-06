//! # Account Decoder SDK
//!
//! The public SDK for the Solana account decoder framework.
//!
//! This crate re-exports everything users need to decode Solana accounts and instructions,
//! providing a unified interface to the decoder ecosystem.
//!
//! ## Quick Start
//!
//! ```rust
//! use account_decoder_sdk::prelude::*;
//! use account_decoder_sdk::{default_registry, program_ids::TOKEN_PROGRAM_ID, TokenAccount};
//!
//! let registry = default_registry();
//!
//! // A 165-byte SPL token account, as it comes off the chain.
//! let mut account_data = vec![0u8; 165];
//! account_data[64..72].copy_from_slice(&1_500_000u64.to_le_bytes()); // amount
//! account_data[108] = 1; // state: initialized
//!
//! match registry.decode_account(&TOKEN_PROGRAM_ID, &account_data) {
//!     Ok(event) => {
//!         println!("{} - {}", event.program_name(), event.event_type());
//!
//!         // Recover the concrete type when you need its fields.
//!         if let Some(token) = event.downcast_ref::<TokenAccount>() {
//!             assert_eq!(token.amount, 1_500_000);
//!         }
//!     }
//!     Err(e) => println!("could not decode: {e}"),
//! }
//! ```
//!
//! ## Features
//!
//! - `default` / `builtin-decoders`: Include built-in decoders for Token, Token-2022, System
//! - `anchor-codegen`: Include the Anchor IDL code generator
//! - `full`: Enable all features
//!
//! ## Custom Decoders
//!
//! Supporting a new program means implementing two traits and registering the
//! result. Nothing in `core` changes.
//!
//! ```rust
//! use account_decoder_sdk::prelude::*;
//! use account_decoder_sdk::{empty_registry, DecoderMetadata, EventKind, Pubkey};
//! use std::any::Any;
//!
//! const MY_PROGRAM_ID: Pubkey = Pubkey::new_from_array([9u8; 32]);
//!
//! /// What this program's accounts mean once decoded.
//! #[derive(Debug)]
//! struct Counter {
//!     count: u64,
//! }
//!
//! impl DecodedEvent for Counter {
//!     fn event_kind(&self) -> EventKind { EventKind::Account }
//!     fn event_type(&self) -> &'static str { "Counter" }
//!     fn program_name(&self) -> &'static str { "my-program" }
//!     fn as_any(&self) -> &dyn Any { self }
//! }
//!
//! #[derive(Debug)]
//! struct MyProgramDecoder;
//!
//! // Identity is separate from decoding, so a decoder can describe itself
//! // before it is asked to do any work.
//! impl DecoderIdentity for MyProgramDecoder {
//!     fn metadata(&self) -> DecoderMetadata {
//!         DecoderMetadata::new("my-program", MY_PROGRAM_ID)
//!     }
//!     fn as_any(&self) -> &dyn Any { self }
//! }
//!
//! impl AccountDecoder for MyProgramDecoder {
//!     fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
//!         let bytes: [u8; 8] = data
//!             .get(..8)
//!             .and_then(|b| b.try_into().ok())
//!             .ok_or_else(|| DecodeError::insufficient_data(8, data.len()))?;
//!         Ok(Box::new(Counter { count: u64::from_le_bytes(bytes) }))
//!     }
//!
//!     fn can_decode(&self, data: &[u8]) -> bool {
//!         data.len() == 8
//!     }
//! }
//!
//! let mut registry = empty_registry();
//! registry.register_account(Box::new(MyProgramDecoder));
//!
//! let event = registry
//!     .decode_account(&MY_PROGRAM_ID, &42u64.to_le_bytes())
//!     .expect("the counter decodes");
//! assert_eq!(event.downcast_ref::<Counter>().unwrap().count, 42);
//!
//! // Truncated data is an error, not a panic.
//! assert!(registry.decode_account(&MY_PROGRAM_ID, &[1, 2, 3]).is_err());
//! ```

// Re-export core types
pub use account_decoder_core::{
    AccountDecoder, ContextualEvent, DecodeError, DecodeResult, DecodedEvent,
    DecoderCapabilities, DecoderIdentity, DecoderInfo, DecoderMetadata, DecoderRegistry,
    EventKind, InstructionDecoder, PartialEvent, ProgramDecoder, RegistryBuilder,
    TypedEvent,
};

// Re-export borsh utilities
pub use account_decoder_borsh_util::{
    read_discriminator, AnchorDiscriminator, BorshSlice, Discriminator, SliceReader,
    ZeroCopyError, ZeroCopyReader,
};

// Re-export built-in decoders (when feature enabled)
#[cfg(feature = "builtin-decoders")]
pub use account_decoder_decoders::{
    program_ids, AmmFees, AmmInfo, Mint, Multisig, NonceAccount, RaydiumAmmDecoder,
    SystemAccount, SystemDecoder, Token2022Decoder, TokenAccount, TokenDecoder,
};

// Re-export anchor codegen (when feature enabled)
#[cfg(feature = "anchor-codegen")]
pub use account_decoder_anchor_gen::{
    CodeGenerator, GeneratorConfig, GeneratorError, IdlAccount, IdlField,
    IdlInstruction, IdlParser, IdlProgram, IdlType, RustType, TypeMapper,
};

// Re-export solana types
pub use solana_sdk::pubkey::Pubkey;

/// Everything needed to decode, and to write a decoder.
///
/// `DecoderIdentity` belongs here as much as `AccountDecoder` does: it is a
/// supertrait of both decoder traits, so a caller who imported only the prelude
/// could not implement one without it.
///
/// ```rust
/// use account_decoder_sdk::prelude::*;
///
/// // The traits, error types and helpers are all in scope.
/// let registry = DecoderRegistry::new();
/// assert_eq!(registry.account_decoder_count(), 0);
///
/// let mut reader = ZeroCopyReader::new(&[1, 0, 0, 0, 0, 0, 0, 0]);
/// assert_eq!(reader.read_u64().unwrap(), 1);
/// ```
pub mod prelude {
    pub use crate::{
        AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities,
        DecoderIdentity, DecoderInfo, DecoderMetadata, DecoderRegistry, EventKind,
        InstructionDecoder, RegistryBuilder, TypedEvent,
    };

    pub use crate::{
        read_discriminator, AnchorDiscriminator, BorshSlice, Discriminator,
        ZeroCopyReader,
    };

    pub use solana_sdk::pubkey::Pubkey;

    #[cfg(feature = "builtin-decoders")]
    pub use crate::{
        program_ids::*, Mint, NonceAccount, SystemAccount, SystemDecoder,
        Token2022Decoder, TokenAccount, TokenDecoder,
    };

    #[cfg(feature = "builtin-decoders")]
    pub use crate::default_registry;
}

/// Create a registry with all built-in decoders pre-registered.
///
/// This is the quickest way to get started with common Solana programs.
///
/// ```rust
/// use account_decoder_sdk::default_registry;
/// use account_decoder_sdk::program_ids::{SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID};
///
/// let registry = default_registry();
///
/// // SPL Token, Token-2022, System and Raydium AMM v4 are all registered.
/// assert!(registry.has_account_decoder(&TOKEN_PROGRAM_ID));
/// assert!(registry.has_account_decoder(&SYSTEM_PROGRAM_ID));
/// assert!(registry.account_decoder_count() >= 4);
///
/// for info in registry.list_account_decoders() {
///     println!("{} -> {}", info.metadata.program_name, info.metadata.program_id);
/// }
/// ```
#[cfg(feature = "builtin-decoders")]
pub fn default_registry() -> DecoderRegistry {
    let mut registry = DecoderRegistry::new();

    // Register Token decoder
    let token_decoder = TokenDecoder::new();
    registry.register_account(Box::new(token_decoder.clone()));
    registry.register_instruction(Box::new(token_decoder));

    // Register Token-2022 decoder
    registry.register_account(Box::new(Token2022Decoder::new()));

    // Register System decoder
    let system_decoder = SystemDecoder::new();
    registry.register_account(Box::new(system_decoder.clone()));
    registry.register_instruction(Box::new(system_decoder));

    // Register Raydium AMM v4
    registry.register_account(Box::new(RaydiumAmmDecoder::new()));

    registry
}

/// Create an empty registry.
///
/// Use this when you want to register only specific decoders.
pub fn empty_registry() -> DecoderRegistry {
    DecoderRegistry::new()
}

/// Builder for creating customized registries.
///
/// ```rust
/// use account_decoder_sdk::{registry_builder, TokenDecoder};
/// use account_decoder_sdk::program_ids::TOKEN_PROGRAM_ID;
///
/// // Start empty and add only what this service actually decodes: an indexer
/// // that never sees Token-2022 should not pay to try it on every account.
/// let registry = registry_builder()
///     .with_account(TokenDecoder::new())
///     .build();
///
/// assert_eq!(registry.account_decoder_count(), 1);
/// assert!(registry.has_account_decoder(&TOKEN_PROGRAM_ID));
/// ```
pub fn registry_builder() -> RegistryBuilder {
    RegistryBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "builtin-decoders")]
    fn test_default_registry() {
        let registry = default_registry();

        // Should have decoders for all built-in programs
        assert!(registry.has_account_decoder(&program_ids::TOKEN_PROGRAM_ID));
        assert!(registry.has_account_decoder(&program_ids::TOKEN_2022_PROGRAM_ID));
        assert!(registry.has_account_decoder(&program_ids::SYSTEM_PROGRAM_ID));
    }

    #[test]
    fn test_empty_registry() {
        let registry = empty_registry();
        assert_eq!(registry.account_decoder_count(), 0);
    }

    #[test]
    #[cfg(feature = "builtin-decoders")]
    fn test_decode_token_mint() {
        let registry = default_registry();

        // Create a minimal mint account
        let mut data = vec![0u8; 82];
        data[44] = 9; // decimals
        data[45] = 1; // is_initialized

        let result = registry.decode_account(&program_ids::TOKEN_PROGRAM_ID, &data);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.event_type(), "Mint");
        assert_eq!(event.program_name(), "spl-token");
    }
}
