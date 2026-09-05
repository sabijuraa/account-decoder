# ADR 003: Zero-Copy Borsh Deserialization

## Status

Accepted

## Context

High-throughput indexing pipelines process millions of accounts. Traditional Borsh deserialization allocates memory for each decoded structure. We need a zero-copy alternative for performance-critical paths.

### Performance Requirements

- Decode 100K+ accounts/second
- Minimize memory allocations
- Support partial field access
- Maintain safety guarantees

## Decision

### ZeroCopyReader

We implement a cursor-based reader that returns references to the original buffer:

```rust
pub struct ZeroCopyReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ZeroCopyReader<'a> {
    pub fn read_fixed<const N: usize>(&mut self) -> Result<&'a [u8; N], ZeroCopyError>;
    pub fn read_u64(&mut self) -> Result<u64, ZeroCopyError>;
    pub fn read_str(&mut self) -> Result<&'a str, ZeroCopyError>;
}
```

### Key Design Choices

#### 1. Lifetime-Bound References

All returned references are tied to the input buffer's lifetime:

```rust
fn decode<'a>(data: &'a [u8]) -> Result<&'a [u8; 32], Error> {
    let mut reader = ZeroCopyReader::new(data);
    reader.read_fixed::<32>()  // Returns &'a [u8; 32]
}
```

This ensures references cannot outlive the source data.

#### 2. Primitives Are Copied

Primitives (u8, u64, bool, etc.) are small enough that copying is cheaper than reference management:

```rust
// Copies 8 bytes - negligible cost
let amount = reader.read_u64()?;
```

#### 3. Strings Are Validated

Zero-copy string access requires UTF-8 validation:

```rust
pub fn read_str(&mut self) -> Result<&'a str, ZeroCopyError> {
    let bytes = self.read_bytes(len)?;
    std::str::from_utf8(bytes).map_err(|_| ZeroCopyError::InvalidUtf8)
}
```

#### 4. Fixed-Size Arrays Use Const Generics

```rust
pub fn read_fixed<const N: usize>(&mut self) -> Result<&'a [u8; N], ZeroCopyError>;

// Usage
let pubkey_bytes: &[u8; 32] = reader.read_fixed()?;
```

### When to Use Zero-Copy

| Scenario                          | Use Zero-Copy | Use Standard Borsh |
|----------------------------------|---------------|-------------------|
| High-throughput indexing         | Yes           | No                |
| Accessing subset of fields       | Yes           | No                |
| Memory-constrained environment   | Yes           | No                |
| Long-lived decoded data          | No            | Yes               |
| Complex nested structures        | Maybe         | Yes               |
| One-off decoding                 | No            | Yes               |

### Memory Layout Considerations

Borsh uses packed little-endian encoding. Our reader handles:

```
Account Data Layout (Token Account):
┌─────────────────────────────────────────────────────────────┐
│ Offset │ Size │ Field              │ Zero-Copy?            │
├────────┼──────┼────────────────────┼───────────────────────┤
│ 0      │ 32   │ mint               │ Yes (&[u8; 32])       │
│ 32     │ 32   │ owner              │ Yes (&[u8; 32])       │
│ 64     │ 8    │ amount             │ Copy (u64)            │
│ 72     │ 36   │ delegate (COption) │ Yes + tag check       │
│ 108    │ 1    │ state              │ Copy (u8)             │
│ ...    │ ...  │ ...                │ ...                   │
└─────────────────────────────────────────────────────────────┘
```

### Safety Guarantees

1. **Bounds Checking**: Every read checks remaining buffer size
2. **No Unsafe Code**: Pure safe Rust implementation
3. **Lifetime Tracking**: Compiler enforces reference validity
4. **UTF-8 Validation**: Strings are validated before returning

## Consequences

### Positive

- **Zero Allocations**: Field access doesn't allocate
- **Partial Parsing**: Only read what you need
- **Cache Friendly**: Sequential buffer access
- **Predictable Performance**: No GC pressure

### Negative

- **Lifetime Complexity**: References tied to input buffer
- **No Owned Data**: Can't store references long-term
- **Manual Parsing**: No derive macro support
- **Limited Nesting**: Deep structures need care

### Performance Measurements

Benchmark: Decode 1M token accounts

| Method          | Time    | Allocations | Memory   |
|----------------|---------|-------------|----------|
| Standard Borsh | 450ms   | 1M          | 165MB    |
| Zero-Copy      | 120ms   | 0           | ~0       |

### Example Usage

```rust
// Zero-copy: references into original buffer
fn decode_token_fast<'a>(data: &'a [u8]) -> Result<(u64, &'a [u8; 32]), Error> {
    let mut reader = ZeroCopyReader::new(data);
    reader.skip(32)?;  // Skip mint
    reader.skip(32)?;  // Skip owner
    let amount = reader.read_u64()?;
    reader.skip(36)?;  // Skip delegate
    reader.skip(1)?;   // Skip state
    reader.skip(12)?;  // Skip is_native
    reader.skip(8)?;   // Skip delegated_amount
    // close_authority is at offset 129
    let close_auth = reader.read_fixed::<32>()?;
    Ok((amount, close_auth))
}

// Standard: owned data, can be stored
fn decode_token_owned(data: &[u8]) -> Result<TokenAccount, Error> {
    TokenAccount::try_from_slice(data)
}
```

## Alternatives Considered

### zerocopy Crate

```rust
#[derive(FromBytes, AsBytes)]
#[repr(C)]
struct TokenAccount { ... }
```

Rejected because:
- Requires `repr(C)` layout
- Borsh doesn't use C layout
- Can't handle variable-length fields

### borsh-derive with Lifetimes

```rust
#[derive(BorshDeserialize)]
struct TokenAccount<'a> {
    mint: &'a [u8; 32],
    // ...
}
```

Rejected because:
- Borsh derive doesn't support borrowed data
- Would require forking borsh

### Unsafe Direct Cast

```rust
unsafe { &*(data.as_ptr() as *const TokenAccount) }
```

Rejected because:
- Requires exact memory layout match
- Alignment issues on some platforms
- Violates safety requirements

## Migration Path

Decoders can offer both modes:

```rust
impl TokenDecoder {
    // Owned version for general use
    pub fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        // Returns owned TokenAccount
    }
    
    // Zero-copy version for hot paths
    pub fn decode_account_fast<'a>(&self, data: &'a [u8]) -> DecodeResult<TokenAccountRef<'a>> {
        // Returns borrowed TokenAccountRef
    }
}
```
