# account-decoder

Turns raw Solana account and instruction bytes into typed Rust values. Write a
decoder by hand for a program with a fixed layout, or generate one from an Anchor
IDL; either way it plugs into the same registry, and nothing in the core changes
when you add one.

The decoders are checked against real mainnet accounts committed to this
repository — the USDC mint, a PayPal USD Token-2022 mint with its extensions, the
Raydium SOL/USDC pool, and Marinade's staking state through a generated decoder.
See [Verified](#verified).

```
raw bytes + program id ─▶ registry ─▶ that program's decoder ─▶ typed event
                                            │
                                    zero-copy borsh reader
```

## Why the layout question is the hard part

Most Solana accounts carry no type tag. An SPL token account is 165 bytes of
struct with nothing at the front saying so, and a Raydium pool is 752 bytes with
nothing saying that either. A decoder's real job is not parsing — it is knowing
when *not* to parse, because bytes it was never meant to see will happily produce
a mint address and an owner read out of the middle of somebody else's account.

So each decoder here states exactly which lengths and discriminators it accepts,
`can_decode` and `decode_account` agree on that, and a test holds every decoder
to it against real accounts from other programs.

## Verified

`cargo test --workspace` runs 102 tests with no network access: the fixtures are
committed `getAccountInfo` responses, so what is being decoded is what was on
chain, not something this repository invented.

**Real accounts decode to their real values.** The USDC mint comes back with six
decimals, a supply of 8,294,306,733,992,283 and Circle's mint and freeze
authorities. The Raydium SOL/USDC pool comes back with wrapped SOL as its base
token, USDC as its quote, decimals of 9 and 6, the OpenBook market it is paired
with, and Raydium's published 0.25% trade fee — every one a value that is true of
that pool and nothing else, so a shifted offset fails rather than decoding into
plausible nonsense.

**Codegen is proven end to end.** Marinade's IDL was fetched from its on-chain
IDL account, a decoder was generated from it, that decoder is a normal workspace
member so it has to compile on every build, and its test decodes the live
`State` singleton. The result names `mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So`
as the mSOL mint, which only happens if the generated struct is byte-correct. CI
regenerates the file and diffs it, so the generator cannot drift from what is
committed.

**Nothing panics on malformed input.** Seven property tests feed arbitrary,
truncated and adversarially-sized bytes to every decoder and to the zero-copy
reader, with `catch_unwind` making the absence of a panic an explicit assertion
rather than something the harness might notice. Attacker-controlled lengths — a
borsh string claiming four gigabytes inside a forty-byte buffer — are refused
rather than allocated on.

**Decoders do not claim each other's accounts.** A cross-cutting test walks every
decoder over every length either side of every real account size, in two fill
patterns, and asserts none of them decodes what its own `can_decode` rejects.
Given the three real fixtures, the Raydium pool is claimed only by Raydium and
the Token-2022 mint only by Token-2022. A plain 82-byte mint is claimed by both
token programs, which is correct and documented: that layout is byte-identical
between them, which is exactly why dispatch is by program id.

**Extending it needs no changes to the core.** A test in the SDK crate — which
can reach only the public API — defines a program the workspace has never heard
of, implements two traits, registers it, and decodes it alongside the built-in
decoders without either interfering with the other.

**The decode path is measured.** On this machine:

| | time | throughput |
|---|---|---|
| SPL mint (82 B) | 46 ns | 1.65 GiB/s |
| SPL token account (165 B) | 59 ns | 2.60 GiB/s |
| System transfer instruction | 30 ns | |
| Direct call | 58 ns | |
| Through the registry | 101 ns | |
| Registry miss (unknown program) | 33 ns | |
| Rejecting a truncated account | 56 ns | |

Registry dispatch costs about 44 ns over a direct call, which is the number worth
knowing before putting it on a hot path.

**Coverage** on the crates that carry the correctness burden:

| | lines | covered |
|---|---|---|
| `decoders` | 1149 | **71.98%** |
| `borsh-util` | 457 | **74.84%** |
| `core` | 449 | **67.04%** |
| `anchor-gen` | 540 | **75.93%** |
| core + decoders + borsh-util | 2055 | **71.53%** |

Reproduce all of it:

```
cargo test --workspace          # 102 tests
cargo bench -p account-decoder-decoders --bench decode_path
scripts/coverage.sh summary
```

## Layout

| Crate | What it does |
|-------|--------------|
| `core` | The decoder traits, the registry, event and error types |
| `borsh-util` | Zero-copy reader, discriminators, Anchor discriminator derivation |
| `decoders` | SPL Token, Token-2022 with extensions, System, Raydium AMM v4 |
| `anchor-gen` | Anchor IDL parsing and decoder generation |
| `generated-marinade` | A generated decoder, kept as a workspace member so it must compile |
| `sdk` | The public API most callers want |
| `anchor-gen-cli` | The `account-decoder` binary: decode, and generate |

## Using it

```rust
use account_decoder_sdk::prelude::*;
use account_decoder_sdk::{default_registry, program_ids::TOKEN_PROGRAM_ID, Mint};

let registry = default_registry();
let event = registry.decode_account(&TOKEN_PROGRAM_ID, account_bytes)?;

println!("{} / {}", event.program_name(), event.event_type());
if let Some(mint) = event.downcast_ref::<Mint>() {
    println!("{} decimals, supply {}", mint.decimals, mint.supply);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Supporting a new program means two trait impls and a `register_account` call —
`crates/sdk/examples/custom_decoder.rs` is a worked example, and
`crates/sdk/tests/extensibility.rs` is the same thing as a test.

Every example in the documentation is compiled and run by `cargo test`. None are
marked `ignore`, so a signature change breaks the build rather than the docs.

## The CLI

```
# Decode with a known program.
account-decoder decode --file account.json \
  --program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA

# Or let it try every registered decoder and say which ones accept the bytes.
account-decoder decode --file account.json

# Generate a decoder from an Anchor IDL.
account-decoder generate path/to/idl.json -o src/generated/
```

`--file` takes a `getAccountInfo` response or a file of raw base64; `--data`
takes base64 directly. There is deliberately no option to fetch an account:
this project decodes bytes it is given and is not an RPC client.

## Documentation

- [SYSTEM_DESIGN.md](SYSTEM_DESIGN.md) — the trait model, the registry, the
  codegen pipeline and the zero-copy approach.
- [docs/adr](docs/adr) — the decisions and what they cost.
- `cargo doc --open` — the API, with examples that are known to compile.
