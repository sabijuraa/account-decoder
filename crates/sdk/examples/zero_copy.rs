//! Example demonstrating zero-copy deserialization for high-throughput scenarios.
//!
//! Run with: `cargo run --example zero_copy`

use account_decoder_sdk::{read_discriminator, Discriminator, ZeroCopyError, ZeroCopyReader};
use solana_sdk::pubkey::Pubkey;

/// Token account layout offsets
mod token_offsets {
    pub const MINT: usize = 0;
    pub const OWNER: usize = 32;
    pub const AMOUNT: usize = 64;
    pub const DELEGATE_TAG: usize = 72;
    pub const DELEGATE: usize = 76;
    pub const STATE: usize = 108;
}

/// Zero-copy token account view.
///
/// This struct holds references into the original buffer rather than owned data.
#[derive(Debug)]
struct TokenAccountView<'a> {
    mint: &'a [u8; 32],
    owner: &'a [u8; 32],
    amount: u64,
    has_delegate: bool,
    delegate: Option<&'a [u8; 32]>,
    state: u8,
}

impl<'a> TokenAccountView<'a> {
    /// Parse a token account using zero-copy reader.
    fn parse(data: &'a [u8]) -> Result<Self, ZeroCopyError> {
        if data.len() < 165 {
            return Err(ZeroCopyError::UnexpectedEof {
                needed: 165,
                remaining: data.len(),
            });
        }

        let mut reader = ZeroCopyReader::new(data);

        // Read mint (zero-copy reference)
        let mint = reader.read_fixed::<32>()?;

        // Read owner (zero-copy reference)
        let owner = reader.read_fixed::<32>()?;

        // Read amount (copied because it's a primitive)
        let amount = reader.read_u64()?;

        // Read delegate (COption<Pubkey>)
        let delegate_tag = reader.read_u32()?;
        let delegate_bytes = reader.read_fixed::<32>()?;
        let has_delegate = delegate_tag != 0;
        let delegate = if has_delegate {
            Some(delegate_bytes)
        } else {
            None
        };

        // Read state
        let state = reader.read_u8()?;

        Ok(Self {
            mint,
            owner,
            amount,
            has_delegate,
            delegate,
            state,
        })
    }

    /// Get the mint as a Pubkey.
    fn mint_pubkey(&self) -> Pubkey {
        Pubkey::new_from_array(*self.mint)
    }

    /// Get the owner as a Pubkey.
    fn owner_pubkey(&self) -> Pubkey {
        Pubkey::new_from_array(*self.owner)
    }
}

/// Demonstrates different ways to use the zero-copy reader.
fn demo_zero_copy_reader() {
    println!("=== ZeroCopyReader Demo ===\n");

    // Create sample data
    let data: [u8; 28] = [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // u64: 1
        0xFF, 0x00, 0x00, 0x00, // u32: 255
        0x05, 0x00, 0x00, 0x00, // string length: 5
        b'H', b'e', b'l', b'l', b'o', // "Hello"
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
    ];

    let mut reader = ZeroCopyReader::new(&data);

    // Read primitives
    let value1 = reader.read_u64().unwrap();
    println!("Read u64: {value1}");

    let value2 = reader.read_u32().unwrap();
    println!("Read u32: {value2}");

    // Read string (borsh format: u32 length + bytes)
    let string = reader.read_str().unwrap();
    println!("Read string: {string}");

    println!("Position after reads: {}", reader.position());
    println!("Remaining bytes: {}", reader.remaining());
    println!();
}

/// Demonstrates discriminator handling.
fn demo_discriminators() {
    println!("=== Discriminator Demo ===\n");

    // Anchor-style 8-byte discriminator
    let anchor_data = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0x00, 0x01];

    let disc = read_discriminator::<8>(&anchor_data).unwrap();
    println!("Anchor discriminator: {:?}", disc.as_bytes());

    // Native-style 1-byte discriminator
    let native_data = [0x03, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04];

    let disc = read_discriminator::<1>(&native_data).unwrap();
    println!("Native discriminator: {:?}", disc.as_bytes());

    // Check if discriminator matches
    let expected = Discriminator::new([0x03]);
    println!("Matches expected: {}", disc.matches(expected.as_bytes()));
    println!();
}

