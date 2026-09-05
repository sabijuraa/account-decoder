//! Decoder registry for program ID to decoder mapping.
//!
//! The registry is the central component that routes decode requests
//! to the appropriate decoder based on program ID.

use crate::decoder::{AccountDecoder, DecoderIdentity, DecoderInfo, InstructionDecoder};
use crate::error::{DecodeError, DecodeResult};
use crate::event::DecodedEvent;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace, warn};

/// A registry mapping program IDs to their decoders.
///
/// The registry provides O(1) lookup of decoders by program ID and supports
/// both account and instruction decoders.
///
/// # Thread Safety
///
/// The registry uses `Arc` for decoder storage, making it safe to share
/// across threads. Individual decoders must be `Send + Sync`.
///
/// # Example
///
/// ```rust,ignore
/// use account_decoder_core::{DecoderRegistry, AccountDecoder};
///
/// let mut registry = DecoderRegistry::new();
///
/// // Register decoders
/// registry.register_account(Box::new(TokenDecoder::new()));
/// registry.register_account(Box::new(SystemDecoder::new()));
///
/// // Look up and use a decoder
/// let program_id = spl_token::id();
/// if let Some(decoder) = registry.get_account(&program_id) {
///     let event = decoder.decode_account(&account_data)?;
/// }
/// ```
#[derive(Default)]
pub struct DecoderRegistry {
    /// Account decoders indexed by program ID.
    account_decoders: HashMap<Pubkey, Arc<dyn AccountDecoder>>,

    /// Instruction decoders indexed by program ID.
    instruction_decoders: HashMap<Pubkey, Arc<dyn InstructionDecoder>>,

    /// Whether to log warnings for unknown programs.
    warn_on_unknown: bool,
}

impl DecoderRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with unknown program warnings enabled.
    pub fn with_warnings() -> Self {
        Self {
            warn_on_unknown: true,
            ..Self::default()
        }
    }

    /// Register an account decoder.
    ///
    /// If a decoder is already registered for this program, it will be replaced.
    pub fn register_account(&mut self, decoder: Box<dyn AccountDecoder>) {
        let program_id = decoder.metadata().program_id;
        let program_name = decoder.metadata().program_name;

        debug!(
            program_id = %program_id,
            program_name = program_name,
            "Registering account decoder"
        );

        self.account_decoders.insert(program_id, Arc::from(decoder));
    }

    /// Register an instruction decoder.
    pub fn register_instruction(&mut self, decoder: Box<dyn InstructionDecoder>) {
        let program_id = decoder.metadata().program_id;
        let program_name = decoder.metadata().program_name;

        debug!(
            program_id = %program_id,
            program_name = program_name,
            "Registering instruction decoder"
        );

        self.instruction_decoders
            .insert(program_id, Arc::from(decoder));
    }

    /// Register a decoder that handles both accounts and instructions.
    ///
    /// This is a convenience method for programs where the same type implements both traits.
    pub fn register_combined<D>(&mut self, decoder: D)
    where
        D: AccountDecoder + InstructionDecoder + Clone + 'static,
    {
        let program_id = decoder.metadata().program_id;
        let program_name = decoder.metadata().program_name;

        debug!(
            program_id = %program_id,
            program_name = program_name,
            "Registering combined decoder"
        );

        self.account_decoders
            .insert(program_id, Arc::new(decoder.clone()));
        self.instruction_decoders.insert(program_id, Arc::new(decoder));
    }

    /// Get an account decoder for a program.
    pub fn get_account(&self, program_id: &Pubkey) -> Option<Arc<dyn AccountDecoder>> {
        let decoder = self.account_decoders.get(program_id).cloned();

        if decoder.is_none() && self.warn_on_unknown {
            warn!(program_id = %program_id, "No account decoder registered");
        } else {
            trace!(program_id = %program_id, found = decoder.is_some(), "Account decoder lookup");
        }

        decoder
    }

    /// Get an instruction decoder for a program.
    pub fn get_instruction(&self, program_id: &Pubkey) -> Option<Arc<dyn InstructionDecoder>> {
        let decoder = self.instruction_decoders.get(program_id).cloned();

        if decoder.is_none() && self.warn_on_unknown {
            warn!(program_id = %program_id, "No instruction decoder registered");
        }

        decoder
    }

    /// Decode account data, returning an error if no decoder is registered.
    pub fn decode_account(
        &self,
        program_id: &Pubkey,
        data: &[u8],
    ) -> DecodeResult<Box<dyn DecodedEvent>> {
        let decoder = self
            .get_account(program_id)
            .ok_or_else(|| DecodeError::UnknownProgram(*program_id))?;

        decoder.decode_account(data)
    }

    /// Decode instruction data, returning an error if no decoder is registered.
    pub fn decode_instruction(
        &self,
        program_id: &Pubkey,
        data: &[u8],
    ) -> DecodeResult<Box<dyn DecodedEvent>> {
        let decoder = self
            .get_instruction(program_id)
            .ok_or_else(|| DecodeError::UnknownProgram(*program_id))?;

        decoder.decode_instruction(data)
    }

    /// Try to decode account data, returning `None` for unknown programs.
    ///
    /// This is useful when processing many accounts where some may be from
    /// unregistered programs.
    pub fn try_decode_account(
        &self,
        program_id: &Pubkey,
        data: &[u8],
    ) -> Option<DecodeResult<Box<dyn DecodedEvent>>> {
        self.get_account(program_id)
            .map(|decoder| decoder.decode_account(data))
    }

    /// Check if an account decoder is registered for a program.
    pub fn has_account_decoder(&self, program_id: &Pubkey) -> bool {
        self.account_decoders.contains_key(program_id)
    }

    /// Check if an instruction decoder is registered for a program.
    pub fn has_instruction_decoder(&self, program_id: &Pubkey) -> bool {
        self.instruction_decoders.contains_key(program_id)
    }

    /// Get information about all registered account decoders.
    pub fn list_account_decoders(&self) -> Vec<DecoderInfo> {
        self.account_decoders
            .values()
            .map(|d| d.info())
            .collect()
    }

    /// Get information about all registered instruction decoders.
    pub fn list_instruction_decoders(&self) -> Vec<DecoderInfo> {
        self.instruction_decoders
            .values()
            .map(|d| d.info())
            .collect()
    }

    /// Get the number of registered account decoders.
    pub fn account_decoder_count(&self) -> usize {
        self.account_decoders.len()
    }

    /// Get the number of registered instruction decoders.
    pub fn instruction_decoder_count(&self) -> usize {
        self.instruction_decoders.len()
    }

    /// Remove an account decoder.
    pub fn unregister_account(&mut self, program_id: &Pubkey) -> bool {
        self.account_decoders.remove(program_id).is_some()
    }

    /// Remove an instruction decoder.
    pub fn unregister_instruction(&mut self, program_id: &Pubkey) -> bool {
        self.instruction_decoders.remove(program_id).is_some()
    }

    /// Clear all registered decoders.
    pub fn clear(&mut self) {
        self.account_decoders.clear();
        self.instruction_decoders.clear();
    }
}

