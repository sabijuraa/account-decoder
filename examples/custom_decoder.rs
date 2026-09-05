//! Example of implementing a custom decoder.
//!
//! Run with: `cargo run --example custom_decoder`

use account_decoder_sdk::prelude::*;
use std::any::Any;

// Define the program ID for our custom program
const MY_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
]);

// Discriminators for our account types
const COUNTER_DISCRIMINATOR: [u8; 8] = [0xFF, 0x06, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
const CONFIG_DISCRIMINATOR: [u8; 8] = [0xFE, 0x07, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];

// ----------------- Event Types -----------------

/// A counter account storing a count value.
#[derive(Debug, Clone, PartialEq)]
pub struct CounterAccount {
    pub count: u64,
    pub authority: Pubkey,
    pub bump: u8,
}

impl DecodedEvent for CounterAccount {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "Counter"
    }

    fn program_name(&self) -> &'static str {
        "my-counter-program"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(8 + 8 + 32 + 1) // discriminator + count + authority + bump
    }
}

/// A config account with program settings.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigAccount {
    pub admin: Pubkey,
    pub max_count: u64,
    pub paused: bool,
}

impl DecodedEvent for ConfigAccount {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "Config"
    }

    fn program_name(&self) -> &'static str {
        "my-counter-program"
    }

}

// ----------------- Decoder Implementation -----------------

/// Decoder for our counter program.
#[derive(Debug, Clone)]
pub struct CounterProgramDecoder;

impl CounterProgramDecoder {
    pub fn new() -> Self {
        Self
    }

    fn decode_counter(data: &[u8]) -> DecodeResult<CounterAccount> {
        if data.len() < 49 {
            // 8 + 8 + 32 + 1
            return Err(DecodeError::insufficient_data(49, data.len()));
        }

        let mut reader = ZeroCopyReader::new(&data[8..]); // Skip discriminator

        let count = reader
            .read_u64()
            .map_err(|e| DecodeError::deserialization(e))?;

        let authority_bytes = reader
            .read_fixed::<32>()
            .map_err(|e| DecodeError::deserialization(e))?;
        let authority = Pubkey::new_from_array(*authority_bytes);

        let bump = reader
            .read_u8()
            .map_err(|e| DecodeError::deserialization(e))?;

        Ok(CounterAccount {
            count,
            authority,
            bump,
        })
    }

    fn decode_config(data: &[u8]) -> DecodeResult<ConfigAccount> {
        if data.len() < 49 {
            // 8 + 32 + 8 + 1
            return Err(DecodeError::insufficient_data(49, data.len()));
        }

        let mut reader = ZeroCopyReader::new(&data[8..]); // Skip discriminator

        let admin_bytes = reader
            .read_fixed::<32>()
            .map_err(|e| DecodeError::deserialization(e))?;
        let admin = Pubkey::new_from_array(*admin_bytes);

        let max_count = reader
            .read_u64()
            .map_err(|e| DecodeError::deserialization(e))?;

        let paused = reader
            .read_bool()
            .map_err(|e| DecodeError::deserialization(e))?;

        Ok(ConfigAccount {
            admin,
            max_count,
            paused,
        })
    }
}

impl Default for CounterProgramDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoderIdentity for CounterProgramDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("my-counter-program", MY_PROGRAM_ID)
            .with_description("Custom counter program decoder")
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities::default()
            .with_zero_copy()
            .with_account_types(vec!["Counter", "Config"])
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AccountDecoder for CounterProgramDecoder {
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        if data.len() < 8 {
            return Err(DecodeError::insufficient_data(8, data.len()));
        }

        let discriminator: [u8; 8] = data[..8].try_into().unwrap();

        match discriminator {
            COUNTER_DISCRIMINATOR => Ok(Box::new(Self::decode_counter(data)?)),
            CONFIG_DISCRIMINATOR => Ok(Box::new(Self::decode_config(data)?)),
            _ => Err(DecodeError::unknown_discriminator(&discriminator)),
        }
    }

    fn can_decode(&self, data: &[u8]) -> bool {
        if data.len() < 8 {
            return false;
        }
        let disc: [u8; 8] = data[..8].try_into().unwrap();
        disc == COUNTER_DISCRIMINATOR || disc == CONFIG_DISCRIMINATOR
    }
}

// ----------------- Main -----------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a registry with built-in decoders
    let mut registry = default_registry();

    // Register our custom decoder
    registry.register_account(Box::new(CounterProgramDecoder::new()));

    println!("Registered decoders: {}", registry.account_decoder_count());
    println!();

    // Create sample counter account data
    println!("Decoding a Counter account...");

    let mut counter_data = vec![0u8; 49];
    counter_data[..8].copy_from_slice(&COUNTER_DISCRIMINATOR);
    counter_data[8..16].copy_from_slice(&42u64.to_le_bytes()); // count = 42
    counter_data[16..48].fill(0xAB); // authority (dummy pubkey)
    counter_data[48] = 255; // bump

    match registry.decode_account(&MY_PROGRAM_ID, &counter_data) {
        Ok(event) => {
            println!("  Program: {}", event.program_name());
            println!("  Type: {}", event.event_type());

            if let Some(counter) = event.downcast_ref::<CounterAccount>() {
                println!("  Count: {}", counter.count);
                println!("  Bump: {}", counter.bump);
            }
        }
        Err(e) => {
            eprintln!("  Failed: {}", e);
        }
    }
    println!();

    // Create sample config account data
    println!("Decoding a Config account...");

    let mut config_data = vec![0u8; 49];
    config_data[..8].copy_from_slice(&CONFIG_DISCRIMINATOR);
    config_data[8..40].fill(0xCD); // admin (dummy pubkey)
    config_data[40..48].copy_from_slice(&1_000_000u64.to_le_bytes()); // max_count
    config_data[48] = 0; // paused = false

    match registry.decode_account(&MY_PROGRAM_ID, &config_data) {
        Ok(event) => {
            println!("  Program: {}", event.program_name());
            println!("  Type: {}", event.event_type());

            if let Some(config) = event.downcast_ref::<ConfigAccount>() {
                println!("  Max Count: {}", config.max_count);
                println!("  Paused: {}", config.paused);
            }
        }
        Err(e) => {
            eprintln!("  Failed: {}", e);
        }
    }

    Ok(())
}
