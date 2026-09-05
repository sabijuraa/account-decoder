# Account Decoder System Design

## Overview

The Account Decoder framework provides a pluggable, extensible system for decoding raw Solana account and instruction data into typed, structured events. It is designed for high-throughput indexing pipelines while maintaining type safety and extensibility.

## Architecture Diagram

```
                                     ┌─────────────────────────────────────────┐
                                     │           User Application               │
                                     │  (Indexer, Explorer, Analytics, etc.)   │
                                     └─────────────────────┬───────────────────┘
                                                           │
                                                           ▼
┌──────────────────────────────────────────────────────────────────────────────────────┐
│                                    SDK (account-decoder-sdk)                         │
│  ┌─────────────────────────────────────────────────────────────────────────────────┐ │
│  │                              Public API Surface                                  │ │
│  │  • default_registry() - Pre-configured registry with built-in decoders         │ │
│  │  • registry_builder() - Fluent API for custom configurations                   │ │
│  │  • Prelude module - Convenient imports                                         │ │
│  └─────────────────────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────────────┘
                                                           │
                    ┌──────────────────────────────────────┼──────────────────────────────────────┐
                    │                                      │                                      │
                    ▼                                      ▼                                      ▼
    ┌───────────────────────────────┐  ┌───────────────────────────────┐  ┌───────────────────────────────┐
    │     Core (account-decoder-    │  │  Decoders (account-decoder-   │  │  Anchor Gen (account-decoder- │
    │            core)              │  │         decoders)             │  │        anchor-gen)            │
    │                               │  │                               │  │                               │
    │  ┌─────────────────────────┐  │  │  ┌─────────────────────────┐  │  │  ┌─────────────────────────┐  │
    │  │    DecoderRegistry     │  │  │  │     TokenDecoder       │  │  │  │      IdlParser         │  │
    │  │  • program_id → decoder │  │  │  │  • Mint, TokenAccount  │  │  │  │  • JSON → IdlProgram   │  │
    │  │  • O(1) lookup         │  │  │  │  • Multisig            │  │  │  │                         │  │
    │  └─────────────────────────┘  │  │  └─────────────────────────┘  │  │  └─────────────────────────┘  │
    │                               │  │                               │  │                               │
    │  ┌─────────────────────────┐  │  │  ┌─────────────────────────┐  │  │  ┌─────────────────────────┐  │
    │  │  AccountDecoder Trait  │  │  │  │   Token2022Decoder     │  │  │  │     CodeGenerator      │  │
    │  │  • metadata()          │  │  │  │  • Extensions support  │  │  │  │  • Types generation    │  │
    │  │  • decode_account()    │  │  │  │                         │  │  │  │  • Decoder impls       │  │
    │  │  • capabilities()      │  │  │  └─────────────────────────┘  │  │  └─────────────────────────┘  │
    │  └─────────────────────────┘  │  │                               │  │                               │
    │                               │  │  ┌─────────────────────────┐  │  │  ┌─────────────────────────┐  │
    │  ┌─────────────────────────┐  │  │  │     SystemDecoder      │  │  │  │      TypeMapper        │  │
    │  │ InstructionDecoder     │  │  │  │  • SystemAccount       │  │  │  │  • IDL → Rust types    │  │
    │  │  • metadata()          │  │  │  │  • NonceAccount        │  │  │  │                         │  │
    │  │  • decode_instruction()│  │  │  └─────────────────────────┘  │  │  └─────────────────────────┘  │
    │  └─────────────────────────┘  │  │                               │  │                               │
    │                               │  └───────────────────────────────┘  └───────────────────────────────┘
    │  ┌─────────────────────────┐  │
    │  │     DecodedEvent       │  │
    │  │  • Type-safe output    │  │
    │  │  • Downcastable        │  │
    │  └─────────────────────────┘  │
    │                               │
    │  ┌─────────────────────────┐  │
    │  │      DecodeError       │  │
    │  │  • Structured errors   │  │
    │  │  • Recovery hints      │  │
    │  └─────────────────────────┘  │
    └───────────────────────────────┘
                    │
                    ▼
    ┌───────────────────────────────┐
    │  Borsh Util (account-decoder- │
    │         borsh-util)           │
    │                               │
    │  ┌─────────────────────────┐  │
    │  │    ZeroCopyReader      │  │
    │  │  • No-alloc parsing    │  │
    │  │  • Cursor-based API    │  │
    │  └─────────────────────────┘  │
    │                               │
    │  ┌─────────────────────────┐  │
    │  │     Discriminator      │  │
    │  │  • Anchor (8 bytes)    │  │
    │  │  • Native (1-4 bytes)  │  │
    │  └─────────────────────────┘  │
    └───────────────────────────────┘
```

## Core Components

### 1. Decoder Traits

The foundation of the framework is a pair of traits:

```rust
pub trait AccountDecoder: Send + Sync + Debug {
    fn metadata(&self) -> DecoderMetadata;
    fn capabilities(&self) -> DecoderCapabilities;
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>>;
    fn can_decode(&self, data: &[u8]) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub trait InstructionDecoder: Send + Sync + Debug {
    fn metadata(&self) -> DecoderMetadata;
    fn decode_instruction(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>>;
    fn as_any(&self) -> &dyn Any;
}
```

**Design Decisions:**
- `Send + Sync` for thread-safety in concurrent indexing pipelines
- `Debug` for observability
- `as_any()` enables downcasting to concrete types
- `capabilities()` advertises features for optimization decisions

### 2. Registry Pattern

The `DecoderRegistry` maps program IDs to their decoders:

```rust
pub struct DecoderRegistry {
    account_decoders: HashMap<Pubkey, Arc<dyn AccountDecoder>>,
    instruction_decoders: HashMap<Pubkey, Arc<dyn InstructionDecoder>>,
}
```

**Performance Characteristics:**
- O(1) lookup by program ID
- Arc-based sharing for concurrent access
- No runtime registration overhead after initialization

### 3. Zero-Copy Deserialization

The `ZeroCopyReader` enables efficient parsing without allocation:

```rust
let mut reader = ZeroCopyReader::new(data);
let mint: &[u8; 32] = reader.read_fixed()?;  // Returns reference to original buffer
let amount: u64 = reader.read_u64()?;         // Copies only primitives
```

**When to Use:**
- High-throughput indexing (100K+ accounts/second)
- Memory-constrained environments
- When only accessing a subset of fields

**Safety Considerations:**
- References are tied to buffer lifetime
- Cannot outlive the source data
- Use owned types for storage

### 4. IDL Code Generation

The Anchor codegen pipeline transforms IDL JSON to Rust:

```
IDL JSON → IdlParser → IdlProgram → CodeGenerator → TokenStream → Rust File
```

**Type Mapping:**
| IDL Type     | Rust Type      |
|-------------|----------------|
| `u64`       | `u64`          |
| `publicKey` | `Pubkey`       |
| `string`    | `String`       |
| `bytes`     | `Vec<u8>`      |
| `Option<T>` | `Option<T>`    |
| `Vec<T>`    | `Vec<T>`       |
| `[T; N]`    | `[T; N]`       |

## Error Handling Strategy

Errors are categorized by recoverability:

```rust
pub enum DecodeError {
    // Data issues (potentially recoverable with different decoder)
    InsufficientData { expected: usize, actual: usize },
    UnknownDiscriminator(Vec<u8>),
    
    // Program routing issues
    UnknownProgram(Pubkey),
    
    // Data corruption (unrecoverable)
    DeserializationError(String),
    
    // Version issues (may need decoder update)
    VersionMismatch { supported: String, found: String },
}
```

**Error Methods:**
- `is_wrong_program()` - Try another decoder
- `is_data_corruption()` - Log and skip

## Extension Points

### Adding a Custom Decoder

1. Implement the trait:
```rust
#[derive(Debug, Clone)]
struct MyDecoder;

impl AccountDecoder for MyDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("my-program", MY_PROGRAM_ID)
    }
    
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        // Parse logic here
    }
    
    fn as_any(&self) -> &dyn Any { self }
}
```

2. Register with the registry:
```rust
registry.register_account(Box::new(MyDecoder));
```

### Using the Anchor Codegen

1. Parse an IDL:
```rust
let idl = IdlParser::parse(&json)?;
```

2. Generate code:
```rust
let generator = CodeGenerator::new(GeneratorConfig::default());
let code = generator.generate(&idl)?;
```

3. Write to file or use at build time.

## Performance Characteristics

| Operation              | Time Complexity | Space Complexity |
|-----------------------|-----------------|------------------|
| Registry lookup       | O(1)            | O(1)             |
| Token account decode  | O(1)            | O(1) zero-copy   |
| Mint decode           | O(1)            | O(1) zero-copy   |
| IDL parse             | O(n)            | O(n)             |
| Code generation       | O(n)            | O(n)             |

**Benchmarks (approximate):**
- Token account decode: ~50ns
- Registry lookup: ~20ns
- Full decode (lookup + parse): ~100ns

## Thread Safety

All components are designed for concurrent use:

- `DecoderRegistry`: Immutable after construction, safe to share via `Arc`
- Decoders: Must be `Send + Sync`, no mutable state
- `DecodedEvent`: Owned values, safe to move between threads

## Versioning Strategy

The SDK follows semantic versioning:

- **Patch**: Bug fixes, documentation
- **Minor**: New decoders, new fields (backward compatible)
- **Major**: Trait changes, breaking API changes

Decoder version is tracked in metadata for debugging and compatibility checks.
