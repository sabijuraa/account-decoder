# ADR 004: Anchor IDL Code Generation

## Status

Accepted

## Context

Anchor programs publish IDL (Interface Definition Language) files describing their accounts and instructions. Manually writing decoders for each program is tedious and error-prone. We need automated code generation from IDLs.

### Goals

1. Generate type-safe Rust structs from IDL
2. Generate decoder implementations
3. Support Anchor's discriminator scheme
4. Handle all IDL type constructs
5. Produce readable, maintainable code

## Decision

### Pipeline Architecture

```
IDL JSON File
      │
      ▼
┌─────────────────┐
│   IdlParser     │  Parse JSON into typed structures
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   IdlProgram    │  Structured IDL representation
│   IdlAccount    │
│   IdlInstruction│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   TypeMapper    │  Map IDL types to Rust types
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  CodeGenerator  │  Generate Rust code via quote!
└────────┬────────┘
         │
         ▼
    TokenStream → Rust Source Code
```

### IDL Types

We support the full Anchor IDL type system:

```rust
pub enum IdlType {
    Primitive(String),  // "u64", "bool", "publicKey"
    Complex(IdlTypeComplex),
}

pub enum IdlTypeComplex {
    Option(Box<IdlType>),
    Vec(Box<IdlType>),
    Array(Box<IdlType>, usize),
    Defined(String),     // Custom type reference
    Generic(String),
}
```

### Type Mapping

The `TypeMapper` converts IDL types to Rust:

| IDL Type       | Rust Type           |
|---------------|---------------------|
| `u8`..`u128`  | `u8`..`u128`        |
| `i8`..`i128`  | `i8`..`i128`        |
| `f32`, `f64`  | `f32`, `f64`        |
| `bool`        | `bool`              |
| `string`      | `String`            |
| `bytes`       | `Vec<u8>`           |
| `publicKey`   | `Pubkey`            |
| `Option<T>`   | `Option<T>`         |
| `Vec<T>`      | `Vec<T>`            |
| `[T; N]`      | `[T; N]`            |
| `MyType`      | `MyType` (defined)  |

### Code Generation

Using `proc_macro2` and `quote` for code generation:

```rust
fn generate_account_type(&self, account: &IdlAccount) -> TokenStream {
    let type_name = format_ident!("{}", account.name);
    let fields = self.generate_fields(&account.fields)?;
    
    quote! {
        #[derive(BorshDeserialize, Debug, Clone)]
        pub struct #type_name {
            #fields
        }
    }
}
```

### Discriminator Handling

Anchor uses 8-byte discriminators (first 8 bytes of SHA256):

```rust
fn generate_account_decoder(&self, idl: &IdlProgram) -> TokenStream {
    let arms = idl.accounts.iter().map(|acc| {
        let disc = &acc.discriminator;
        let type_name = format_ident!("{}", acc.name);
        
        quote! {
            [#(#disc),*] => {
                let value = #type_name::try_from_slice(&data[8..])?;
                Ok(Box::new(value))
            }
        }
    });
    
    quote! {
        match discriminator {
            #(#arms)*
            _ => Err(DecodeError::unknown_discriminator(&discriminator)),
        }
    }
}
```

### Configuration

The generator is configurable:

```rust
pub struct GeneratorConfig {
    pub derive_debug: bool,
    pub derive_clone: bool,
    pub derive_serde: bool,
    pub use_zero_copy: bool,
    pub generate_instructions: bool,
    pub generate_accounts: bool,
    pub include_docs: bool,
}
```

## Consequences

### Positive

- **Type Safety**: Generated code is type-checked
- **Consistency**: All decoders follow the same pattern
- **Maintainability**: Regenerate when IDL changes
- **Completeness**: Handles full IDL specification

### Negative

- **Build Dependency**: Requires proc-macro2, quote, syn
- **Generated Code Size**: May produce verbose code
- **Anchor-Specific**: Designed for Anchor IDL format

### Generated Code Example

Input IDL:
```json
{
  "name": "counter",
  "accounts": [{
    "name": "Counter",
    "discriminator": [255, 176, 4, 245, 188, 253, 124, 25],
    "type": {
      "kind": "struct",
      "fields": [
        {"name": "count", "type": "u64"},
        {"name": "authority", "type": "publicKey"}
      ]
    }
  }]
}
```

Generated code:
```rust
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct Counter {
    pub count: u64,
    pub authority: Pubkey,
}

impl DecodedEvent for Counter {
    fn event_kind(&self) -> EventKind { EventKind::Account }
    fn event_type(&self) -> &'static str { "Counter" }
    fn program_name(&self) -> &'static str { "counter" }
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Debug, Clone)]
pub struct CounterAccountDecoder {
    program_id: Pubkey,
}

impl AccountDecoder for CounterAccountDecoder {
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        let discriminator: [u8; 8] = data[..8].try_into().unwrap();
        match discriminator {
            [255, 176, 4, 245, 188, 253, 124, 25] => {
                let value = Counter::try_from_slice(&data[8..])?;
                Ok(Box::new(value))
            }
            _ => Err(DecodeError::unknown_discriminator(&discriminator)),
        }
    }
    // ...
}
```

## Alternatives Considered

### Proc-Macro Approach

```rust
#[derive(AnchorDecoder)]
#[anchor_idl = "path/to/idl.json"]
struct MyDecoder;
```

Rejected because:
- Proc macros are harder to debug
- IDL changes require recompilation
- Less flexible configuration

### Runtime IDL Parsing

```rust
let decoder = RuntimeDecoder::from_idl(&idl);
```

Rejected because:
- No compile-time type checking
- Runtime overhead
- Complex value representation

### Template-Based Generation

Using Tera or Handlebars templates.

Rejected because:
- Less type-safe than quote!
- Template errors at runtime
- Rust syntax not validated

## Future Enhancements

### Build Script Integration

```rust
// build.rs
fn main() {
    account_decoder_anchor_gen::generate_from_idl(
        "target/idl/my_program.json",
        "src/generated/",
    );
}
```

### IDL Version Detection

Support for different Anchor IDL versions:
- Anchor 0.24 format
- Anchor 0.29+ format (new discriminator style)

### Incremental Generation

Only regenerate when IDL changes:
```rust
if idl_modified_since_last_gen() {
    regenerate();
}
```
