# System design

Five crates, one dependency. `account-decoder-sdk` is what a consumer names in
`Cargo.toml`; it re-exports the rest.

```
sdk ──┬── core          traits, registry, events, errors
      ├── decoders      SPL Token, Token-2022, System, Raydium AMM v4, Marinade
      ├── borsh-util    ZeroCopyReader, discriminators
      └── anchor-gen    IDL -> Rust decoder  (feature: anchor-codegen)

account-decoder (CLI)   decode <program> <file> | generate <idl>
```

`core` depends on nothing in the workspace. `decoders` depends on `core` and
`borsh-util`. `anchor-gen` depends on `core` and `borsh-util` and emits code that
depends on both. Nothing depends on `decoders`, which is what makes adding a
program a matter of adding a crate rather than editing one.

## Decoding, end to end

A caller has raw account bytes and the program that owns them.

```
(program_id, &[u8])
      │
      ▼
DecoderRegistry.decode_account         HashMap<Pubkey, Arc<dyn AccountDecoder>>
      │                                miss -> DecodeError::UnknownProgram
      ▼
decoder.decode_account(&[u8])          ZeroCopyReader over the slice,
      │                                every read bounds-checked
      ▼
Box<dyn DecodedEvent>                  .downcast_ref::<TokenAccount>()
```

Dispatch is by program id and never by sniffing the bytes, because the bytes are
often ambiguous — a Token-2022 mint with no extensions is byte-for-byte an SPL
Token mint. See [ADR 002](docs/adr/002-registry-pattern.md).

## The traits

`DecoderIdentity` is the supertrait: `metadata()`, `capabilities()` (defaulted),
`info()` (defaulted), `as_any()`. `AccountDecoder` adds `decode_account` and
`can_decode`; `InstructionDecoder` adds `decode_instruction`. `ProgramDecoder` is
a blanket impl over anything with both halves.

Identity is split out so the registry can list and describe a decoder without
running it. The `_with_key` / `_with_accounts` variants are defaulted to delegate
to the plain ones, so a decoder that wants the account key or the instruction's
account list overrides one method and the rest keeps working.

## `can_decode` and `decode_account` must agree

**A decoder must never successfully decode data its own `can_decode` rejects.**

Solana accounts carry no type tag, so length and discriminator are all a decoder
has. Every decoder in this workspace once violated the rule in some form:

- `TokenDecoder::decode_account` accepted anything `>= 165` bytes and read a
  token account out of the first 165 of it. A 752-byte Raydium pool decoded
  cleanly into a mint and an owner taken from the middle of the pool state.
- `SystemDecoder::can_decode` returned `true` unconditionally — it claimed every
  account on the chain.
- `Token2022Decoder` claimed an account on a single plausible TLV type byte,
  with no check that the rest of the TLV was well formed.

Fixed, and held by `crates/decoders/tests/decoder_discipline.rs`, which runs
every decoder against real accounts belonging to other programs. That test also
records the one genuine ambiguity: a plain 82-byte mint is claimed by both token
programs and nothing in the data can separate them.

## Reading bytes

`ZeroCopyReader` walks the slice with a cursor. Bounds are checked on every read
and failures are `Result`, never panics — decoder input is chain data, which is
to say attacker-chosen. `crates/decoders/tests/fuzz_robustness.rs` runs the
decoder set over generated garbage under `catch_unwind` and asserts an error
comes back.

Length prefixes inside Token-2022 TLV extensions are the sharp edge: they are
attacker-controlled, so `read_string` checks the prefix against what remains of
the slice before reading.

## Layouts

Offsets come from real accounts, not from documentation. Fixtures in
`crates/decoders/tests/fixtures/` are mainnet accounts fetched over RPC, and
`real_accounts.rs` decodes them and asserts against values that can be checked
independently — USDC's 6 decimals, its known mint authority.

Raydium AMM v4's layout was derived empirically against pool
`58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2` (SOL/USDC): the pubkey block
starts at 336, coin mint at 400 (wSOL), pc mint at 432 (USDC), lp mint at 464,
market at 528, market program at 560; total size 752; the fee block at 128.
`AmmFees::trade_fee_rate()` returns `Option<f64>` because a zero denominator is
representable on chain.

## Codegen

`account-decoder generate <idl.json>` parses an Anchor IDL and emits a module
with the account structs, the instruction enum, the discriminators and a decoder
implementing the same traits a hand-written one does. Output is committed;
generation is not part of the build. CI regenerates and diffs, running both
sides through rustfmt first because `prettyplease` and `cargo fmt` break lines
differently and that is not drift worth failing on.

Discriminators are `sha256("<namespace>:<name>")[..8]`, computed in exactly one
place — `AnchorDiscriminator::compute` in `borsh-util`, whose tests pin real
on-chain values (Marinade's `State` account and `deposit` instruction).

IDL type mapping: the `u8`–`u128`, `i8`–`i128`, `f32`/`f64`, `bool` and `string`
scalars map to their Rust equivalents, `publicKey` (and `pubkey`) to `Pubkey`,
`bytes` to `Vec<u8>`, and `option`, `vec`, `array` and `defined` map
structurally. Field names are converted to snake_case and escaped with `r#` if
they collide with a Rust keyword.

Marinade Finance ships as a generated decoder, which is the evidence the
pipeline produces something that runs rather than something that compiles.

## Errors

`DecodeError` distinguishes "this is not ours" from "this is broken":
`UnknownProgram` and `UnknownDiscriminator` say try something else;
`InsufficientData { expected, actual }`, `InvalidFormat`, `InvalidField`,
`DeserializationError` and `VersionMismatch` say the data did not fit;
`ClosedAccount` is the empty-data case. `is_wrong_program()` and
`is_data_corruption()` let an indexer decide whether to retry or to log and skip.

## Concurrency

The registry is a `HashMap` of `Arc<dyn Decoder>` and is immutable once built, so
it is shared across threads without a lock. Decoders are `Send + Sync` with no
mutable state — the trait bound enforces it. Decoded events are owned values.

## Measured

Benchmarks from `cargo bench --bench decode_path` on the development machine:

| Path | Time |
|---|---|
| Mint decode (82 bytes) | 46 ns — 1.65 GiB/s |
| Token account decode (165 bytes) | 59 ns — 2.60 GiB/s |
| Direct decoder call | 58 ns |
| Through the registry | 101 ns |
| Registry miss | 33 ns |

Dispatch costs about 44 ns. A consumer decoding many accounts from one program
should pull the decoder out once with `get_account` and skip the lookup.

Line coverage from `scripts/coverage.sh`: decoders 71.98%, borsh-util 74.84%,
core 67.04%, anchor-gen 75.93%; 71.53% across core, decoders and borsh-util
together.

## Decision records

- [001 — Splitting identity from decoding](docs/adr/001-decoder-trait-design.md)
- [002 — Dispatch by program id](docs/adr/002-registry-pattern.md)
- [003 — Reading fields in place](docs/adr/003-zero-copy-borsh.md)
- [004 — Generating decoders from IDLs](docs/adr/004-anchor-codegen.md)
- [005 — One crate to depend on](docs/adr/005-sdk-api-design.md)
