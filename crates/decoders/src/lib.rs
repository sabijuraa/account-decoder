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
//! ```rust
//! use account_decoder_core::{AccountDecoder, TypedEvent};
//! use account_decoder_decoders::{Mint, TokenDecoder};
//!
//! // An SPL mint: 82 bytes, with both authorities present.
//! let mut data = Vec::new();
//! data.extend_from_slice(&1u32.to_le_bytes());  // COption::Some
//! data.extend_from_slice(&[1u8; 32]);            // mint authority
//! data.extend_from_slice(&1_000_000u64.to_le_bytes()); // supply
//! data.push(6);                                   // decimals
//! data.push(1);                                   // initialized
//! data.extend_from_slice(&0u32.to_le_bytes());   // COption::None
//! data.extend_from_slice(&[0u8; 32]);
//!
//! let event = TokenDecoder::new().decode_account(&data)?;
//! let mint = event.downcast_ref::<Mint>().expect("a mint");
//! assert_eq!(mint.decimals, 6);
//! assert_eq!(mint.supply, 1_000_000);
//! assert!(mint.freeze_authority.is_none());
//!
//! // Sizes are the only type tag SPL Token has, so anything else is refused.
//! assert!(TokenDecoder::new().decode_account(&[0u8; 100]).is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod raydium_amm;
mod system;
mod token;
mod token_2022;

pub use raydium_amm::{AmmFees, AmmInfo, RaydiumAmmDecoder, RAYDIUM_AMM_V4_PROGRAM_ID};
pub use system::{NonceAccount, SystemAccount, SystemDecoder};
pub use token::{Mint, Multisig, TokenAccount, TokenDecoder};
pub use token_2022::Token2022Decoder;

/// Program IDs for built-in decoders.
pub mod program_ids {
    use solana_sdk::pubkey::Pubkey;

    /// SPL Token program ID.
    pub const TOKEN_PROGRAM_ID: Pubkey =
        solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    /// SPL Token-2022 program ID.
    pub const TOKEN_2022_PROGRAM_ID: Pubkey =
        solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

    /// System program ID.
    pub const SYSTEM_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");
}

/// Re-export core types for convenience.
pub mod prelude {
    pub use crate::program_ids::*;
    pub use crate::system::{SystemAccount, SystemDecoder};
    pub use crate::token::{Mint, TokenAccount, TokenDecoder};
}
