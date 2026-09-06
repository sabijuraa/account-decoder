//! Anchor IDL parsing.
//!
//! This module provides types and parsers for Anchor's Interface Definition Language (IDL).
//! The IDL describes a program's accounts, instructions, types, and events.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during IDL parsing.
#[derive(Error, Debug)]
pub enum IdlError {
    /// Failed to parse the JSON.
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// The IDL is missing required fields.
    #[error("invalid IDL: {0}")]
    InvalidIdl(String),

    /// Unsupported IDL version.
    #[error("unsupported IDL version: {0}")]
    UnsupportedVersion(String),
}

/// Result type for IDL operations.
pub type IdlResult<T> = Result<T, IdlError>;

/// Parser for Anchor IDL JSON files.
pub struct IdlParser;

impl IdlParser {
    /// Parse an IDL from a JSON string.
    pub fn parse(json: &str) -> IdlResult<IdlProgram> {
        let mut idl: IdlProgram = serde_json::from_str(json)?;

        // Validate required fields
        if idl.name.is_empty() {
            return Err(IdlError::InvalidIdl("program name is required".into()));
        }

        fill_missing_discriminators(&mut idl);

        Ok(idl)
    }

    /// Parse an IDL from a file path.
    pub fn parse_file(path: &std::path::Path) -> IdlResult<IdlProgram> {
        let json = std::fs::read_to_string(path).map_err(|e| {
            IdlError::InvalidIdl(format!("failed to read file: {}", e))
        })?;
        Self::parse(&json)
    }
}

/// Compute the discriminators that older IDLs leave out.
///
/// Anchor only started writing `discriminator` into the IDL in 0.30. Before
/// that the value was implied by a convention: the first eight bytes of
/// `sha256("account:<TypeName>")` for accounts and
/// `sha256("global:<snake_case_name>")` for instructions. Without this, an IDL
/// fetched from a program deployed with an older Anchor produces a decoder
/// whose dispatch table is empty -- it compiles, and then matches nothing.
fn fill_missing_discriminators(idl: &mut IdlProgram) {
    use heck::ToSnakeCase;

    for account in &mut idl.accounts {
        if account.discriminator.is_empty() {
            account.discriminator = anchor_discriminator("account", &account.name);
        }
    }

    for instruction in &mut idl.instructions {
        if instruction.discriminator.is_empty() {
            let name = instruction.name.to_snake_case();
            instruction.discriminator = anchor_discriminator("global", &name);
        }
    }
}

/// First eight bytes of `sha256("<namespace>:<name>")`.
///
/// Delegates to `borsh-util` rather than hashing here. There used to be two
/// implementations of this -- one in each crate -- and the one in `borsh-util`
/// was behind a `sha2` feature that was never declared, so it never compiled at
/// all. Two copies of the rule that decides which decoder runs is one too many.
pub fn anchor_discriminator(namespace: &str, name: &str) -> Vec<u8> {
    account_decoder_borsh_util::AnchorDiscriminator::compute(namespace, name)
        .as_bytes()
        .to_vec()
}

/// An Anchor program IDL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlProgram {
    /// Program name.
    pub name: String,

    /// IDL version (semantic versioning).
    #[serde(default)]
    pub version: String,

    /// Program documentation.
    #[serde(default)]
    pub docs: Vec<String>,

    /// Account types defined by the program.
    #[serde(default)]
    pub accounts: Vec<IdlAccount>,

    /// Instructions defined by the program.
    #[serde(default)]
    pub instructions: Vec<IdlInstruction>,

    /// Custom types (structs, enums).
    #[serde(default, alias = "types")]
    pub types: Vec<IdlTypeDef>,

    /// Events emitted by the program.
    #[serde(default)]
    pub events: Vec<IdlEvent>,

    /// Error codes.
    #[serde(default)]
    pub errors: Vec<IdlErrorCode>,

    /// Program metadata.
    #[serde(default)]
    pub metadata: Option<IdlMetadata>,
}

impl IdlProgram {
    /// Get the program ID from metadata, if available.
    pub fn program_id(&self) -> Option<&str> {
        self.metadata.as_ref()?.address.as_deref()
    }

    /// Find an account type by name.
    pub fn find_account(&self, name: &str) -> Option<&IdlAccount> {
        self.accounts.iter().find(|a| a.name == name)
    }

