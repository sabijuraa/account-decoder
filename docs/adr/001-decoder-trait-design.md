# ADR 001: Decoder Trait Design

## Status

Accepted

## Context

We need to define traits for decoding Solana account and instruction data. The design must balance:

1. **Type Safety**: Decoders should produce typed outputs, not raw bytes
2. **Uniformity**: All decoded data should implement a common interface for registry use
3. **Extensibility**: New decoders can be added without modifying core code
4. **Performance**: Support zero-copy and minimal allocation patterns
5. **Ergonomics**: Easy to implement for common cases

## Decision

### Trait Hierarchy

We define two primary traits:

```rust
pub trait AccountDecoder: Send + Sync + Debug {
    fn metadata(&self) -> DecoderMetadata;
    fn capabilities(&self) -> DecoderCapabilities;
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>>;
    fn decode_account_with_key(&self, pubkey: &Pubkey, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>>;
    fn can_decode(&self, data: &[u8]) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub trait InstructionDecoder: Send + Sync + Debug {
    fn metadata(&self) -> DecoderMetadata;
    fn decode_instruction(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>>;
    fn decode_instruction_with_accounts(&self, data: &[u8], accounts: &[Pubkey]) -> DecodeResult<Box<dyn DecodedEvent>>;
    fn as_any(&self) -> &dyn Any;
}
```

### Associated Types vs Dynamic Dispatch

We chose dynamic dispatch (`Box<dyn DecodedEvent>`) over associated types because:

1. **Registry Compatibility**: A registry must store heterogeneous decoders
2. **Runtime Flexibility**: Decoders can return different event types based on discriminator
3. **Simpler API**: Users don't need to specify type parameters

The cost is one heap allocation per decode, acceptable for our use case.

### DecodedEvent Trait

```rust
pub trait DecodedEvent: Debug + Send + Sync {
    fn event_kind(&self) -> EventKind;
    fn event_type(&self) -> &'static str;
    fn program_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
}
```

The `as_any()` method enables type-safe downcasting:

```rust
if let Some(mint) = event.downcast_ref::<Mint>() {
    println!("Supply: {}", mint.supply);
}
```

### Error Handling

We use a custom error enum rather than generic errors:

```rust
pub enum DecodeError {
    InsufficientData { expected: usize, actual: usize },
    UnknownDiscriminator(Vec<u8>),
    DeserializationError(String),
    UnknownProgram(Pubkey),
    // ...
}
```

This allows:
- Pattern matching for error recovery
- Rich context for debugging
- Typed conversion from underlying errors

### Metadata and Capabilities

Decoders provide metadata for observability:

```rust
pub struct DecoderMetadata {
    pub program_name: &'static str,
    pub program_id: Pubkey,
    pub version: &'static str,
    pub description: Option<&'static str>,
}

pub struct DecoderCapabilities {
    pub zero_copy: bool,
    pub detailed_fields: bool,
    pub streaming: bool,
    pub account_types: Vec<&'static str>,
}
```

This enables:
- Debug logging
- Feature detection
- Performance optimization decisions

## Consequences

### Positive

- Clear separation between decoding logic and event types
- Easy to add new decoders without changing core
- Type-safe downcasting when specific types are needed
- Rich error information for debugging

### Negative

- One heap allocation per decode (`Box<dyn DecodedEvent>`)
- Downcasting requires knowing concrete types
- Traits are not object-safe without workarounds

### Mitigations

- For high-performance scenarios, implement direct parsing methods on decoders
- Provide helper traits (`TypedEvent`) for ergonomic downcasting
- Document concrete types in decoder documentation

## Alternatives Considered

### Associated Types

```rust
trait AccountDecoder {
    type Output: DecodedEvent;
    fn decode(&self, data: &[u8]) -> Result<Self::Output>;
}
```

Rejected because registries would need type erasure anyway, and this complicates multi-type decoders.

### Enum-Based Events

```rust
enum TokenEvent {
    Mint(Mint),
    TokenAccount(TokenAccount),
}
```

Rejected because it doesn't scale to many programs and requires modifying the enum for new types.

### Generic Return Types

```rust
fn decode<T: DecodedEvent>(&self, data: &[u8]) -> Result<T>;
```

Rejected because callers would need to know the exact type, defeating the purpose of a registry.
