//! The `decode` subcommand.
//!
//! Turns bytes a caller already has -- or can fetch -- into a printed structured
//! value, using the same registry the SDK exposes. It exists so that "what is in
//! this account?" is answerable without writing a program.

use std::path::Path;

use account_decoder_core::{DecoderRegistry, TypedEvent};
use account_decoder_decoders::{AmmInfo, Mint, TokenAccount};
use account_decoder_sdk::default_registry;
use anyhow::{bail, Context, Result};
use base64::Engine;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::DecodeArgs;

/// Run the subcommand.
pub fn run(args: DecodeArgs) -> Result<()> {
    let data = load_data(&args)?;
    if data.is_empty() {
        bail!("no data to decode");
    }

    println!("{} bytes", data.len());

    let registry = default_registry();

    match &args.program {
        Some(program) => {
            let program_id = Pubkey::from_str(program)
                .with_context(|| format!("{program} is not a base58 pubkey"))?;
            decode_with(&registry, &program_id, &data, &args)
        }
        None => try_every_decoder(&registry, &data, &args),
    }
}

/// Decode against one named program.
fn decode_with(
    registry: &DecoderRegistry,
    program_id: &Pubkey,
    data: &[u8],
    args: &DecodeArgs,
) -> Result<()> {
    let decoded = if args.instruction {
        registry.decode_instruction(program_id, data)
    } else {
        registry.decode_account(program_id, data)
    };

    match decoded {
        Ok(event) => {
            print_event(event.as_ref(), args.raw);
            Ok(())
        }
        Err(e) => {
            // A decode failure is the expected answer for the wrong program, so
            // it is reported rather than dressed up as a crash.
            bail!("no decoder for {program_id} accepted this data: {e}")
        }
    }
}

/// With no program given, report which decoders accept the data.
///
/// Useful when the caller has bytes and no idea what produced them, which is
/// most of the time someone reaches for a tool like this.
fn try_every_decoder(registry: &DecoderRegistry, data: &[u8], args: &DecodeArgs) -> Result<()> {
    println!("no --program given; trying every registered decoder\n");

    let mut matched = 0;
    let candidates = if args.instruction {
        registry.list_instruction_decoders()
    } else {
        registry.list_account_decoders()
    };

    for info in candidates {
        let program_id = info.metadata.program_id;
        let attempt = if args.instruction {
            registry.decode_instruction(&program_id, data)
        } else {
            registry.decode_account(&program_id, data)
        };

        match attempt {
            Ok(event) => {
                matched += 1;
                println!("{} ({})", info.metadata.program_name, program_id);
                print_event(event.as_ref(), args.raw);
                println!();
            }
            Err(e) => {
                println!("{:24} declined: {e}", info.metadata.program_name);
            }
        }
    }

    if matched == 0 {
        bail!("no registered decoder accepted this data");
    }
    Ok(())
}

/// Print a decoded event, in summary or in full.
fn print_event(event: &dyn account_decoder_core::DecodedEvent, raw: bool) {
    println!("  type    : {}", event.event_type());
    println!("  program : {}", event.program_name());
    println!("  kind    : {:?}", event.event_kind());

    if raw {
        println!("  value   : {event:#?}");
        return;
    }

    // A handful of well-known shapes get a readable summary; everything else
    // falls back to the debug form rather than printing nothing.
    if let Some(mint) = event.downcast_ref::<Mint>() {
        println!("  decimals: {}", mint.decimals);
        println!("  supply  : {}", mint.supply);
        println!("  mint authority  : {:?}", mint.mint_authority);
        println!("  freeze authority: {:?}", mint.freeze_authority);
    } else if let Some(account) = event.downcast_ref::<TokenAccount>() {
        println!("  mint    : {}", account.mint);
        println!("  owner   : {}", account.owner);
        println!("  amount  : {}", account.amount);
    } else if let Some(pool) = event.downcast_ref::<AmmInfo>() {
        println!("  base    : {} ({} decimals)", pool.coin_mint, pool.coin_decimals);
        println!("  quote   : {} ({} decimals)", pool.pc_mint, pool.pc_decimals);
        println!("  lp mint : {}", pool.lp_mint);
        println!("  market  : {}", pool.market);
        println!("  trade fee: {:?}", pool.fees.trade_fee_rate());
    } else {
        println!("  value   : {event:#?}");
    }
}

/// Get the bytes, from whichever source was named.
fn load_data(args: &DecodeArgs) -> Result<Vec<u8>> {
    if let Some(encoded) = &args.data {
        return decode_base64(encoded.trim());
    }

    if let Some(path) = &args.file {
        return load_file(path);
    }

    bail!("one of --data or --file is required")
}

/// Read from a file, accepting either a `getAccountInfo` response or raw base64.
fn load_file(path: &Path) -> Result<Vec<u8>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(encoded) = account_data_from_json(&json) {
            return decode_base64(&encoded);
        }
        bail!("{} is JSON but has no result.value.data", path.display());
    }

    decode_base64(text.trim())
}

/// Pull `result.value.data[0]` out of a `getAccountInfo` response.
fn account_data_from_json(json: &serde_json::Value) -> Option<String> {
    json.get("result")?
        .get("value")?
        .get("data")?
        .get(0)?
        .as_str()
        .map(str::to_string)
}

fn decode_base64(encoded: &str) -> Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .context("data is not valid base64")
}
