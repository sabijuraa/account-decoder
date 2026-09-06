//! Code generation from parsed IDL.
//!
//! This module transforms parsed IDL structures into Rust source code
//! implementing the decoder traits.

use crate::idl::{IdlAccount, IdlField, IdlInstruction, IdlProgram, IdlTypeDef, IdlTypeDefTy};
use crate::types::TypeMapper;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use thiserror::Error;

/// Errors during code generation.
#[derive(Error, Debug)]
pub enum GeneratorError {
    /// The IDL is missing required information.
    #[error("missing required field: {0}")]
    MissingField(String),

    /// A type reference couldn't be resolved.
    #[error("unresolved type: {0}")]
    UnresolvedType(String),

    /// Code generation failed.
    #[error("codegen error: {0}")]
    CodegenError(String),
}

/// Configuration for code generation.
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Generate `#[derive(Debug)]` on all types.
    pub derive_debug: bool,

    /// Generate `#[derive(Clone)]` on all types.
    pub derive_clone: bool,

    /// Generate `#[derive(PartialEq)]` on all types.
    pub derive_partial_eq: bool,

    /// Generate serde derives for JSON serialization.
    pub derive_serde: bool,

    /// Use the zero-copy reader where possible.
    pub use_zero_copy: bool,

    /// Module name for the generated code.
    pub module_name: Option<String>,

    /// Generate instruction decoders.
    pub generate_instructions: bool,

    /// Generate account decoders.
    pub generate_accounts: bool,

    /// Include doc comments from IDL.
    pub include_docs: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            derive_debug: true,
            derive_clone: true,
            derive_partial_eq: true,
            derive_serde: false,
            use_zero_copy: true,
            module_name: None,
            generate_instructions: true,
            generate_accounts: true,
            include_docs: true,
        }
    }
}

impl GeneratorConfig {
    /// Create a minimal config (only Debug derive).
    pub fn minimal() -> Self {
        Self {
            derive_debug: true,
            derive_clone: false,
            derive_partial_eq: false,
            derive_serde: false,
            use_zero_copy: false,
            module_name: None,
            generate_instructions: true,
            generate_accounts: true,
            include_docs: false,
        }
    }

    /// Enable serde derives.
    pub fn with_serde(mut self) -> Self {
        self.derive_serde = true;
        self
    }

    /// Set the module name.
    pub fn with_module_name(mut self, name: impl Into<String>) -> Self {
        self.module_name = Some(name.into());
        self
    }
}

/// Code generator for Anchor IDLs.
pub struct CodeGenerator {
    config: GeneratorConfig,
    type_mapper: TypeMapper,
}

impl CodeGenerator {
    /// Create a new code generator with the given config.
    pub fn new(config: GeneratorConfig) -> Self {
        Self {
            config,
            type_mapper: TypeMapper::new(),
        }
    }

    /// Generate code from an IDL.
    pub fn generate(&self, idl: &IdlProgram) -> Result<TokenStream, GeneratorError> {
        let mut tokens = TokenStream::new();

        // Generate header
        tokens.extend(self.generate_header(idl));

        // Generate custom types
        for type_def in &idl.types {
            tokens.extend(self.generate_type_def(type_def)?);
        }

        // Generate account types
        if self.config.generate_accounts {
            for account in &idl.accounts {
                tokens.extend(self.generate_account_type(account)?);
            }
            tokens.extend(self.generate_account_decoder(idl)?);
        }

        // Generate instruction types
        if self.config.generate_instructions {
            for instruction in &idl.instructions {
                tokens.extend(self.generate_instruction_type(instruction)?);
            }
            tokens.extend(self.generate_instruction_decoder(idl)?);
        }

        Ok(tokens)
    }

    /// Generate the file header with imports.
    fn generate_header(&self, idl: &IdlProgram) -> TokenStream {
        let program_name = &idl.name;
        let version = &idl.version;

        let serde_import = if self.config.derive_serde {
            quote! { use serde::{Deserialize, Serialize}; }
        } else {
            quote! {}
        };

        // `#![doc = "..."]` rather than `//!`: a line comment inside `quote!` is
        // emitted verbatim, so the interpolations were never substituted and
        // every generated file carried a header reading literally
        // "Generated decoder for #program_name v#version".
        let header = format!(
            "Generated from the Anchor IDL for {program_name} v{version}. Do not edit; \
             regenerate with the `account-decoder generate` command."
        );

        quote! {
            #![doc = #header]

            use account_decoder_core::{
                AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities,
                DecoderIdentity, DecoderMetadata, EventKind, InstructionDecoder,
            };
            use borsh::BorshDeserialize;
            use solana_sdk::pubkey::Pubkey;
            use std::any::Any;

            #serde_import
        }
    }

