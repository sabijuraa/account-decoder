# Account Decoder

A pluggable framework for decoding raw Solana account and instruction data into typed, structured events.

[![Crates.io](https://img.shields.io/crates/v/account-decoder-sdk.svg)](https://crates.io/crates/account-decoder-sdk)
[![Documentation](https://docs.rs/account-decoder-sdk/badge.svg)](https://docs.rs/account-decoder-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Features

- **Trait-Based Decoder Registry**: Extensible architecture allowing new program decoders without modifying core
- **Zero-Copy Borsh Utilities**: High-performance parsing for indexing pipelines
- **Anchor IDL Codegen**: Generate decoders automatically from Anchor IDL files
- **Built-in Decoders**: Ready-to-use decoders for Token, Token-2022, and System programs
- **Type Safety**: Strongly typed decoded events with safe downcasting

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
account-decoder-sdk = "0.1"
```

Basic usage:

```rust
use account_decoder_sdk::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a registry with all built-in decoders
    let registry = default_registry();
    
    // Decode an account
    let program_id = TOKEN_PROGRAM_ID;
    let account_data: Vec<u8> = /* fetch from RPC or geyser */;
    
    match registry.decode_account(&program_id, &account_data) {
        Ok(event) => {
            println!("Program: {}", event.program_name());
            println!("Type: {}", event.event_type());
            
            // Downcast to specific type if needed
            if let Some(mint) = event.downcast_ref::<Mint>() {
                println!("Supply: {}", mint.supply);
                println!("Decimals: {}", mint.decimals);
            }
        }
        Err(DecodeError::UnknownProgram(id)) => {
            println!("No decoder for program: {}", id);
        }
        Err(e) => {
            println!("Decode error: {}", e);
        }
    }
    
    Ok(())
}
```

## Architecture

```
                     ┌─────────────────────┐
                     │   Your Application  │
                     └──────────┬──────────┘
                                │
                     ┌──────────▼──────────┐
                     │        SDK          │
                     │  (re-exports all)   │
                     └──────────┬──────────┘
           ┌────────────────────┼────────────────────┐
           │                    │                    │
    ┌──────▼──────┐     ┌───────▼──────┐     ┌──────▼──────┐
    │    Core     │     │   Decoders   │     │ Anchor Gen  │
    │  (traits)   │     │  (built-in)  │     │  (codegen)  │
    └─────────────┘     └──────────────┘     └─────────────┘
           │
    ┌──────▼──────┐
    │ Borsh Util  │
    │ (zero-copy) │
    └─────────────┘
```

## Built-in Decoders

| Program | Account Types | Instruction Types |
|---------|---------------|-------------------|
| SPL Token | `Mint`, `TokenAccount`, `Multisig` | All standard instructions |
| Token-2022 | `Token2022Mint`, `Token2022Account` with extensions | - |
| System | `SystemAccount`, `NonceAccount` | All system instructions |

## Implementing a Custom Decoder

```rust
use account_decoder_sdk::prelude::*;
use std::any::Any;

// Define your event types
#[derive(Debug, Clone)]
struct MyProgramState {
    pub value: u64,
    pub owner: Pubkey,
}

impl DecodedEvent for MyProgramState {
    fn event_kind(&self) -> EventKind { EventKind::Account }
    fn event_type(&self) -> &'static str { "MyProgramState" }
    fn program_name(&self) -> &'static str { "my-program" }
    fn as_any(&self) -> &dyn Any { self }
}

// Implement the decoder
#[derive(Debug, Clone)]
struct MyProgramDecoder {
    program_id: Pubkey,
}

impl AccountDecoder for MyProgramDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("my-program", self.program_id)
    }
    
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        // Check discriminator
        if data.len() < 8 {
            return Err(DecodeError::insufficient_data(8, data.len()));
        }
        
        // Parse using zero-copy reader
        let mut reader = ZeroCopyReader::new(&data[8..]);
        let value = reader.read_u64().map_err(|e| DecodeError::deserialization(e))?;
        let owner = Pubkey::new_from_array(
            *reader.read_fixed::<32>().map_err(|e| DecodeError::deserialization(e))?
        );
        
        Ok(Box::new(MyProgramState { value, owner }))
    }
    
    fn as_any(&self) -> &dyn Any { self }
}

