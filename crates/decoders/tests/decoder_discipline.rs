//! One rule that every decoder in the registry has to obey.
//!
//! `can_decode` is what a caller consults to decide whether a decoder claims
//! some bytes; `decode_account` is what actually runs. If the second is more
//! permissive than the first, the registry's dispatch and any "which decoder
//! handles this?" search become wrong in the worst way: a decoder returns a
//! confident, structurally valid answer read out of another program's account.
//!
//! That is not hypothetical. Every decoder here had this bug. SPL Token
//! accepted anything at least 165 bytes, so a 752-byte Raydium pool decoded as
//! a token account with a mint and owner read from the middle of it. The System
//! decoder accepted literally anything. Token-2022 checked a single byte.
//! Raydium accepted anything at least 752 bytes, so it claimed an 866-byte
//! Token-2022 mint.

use account_decoder_core::AccountDecoder;
use account_decoder_decoders::{RaydiumAmmDecoder, SystemDecoder, Token2022Decoder, TokenDecoder};

/// Every account decoder, with a name for failure messages.
fn decoders() -> Vec<(&'static str, Box<dyn AccountDecoder>)> {
    vec![
        ("spl-token", Box::new(TokenDecoder::new())),
        ("spl-token-2022", Box::new(Token2022Decoder::new())),
        ("system", Box::new(SystemDecoder::new())),
        ("raydium-amm-v4", Box::new(RaydiumAmmDecoder::new())),
    ]
}

#[test]
fn no_decoder_decodes_what_it_says_it_cannot() {
    // Lengths spanning every real account size in this workspace and the gaps
    // between them, so each decoder meets its neighbours' data.
    let lengths = [
        0usize, 1, 8, 80, 81, 82, 83, 100, 164, 165, 166, 200, 354, 355, 356, 500, 751, 752, 753,
        800, 866, 900, 2048,
    ];

    let mut violations = Vec::new();

    for (name, decoder) in decoders() {
        for len in lengths {
            // Two fills: zeroes, and a pattern that puts non-zero bytes in the
            // discriminating positions.
            for (fill_name, data) in [
                ("zeroes", vec![0u8; len]),
                (
                    "pattern",
                    (0..len).map(|i| (i % 251) as u8).collect::<Vec<u8>>(),
                ),
            ] {
                if !decoder.can_decode(&data) && decoder.decode_account(&data).is_ok() {
                    violations.push(format!(
                        "{name} decoded {len} bytes of {fill_name} that can_decode rejected"
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "decoders must not accept what they disclaim:\n  {}",
        violations.join("\n  ")
    );
}

/// Which decoders accept a fixture's bytes.
fn claimants(raw: &str) -> (usize, Vec<&'static str>) {
    use base64::Engine;

    let json: serde_json::Value = serde_json::from_str(raw).expect("valid json");
    let data = base64::engine::general_purpose::STANDARD
        .decode(json["result"]["value"]["data"][0].as_str().expect("data"))
        .expect("base64");

    let accepted = decoders()
        .iter()
        .filter(|(_, d)| d.decode_account(&data).is_ok())
        .map(|(name, _)| *name)
        .collect();

    (data.len(), accepted)
}

#[test]
fn real_accounts_are_claimed_only_by_decoders_that_could_own_them() {
    // What "the right decoder" means depends on whether the bytes are
    // distinguishable at all, and for one of these three they are not.
    let (len, accepted) = claimants(include_str!("fixtures/raydium_amm_sol_usdc.json"));
    println!("Raydium SOL/USDC pool  ({len:4} bytes) -> {accepted:?}");
    assert_eq!(accepted, vec!["raydium-amm-v4"]);

    let (len, accepted) = claimants(include_str!("fixtures/pyusd_mint_t22.json"));
    println!("PYUSD Token-2022 mint  ({len:4} bytes) -> {accepted:?}");
    assert_eq!(
        accepted,
        vec!["spl-token-2022"],
        "extensions make a Token-2022 mint unambiguous"
    );
}

#[test]
fn a_plain_mint_is_genuinely_ambiguous_between_the_two_token_programs() {
    // Not a defect to fix. A Token-2022 mint with no extensions has exactly the
    // same 82-byte layout as an SPL Token mint -- byte for byte, with no
    // discriminator. Nothing in the data distinguishes them, which is precisely
    // why the registry dispatches on the account's owner rather than sniffing
    // the bytes, and why `decode_account` takes a program id.
    let (len, accepted) = claimants(include_str!("fixtures/usdc_mint.json"));
    println!("USDC mint              ({len:4} bytes) -> {accepted:?}");

    assert_eq!(
        accepted,
        vec!["spl-token", "spl-token-2022"],
        "both token programs share this layout, so both accept it"
    );
    assert!(
        !accepted.contains(&"raydium-amm-v4") && !accepted.contains(&"system"),
        "but nothing unrelated may claim it"
    );
}
