//! Decode a real mainnet account through the generated decoder.
//!
//! The fixture is the Marinade Finance `State` singleton
//! (`8szGkuLTAux9XMgZ2vtY39jVSowEcpBfFfD8hXSEqdGC`) as returned by
//! `getAccountInfo`, committed so this runs without network access.
//!
//! This is the end-to-end check on the code generator: the IDL came off chain,
//! the decoder was generated from it, this crate compiles it, and the bytes
//! below came off chain too. Nothing here is hand-written to match.

use account_decoder_core::{AccountDecoder, DecoderIdentity, TypedEvent};
use account_decoder_generated_marinade::{MarinadeFinanceAccountDecoder, State};
use base64::Engine;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const MARINADE_PROGRAM: &str = "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD";

fn load_fixture() -> (String, Vec<u8>) {
    let raw = include_str!("marinade_state.json");
    let json: serde_json::Value = serde_json::from_str(raw).expect("fixture is valid json");
    let value = &json["result"]["value"];
    let owner = value["owner"].as_str().expect("owner").to_string();
    let b64 = value["data"][0].as_str().expect("base64 data");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .expect("fixture data decodes");
    (owner, bytes)
}

#[test]
fn fixture_is_a_real_marinade_account() {
    let (owner, data) = load_fixture();
    assert_eq!(owner, MARINADE_PROGRAM, "fixture must be owned by Marinade");

    // Anchor's account discriminator is sha256("account:State")[..8]. This is
    // the value the generator derived, and it has to match the live account.
    assert_eq!(
        &data[..8],
        &[216, 146, 107, 94, 104, 75, 182, 177],
        "discriminator of the on-chain account"
    );
}

#[test]
fn generated_decoder_decodes_the_real_account() {
    let (_, data) = load_fixture();

    let decoder = MarinadeFinanceAccountDecoder::new(
        Pubkey::from_str(MARINADE_PROGRAM).expect("program id parses"),
    );

    let event = decoder
        .decode_account(&data)
        .expect("generated decoder should decode a real Marinade State account");

    assert_eq!(event.event_type(), "State");

    let state = event
        .downcast_ref::<State>()
        .expect("decoded event should be a State");

    // Structural checks against values that must hold for the live account.
    // msol_price is stored scaled by 2^32 and mSOL has never traded below SOL,
    // so the price is at or above 1.0.
    assert!(
        state.msol_price >= 1u64 << 32,
        "msol_price {} should be >= 1.0 (2^32)",
        state.msol_price
    );

    assert!(
        state.rent_exempt_for_token_acc > 0,
        "rent exemption should be non-zero"
    );

    // A u8 is always in range, so asserting that proved nothing. This is the
    // canonical PDA bump for Marinade's reserve, and a shifted struct would
    // read some other byte here.
    assert_eq!(
        state.reserve_bump_seed, 255,
        "the reserve PDA's canonical bump"
    );

    println!("decoded State from a real mainnet account:");
    println!("  msol_mint                 = {}", state.msol_mint);
    println!("  admin_authority           = {}", state.admin_authority);
    println!("  msol_price                = {}", state.msol_price);
    println!(
        "  rent_exempt_for_token_acc = {}",
        state.rent_exempt_for_token_acc
    );
    println!("  reserve_bump_seed         = {}", state.reserve_bump_seed);
}

#[test]
fn decoder_reports_its_identity() {
    let decoder =
        MarinadeFinanceAccountDecoder::new(Pubkey::from_str(MARINADE_PROGRAM).expect("program id"));
    let info = decoder.info();
    assert_eq!(info.metadata.program_name, "marinade_finance");
}

#[test]
fn truncated_account_data_is_an_error_not_a_panic() {
    let (_, data) = load_fixture();
    let decoder =
        MarinadeFinanceAccountDecoder::new(Pubkey::from_str(MARINADE_PROGRAM).expect("program id"));

    for len in [0usize, 1, 7, 8, 9, 64, 512] {
        let result = decoder.decode_account(&data[..len.min(data.len())]);
        assert!(
            result.is_err(),
            "truncated to {len} bytes should be an error"
        );
    }
}
