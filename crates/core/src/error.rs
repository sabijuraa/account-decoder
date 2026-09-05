//! Error types for the decoder framework.
//!
//! This module defines a comprehensive error hierarchy that:
//! - Distinguishes between recoverable and unrecoverable errors
//! - Provides detailed context for debugging
//! - Supports conversion from underlying library errors

use solana_sdk::pubkey::Pubkey;
use std::fmt;
use thiserror::Error;

/// Result type alias for decoder operations.
pub type DecodeResult<T> = Result<T, DecodeError>;

/// Errors that can occur during decoding.
///
/// The error variants are ordered by likelihood in typical usage,
/// with the most common errors first for efficient matching.
#[derive(Error, Debug)]
pub enum DecodeError {
    /// The input data is too short to contain valid account/instruction data.
    ///
    /// This is the most common error, often indicating truncated data or
    /// an attempt to decode an uninitialized account.
    #[error("insufficient data: expected at least {expected} bytes, got {actual}")]
    InsufficientData {
        /// Minimum bytes expected.
        expected: usize,
        /// Actual bytes received.
        actual: usize,
    },

    /// The discriminator byte(s) don't match any known account/instruction type.
    ///
    /// For Anchor programs, this is typically the first 8 bytes.
    /// For native programs, it's often a single byte.
    #[error("unknown discriminator: {0:?}")]
    UnknownDiscriminator(Vec<u8>),

    /// Borsh deserialization failed.
    ///
    /// Contains the underlying error message for debugging.
    #[error("deserialization failed: {0}")]
    DeserializationError(String),

    /// No decoder registered for the given program ID.
    #[error("no decoder registered for program: {0}")]
    UnknownProgram(Pubkey),

    /// The data doesn't match the expected format for this decoder.
    ///
    /// This is more specific than `DeserializationError` and indicates
    /// the data was valid but not for this program/version.
    #[error("invalid data format: {0}")]
    InvalidFormat(String),

    /// Version mismatch - the account uses a newer/older schema.
    #[error("version mismatch: decoder supports {supported}, found {found}")]
    VersionMismatch {
        /// Version(s) the decoder supports.
        supported: String,
        /// Version found in the data.
        found: String,
    },

    /// A field value is outside the expected range.
    #[error("field '{field}' has invalid value: {message}")]
    InvalidField {
        /// Name of the field.
        field: &'static str,
        /// Description of why it's invalid.
        message: String,
    },

    /// The account is closed/uninitialized (zero lamports, zeroed data).
    #[error("account is closed or uninitialized")]
    ClosedAccount,

    /// Internal decoder error - this shouldn't happen in normal operation.
    #[error("internal error: {0}")]
    Internal(String),
}

impl DecodeError {
    /// Create an `InsufficientData` error.
    pub fn insufficient_data(expected: usize, actual: usize) -> Self {
        Self::InsufficientData { expected, actual }
    }

    /// Create an `UnknownDiscriminator` error from a byte slice.
    pub fn unknown_discriminator(discriminator: &[u8]) -> Self {
        Self::UnknownDiscriminator(discriminator.to_vec())
    }

    /// Create a `DeserializationError` from any error type.
    pub fn deserialization<E: fmt::Display>(error: E) -> Self {
        Self::DeserializationError(error.to_string())
    }

    /// Create an `InvalidFormat` error.
    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::InvalidFormat(message.into())
    }

    /// Create a `VersionMismatch` error.
    pub fn version_mismatch(supported: impl Into<String>, found: impl Into<String>) -> Self {
        Self::VersionMismatch {
            supported: supported.into(),
            found: found.into(),
        }
    }

    /// Create an `InvalidField` error.
    pub fn invalid_field(field: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidField {
            field,
            message: message.into(),
        }
    }

    /// Check if this error indicates the data might be for a different program.
    ///
    /// Useful for multi-decoder routing where we try decoders in sequence.
    pub fn is_wrong_program(&self) -> bool {
        matches!(
            self,
            Self::UnknownDiscriminator(_) | Self::InvalidFormat(_) | Self::VersionMismatch { .. }
        )
    }

    /// Check if this error indicates corrupt or truncated data.
    pub fn is_data_corruption(&self) -> bool {
        matches!(
            self,
            Self::InsufficientData { .. } | Self::DeserializationError(_)
        )
    }
}

/// Convert from borsh errors.
impl From<borsh::io::Error> for DecodeError {
    fn from(err: borsh::io::Error) -> Self {
        Self::DeserializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = DecodeError::insufficient_data(32, 16);
        assert!(matches!(err, DecodeError::InsufficientData { expected: 32, actual: 16 }));

        let err = DecodeError::unknown_discriminator(&[1, 2, 3]);
        assert!(matches!(err, DecodeError::UnknownDiscriminator(ref v) if v == &[1, 2, 3]));
    }

    #[test]
    fn test_is_wrong_program() {
        assert!(DecodeError::unknown_discriminator(&[0]).is_wrong_program());
        assert!(DecodeError::invalid_format("test").is_wrong_program());
        assert!(!DecodeError::insufficient_data(10, 5).is_wrong_program());
    }

    #[test]
    fn test_is_data_corruption() {
        assert!(DecodeError::insufficient_data(10, 5).is_data_corruption());
        assert!(DecodeError::deserialization("parse error").is_data_corruption());
        assert!(!DecodeError::unknown_discriminator(&[0]).is_data_corruption());
    }
}
