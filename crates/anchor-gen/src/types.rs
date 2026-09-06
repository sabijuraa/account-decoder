//! Type mapping from IDL types to Rust types.
//!
//! This module handles the conversion of Anchor IDL type representations
//! to actual Rust type syntax.

use crate::idl::{IdlType, IdlTypeComplex};
use heck::{ToSnakeCase, ToUpperCamelCase};

/// A Rust type representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustType {
    /// A primitive type (u8, u64, bool, etc.)
    Primitive(String),

    /// A Solana pubkey
    Pubkey,

    /// Option<T>
    Option(Box<RustType>),

    /// Vec<T>
    Vec(Box<RustType>),

    /// [T; N]
    Array(Box<RustType>, usize),

    /// A custom/defined type
    Custom(String),

    /// String type
    String,

    /// Bytes (Vec<u8>)
    Bytes,
}

impl RustType {
    /// Convert to Rust type syntax string.
    pub fn to_rust_string(&self) -> String {
        match self {
            RustType::Primitive(s) => s.clone(),
            RustType::Pubkey => "Pubkey".to_string(),
            RustType::Option(inner) => format!("Option<{}>", inner.to_rust_string()),
            RustType::Vec(inner) => format!("Vec<{}>", inner.to_rust_string()),
            RustType::Array(inner, size) => format!("[{}; {}]", inner.to_rust_string(), size),
            RustType::Custom(name) => name.to_upper_camel_case(),
            RustType::String => "String".to_string(),
            RustType::Bytes => "Vec<u8>".to_string(),
        }
    }

    /// Check if this type requires borsh derive.
    pub fn needs_borsh(&self) -> bool {
        match self {
            RustType::Custom(_) => true,
            RustType::Option(inner) | RustType::Vec(inner) | RustType::Array(inner, _) => {
                inner.needs_borsh()
            }
            _ => false,
        }
    }

    /// Get the size in bytes if fixed-size, None otherwise.
    pub fn fixed_size(&self) -> Option<usize> {
        match self {
            RustType::Primitive(p) => primitive_size(p),
            RustType::Pubkey => Some(32),
            RustType::Array(inner, size) => inner.fixed_size().map(|s| s * size),
            RustType::Option(_) | RustType::Vec(_) | RustType::Custom(_) | RustType::String | RustType::Bytes => None,
        }
    }
}

/// Get the size of a primitive type.
fn primitive_size(name: &str) -> Option<usize> {
    match name {
        "bool" | "u8" | "i8" => Some(1),
        "u16" | "i16" => Some(2),
        "u32" | "i32" | "f32" => Some(4),
        "u64" | "i64" | "f64" => Some(8),
        "u128" | "i128" => Some(16),
        _ => None,
    }
}

/// Maps IDL types to Rust types.
#[derive(Debug, Clone, Default)]
pub struct TypeMapper {
    /// Custom type name mappings.
    type_overrides: std::collections::HashMap<String, String>,
}

impl TypeMapper {
    /// Create a new type mapper.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom type name override.
    pub fn with_override(mut self, idl_name: &str, rust_name: &str) -> Self {
        self.type_overrides
            .insert(idl_name.to_string(), rust_name.to_string());
        self
    }

    /// Map an IDL type to a Rust type.
    pub fn map(&self, idl_type: &IdlType) -> RustType {
        match idl_type {
            IdlType::Primitive(name) => self.map_primitive(name),
            IdlType::Complex(complex) => self.map_complex(complex),
        }
    }

    /// Map a primitive IDL type name.
    fn map_primitive(&self, name: &str) -> RustType {
        match name {
            // Standard primitives
            "bool" | "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128"
            | "i128" | "f32" | "f64" => RustType::Primitive(name.to_string()),

            // Solana types
            "pubkey" | "publicKey" | "Pubkey" => RustType::Pubkey,

            // String types
            "string" | "String" => RustType::String,

            // Bytes
            "bytes" => RustType::Bytes,

            // Unknown - treat as custom
            other => {
                if let Some(override_name) = self.type_overrides.get(other) {
                    RustType::Custom(override_name.clone())
                } else {
                    RustType::Custom(other.to_string())
                }
            }
        }
    }

