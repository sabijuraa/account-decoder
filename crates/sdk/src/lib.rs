//! # Account Decoder SDK
//!
//! The public SDK for the Solana account decoder framework.
//!
//! This crate re-exports everything users need to decode Solana accounts and instructions,
//! providing a unified interface to the decoder ecosystem.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use account_decoder_sdk::prelude::*;
//!
//! // Create a registry with built-in decoders
//! let registry = default_registry();
//!
//! // Decode an account
//! let program_id = TOKEN_PROGRAM_ID;
//! let account_data: &[u8] = /* raw account data */;
//!
//! match registry.decode_account(&program_id, account_data) {
//!     Ok(event) => {
//!         println!("Decoded: {} - {}", event.program_name(), event.event_type());
//!
//!         // Downcast to specific type if needed
//!         if let Some(token) = event.downcast_ref::<TokenAccount>() {
//!             println!("Balance: {}", token.amount);
//!         }
//!     }
//!     Err(DecodeError::UnknownProgram(_)) => {
//!         println!("No decoder for this program");
//!     }
//!     Err(e) => {
//!         println!("Decode error: {}", e);
//!     }
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
//! Implement `AccountDecoder` or `InstructionDecoder` to add support for new programs:
//!
//! ```rust,ignore
//! use account_decoder_sdk::prelude::*;
//!
//! #[derive(Debug)]
//! struct MyProgramDecoder;
//!
//! impl AccountDecoder for MyProgramDecoder {
//!     fn metadata(&self) -> DecoderMetadata {
//!         DecoderMetadata::new("my-program", MY_PROGRAM_ID)
//!     }
//!
//!     fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
//!         // Parse the data and return a DecodedEvent
//!         todo!()
//!     }
//!
//!     fn as_any(&self) -> &dyn std::any::Any {
//!         self
//!     }
//! }
//! ```

// Re-export core types
pub use account_decoder_core::{
    AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities,
    DecoderInfo, DecoderMetadata, EventKind, InstructionDecoder, DecoderRegistry,
    RegistryBuilder, TypedEvent,
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

/// Prelude module for convenient imports.
///
/// ```rust,ignore
/// use account_decoder_sdk::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities,
        DecoderInfo, DecoderMetadata, DecoderRegistry, EventKind, InstructionDecoder,
        RegistryBuilder, TypedEvent,
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
/// ```rust,ignore
/// use account_decoder_sdk::default_registry;
///
/// let registry = default_registry();
/// // Registry now contains decoders for:
/// // - SPL Token
/// // - SPL Token-2022
/// // - System Program
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
/// ```rust,ignore
/// use account_decoder_sdk::{registry_builder, TokenDecoder};
///
/// let registry = registry_builder()
///     .with_account(TokenDecoder::new())
///     .with_warnings(true)
///     .build();
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