/// Demonstrates parsing token accounts.
fn demo_token_parsing() {
    println!("=== Token Account Parsing Demo ===\n");

    // Create a sample token account (165 bytes)
    let mut data = vec![0u8; 165];

    // Set mint (first 32 bytes)
    data[0] = 0x11;
    data[31] = 0x11;

    // Set owner (bytes 32-63)
    data[32] = 0x22;
    data[63] = 0x22;

    // Set amount (bytes 64-71): 1,000,000
    data[64..72].copy_from_slice(&1_000_000u64.to_le_bytes());

    // Set delegate: None (tag = 0)
    data[72..76].copy_from_slice(&0u32.to_le_bytes());

    // Set state: Initialized (1)
    data[108] = 1;

    // Parse using zero-copy view
    let view = TokenAccountView::parse(&data).unwrap();

    println!("Parsed TokenAccount (zero-copy):");
    println!("  Amount: {}", view.amount);
    println!("  Has delegate: {}", view.has_delegate);
    println!("  State: {}", view.state);

    // The fields are borrowed slices into the original buffer. Converting one
    // to a Pubkey is the only place a copy happens, and only for the field the
    // caller actually asked about -- which is the whole argument for the view.
    println!("  Mint:  {}", view.mint_pubkey());
    println!("  Owner: {}", view.owner_pubkey());
    match view.delegate {
        Some(delegate) => println!("  Delegate: {}", Pubkey::new_from_array(*delegate)),
        None => println!("  Delegate: none"),
    }

    // The same fields again, reached by offset without a reader at all. This is
    // what you do when one field out of a hundred is wanted: no parse, no
    // allocation, just an indexed read.
    let amount_bytes: [u8; 8] = data[token_offsets::AMOUNT..token_offsets::AMOUNT + 8]
        .try_into()
        .expect("eight bytes");
    println!(
        "  Amount read directly at offset {}: {}",
        token_offsets::AMOUNT,
        u64::from_le_bytes(amount_bytes)
    );
    println!(
        "  State byte at offset {}: {}",
        token_offsets::STATE,
        data[token_offsets::STATE]
    );
    println!(
        "  Mint occupies bytes {}..{}, owner {}..{}, delegate tag at {}",
        token_offsets::MINT,
        token_offsets::MINT + 32,
        token_offsets::OWNER,
        token_offsets::OWNER + 32,
        token_offsets::DELEGATE_TAG
    );
    debug_assert_eq!(token_offsets::DELEGATE, token_offsets::DELEGATE_TAG + 4);
    println!("  Mint (first byte): 0x{:02x}", view.mint[0]);
    println!("  Owner (first byte): 0x{:02x}", view.owner[0]);
    println!();

    // Convert to Pubkey when needed (this allocates, but only on demand)
    let mint_pubkey = view.mint_pubkey();
    println!("  Mint Pubkey: {}...", &mint_pubkey.to_string()[..8]);
    println!();
}

/// Demonstrates peeking without advancing.
fn demo_peek_and_skip() {
    println!("=== Peek and Skip Demo ===\n");

    let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut reader = ZeroCopyReader::new(&data);

    // Peek at data without consuming
    println!("Peek first 4 bytes: {:?}", reader.peek(4));
    println!("Position after peek: {}", reader.position());

    // Skip some bytes
    reader.skip(4).unwrap();
    println!("Position after skip(4): {}", reader.position());

    // Peek again
    println!("Peek next 4 bytes: {:?}", reader.peek(4));

    // Seek to specific position
    reader.seek(2).unwrap();
    println!("Position after seek(2): {}", reader.position());
    println!("Remaining data: {:?}", reader.remaining_data());
    println!();
}

/// Benchmark comparison.
fn demo_performance_note() {
    println!("=== Performance Notes ===\n");

    println!("Zero-copy parsing advantages:");
    println!("  1. No heap allocations for fixed-size fields");
    println!("  2. References into original buffer");
    println!("  3. Only copy when actually needed (e.g., Pubkey conversion)");
    println!("  4. Great for filtering/routing decisions");
    println!();

    println!("When to use owned types instead:");
    println!("  1. Data needs to outlive the buffer");
    println!("  2. Complex nested structures");
    println!("  3. When you need all fields anyway");
    println!();
}

fn main() {
    demo_zero_copy_reader();
    demo_discriminators();
    demo_token_parsing();
    demo_peek_and_skip();
    demo_performance_note();
}
