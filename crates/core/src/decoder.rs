//! Decoder trait definitions.
//!
//! This module defines the core traits that all decoders must implement.
//! The trait hierarchy is designed to be:
//!
//! - **Ergonomic**: Easy to implement for common cases
//! - **Flexible**: Supports both simple and complex decoding scenarios
//! - **Extensible**: New decoder types can be added without breaking changes

use crate::error::DecodeResult;
use crate::event::DecodedEvent;
use solana_sdk::pubkey::Pubkey;
use std::any::Any;
use std::fmt::Debug;

/// Metadata about a decoder, used for registration and debugging.
///
/// This provides human-readable information about what a decoder handles,
/// making it easier to debug registry lookups and understand the decoder landscape.
#[derive(Debug, Clone)]
pub struct DecoderMetadata {
    /// Human-readable name of the program this decoder handles.
    pub program_name: &'static str,

    /// The program ID this decoder is registered for.
    pub program_id: Pubkey,

    /// Version of the decoder implementation.
    pub version: &'static str,

    /// Optional description of what this decoder handles.
    pub description: Option<&'static str>,
}

impl DecoderMetadata {
    /// Create new decoder metadata.
    pub const fn new(program_name: &'static str, program_id: Pubkey) -> Self {
        Self {
            program_name,
            program_id,
            version: env!("CARGO_PKG_VERSION"),
            description: None,
        }
    }

    /// Add a description to the metadata.
    pub const fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }
}

/// Information about a decoder's capabilities.
///
/// This helps consumers understand what features a decoder supports,
/// enabling graceful degradation when certain features aren't available.
#[derive(Debug, Clone, Default)]
pub struct DecoderCapabilities {
    /// Whether this decoder supports zero-copy deserialization.
    pub zero_copy: bool,

    /// Whether this decoder can provide detailed field-level information.
    pub detailed_fields: bool,

    /// Whether this decoder supports incremental/streaming decoding.
    pub streaming: bool,

    /// List of account types this decoder can handle.
    pub account_types: Vec<&'static str>,

    /// List of instruction types this decoder can handle.
    pub instruction_types: Vec<&'static str>,
}

impl DecoderCapabilities {
    /// Create capabilities with zero-copy support.
    pub fn with_zero_copy(mut self) -> Self {
        self.zero_copy = true;
        self
    }

    /// Add account types this decoder handles.
    pub fn with_account_types(mut self, types: Vec<&'static str>) -> Self {
        self.account_types = types;
        self
    }

    /// Add instruction types this decoder handles.
    pub fn with_instruction_types(mut self, types: Vec<&'static str>) -> Self {
        self.instruction_types = types;
        self
    }
}

/// Extended decoder information combining metadata and capabilities.
#[derive(Debug, Clone)]
pub struct DecoderInfo {
    /// Basic metadata about the decoder.
    pub metadata: DecoderMetadata,

    /// Capabilities this decoder supports.
    pub capabilities: DecoderCapabilities,
}

/// Identity and capability reporting, shared by both decoder traits.
///
/// This exists as a separate supertrait so that `metadata()`, `info()` and
/// `as_any()` are declared exactly once. A program decoder normally implements
/// both [`AccountDecoder`] and [`InstructionDecoder`]; if each declared its own
/// `metadata()`, every call on such a type would be ambiguous and the
/// [`ProgramDecoder`] blanket impl would not compile.
pub trait DecoderIdentity: Send + Sync + Debug {
    /// Returns metadata about this decoder.
    fn metadata(&self) -> DecoderMetadata;

    /// Returns the capabilities of this decoder.
    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities::default()
    }

    /// Returns combined info (metadata + capabilities).
    fn info(&self) -> DecoderInfo {
        DecoderInfo {
            metadata: self.metadata(),
            capabilities: self.capabilities(),
        }
    }

    /// Allow downcasting to concrete decoder types.
    fn as_any(&self) -> &dyn Any;
}