impl std::fmt::Debug for DecoderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecoderRegistry")
            .field("account_decoders", &self.account_decoders.len())
            .field("instruction_decoders", &self.instruction_decoders.len())
            .field("warn_on_unknown", &self.warn_on_unknown)
            .finish()
    }
}

/// Builder for constructing a decoder registry.
///
/// Provides a fluent API for registering multiple decoders.
///
/// # Example
///
/// ```rust,ignore
/// let registry = RegistryBuilder::new()
///     .with_account(TokenDecoder::new())
///     .with_account(SystemDecoder::new())
///     .with_warnings(true)
///     .build();
/// ```
#[derive(Default)]
pub struct RegistryBuilder {
    registry: DecoderRegistry,
}

impl RegistryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an account decoder.
    pub fn with_account(mut self, decoder: impl AccountDecoder + 'static) -> Self {
        self.registry.register_account(Box::new(decoder));
        self
    }

    /// Add an instruction decoder.
    pub fn with_instruction(mut self, decoder: impl InstructionDecoder + 'static) -> Self {
        self.registry.register_instruction(Box::new(decoder));
        self
    }

    /// Enable or disable warnings for unknown programs.
    pub fn with_warnings(mut self, enabled: bool) -> Self {
        self.registry.warn_on_unknown = enabled;
        self
    }

    /// Build the registry.
    pub fn build(self) -> DecoderRegistry {
        self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::DecoderMetadata;
    use crate::event::EventKind;
    use std::any::Any;

    #[derive(Debug, Clone)]
    struct MockDecoder {
        program_id: Pubkey,
    }

    impl DecoderIdentity for MockDecoder {
        fn metadata(&self) -> DecoderMetadata {
            DecoderMetadata::new("Mock", self.program_id)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl AccountDecoder for MockDecoder {
        fn decode_account(&self, _data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
            Ok(Box::new(MockEvent))
        }
    }

    impl InstructionDecoder for MockDecoder {
        fn decode_instruction(&self, _data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
            Ok(Box::new(MockEvent))
        }
    }

    #[derive(Debug)]
    struct MockEvent;

    impl DecodedEvent for MockEvent {
        fn event_kind(&self) -> EventKind {
            EventKind::Account
        }

        fn event_type(&self) -> &'static str {
            "MockEvent"
        }

        fn program_name(&self) -> &'static str {
            "mock"
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_register_and_lookup() {
        let mut registry = DecoderRegistry::new();
        let program_id = Pubkey::new_unique();

        registry.register_account(Box::new(MockDecoder { program_id }));

        assert!(registry.has_account_decoder(&program_id));
        assert!(!registry.has_instruction_decoder(&program_id));
        assert!(registry.get_account(&program_id).is_some());
    }

    #[test]
    fn test_decode_unknown_program() {
        let registry = DecoderRegistry::new();
        let unknown_id = Pubkey::new_unique();

        let result = registry.decode_account(&unknown_id, &[1, 2, 3]);
        assert!(matches!(result, Err(DecodeError::UnknownProgram(_))));
    }

    #[test]
    fn test_builder() {
        let program_id = Pubkey::new_unique();

        let registry = RegistryBuilder::new()
            .with_account(MockDecoder { program_id })
            .with_warnings(true)
            .build();

        assert!(registry.has_account_decoder(&program_id));
        assert!(registry.warn_on_unknown);
    }

    #[test]
    fn test_register_combined() {
        let mut registry = DecoderRegistry::new();
        let program_id = Pubkey::new_unique();

        registry.register_combined(MockDecoder { program_id });

        assert!(registry.has_account_decoder(&program_id));
        assert!(registry.has_instruction_decoder(&program_id));
    }
}
