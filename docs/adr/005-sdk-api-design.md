# ADR 005: SDK API Design

## Status

Accepted

## Context

The SDK crate is the primary interface for users. It must:

1. Provide a clean, intuitive API
2. Re-export necessary types from internal crates
3. Support different use cases (simple → advanced)
4. Maintain backward compatibility
5. Be well-documented

## Decision

### Layered API Design

```
┌─────────────────────────────────────────────────────────────┐
│                    Convenience Layer                        │
│   default_registry(), prelude::*, quick-start functions    │
├─────────────────────────────────────────────────────────────┤
│                    Standard Layer                           │
│   DecoderRegistry, RegistryBuilder, specific decoders      │
├─────────────────────────────────────────────────────────────┤
│                    Advanced Layer                           │
│   Traits, zero-copy utilities, codegen                     │
└─────────────────────────────────────────────────────────────┘
```

### Prelude Module

The prelude provides the most common imports:

```rust
pub mod prelude {
    // Traits
    pub use crate::{
        AccountDecoder, InstructionDecoder, DecodedEvent, TypedEvent,
    };
    
    // Types
    pub use crate::{
        DecoderRegistry, DecodeError, DecodeResult,
        DecoderMetadata, DecoderCapabilities,
    };
    
    // Built-in decoders (when feature enabled)
    #[cfg(feature = "builtin-decoders")]
    pub use crate::{
        TokenDecoder, Token2022Decoder, SystemDecoder,
        TokenAccount, Mint, program_ids::*,
    };
    
    // Convenience function
    #[cfg(feature = "builtin-decoders")]
    pub use crate::default_registry;
}
```

Usage:
```rust
use account_decoder_sdk::prelude::*;
```

### Feature Flags

```toml
[features]
default = ["builtin-decoders"]
builtin-decoders = ["account-decoder-decoders"]
anchor-codegen = ["account-decoder-anchor-gen"]
full = ["builtin-decoders", "anchor-codegen"]
```

This allows:
- Minimal builds without built-in decoders
- Codegen only when needed
- Full-featured builds

### Convenience Functions

```rust
// Quickest path to working decoder
pub fn default_registry() -> DecoderRegistry {
    let mut registry = DecoderRegistry::new();
    registry.register_account(Box::new(TokenDecoder::new()));
    registry.register_account(Box::new(Token2022Decoder::new()));
    registry.register_account(Box::new(SystemDecoder::new()));
    registry
}

// For custom configurations
pub fn registry_builder() -> RegistryBuilder {
    RegistryBuilder::new()
}

// Empty starting point
pub fn empty_registry() -> DecoderRegistry {
    DecoderRegistry::new()
}
```

### Documentation Strategy

1. **Crate-level docs**: Quick start, feature overview
2. **Module-level docs**: Detailed usage, examples
3. **Type-level docs**: API reference, parameters
4. **Examples directory**: Complete working examples

```rust
//! # Account Decoder SDK
//!
//! ## Quick Start
//!
//! ```rust
//! use account_decoder_sdk::prelude::*;
//!
//! let registry = default_registry();
//! let event = registry.decode_account(&TOKEN_PROGRAM_ID, &data)?;
//! ```
//!
//! ## Features
//!
//! - `builtin-decoders`: Token, Token-2022, System decoders
//! - `anchor-codegen`: Generate decoders from Anchor IDL
```

### Versioning Strategy

We follow semantic versioning:

| Change Type                      | Version Bump |
|---------------------------------|--------------|
| Bug fix                         | Patch (0.1.x)|
| New decoder, new field          | Minor (0.x.0)|
| Trait change, removed API       | Major (x.0.0)|

Deprecation policy:
1. Mark as `#[deprecated]` in minor release
2. Document migration path
3. Remove in next major release

### Error Handling Philosophy

Users should never need `unwrap()` on decode results:

```rust
// Good: Pattern matching on specific errors
match registry.decode_account(&program_id, &data) {
    Ok(event) => handle_event(event),
    Err(DecodeError::UnknownProgram(_)) => {
        // Expected for unknown programs
    }
    Err(e) => {
        tracing::warn!("Decode failed: {}", e);
    }
}

// Good: Using try_decode for optional handling
if let Some(Ok(event)) = registry.try_decode_account(&program_id, &data) {
    handle_event(event);
}
```

## Consequences

### Positive

- **Discoverable**: Prelude makes common imports easy
- **Flexible**: Feature flags control dependencies
- **Stable**: Clear versioning policy
- **Documented**: Multi-level documentation

### Negative

- **Feature Complexity**: Multiple feature combinations
- **Re-export Maintenance**: Must keep re-exports in sync
- **Version Coordination**: All crates versioned together

### Public API Surface

The following are considered stable API:

```rust
// Functions
pub fn default_registry() -> DecoderRegistry;
pub fn empty_registry() -> DecoderRegistry;
pub fn registry_builder() -> RegistryBuilder;

// Traits
pub trait AccountDecoder { ... }
pub trait InstructionDecoder { ... }
pub trait DecodedEvent { ... }

// Types
pub struct DecoderRegistry { ... }
pub struct RegistryBuilder { ... }
pub enum DecodeError { ... }

// Type aliases
pub type DecodeResult<T> = Result<T, DecodeError>;
```

Changes to these require a major version bump.

### Internal API

The following are NOT stable:
- Private modules
- Internal helper functions
- Implementation details of decoders

## Alternatives Considered

### Single Crate Architecture

Everything in one crate:
```
account-decoder/
├── src/
│   ├── core/
│   ├── borsh/
│   ├── decoders/
│   └── codegen/
```

Rejected because:
- Slower compilation
- Can't feature-flag internal code
- Harder to maintain

### Workspace Without SDK

Users import individual crates:
```rust
use account_decoder_core::*;
use account_decoder_decoders::*;
```

Rejected because:
- More imports needed
- Version coordination harder
- Less discoverable

### Facade Pattern

SDK has no implementation, only re-exports:
```rust
// sdk/src/lib.rs
pub use account_decoder_core::*;
pub use account_decoder_decoders::*;
```

Accepted (this is what we do), but we also add convenience functions and prelude.

## Example Use Cases

### 1. Basic Indexer

```rust
use account_decoder_sdk::prelude::*;

fn main() {
    let registry = default_registry();
    
    for account in get_accounts() {
        if let Some(Ok(event)) = registry.try_decode_account(
            &account.owner,
            &account.data,
        ) {
            process_event(event);
        }
    }
}
```

### 2. Custom Decoder Integration

```rust
use account_decoder_sdk::prelude::*;

fn main() {
    let registry = registry_builder()
        .with_account(TokenDecoder::new())
        .with_account(MyCustomDecoder::new())
        .with_warnings(true)
        .build();
}
```

### 3. Anchor Codegen

```rust
use account_decoder_sdk::{CodeGenerator, GeneratorConfig, IdlParser};

fn main() {
    let idl = IdlParser::parse_file("idl/my_program.json")?;
    let gen = CodeGenerator::new(GeneratorConfig::default().with_serde());
    let code = gen.generate(&idl)?;
    std::fs::write("src/generated.rs", code.to_string())?;
}
```
