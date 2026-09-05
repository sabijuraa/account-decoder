# ADR 002: Registry Pattern

## Status

Accepted

## Context

We need a mechanism to route decode requests to the appropriate decoder based on program ID. The solution must support:

1. **Fast Lookup**: O(1) or near-O(1) by program ID
2. **Dynamic Registration**: Add decoders at runtime
3. **Thread Safety**: Concurrent access from multiple threads
4. **Type Heterogeneity**: Store different decoder implementations together

## Decision

### Registry Structure

```rust
pub struct DecoderRegistry {
    account_decoders: HashMap<Pubkey, Arc<dyn AccountDecoder>>,
    instruction_decoders: HashMap<Pubkey, Arc<dyn InstructionDecoder>>,
    warn_on_unknown: bool,
}
```

### Key Design Choices

#### 1. HashMap for O(1) Lookup

Program IDs are 32-byte public keys. Using a HashMap provides constant-time lookup regardless of the number of registered decoders.

```rust
pub fn get_account(&self, program_id: &Pubkey) -> Option<Arc<dyn AccountDecoder>> {
    self.account_decoders.get(program_id).cloned()
}
```

#### 2. Arc for Shared Ownership

Using `Arc<dyn Decoder>` allows:
- Multiple threads to hold references to the same decoder
- Registry to be cloned cheaply
- Decoders to be reused across registries

#### 3. Separate Maps for Account/Instruction

Account and instruction decoders are stored separately because:
- They implement different traits
- Some programs only need one type
- Lookup is cleaner without an enum wrapper

#### 4. Builder Pattern for Construction

```rust
let registry = RegistryBuilder::new()
    .with_account(TokenDecoder::new())
    .with_account(SystemDecoder::new())
    .with_warnings(true)
    .build();
```

Benefits:
- Fluent API for complex configurations
- Immutability after construction
- Clear initialization intent

### Registration API

```rust
// Single decoder registration
registry.register_account(Box::new(TokenDecoder::new()));
registry.register_instruction(Box::new(SystemDecoder::new()));

// Combined decoder (implements both traits)
registry.register_combined(MyDecoder::new());
```

### Lookup API

```rust
// Get decoder (for manual use)
if let Some(decoder) = registry.get_account(&program_id) {
    let event = decoder.decode_account(data)?;
}

// Direct decode (includes error handling)
let event = registry.decode_account(&program_id, data)?;

// Try decode (returns None for unknown programs)
if let Some(result) = registry.try_decode_account(&program_id, data) {
    // Handle result
}
```

## Consequences

### Positive

- **O(1) Lookup**: HashMap provides constant-time access
- **Thread Safe**: Arc enables safe concurrent access
- **Flexible**: Easy to add/remove decoders
- **Type Safe**: Separate maps for different decoder types

### Negative

- **Memory Overhead**: Arc adds reference counting overhead
- **No Static Dispatch**: All calls go through vtable
- **Registration Mutability**: Must lock for runtime registration

### Performance Analysis

| Operation              | Time Complexity | Notes                    |
|-----------------------|-----------------|--------------------------|
| Lookup                | O(1)            | HashMap get              |
| Registration          | O(1) amortized  | HashMap insert           |
| Clone registry        | O(n)            | Arc::clone is cheap      |
| Decode (full path)    | O(1) + decode   | Lookup + decoder work    |

## Alternatives Considered

### Static Registration (compile-time)

```rust
static REGISTRY: LazyLock<DecoderRegistry> = LazyLock::new(|| {
    // Build registry
});
```

Rejected because:
- Less flexible for plugin architectures
- Can't add decoders at runtime
- Harder to test with different configurations

### BTreeMap for Ordered Iteration

```rust
instruction_decoders: BTreeMap<Pubkey, Arc<dyn InstructionDecoder>>,
```

Rejected because:
- O(log n) lookup vs O(1)
- Ordering rarely needed
- Program IDs don't have meaningful order

### Vec with Linear Search

```rust
decoders: Vec<(Pubkey, Arc<dyn AccountDecoder>)>,
```

Rejected because:
- O(n) lookup
- Doesn't scale beyond ~10 decoders

### Phf (Perfect Hash Function)

Compile-time perfect hashing for known program IDs.

Rejected because:
- Requires knowing all IDs at compile time
- Doesn't support dynamic registration
- Adds build complexity

## Future Considerations

### RwLock for Concurrent Modification

If runtime registration becomes common:

```rust
pub struct DecoderRegistry {
    account_decoders: RwLock<HashMap<Pubkey, Arc<dyn AccountDecoder>>>,
}
```

### Tiered Lookup

For very large registries (100+ programs):

```rust
// Common programs in hot path
hot_cache: [Option<Arc<dyn AccountDecoder>>; 16],
// Full registry for cold path
full_registry: HashMap<Pubkey, Arc<dyn AccountDecoder>>,
```

### Automatic Discovery

Plugin-based decoder loading:

```rust
registry.load_plugins("./decoders/")?;
```
