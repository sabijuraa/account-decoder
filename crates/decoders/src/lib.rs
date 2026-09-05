//! # Account Decoder Decoders
//!
//! Built-in decoders for common Solana programs.
//!
//! This crate provides ready-to-use decoders for the most common Solana programs:
//!
//! - **Token Program** (`spl-token`): Token accounts, mints, multisigs
//! - **Token-2022** (`spl-token-2022`): Extended token functionality
//! - **System Program**: Native accounts, nonces
//!
//! ## Usage
//!
//! ```rust,ignore
//! use account_decoder_decoders::{TokenDecoder, SystemDecoder};
//! use account_decoder_core::DecoderRegistry;
//!
//! let mut registry = DecoderRegistry::new();
//! registry.register_account(Box::new(TokenDecoder::new()));
//! registry.register_account(Box::new(SystemDecoder::new()));
//! ```

mod system;
mod token;
mod token_2022;

pub use system::{SystemDecoder, SystemAccount, NonceAccount};
pub use token::{TokenDecoder, TokenAccount, Mint, Multisig};
pub use token_2022::Token2022Decoder;

/// Program IDs for built-in decoders.
pub mod program_ids {
    use solana_sdk::pubkey::Pubkey;

    /// SPL Token program ID.
    pub const TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    /// SPL Token-2022 program ID.
    pub const TOKEN_2022_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

    /// System program ID.
    pub const SYSTEM_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");
}

/// Re-export core types for convenience.
pub mod prelude {
    pub use crate::system::{SystemDecoder, SystemAccount};
    pub use crate::token::{TokenDecoder, TokenAccount, Mint};
    pub use crate::program_ids::*;
}