// Register it
fn main() {
    let mut registry = default_registry();
    registry.register_account(Box::new(MyProgramDecoder {
        program_id: Pubkey::new_unique(),
    }));
}
```

## Anchor IDL Code Generation

Generate decoders from Anchor IDL files:

### CLI Usage

```bash
# Install the CLI
cargo install --path anchor-gen-cli

# Generate from IDL
anchor-gen target/idl/my_program.json -o src/generated/

# Generate with serde support
anchor-gen my_program.json -o src/ --serde
```

### Programmatic Usage

```rust
use account_decoder_sdk::{IdlParser, CodeGenerator, GeneratorConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let idl_json = std::fs::read_to_string("my_program.json")?;
    let idl = IdlParser::parse(&idl_json)?;
    
    let config = GeneratorConfig::default().with_serde();
    let generator = CodeGenerator::new(config);
    let code = generator.generate(&idl)?;
    
    std::fs::write("src/generated/my_program.rs", code.to_string())?;
    Ok(())
}
```

## Zero-Copy Parsing

For high-throughput scenarios:

```rust
use account_decoder_sdk::ZeroCopyReader;

fn decode_fast(data: &[u8]) -> Result<(u64, &[u8; 32]), Box<dyn std::error::Error>> {
    let mut reader = ZeroCopyReader::new(data);
    
    // Skip to the field you need
    reader.skip(8)?;  // Skip discriminator
    
    // Read values
    let amount = reader.read_u64()?;
    let owner = reader.read_fixed::<32>()?;  // Zero-copy reference
    
    Ok((amount, owner))
}
```

## Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Registry lookup | ~20ns | O(1) HashMap |
| Token account decode | ~50ns | Zero-copy |
| Full decode path | ~100ns | Lookup + parse |
| IDL parse | O(n) | One-time cost |

Benchmark: Decoding 1M token accounts

| Method | Time | Allocations |
|--------|------|-------------|
| Standard Borsh | 450ms | 1M |
| Zero-Copy | 120ms | 0 |

## Feature Flags

```toml
[dependencies]
# Default: includes built-in decoders
account-decoder-sdk = "0.1"

# Minimal: just core traits
account-decoder-sdk = { version = "0.1", default-features = false }

# With codegen
account-decoder-sdk = { version = "0.1", features = ["anchor-codegen"] }

# Everything
account-decoder-sdk = { version = "0.1", features = ["full"] }
```

## Crate Structure

| Crate | Description |
|-------|-------------|
| `account-decoder-sdk` | Public SDK, re-exports everything |
| `account-decoder-core` | Core traits and registry |
| `account-decoder-borsh-util` | Zero-copy utilities |
| `account-decoder-decoders` | Built-in program decoders |
| `account-decoder-anchor-gen` | IDL code generator |
| `anchor-gen-cli` | CLI for code generation |

## Documentation

- [System Design](SYSTEM_DESIGN.md) - Architecture overview
- [ADR 001](docs/adr/001-decoder-trait-design.md) - Decoder trait design
- [ADR 002](docs/adr/002-registry-pattern.md) - Registry pattern
- [ADR 003](docs/adr/003-zero-copy-borsh.md) - Zero-copy strategy
- [ADR 004](docs/adr/004-anchor-codegen.md) - Anchor codegen
- [ADR 005](docs/adr/005-sdk-api-design.md) - SDK API design

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Run `cargo test --workspace`
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built for the Solana ecosystem. Inspired by:
- [Anchor](https://github.com/coral-xyz/anchor)
- [solana-account-decoder](https://docs.rs/solana-account-decoder)
- [borsh](https://github.com/near/borsh-rs)