/// Trait for decoding Solana account data.
///
/// Implementors parse raw account bytes into structured [`DecodedEvent`] values.
/// Each decoder is responsible for a single program and knows how to interpret
/// all account types that program creates.
///
/// Identity (`metadata`, `capabilities`, `info`, `as_any`) comes from
/// [`DecoderIdentity`], which must be implemented alongside this trait.
///
/// # Implementing a Decoder
///
/// ```rust,ignore
/// use account_decoder_core::{
///     AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderIdentity, DecoderMetadata,
/// };
/// use std::any::Any;
///
/// #[derive(Debug)]
/// struct MyProgramDecoder;
///
/// impl DecoderIdentity for MyProgramDecoder {
///     fn metadata(&self) -> DecoderMetadata {
///         DecoderMetadata::new("MyProgram", my_program::ID)
///     }
///
///     fn as_any(&self) -> &dyn Any {
///         self
///     }
/// }
///
/// impl AccountDecoder for MyProgramDecoder {
///     fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
///         let discriminator = data.first().ok_or(DecodeError::InsufficientData {
///             needed: 1,
///             available: 0,
///         })?;
///         match discriminator {
///             0 => self.decode_state_v1(data),
///             1 => self.decode_state_v2(data),
///             other => Err(DecodeError::UnknownDiscriminator(vec![*other])),
///         }
///     }
/// }
/// ```
pub trait AccountDecoder: DecoderIdentity {
    /// Decode raw account data into a structured event.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw account data bytes
    ///
    /// # Returns
    ///
    /// A boxed [`DecodedEvent`] on success, or a [`DecodeError`] on failure.
    ///
    /// # Errors
    ///
    /// - [`DecodeError::InsufficientData`] if the data is too short
    /// - [`DecodeError::UnknownDiscriminator`] if the account type is unrecognized
    /// - [`DecodeError::DeserializationError`] if borsh parsing fails
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>>;

    /// Decode account data with additional context.
    ///
    /// This variant provides the account's public key, which some decoders
    /// may use for validation or to include in the decoded event.
    ///
    /// Default implementation ignores the pubkey and delegates to [`decode_account`].
    fn decode_account_with_key(
        &self,
        pubkey: &Pubkey,
        data: &[u8],
    ) -> DecodeResult<Box<dyn DecodedEvent>> {
        let _ = pubkey;
        self.decode_account(data)
    }

    /// Check if this decoder can handle the given account data.
    ///
    /// This is a quick heuristic check, typically examining the discriminator
    /// without fully parsing the data. Useful for routing decisions.
    ///
    /// Default implementation returns `true` (optimistic).
    fn can_decode(&self, data: &[u8]) -> bool {
        !data.is_empty()
    }
}

/// Trait for decoding Solana instruction data.
///
/// Complementary to [`AccountDecoder`], this trait handles instruction parsing.
/// Instructions contain the "what to do" while accounts contain the "state".
///
/// # Design
///
/// Instructions are typically simpler than accounts (smaller, single-purpose),
/// but may reference multiple accounts. The decoder receives the instruction
/// data and can optionally receive the account keys for validation.
pub trait InstructionDecoder: DecoderIdentity {
    /// Decode raw instruction data.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw instruction data bytes
    ///
    /// # Returns
    ///
    /// A boxed [`DecodedEvent`] representing the instruction.
    fn decode_instruction(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>>;

    /// Decode instruction with account keys context.
    ///
    /// Some instructions' meaning depends on which accounts are passed.
    /// This variant provides that context for more complete decoding.
    fn decode_instruction_with_accounts(
        &self,
        data: &[u8],
        accounts: &[Pubkey],
    ) -> DecodeResult<Box<dyn DecodedEvent>> {
        let _ = accounts;
        self.decode_instruction(data)
    }
}

/// Combined decoder that handles both accounts and instructions.
///
/// Many programs need decoders for both. This trait provides a unified interface.
pub trait ProgramDecoder: AccountDecoder + InstructionDecoder {
    /// Get the program ID this decoder handles.
    fn program_id(&self) -> Pubkey {
        self.metadata().program_id
    }
}

// Blanket implementation for types that implement both traits
impl<T: AccountDecoder + InstructionDecoder> ProgramDecoder for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockDecoder;

    impl DecoderIdentity for MockDecoder {
        fn metadata(&self) -> DecoderMetadata {
            DecoderMetadata::new("Mock", Pubkey::default())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl AccountDecoder for MockDecoder {
        fn decode_account(&self, _data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
            Err(crate::error::DecodeError::insufficient_data(1, 0))
        }
    }

    #[test]
    fn test_metadata_creation() {
        let meta = DecoderMetadata::new("Test", Pubkey::default());
        assert_eq!(meta.program_name, "Test");
        assert!(meta.description.is_none());

        let meta_with_desc = meta.with_description("A test decoder");
        assert_eq!(meta_with_desc.description, Some("A test decoder"));
    }

    #[test]
    fn test_capabilities() {
        let caps = DecoderCapabilities::default()
            .with_zero_copy()
            .with_account_types(vec!["TokenAccount", "Mint"]);

        assert!(caps.zero_copy);
        assert_eq!(caps.account_types.len(), 2);
    }
}
