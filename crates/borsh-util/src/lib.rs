//! # Account Decoder Borsh Utilities
//!
//! Zero-copy and efficient Borsh deserialization utilities optimized for
//! high-throughput Solana account indexing.
//!
//! ## When to Use Zero-Copy
//!
//! Zero-copy deserialization avoids allocating new memory by referencing
//! the original buffer directly. This is beneficial when:
//!
//! - Processing high volumes of accounts (indexers, validators)
//! - Only accessing a subset of fields
//! - Memory pressure is a concern
//!
//! ## Safety Considerations
//!
//! Zero-copy references are tied to the lifetime of the source buffer.
//! Ensure the buffer outlives any references extracted from it.
//!
//! ## Example
//!
//! ```rust,ignore
//! use account_decoder_borsh_util::{ZeroCopyReader, read_discriminator};
//!
//! let data: &[u8] = /* account data */;
//!
//! // Quick discriminator check without full deserialization
//! let discriminator = read_discriminator::<8>(data)?;
//!
//! // Zero-copy field access
//! let mut reader = ZeroCopyReader::new(data);
//! let amount: u64 = reader.read_u64()?;
//! let owner: &[u8; 32] = reader.read_fixed()?;
//! ```

mod discriminator;
mod reader;
mod slice;

pub use discriminator::{
    read_discriminator, AnchorDiscriminator, Discriminator, DiscriminatorTable,
};
pub use reader::{ZeroCopyReader, ZeroCopyError};
pub use slice::{BorshSlice, SliceReader};

/// Re-export borsh for convenience.
pub use borsh;
