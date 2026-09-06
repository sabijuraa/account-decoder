//! Adding support for a new program without touching the core.
//!
//! The claim is architectural rather than behavioural: a third party can support
//! a program this workspace has never heard of by writing two trait impls in
//! their own crate and registering the result. Nothing in `core`, `decoders` or
//! the registry changes, and no enum anywhere gains a variant.
//!
//! This file lives outside every decoder crate on purpose. It can reach only the
//! public API, so if extending the system required a private hook it would not
//! compile.

use std::any::Any;

use account_decoder_sdk::prelude::*;
use account_decoder_sdk::{default_registry, empty_registry};

/// A program invented for this test. Nothing in the workspace knows about it.
const ESCROW_PROGRAM: Pubkey = Pubkey::new_from_array([0xEE; 32]);

/// Its account layout: an 8-byte tag, an amount, and a beneficiary.
const ESCROW_TAG: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

#[derive(Debug, PartialEq)]
struct Escrow {
    amount: u64,
    beneficiary: Pubkey,
}

impl DecodedEvent for Escrow {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }
    fn event_type(&self) -> &'static str {
        "Escrow"
    }
    fn program_name(&self) -> &'static str {
        "escrow"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn encoded_size(&self) -> Option<usize> {
        Some(48)
    }
}

#[derive(Debug)]
struct EscrowDecoder;

impl DecoderIdentity for EscrowDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("escrow", ESCROW_PROGRAM)
            .with_description("an escrow account, invented for this test")
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities::default().with_account_types(vec!["Escrow"])
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AccountDecoder for EscrowDecoder {
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        if data.len() != 48 {
            return Err(DecodeError::insufficient_data(48, data.len()));
        }
        if data[..8] != ESCROW_TAG {
            return Err(DecodeError::unknown_discriminator(&data[..8]));
        }

        let mut reader = ZeroCopyReader::new(&data[8..]);
        let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
        let beneficiary = Pubkey::new_from_array(
            *reader
                .read_fixed::<32>()
                .map_err(DecodeError::deserialization)?,
        );

        Ok(Box::new(Escrow {
            amount,
            beneficiary,
        }))
    }

    fn can_decode(&self, data: &[u8]) -> bool {
        data.len() == 48 && data[..8] == ESCROW_TAG
    }
}

/// A well-formed escrow account.
fn escrow_bytes(amount: u64, beneficiary: [u8; 32]) -> Vec<u8> {
    let mut data = ESCROW_TAG.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&beneficiary);
    data
}

#[test]
fn a_new_program_works_with_only_the_public_api() {
    let mut registry = empty_registry();
    registry.register_account(Box::new(EscrowDecoder));

    let data = escrow_bytes(9_000, [7u8; 32]);
    let event = registry
        .decode_account(&ESCROW_PROGRAM, &data)
        .expect("the new decoder is dispatched to");

    assert_eq!(event.event_type(), "Escrow");
    assert_eq!(event.program_name(), "escrow");
    assert_eq!(
        event.downcast_ref::<Escrow>(),
        Some(&Escrow {
            amount: 9_000,
            beneficiary: Pubkey::new_from_array([7u8; 32]),
        })
    );
}

#[test]
fn the_new_decoder_sits_alongside_the_built_in_ones() {
    // Extending a registry that already has decoders in it, and confirming
    // neither side interferes with the other.
    use account_decoder_sdk::program_ids::TOKEN_PROGRAM_ID;

    let mut registry = default_registry();
    let built_in = registry.account_decoder_count();
    registry.register_account(Box::new(EscrowDecoder));

    assert_eq!(registry.account_decoder_count(), built_in + 1);
    assert!(registry.has_account_decoder(&ESCROW_PROGRAM));
    assert!(registry.has_account_decoder(&TOKEN_PROGRAM_ID));

    let data = escrow_bytes(1, [1u8; 32]);
    assert!(registry.decode_account(&ESCROW_PROGRAM, &data).is_ok());

    // The built-in decoders are untouched by its arrival.
    let mut mint = Vec::new();
    mint.extend_from_slice(&0u32.to_le_bytes());
    mint.extend_from_slice(&[0u8; 32]);
    mint.extend_from_slice(&500u64.to_le_bytes());
    mint.push(6);
    mint.push(1);
    mint.extend_from_slice(&0u32.to_le_bytes());
    mint.extend_from_slice(&[0u8; 32]);
    assert!(registry.decode_account(&TOKEN_PROGRAM_ID, &mint).is_ok());

    // And neither claims the other's data.
    assert!(registry.decode_account(&ESCROW_PROGRAM, &mint).is_err());
    assert!(registry.decode_account(&TOKEN_PROGRAM_ID, &data).is_err());
}

#[test]
fn the_new_decoder_describes_itself_to_the_registry() {
    // Discovery works for a decoder the workspace has never seen, because the
    // metadata comes from the trait rather than a table in core.
    let mut registry = empty_registry();
    registry.register_account(Box::new(EscrowDecoder));

    let listed = registry.list_account_decoders();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].metadata.program_name, "escrow");
    assert_eq!(listed[0].metadata.program_id, ESCROW_PROGRAM);
    assert_eq!(listed[0].capabilities.account_types, vec!["Escrow"]);
    assert!(listed[0].metadata.description.is_some());
}

#[test]
fn a_new_decoder_is_held_to_the_same_robustness_rules() {
    // Extensibility does not mean the extension gets to panic. The malformed
    // inputs the built-in decoders survive go through this one too.
    let decoder = EscrowDecoder;

    for len in [0usize, 7, 8, 47, 48, 49, 100, 1024] {
        let data = vec![0u8; len];
        assert!(
            !decoder.can_decode(&data),
            "all-zero data has the wrong tag, so it must not be claimed"
        );
        assert!(
            decoder.decode_account(&data).is_err(),
            "{len} bytes of zeroes must not decode"
        );
    }

    // The right length with the wrong tag is a discriminator error, not a
    // successful decode of someone else's account.
    let mut wrong_tag = vec![0xAAu8; 48];
    wrong_tag[..8].copy_from_slice(&[9, 9, 9, 9, 9, 9, 9, 9]);
    assert!(decoder.decode_account(&wrong_tag).is_err());
}
