//! Discriminator handling utilities.
//!
//! Discriminators identify account and instruction types. Different programs
//! use different discriminator schemes:
//!
//! - **Native programs**: Often a single byte (0, 1, 2, etc.)
//! - **Anchor programs**: First 8 bytes (SHA256 hash of type name)
//! - **Custom**: Some programs use 4-byte or other schemes

use thiserror::Error;

/// Errors that can occur when reading discriminators.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DiscriminatorError {
    /// The buffer is too short to contain a discriminator.
    #[error("insufficient data: expected {expected} bytes, got {actual}")]
    InsufficientData { expected: usize, actual: usize },
}

/// Generic discriminator type with const size.
///
/// This is a zero-copy wrapper around a fixed-size discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Discriminator<const N: usize>([u8; N]);

impl<const N: usize> Discriminator<N> {
    /// Create a new discriminator from bytes.
    pub const fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes.
    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }

    /// Convert to a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Check if this matches another discriminator.
    pub fn matches(&self, other: &[u8]) -> bool {
        self.0.as_slice() == other
    }
}

impl<const N: usize> From<[u8; N]> for Discriminator<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self(bytes)
    }
}

impl<const N: usize> AsRef<[u8]> for Discriminator<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Read a discriminator from a byte slice.
///
/// This is a zero-copy operation that extracts exactly N bytes from the start
/// of the buffer.
///
/// # Example
///
/// ```rust
/// use account_decoder_borsh_util::read_discriminator;
///
/// let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
///
/// // Read 8-byte Anchor discriminator
/// let disc = read_discriminator::<8>(&data).unwrap();
/// assert_eq!(disc.as_bytes(), &[1, 2, 3, 4, 5, 6, 7, 8]);
///
/// // Read single-byte native discriminator
/// let disc = read_discriminator::<1>(&data).unwrap();
/// assert_eq!(disc.as_bytes(), &[1]);
/// ```
pub fn read_discriminator<const N: usize>(
    data: &[u8],
) -> Result<Discriminator<N>, DiscriminatorError> {
    if data.len() < N {
        return Err(DiscriminatorError::InsufficientData {
            expected: N,
            actual: data.len(),
        });
    }

    let mut bytes = [0u8; N];
    bytes.copy_from_slice(&data[..N]);
    Ok(Discriminator::new(bytes))
}

/// Anchor-specific 8-byte discriminator.
///
/// Anchor uses the first 8 bytes of SHA256("account:{TypeName}") or
/// "global:{instruction_name}" as discriminators.
pub type AnchorDiscriminator = Discriminator<8>;

impl AnchorDiscriminator {
    /// Create an Anchor discriminator from a namespace and name.
    ///
    /// This computes the discriminator the same way Anchor does:
    /// SHA256("{namespace}:{name}")[0..8]
    ///
    /// Common namespaces:
    /// - "account" for account types
    /// - "global" for instructions
    pub fn compute(namespace: &str, name: &str) -> Self {
        use sha2::{Digest, Sha256};

        let preimage = format!("{namespace}:{name}");
        let hash = Sha256::digest(preimage.as_bytes());

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hash[..8]);
        Self::new(bytes)
    }

    /// The discriminator Anchor gives an account type.
    pub fn account(type_name: &str) -> Self {
        Self::compute("account", type_name)
    }

    /// The discriminator Anchor gives an instruction.
    ///
    /// Anchor snake-cases the method name before hashing, so the caller must
    /// pass the name in the form the IDL uses.
    pub fn instruction(method_name: &str) -> Self {
        Self::compute("global", method_name)
    }

    /// Check if data starts with this discriminator.
    pub fn matches_data(&self, data: &[u8]) -> bool {
        data.len() >= 8 && &data[..8] == self.as_bytes()
    }
}

/// A table of known discriminators for quick lookup.
///
/// Useful for programs with many account types that need fast routing.
#[derive(Debug)]
pub struct DiscriminatorTable<const N: usize, V> {
    entries: Vec<(Discriminator<N>, V)>,
}

impl<const N: usize, V> DiscriminatorTable<N, V> {
    /// Create a new empty table.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create a table with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    /// Add an entry to the table.
    pub fn insert(&mut self, discriminator: Discriminator<N>, value: V) {
        self.entries.push((discriminator, value));
    }