    /// Find an instruction by name.
    pub fn find_instruction(&self, name: &str) -> Option<&IdlInstruction> {
        self.instructions.iter().find(|i| i.name == name)
    }

    /// Find a type definition by name.
    pub fn find_type(&self, name: &str) -> Option<&IdlTypeDef> {
        self.types.iter().find(|t| t.name == name)
    }
}

/// An account type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlAccount {
    /// Account name (PascalCase).
    pub name: String,

    /// Account documentation.
    #[serde(default)]
    pub docs: Vec<String>,

    /// The type definition, either inline or a reference.
    #[serde(default)]
    pub r#type: Option<IdlTypeDefTy>,

    /// Discriminator bytes (Anchor uses first 8 bytes).
    #[serde(default)]
    pub discriminator: Vec<u8>,
}

/// An instruction definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlInstruction {
    /// Instruction name (camelCase in IDL, will be converted).
    pub name: String,

    /// Instruction documentation.
    #[serde(default)]
    pub docs: Vec<String>,

    /// Accounts required by this instruction.
    #[serde(default)]
    pub accounts: Vec<IdlAccountItem>,

    /// Arguments to the instruction.
    #[serde(default)]
    pub args: Vec<IdlField>,

    /// Discriminator bytes.
    #[serde(default)]
    pub discriminator: Vec<u8>,

    /// Return type, if any.
    #[serde(default)]
    pub returns: Option<IdlType>,
}

/// An account reference in an instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlAccountItem {
    /// A single account.
    Single(IdlAccountRef),
    /// A composite of accounts.
    Composite(IdlAccounts),
}

/// A single account reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlAccountRef {
    /// Account name.
    pub name: String,

    /// Whether this account is mutable.
    #[serde(default, alias = "isMut")]
    pub is_mut: bool,

    /// Whether this account is a signer.
    #[serde(default, alias = "isSigner")]
    pub is_signer: bool,

    /// Whether this account is optional.
    #[serde(default, alias = "isOptional")]
    pub is_optional: bool,

    /// Account documentation.
    #[serde(default)]
    pub docs: Vec<String>,

    /// Program-derived address seeds.
    #[serde(default)]
    pub pda: Option<IdlPda>,
}

/// A composite of accounts (for nested structs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlAccounts {
    /// Name of the composite.
    pub name: String,

    /// Nested accounts.
    pub accounts: Vec<IdlAccountItem>,
}

/// PDA derivation information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlPda {
    /// Seeds used to derive the PDA.
    pub seeds: Vec<IdlSeed>,

    /// Program ID to derive against (defaults to current program).
    #[serde(default)]
    pub program: Option<IdlSeed>,
}

/// A PDA seed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IdlSeed {
    /// A constant seed.
    #[serde(rename = "const")]
    Const { value: serde_json::Value },

    /// A seed from an account.
    #[serde(rename = "account")]
    Account { path: String },

    /// A seed from an argument.
    #[serde(rename = "arg")]
    Arg { path: String },
}

/// A type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlTypeDef {
    /// Type name.
    pub name: String,

    /// Type documentation.
    #[serde(default)]
    pub docs: Vec<String>,

    /// The type definition (struct or enum).
    pub r#type: IdlTypeDefTy,

    /// Generic parameters.
    #[serde(default)]
    pub generics: Vec<String>,
}

/// The kind of type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IdlTypeDefTy {
    /// A struct type.
    #[serde(rename = "struct")]
    Struct { fields: Vec<IdlField> },

    /// An enum type.
    #[serde(rename = "enum")]
    Enum { variants: Vec<IdlEnumVariant> },
}

/// A field in a struct or instruction args.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlField {
    /// Field name.
    pub name: String,

    /// Field type.
    pub r#type: IdlType,

    /// Field documentation.
    #[serde(default)]
    pub docs: Vec<String>,
}

/// An enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlEnumVariant {
    /// Variant name.
    pub name: String,

    /// Fields (for tuple or struct variants).
    #[serde(default)]
    pub fields: Option<IdlEnumFields>,
}

/// Fields in an enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlEnumFields {
    /// Tuple fields (unnamed).
    Tuple(Vec<IdlType>),
    /// Struct fields (named).
    Named(Vec<IdlField>),
}

/// An IDL type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlType {
    /// A primitive or built-in type.
    Primitive(String),

    /// A complex type.
    Complex(IdlTypeComplex),
}