    /// Map a complex IDL type.
    fn map_complex(&self, complex: &IdlTypeComplex) -> RustType {
        match complex {
            IdlTypeComplex::Option(inner) => RustType::Option(Box::new(self.map(inner))),
            IdlTypeComplex::Vec(inner) => RustType::Vec(Box::new(self.map(inner))),
            IdlTypeComplex::Array(inner, size) => {
                RustType::Array(Box::new(self.map(inner)), *size)
            }
            IdlTypeComplex::Defined(defined) => {
                let name = defined.name();
                if let Some(override_name) = self.type_overrides.get(name) {
                    RustType::Custom(override_name.clone())
                } else {
                    RustType::Custom(name.to_string())
                }
            }
            IdlTypeComplex::Generic(name) => RustType::Custom(name.clone()),
        }
    }

    /// Convert a field name from IDL naming to Rust snake_case.
    pub fn map_field_name(&self, name: &str) -> String {
        // Handle special cases
        match name {
            "type" => "r#type".to_string(),
            "self" => "r#self".to_string(),
            "const" => "r#const".to_string(),
            "static" => "r#static".to_string(),
            _ => name.to_snake_case(),
        }
    }

    /// Convert a type name from IDL naming to Rust PascalCase.
    pub fn map_type_name(&self, name: &str) -> String {
        if let Some(override_name) = self.type_overrides.get(name) {
            return override_name.clone();
        }
        name.to_upper_camel_case()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_mapping() {
        let mapper = TypeMapper::new();

        assert_eq!(
            mapper.map(&IdlType::Primitive("u64".into())),
            RustType::Primitive("u64".into())
        );

        assert_eq!(
            mapper.map(&IdlType::Primitive("publicKey".into())),
            RustType::Pubkey
        );

        assert_eq!(
            mapper.map(&IdlType::Primitive("string".into())),
            RustType::String
        );
    }

    #[test]
    fn test_complex_mapping() {
        let mapper = TypeMapper::new();

        let option_type = IdlType::Complex(IdlTypeComplex::Option(Box::new(IdlType::Primitive(
            "u64".into(),
        ))));

        assert_eq!(
            mapper.map(&option_type),
            RustType::Option(Box::new(RustType::Primitive("u64".into())))
        );
    }

    #[test]
    fn test_rust_string_generation() {
        assert_eq!(RustType::Primitive("u64".into()).to_rust_string(), "u64");
        assert_eq!(RustType::Pubkey.to_rust_string(), "Pubkey");
        assert_eq!(
            RustType::Option(Box::new(RustType::Primitive("u64".into()))).to_rust_string(),
            "Option<u64>"
        );
        assert_eq!(
            RustType::Vec(Box::new(RustType::Pubkey)).to_rust_string(),
            "Vec<Pubkey>"
        );
        assert_eq!(
            RustType::Array(Box::new(RustType::Primitive("u8".into())), 32).to_rust_string(),
            "[u8; 32]"
        );
    }

    #[test]
    fn test_field_name_mapping() {
        let mapper = TypeMapper::new();

        assert_eq!(mapper.map_field_name("myField"), "my_field");
        assert_eq!(mapper.map_field_name("type"), "r#type");
        assert_eq!(mapper.map_field_name("amount"), "amount");
    }

    #[test]
    fn test_fixed_size() {
        assert_eq!(RustType::Primitive("u64".into()).fixed_size(), Some(8));
        assert_eq!(RustType::Pubkey.fixed_size(), Some(32));
        assert_eq!(
            RustType::Array(Box::new(RustType::Primitive("u8".into())), 32).fixed_size(),
            Some(32)
        );
        assert_eq!(RustType::String.fixed_size(), None);
        assert_eq!(RustType::Vec(Box::new(RustType::Pubkey)).fixed_size(), None);
    }
}