    /// Look up a value by discriminator bytes.
    pub fn get(&self, bytes: &[u8]) -> Option<&V> {
        if bytes.len() < N {
            return None;
        }
        self.entries
            .iter()
            .find(|(d, _)| d.as_slice() == &bytes[..N])
            .map(|(_, v)| v)
    }

    /// Check if a discriminator is in the table.
    pub fn contains(&self, bytes: &[u8]) -> bool {
        self.get(bytes).is_some()
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<const N: usize, V> Default for DiscriminatorTable<N, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_discriminators_match_the_ones_on_chain() {
        // These are not computed expectations -- they are the first eight bytes
        // of accounts and instructions as they exist on mainnet today. If the
        // preimage or the hash were wrong, generated decoders would silently
        // fail to match any real account.
        //
        // Marinade Finance `State`, account 8szGkuLTAux9XMgZ2vtY39jVSowEcpBfFfD8hXSEqdGC.
        assert_eq!(
            AnchorDiscriminator::account("State").as_bytes(),
            &[216, 146, 107, 94, 104, 75, 182, 177]
        );

        // Anchor hashes the *snake_case* method name for instructions.
        assert_eq!(
            AnchorDiscriminator::instruction("deposit").as_bytes(),
            &[242, 35, 198, 137, 82, 225, 242, 182]
        );
        assert_eq!(
            AnchorDiscriminator::instruction("liquid_unstake").as_bytes(),
            &[30, 30, 119, 240, 191, 227, 12, 16]
        );
    }

    #[test]
    fn the_namespace_is_part_of_the_preimage() {
        // "account:X" and "global:X" must differ, or an account type and an
        // instruction sharing a name would route to each other.
        assert_ne!(
            AnchorDiscriminator::account("Deposit").as_bytes(),
            AnchorDiscriminator::instruction("Deposit").as_bytes()
        );
    }

    #[test]
    fn a_discriminator_table_routes_to_the_right_value() {
        let mut table: DiscriminatorTable<8, &str> = DiscriminatorTable::new();
        table.insert(Discriminator::new(*AnchorDiscriminator::account("State").as_bytes()), "State");
        table.insert(
            Discriminator::new(*AnchorDiscriminator::account("TicketAccountData").as_bytes()),
            "TicketAccountData",
        );

        let state = AnchorDiscriminator::account("State");
        let mut data = state.as_bytes().to_vec();
        data.extend_from_slice(&[0u8; 32]);

        assert_eq!(table.get(&data), Some(&"State"));
        assert!(table.contains(&data));
        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());

        assert_eq!(table.get(&[0u8; 8]), None, "an unknown discriminator matches nothing");
        assert_eq!(table.get(&[0u8; 3]), None, "and short data cannot match");
    }

    #[test]
    fn test_read_discriminator() {
        let data = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A];

        let disc = read_discriminator::<8>(&data).unwrap();
        assert_eq!(disc.as_bytes(), &data);

        let disc = read_discriminator::<4>(&data).unwrap();
        assert_eq!(disc.as_bytes(), &[0xAB, 0xCD, 0xEF, 0x12]);

        let disc = read_discriminator::<1>(&data).unwrap();
        assert_eq!(disc.as_bytes(), &[0xAB]);
    }

    #[test]
    fn test_insufficient_data() {
        let data = [1, 2, 3];

        let result = read_discriminator::<8>(&data);
        assert!(matches!(
            result,
            Err(DiscriminatorError::InsufficientData { expected: 8, actual: 3 })
        ));
    }

    #[test]
    fn test_discriminator_matches() {
        let disc = Discriminator::new([1, 2, 3, 4]);
        assert!(disc.matches(&[1, 2, 3, 4]));
        assert!(!disc.matches(&[1, 2, 3, 5]));
        assert!(!disc.matches(&[1, 2, 3]));
    }

    #[test]
    fn test_discriminator_table() {
        let mut table: DiscriminatorTable<4, &str> = DiscriminatorTable::new();

        table.insert(Discriminator::new([1, 0, 0, 0]), "type_a");
        table.insert(Discriminator::new([2, 0, 0, 0]), "type_b");

        assert_eq!(table.get(&[1, 0, 0, 0, 0xFF, 0xFF]), Some(&"type_a"));
        assert_eq!(table.get(&[2, 0, 0, 0]), Some(&"type_b"));
        assert_eq!(table.get(&[3, 0, 0, 0]), None);
    }
}
