//! Basic usage example demonstrating the account decoder framework.
//!
//! Run with: `cargo run --example basic_usage`

use account_decoder_sdk::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a registry with all built-in decoders
    let registry = default_registry();

    println!("Registered decoders:");
    for info in registry.list_account_decoders() {
        println!(
            "  - {} ({})",
            info.metadata.program_name, info.metadata.program_id
        );
    }
    println!();

    // Example: Decode a token mint account
    println!("Decoding a Token Mint account...");

    // Create sample mint data (82 bytes)
    let mut mint_data = vec![0u8; 82];
    // mint_authority: None (COption tag = 0)
    mint_data[0..4].copy_from_slice(&0u32.to_le_bytes());
    // supply: 1_000_000_000 (1 billion)
    mint_data[36..44].copy_from_slice(&1_000_000_000u64.to_le_bytes());
    // decimals: 9
    mint_data[44] = 9;
    // is_initialized: true
    mint_data[45] = 1;
    // freeze_authority: None
    mint_data[46..50].copy_from_slice(&0u32.to_le_bytes());

    match registry.decode_account(&TOKEN_PROGRAM_ID, &mint_data) {
        Ok(event) => {
            println!("  Program: {}", event.program_name());
            println!("  Type: {}", event.event_type());
            println!("  Kind: {}", event.event_kind());

            // Downcast to get specific fields
            if let Some(mint) = event.downcast_ref::<Mint>() {
                println!("  Supply: {}", mint.supply);
                println!("  Decimals: {}", mint.decimals);
                println!("  Initialized: {}", mint.is_initialized);
                println!(
                    "  Mint Authority: {}",
                    mint.mint_authority
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "None".to_string())
                );
            }
        }
        Err(e) => {
            eprintln!("  Failed to decode: {}", e);
        }
    }
    println!();

    // Example: Decode a token account
    println!("Decoding a Token Account...");

    let mut token_data = vec![0u8; 165];
    // mint: 32 bytes (using default pubkey for demo)
    // owner: 32 bytes (using default pubkey for demo)
    // amount: 500_000_000
    token_data[64..72].copy_from_slice(&500_000_000u64.to_le_bytes());
    // delegate: None (COption tag = 0)
    token_data[72..76].copy_from_slice(&0u32.to_le_bytes());
    // state: Initialized (1)
    token_data[108] = 1;
    // is_native: None
    token_data[109..113].copy_from_slice(&0u32.to_le_bytes());
    // delegated_amount: 0
    // close_authority: None
    token_data[125..129].copy_from_slice(&0u32.to_le_bytes());

    match registry.decode_account(&TOKEN_PROGRAM_ID, &token_data) {
        Ok(event) => {
            println!("  Type: {}", event.event_type());

            if let Some(token) = event.downcast_ref::<TokenAccount>() {
                println!("  Amount: {}", token.amount);
                println!("  State: {:?}", token.state);
            }
        }
        Err(e) => {
            eprintln!("  Failed to decode: {}", e);
        }
    }
    println!();

    // Example: Decode an instruction
    println!("Decoding a Transfer instruction...");

    let mut transfer_data = vec![0u8; 9];
    transfer_data[0] = 3; // Transfer discriminator
    transfer_data[1..9].copy_from_slice(&100_000_000u64.to_le_bytes()); // amount

    match registry.decode_instruction(&TOKEN_PROGRAM_ID, &transfer_data) {
        Ok(event) => {
            println!("  Type: {}", event.event_type());
            println!("  Full name: {}", event.full_name());
        }
        Err(e) => {
            eprintln!("  Failed to decode: {}", e);
        }
    }
    println!();

    // Example: Handle unknown program
    println!("Attempting to decode unknown program...");

    let unknown_program = Pubkey::new_unique();
    let result = registry.try_decode_account(&unknown_program, &[1, 2, 3]);

    match result {
        Some(Ok(event)) => println!("  Decoded: {}", event.event_type()),
        Some(Err(e)) => println!("  Decode error: {}", e),
        None => println!("  No decoder registered for this program"),
    }

    Ok(())
}
