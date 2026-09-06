# ADR 003: Reading fields in place

Status: accepted. Implemented in `crates/borsh-util/`.

## Context

Solana account layouts are fixed-offset C structs, not self-describing formats.
An SPL Token account is 165 bytes at known offsets; a Raydium AMM pool is 752.
Running full Borsh deserialization over one allocates a struct, copies every
field, and — in the common case where a caller wants one balance — throws almost
all of it away.

## Decision

`ZeroCopyReader` walks a `&[u8]` with a cursor, reading integers and pubkeys
directly out of the slice. Every read is bounds-checked and returns
`Result`; nothing indexes a slice without checking first.

The benchmark says an 82-byte mint decodes in 46 ns (1.65 GiB/s) and a 165-byte
token account in 59 ns (2.60 GiB/s). Those numbers are why this exists.

Borsh is still used where the data is genuinely Borsh — Anchor account bodies
behind their discriminator, and the generated decoders — because reimplementing
variable-length Borsh by hand would be a worse trade than the one this makes.

## The bounds checks are the point

A decoder is fed bytes from the chain, which means bytes an attacker can choose.
A length prefix inside a Token-2022 TLV extension is attacker-controlled, so
`read_string` checks the prefix against the remaining slice before it reads.
`crates/decoders/tests/fuzz_robustness.rs` runs the whole decoder set over
generated garbage under `catch_unwind` and asserts what comes back is an error —
never a panic, never a wrong answer.

## Consequences

Decoders read explicit offsets, so a wrong constant is a wrong answer rather
than a compile error. That is the cost. It is paid down by testing against real
mainnet accounts (`crates/decoders/tests/real_accounts.rs`) instead of against
data this repo made up: a fixture from mainnet fails loudly if an offset is
wrong, and a synthetic one agrees with whatever the decoder does.

## What was rejected

*`bytemuck` / `zerocopy` casts.* They need `repr(C)` types with no padding and
alignment guarantees the input slice does not have. Account data arrives at
whatever alignment the caller's buffer happens to have.

*Deserializing everything with `borsh::from_slice`.* Correct, and roughly an
order of magnitude slower for the read-one-field case that dominates.