/// Complex IDL types.
///
/// Anchor writes these externally tagged, e.g. `{"defined": "State"}`,
/// `{"option": {...}}`, `{"vec": {...}}`, `{"array": [{...}, 32]}`. Anchor
/// 0.30 changed `defined` from a bare string to `{"name": "State"}`, so both
/// spellings are accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdlTypeComplex {
    /// Option<T>
    Option(Box<IdlType>),

    /// Vec<T>
    Vec(Box<IdlType>),

    /// [T; N]
    Array(Box<IdlType>, usize),

    /// A defined type (reference by name).
    Defined(IdlDefinedType),

    /// A generic type.
    Generic(String),
}

/// The payload of a `defined` type reference.
///
/// Legacy IDLs write `{"defined": "State"}`; Anchor 0.30 and later write
/// `{"defined": {"name": "State"}}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlDefinedType {
    /// Legacy form: the type name directly.
    Name(String),
    /// Current form: an object carrying the name.
    Named {
        /// The referenced type name.
        name: String,
    },
}

impl IdlDefinedType {
    /// The referenced type name, whichever spelling was used.
    pub fn name(&self) -> &str {
        match self {
            IdlDefinedType::Name(n) => n,
            IdlDefinedType::Named { name } => name,
        }
    }
}

impl IdlType {
    /// Check if this is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(self, IdlType::Primitive(_))
    }

    /// Get the type name for primitives.
    pub fn as_primitive(&self) -> Option<&str> {
        match self {
            IdlType::Primitive(s) => Some(s),
            _ => None,
        }
    }
}

/// An event definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlEvent {
    /// Event name.
    pub name: String,

    /// Event fields.
    #[serde(default)]
    pub fields: Vec<IdlField>,

    /// Discriminator bytes.
    #[serde(default)]
    pub discriminator: Vec<u8>,
}

/// An error code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlErrorCode {
    /// Error code number.
    pub code: u32,

    /// Error name.
    pub name: String,

    /// Error message.
    #[serde(default)]
    pub msg: Option<String>,
}

/// Program metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlMetadata {
    /// Program address (base58 pubkey).
    #[serde(default)]
    pub address: Option<String>,

    /// Origin (e.g., "anchor", "shank").
    #[serde(default)]
    pub origin: Option<String>,

    /// Build information.
    #[serde(default)]
    pub build: Option<IdlBuildInfo>,
}

/// Build information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlBuildInfo {
    /// Rust version used.
    #[serde(default)]
    pub rustc: Option<String>,

    /// Anchor version used.
    #[serde(default)]
    pub anchor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_idl() {
        let json = r#"{
            "name": "test_program",
            "version": "0.1.0",
            "instructions": []
        }"#;

        let idl = IdlParser::parse(json).unwrap();
        assert_eq!(idl.name, "test_program");
        assert_eq!(idl.version, "0.1.0");
    }

    #[test]
    fn test_parse_with_accounts() {
        let json = r#"{
            "name": "test_program",
            "accounts": [
                {
                    "name": "MyAccount",
                    "discriminator": [1, 2, 3, 4, 5, 6, 7, 8],
                    "type": {
                        "kind": "struct",
                        "fields": [
                            {"name": "value", "type": "u64"},
                            {"name": "owner", "type": "publicKey"}
                        ]
                    }
                }
            ]
        }"#;

        let idl = IdlParser::parse(json).unwrap();
        assert_eq!(idl.accounts.len(), 1);
        assert_eq!(idl.accounts[0].name, "MyAccount");
        assert_eq!(idl.accounts[0].discriminator, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_parse_with_instructions() {
        let json = r#"{
            "name": "test_program",
            "instructions": [
                {
                    "name": "initialize",
                    "discriminator": [0, 1, 2, 3, 4, 5, 6, 7],
                    "accounts": [
                        {"name": "authority", "isMut": false, "isSigner": true}
                    ],
                    "args": [
                        {"name": "amount", "type": "u64"}
                    ]
                }
            ]
        }"#;

        let idl = IdlParser::parse(json).unwrap();
        assert_eq!(idl.instructions.len(), 1);
        assert_eq!(idl.instructions[0].name, "initialize");
        assert_eq!(idl.instructions[0].args.len(), 1);
    }

    #[test]
    fn test_empty_name_error() {
        let json = r#"{"name": ""}"#;

        let result = IdlParser::parse(json);
        assert!(matches!(result, Err(IdlError::InvalidIdl(_))));
    }
}