    /// Generate a type definition (struct or enum).
    fn generate_type_def(&self, type_def: &IdlTypeDef) -> Result<TokenStream, GeneratorError> {
        let type_name = format_ident!("{}", self.type_mapper.map_type_name(&type_def.name));
        let derives = self.generate_derives();
        let docs = self.generate_docs(&type_def.docs);

        match &type_def.r#type {
            IdlTypeDefTy::Struct { fields } => {
                let field_tokens = self.generate_fields(fields)?;

                Ok(quote! {
                    #docs
                    #derives
                    pub struct #type_name {
                        #field_tokens
                    }
                })
            }
            IdlTypeDefTy::Enum { variants } => {
                let variant_tokens = variants.iter().map(|v| {
                    let variant_name = format_ident!("{}", v.name.to_upper_camel_case());
                    quote! { #variant_name }
                });

                Ok(quote! {
                    #docs
                    #derives
                    pub enum #type_name {
                        #(#variant_tokens,)*
                    }
                })
            }
        }
    }

    /// Generate an account type struct.
    fn generate_account_type(&self, account: &IdlAccount) -> Result<TokenStream, GeneratorError> {
        let type_name = format_ident!("{}", self.type_mapper.map_type_name(&account.name));
        let derives = self.generate_derives();
        let docs = self.generate_docs(&account.docs);

        let fields = match &account.r#type {
            Some(IdlTypeDefTy::Struct { fields }) => self.generate_fields(fields)?,
            _ => quote! {},
        };

        let event_impl = self.generate_event_impl(&type_name, &account.name, "account");

        Ok(quote! {
            #docs
            #derives
            pub struct #type_name {
                #fields
            }

            #event_impl
        })
    }

    /// Generate an instruction args struct.
    fn generate_instruction_type(
        &self,
        instruction: &IdlInstruction,
    ) -> Result<TokenStream, GeneratorError> {
        let type_name = format_ident!(
            "{}Instruction",
            self.type_mapper.map_type_name(&instruction.name)
        );
        let derives = self.generate_derives();
        let docs = self.generate_docs(&instruction.docs);

        let field_tokens = self.generate_fields(&instruction.args)?;
        let event_impl = self.generate_event_impl(&type_name, &instruction.name, "instruction");

        Ok(quote! {
            #docs
            #derives
            pub struct #type_name {
                #field_tokens
            }

            #event_impl
        })
    }

    /// Generate fields for a struct.
    fn generate_fields(&self, fields: &[IdlField]) -> Result<TokenStream, GeneratorError> {
        let field_tokens: Vec<_> = fields
            .iter()
            .map(|field| {
                let name = format_ident!("{}", self.type_mapper.map_field_name(&field.name));
                let rust_type = self.type_mapper.map(&field.r#type);
                let type_str: proc_macro2::TokenStream =
                    rust_type.to_rust_string().parse().unwrap();
                let docs = self.generate_docs(&field.docs);

                quote! {
                    #docs
                    pub #name: #type_str,
                }
            })
            .collect();

        Ok(quote! { #(#field_tokens)* })
    }

    /// Generate DecodedEvent impl for a type.
    ///
    /// `type_name` is the identifier of the struct actually emitted, which is
    /// not always derived from `event_name`: instruction structs get an
    /// `Instruction` suffix so they cannot collide with a same-named account or
    /// defined type.
    fn generate_event_impl(&self, type_name: &syn::Ident, name: &str, kind: &str) -> TokenStream {
        let type_name = type_name.clone();
        let event_kind = if kind == "account" {
            quote! { EventKind::Account }
        } else {
            quote! { EventKind::Instruction }
        };
        let event_type_str = name;

        quote! {
            impl DecodedEvent for #type_name {
                fn event_kind(&self) -> EventKind {
                    #event_kind
                }

                fn event_type(&self) -> &'static str {
                    #event_type_str
                }

                fn program_name(&self) -> &'static str {
                    env!("CARGO_PKG_NAME")
                }

                fn as_any(&self) -> &dyn Any {
                    self
                }
            }
        }
    }

    /// Generate the account decoder implementation.
    fn generate_account_decoder(&self, idl: &IdlProgram) -> Result<TokenStream, GeneratorError> {
        let decoder_name = format_ident!(
            "{}AccountDecoder",
            self.type_mapper.map_type_name(&idl.name)
        );
        let program_name = &idl.name;

        let discriminator_arms: Vec<_> = idl
            .accounts
            .iter()
            .filter_map(|account| {
                if account.discriminator.is_empty() {
                    return None;
                }
                let disc_bytes = &account.discriminator;
                let type_name = format_ident!("{}", self.type_mapper.map_type_name(&account.name));

                Some(quote! {
                    [#(#disc_bytes),*] => {
                        // Deserialize from a cursor rather than `try_from_slice`:
                        // Anchor accounts are frequently allocated larger than
                        // the struct, and `try_from_slice` rejects any trailing
                        // bytes with "Not all bytes read".
                        let mut cursor = &data[8..];
                        let value = #type_name::deserialize(&mut cursor)?;
                        Ok(Box::new(value))
                    }
                })
            })
            .collect();

        // The IDL already names every account type, so the generated decoder can
        // describe itself the way a hand-written one does. This also uses the
        // `DecoderCapabilities` import, which was previously emitted and never
        // referenced -- enough on its own to fail a generated crate built with
        // warnings denied.
        let account_type_names: Vec<String> = idl.accounts.iter().map(|a| a.name.clone()).collect();

        Ok(quote! {
            /// Account decoder for #program_name.
            #[derive(Debug, Clone)]
            pub struct #decoder_name {
                program_id: Pubkey,
            }

            impl #decoder_name {
                /// Create a new decoder with the given program ID.
                pub fn new(program_id: Pubkey) -> Self {
                    Self { program_id }
                }
            }

            impl DecoderIdentity for #decoder_name {
                fn metadata(&self) -> DecoderMetadata {
                    DecoderMetadata::new(#program_name, self.program_id)
                }

                fn capabilities(&self) -> DecoderCapabilities {
                    DecoderCapabilities::default()
                        .with_account_types(vec![#(#account_type_names),*])
                }

                fn as_any(&self) -> &dyn Any {
                    self
                }
            }

            impl AccountDecoder for #decoder_name {
                fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
                    if data.len() < 8 {
                        return Err(DecodeError::insufficient_data(8, data.len()));
                    }

                    let discriminator: [u8; 8] = data[..8].try_into().unwrap();

                    match discriminator {
                        #(#discriminator_arms)*
                        _ => Err(DecodeError::unknown_discriminator(&discriminator)),
                    }
                }
            }
        })
    }

    /// Generate the instruction decoder implementation.
    fn generate_instruction_decoder(
        &self,
        idl: &IdlProgram,
    ) -> Result<TokenStream, GeneratorError> {
        let decoder_name = format_ident!(
            "{}InstructionDecoder",
            self.type_mapper.map_type_name(&idl.name)
        );
        let program_name = &idl.name;

        let discriminator_arms: Vec<_> = idl
            .instructions
            .iter()
            .filter_map(|instruction| {
                if instruction.discriminator.is_empty() {
                    return None;
                }
                let disc_bytes = &instruction.discriminator;
                let type_name = format_ident!(
                    "{}Instruction",
                    self.type_mapper.map_type_name(&instruction.name)
                );

                Some(quote! {
                    [#(#disc_bytes),*] => {
                        // Deserialize from a cursor rather than `try_from_slice`:
                        // Anchor accounts are frequently allocated larger than
                        // the struct, and `try_from_slice` rejects any trailing
                        // bytes with "Not all bytes read".
                        let mut cursor = &data[8..];
                        let value = #type_name::deserialize(&mut cursor)?;
                        Ok(Box::new(value))
                    }
                })
            })
            .collect();

        let instruction_type_names: Vec<String> =
            idl.instructions.iter().map(|i| i.name.clone()).collect();

        Ok(quote! {
            /// Instruction decoder for #program_name.
            #[derive(Debug, Clone)]
            pub struct #decoder_name {
                program_id: Pubkey,
            }

            impl #decoder_name {
                /// Create a new decoder with the given program ID.
                pub fn new(program_id: Pubkey) -> Self {
                    Self { program_id }
                }
            }

            impl DecoderIdentity for #decoder_name {
                fn metadata(&self) -> DecoderMetadata {
                    DecoderMetadata::new(#program_name, self.program_id)
                }

                fn capabilities(&self) -> DecoderCapabilities {
                    DecoderCapabilities::default()
                        .with_instruction_types(vec![#(#instruction_type_names),*])
                }

                fn as_any(&self) -> &dyn Any {
                    self
                }
            }

            impl InstructionDecoder for #decoder_name {
                fn decode_instruction(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
                    if data.len() < 8 {
                        return Err(DecodeError::insufficient_data(8, data.len()));
                    }

                    let discriminator: [u8; 8] = data[..8].try_into().unwrap();

                    match discriminator {
                        #(#discriminator_arms)*
                        _ => Err(DecodeError::unknown_discriminator(&discriminator)),
                    }
                }
            }
        })
    }

    /// Generate derive attributes based on config.
    fn generate_derives(&self) -> TokenStream {
        let mut derives = vec![quote! { BorshDeserialize }];

        if self.config.derive_debug {
            derives.push(quote! { Debug });
        }
        if self.config.derive_clone {
            derives.push(quote! { Clone });
        }
        if self.config.derive_partial_eq {
            derives.push(quote! { PartialEq });
        }
        if self.config.derive_serde {
            derives.push(quote! { Serialize, Deserialize });
        }

        quote! { #[derive(#(#derives),*)] }
    }

    /// Generate doc comments from IDL docs.
    fn generate_docs(&self, docs: &[String]) -> TokenStream {
        if !self.config.include_docs || docs.is_empty() {
            return quote! {};
        }

        let doc_lines: Vec<_> = docs.iter().map(|s| quote! { #[doc = #s] }).collect();
        quote! { #(#doc_lines)* }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::IdlParser;

    #[test]
    fn test_generate_simple_account() {
        let json = r#"{
            "name": "test_program",
            "accounts": [
                {
                    "name": "Counter",
                    "discriminator": [1, 2, 3, 4, 5, 6, 7, 8],
                    "type": {
                        "kind": "struct",
                        "fields": [
                            {"name": "count", "type": "u64"}
                        ]
                    }
                }
            ]
        }"#;

        let idl = IdlParser::parse(json).unwrap();
        let generator = CodeGenerator::new(GeneratorConfig::default());
        let code = generator.generate(&idl).unwrap();

        // Parsing the output back is the assertion that matters: it proves the
        // generator emitted syntactically valid Rust, which a substring check
        // on the token stream does not.
        let code_str = code.to_string();
        let file = syn::parse_file(&code_str).expect("generated code must be valid Rust");

        let counter = find_struct(&file, "Counter").expect("Counter struct should be generated");
        assert!(
            has_field(counter, "count", "u64"),
            "Counter should carry `count: u64`"
        );
    }

    #[test]
    fn test_generate_instruction() {
        let json = r#"{
            "name": "test_program",
            "instructions": [
                {
                    "name": "initialize",
                    "discriminator": [0, 1, 2, 3, 4, 5, 6, 7],
                    "args": [
                        {"name": "amount", "type": "u64"},
                        {"name": "owner", "type": "publicKey"}
                    ]
                }
            ]
        }"#;

        let idl = IdlParser::parse(json).unwrap();
        let generator = CodeGenerator::new(GeneratorConfig::default());
        let code = generator.generate(&idl).unwrap();

        let code_str = code.to_string();
        let file = syn::parse_file(&code_str).expect("generated code must be valid Rust");

        let init = find_struct(&file, "InitializeInstruction")
            .expect("InitializeInstruction struct should be generated");
        assert!(has_field(init, "amount", "u64"), "expected `amount: u64`");
        assert!(
            has_field(init, "owner", "Pubkey"),
            "expected `owner: Pubkey`"
        );
    }

    /// Locate a generated struct by name in the parsed output.
    fn find_struct<'a>(file: &'a syn::File, name: &str) -> Option<&'a syn::ItemStruct> {
        file.items.iter().find_map(|item| match item {
            syn::Item::Struct(s) if s.ident == name => Some(s),
            _ => None,
        })
    }

    /// Check that a struct has a public field of the given name and type.
    fn has_field(item: &syn::ItemStruct, field: &str, ty: &str) -> bool {
        item.fields.iter().any(|f| {
            f.ident.as_ref().is_some_and(|i| i == field)
                && matches!(f.vis, syn::Visibility::Public(_))
                && {
                    let rendered = quote::quote!(#f).to_string();
                    rendered.replace(' ', "").ends_with(&format!(":{ty}"))
                }
        })
    }
}
