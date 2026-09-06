//! Throughput of the decode path.
//!
//! What is measured is the work an indexer actually repeats: taking bytes it
//! already holds and turning them into a typed value. Fixture construction is
//! outside the timed section, so the numbers describe decoding rather than
//! setup.
//!
//! The registry case is measured separately from the direct case because the
//! difference between them is the cost of dispatch, and that is the number that
//! decides whether the registry belongs on a hot path.

use account_decoder_core::{AccountDecoder, DecoderRegistry, InstructionDecoder};
use account_decoder_decoders::program_ids::TOKEN_PROGRAM_ID;
use account_decoder_decoders::{SystemDecoder, TokenDecoder};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use solana_sdk::pubkey::Pubkey;

/// A well-formed SPL token mint: 82 bytes, both authorities present.
fn mint_bytes() -> Vec<u8> {
    let mut data = Vec::with_capacity(82);
    data.extend_from_slice(&1u32.to_le_bytes()); // COption::Some
    data.extend_from_slice(&[7u8; 32]); // mint authority
    data.extend_from_slice(&1_000_000_000u64.to_le_bytes()); // supply
    data.push(9); // decimals
    data.push(1); // is_initialized
    data.extend_from_slice(&1u32.to_le_bytes()); // COption::Some
    data.extend_from_slice(&[8u8; 32]); // freeze authority
    data
}

/// A well-formed SPL token account: 165 bytes.
fn token_account_bytes() -> Vec<u8> {
    let mut data = Vec::with_capacity(165);
    data.extend_from_slice(&[1u8; 32]); // mint
    data.extend_from_slice(&[2u8; 32]); // owner
    data.extend_from_slice(&5_000u64.to_le_bytes()); // amount
    data.extend_from_slice(&0u32.to_le_bytes()); // delegate: None
    data.extend_from_slice(&[0u8; 32]);
    data.push(1); // state: initialized
    data.extend_from_slice(&0u32.to_le_bytes()); // is_native: None
    data.extend_from_slice(&0u64.to_le_bytes());
    data.extend_from_slice(&0u64.to_le_bytes()); // delegated_amount
    data.extend_from_slice(&0u32.to_le_bytes()); // close_authority: None
    data.extend_from_slice(&[0u8; 32]);
    data.resize(165, 0);
    data
}

/// A System transfer instruction: tag 2 plus a u64.
fn transfer_instruction_bytes() -> Vec<u8> {
    let mut data = 2u32.to_le_bytes().to_vec();
    data.extend_from_slice(&1_500_000u64.to_le_bytes());
    data
}

fn bench_accounts(c: &mut Criterion) {
    let decoder = TokenDecoder::new();
    let mut group = c.benchmark_group("account");

    for (name, data) in [
        ("mint", mint_bytes()),
        ("token_account", token_account_bytes()),
    ] {
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &data, |b, data| {
            b.iter(|| black_box(decoder.decode_account(black_box(data))).map(|_| ()))
        });
    }

    group.finish();
}

fn bench_instructions(c: &mut Criterion) {
    let decoder = SystemDecoder::new();
    let data = transfer_instruction_bytes();

    let mut group = c.benchmark_group("instruction");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("system_transfer", |b| {
        b.iter(|| black_box(decoder.decode_instruction(black_box(&data))).map(|_| ()))
    });
    group.finish();
}

/// Direct call against registry dispatch: the delta is what lookup costs.
fn bench_dispatch(c: &mut Criterion) {
    let data = mint_bytes();
    let direct = TokenDecoder::new();

    let mut registry = DecoderRegistry::new();
    registry.register_account(Box::new(TokenDecoder::new()));

    let mut group = c.benchmark_group("dispatch");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("direct", |b| {
        b.iter(|| black_box(direct.decode_account(black_box(&data))).map(|_| ()))
    });

    group.bench_function("through_registry", |b| {
        b.iter(|| {
            black_box(registry.decode_account(black_box(&TOKEN_PROGRAM_ID), black_box(&data)))
                .map(|_| ())
        })
    });

    // A program the registry has never heard of: the cost of saying no, which
    // an indexer pays for every account belonging to a program it does not
    // decode -- in practice, most of them.
    let unknown = Pubkey::new_from_array([42u8; 32]);
    group.bench_function("registry_miss", |b| {
        b.iter(|| {
            black_box(registry.decode_account(black_box(&unknown), black_box(&data))).map(|_| ())
        })
    });

    group.finish();
}

/// Rejecting bad input has to be cheap too, or malformed data becomes a lever.
fn bench_rejection(c: &mut Criterion) {
    let decoder = TokenDecoder::new();
    let truncated = vec![0u8; 40];

    c.bench_function("reject_truncated_account", |b| {
        b.iter(|| black_box(decoder.decode_account(black_box(&truncated))).is_err())
    });
}

criterion_group!(
    benches,
    bench_accounts,
    bench_instructions,
    bench_dispatch,
    bench_rejection
);
criterion_main!(benches);
