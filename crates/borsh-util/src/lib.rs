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
//! ```rust
//! use account_decoder_borsh_util::{read_discriminator, AnchorDiscriminator, ZeroCopyReader};
//!
//! // An Anchor account: eight discriminator bytes, then the fields.
//! let expected = AnchorDiscriminator::account("State");
//! let mut data = expected.as_bytes().to_vec();
//! data.extend_from_slice(&7_500u64.to_le_bytes());
//! data.extend_from_slice(&[3u8; 32]);
//!
//! // Check the type before doing any parsing work.
//! let discriminator = read_discriminator::<8>(&data)?;
//! assert_eq!(discriminator.as_bytes(), expected.as_bytes());
//!
//! // Then read fields without copying the buffer.
//! let mut reader = ZeroCopyReader::new(&data[8..]);
//! assert_eq!(reader.read_u64()?, 7_500);
//! let owner: &[u8; 32] = reader.read_fixed()?;
//! assert_eq!(owner, &[3u8; 32]);
//!
//! // Reading past the end is an error, never a panic.
//! assert!(reader.read_u64().is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
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
