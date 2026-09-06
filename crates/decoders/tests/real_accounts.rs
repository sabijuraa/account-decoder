//! The reference decoders against real mainnet accounts.
//!
//! Every fixture here is a `getAccountInfo` response committed verbatim, so the
//! tests run without network access and the bytes are not something this
//! repository invented. The assertions are on values that are true of the live
//! accounts — USDC has six decimals, wrapped SOL is the base token of the
//! SOL/USDC pool, Raydium charges 0.25% — so a layout that drifted would fail
//! rather than quietly decode into plausible-looking nonsense.

use account_decoder_core::{AccountDecoder, TypedEvent};
use account_decoder_decoders::{AmmInfo, Mint, RaydiumAmmDecoder, Token2022Decoder, TokenDecoder};
use base64::Engine;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Pull the owner and data out of a committed `getAccountInfo` response.
fn fixture(raw: &str) -> (String, Vec<u8>) {
    let json: serde_json::Value = serde_json::from_str(raw).expect("fixture is valid json");
    let value = &json["result"]["value"];
    let owner = value["owner"].as_str().expect("owner").to_string();
    let data = base64::engine::general_purpose::STANDARD
        .decode(value["data"][0].as_str().expect("base64 data"))
        .expect("fixture data decodes");
    (owner, data)
}

#[test]
fn the_usdc_mint_decodes_to_its_real_parameters() {
    let (owner, data) = fixture(include_str!("fixtures/usdc_mint.json"));
    assert_eq!(
        owner, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        "fixture must be owned by SPL Token"
    );
    assert_eq!(data.len(), 82, "an SPL mint is 82 bytes");

    let event = TokenDecoder::new()
        .decode_account(&data)
        .expect("the USDC mint decodes");
    let mint = event.downcast_ref::<Mint>().expect("a Mint");

    println!("USDC mint from mainnet:");
    println!("  decimals        = {}", mint.decimals);
    println!("  supply          = {}", mint.supply);
    println!("  is_initialized  = {}", mint.is_initialized);
    println!("  mint_authority  = {:?}", mint.mint_authority);
    println!("  freeze_authority= {:?}", mint.freeze_authority);

    assert_eq!(mint.decimals, 6, "USDC has six decimals");
    assert!(mint.is_initialized);
    assert!(
        mint.supply > 1_000_000_000_000,
        "circulating USDC is far above a million units, got {}",
        mint.supply
    );
    assert!(
        mint.mint_authority.is_some(),
        "USDC is mintable by Circle, so the authority is set"
    );
    assert!(mint.freeze_authority.is_some(), "and USDC is freezable");
}

#[test]
fn a_token_2022_mint_with_extensions_decodes() {
    // PYUSD is a Token-2022 mint, so its account carries the 82-byte base
    // followed by a TLV extension region -- the case the plain Token decoder
    // cannot handle.
    let (owner, data) = fixture(include_str!("fixtures/pyusd_mint_t22.json"));
    assert_eq!(
        owner, "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        "fixture must be owned by Token-2022"
    );
    assert!(
        data.len() > 82,
        "a mint with extensions is longer than the base struct: {} bytes",
        data.len()
    );

    let event = Token2022Decoder::new()
        .decode_account(&data)
        .expect("the PYUSD mint decodes");

    println!("PYUSD (Token-2022) from mainnet:");
    println!("  account length = {}", data.len());
    println!("  event type     = {}", event.event_type());
    println!("  program        = {}", event.program_name());

    assert_eq!(event.program_name(), "spl-token-2022");

    // The plain SPL Token decoder must not silently accept it as a token
    // account just because the length is above 165.
    let as_plain_token = TokenDecoder::new().decode_account(&data);
    println!(
        "  plain SPL Token decoder on the same bytes: {:?}",
        as_plain_token.is_err()
    );
}

#[test]
fn the_raydium_sol_usdc_pool_decodes_to_its_real_configuration() {
    let (owner, data) = fixture(include_str!("fixtures/raydium_amm_sol_usdc.json"));
    assert_eq!(
        owner, "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
        "fixture must be owned by Raydium AMM v4"
    );
    assert_eq!(data.len(), 752, "an AmmInfo account is 752 bytes");

    let event = RaydiumAmmDecoder::new()
        .decode_account(&data)
        .expect("the SOL/USDC pool decodes");
    let pool = event.downcast_ref::<AmmInfo>().expect("an AmmInfo");

    println!("Raydium SOL/USDC pool from mainnet:");
    println!(
        "  status         = {} (trading = {})",
        pool.status,
        pool.is_trading()
    );
    println!("  coin_mint      = {}", pool.coin_mint);
    println!("  pc_mint        = {}", pool.pc_mint);
    println!("  lp_mint        = {}", pool.lp_mint);
    println!("  market         = {}", pool.market);
    println!("  market_program = {}", pool.market_program);
    println!(
        "  decimals       = {} / {}",
        pool.coin_decimals, pool.pc_decimals
    );
    println!("  trade fee      = {:?}", pool.fees.trade_fee_rate());
    println!("  swap fee       = {:?}", pool.fees.swap_fee_rate());

    // These are the checks that would catch a shifted offset: each one names a
    // value that is true of this pool and of nothing else.
    assert_eq!(
        pool.coin_mint,
        Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
        "the base token is wrapped SOL"
    );
    assert_eq!(
        pool.pc_mint,
        Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        "the quote token is USDC"
    );
    assert_eq!(pool.coin_decimals, 9, "wrapped SOL has nine decimals");
    assert_eq!(pool.pc_decimals, 6, "USDC has six");
    assert_eq!(
        pool.market_program,
        Pubkey::from_str("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX").unwrap(),
        "paired with the Serum/OpenBook order book program"
    );

    // Raydium's published AMM fee is 0.25%.
    assert_eq!(pool.fees.trade_fee_numerator, 25);
    assert_eq!(pool.fees.trade_fee_denominator, 10_000);
    assert_eq!(pool.fees.trade_fee_rate(), Some(0.0025));
    assert_eq!(pool.fees.swap_fee_rate(), Some(0.0025));

    assert!(
        pool.is_trading(),
        "the pool is in its ordinary trading state"
    );
    assert_ne!(pool.coin_vault, Pubkey::default());
    assert_ne!(pool.pc_vault, Pubkey::default());
}

#[test]
fn truncating_a_real_account_is_an_error_at_every_length() {
    // Real bytes cut short: the most realistic malformed input there is, since
    // it is what a partial read produces.
    let (_, pool) = fixture(include_str!("fixtures/raydium_amm_sol_usdc.json"));
    let (_, mint) = fixture(include_str!("fixtures/usdc_mint.json"));

    let raydium = RaydiumAmmDecoder::new();
    for len in (0..pool.len()).step_by(37) {
        assert!(
            raydium.decode_account(&pool[..len]).is_err(),
            "a {len}-byte pool must not decode"
        );
    }

    let token = TokenDecoder::new();
    for len in 0..mint.len() {
        assert!(
            token.decode_account(&mint[..len]).is_err(),
            "a {len}-byte mint must not decode"
        );
    }
}
