# ADR 004: Generating decoders from IDLs

Status: accepted. Implemented in `crates/anchor-gen/`.

## Context

Most Solana programs worth decoding are Anchor programs, and every Anchor
program publishes an IDL with its account layouts and instruction arguments.
Hand-writing a decoder from one is transcription: copy the field list, compute
eight discriminator bytes, read the fields in order. It is also where the
mistakes are.

## Decision

`anchor-gen` parses an IDL and emits a Rust module: the account structs, the
instruction enum, the discriminators, and a decoder implementing the same traits
a hand-written decoder implements. Generation happens ahead of time via the
`account-decoder generate` CLI and the output is committed, so the build has no
codegen step and the generated code is reviewable in diffs.

CI regenerates from the IDL and diffs against the committed file, so the two
cannot drift. Both sides are run through rustfmt before the diff, because
`prettyplease` and `cargo fmt` disagree on line breaking and that disagreement is
not a drift worth failing on.

## Discriminators come from one place

`AnchorDiscriminator::compute` in `borsh-util` is the single implementation of
`sha256("<namespace>:<name>")[..8]`; `anchor-gen` calls it rather than repeating
the hash. Its tests pin real on-chain values — Marinade's `State` account
discriminator and the `deposit` instruction's — so a change to the hashing is a
test failure rather than a decoder that silently matches nothing.

## Consequences

Generated decoders populate `DecoderCapabilities` from the IDL, so a generated
decoder describes what it handles as well as a hand-written one does.

Marinade Finance ships as a generated decoder and is the proof that the pipeline
produces something that works, rather than something that compiles.

IDLs do not describe non-Anchor programs, so SPL Token, Token-2022, System and
Raydium AMM v4 are hand-written. That is the split: codegen for programs that
publish a schema, hands for programs that do not.

## What was rejected

*A `build.rs` that generates at compile time.* It hides the output, makes the
build depend on IDL files, and turns a codegen bug into a build failure in a
crate the user did not write.

*A derive macro over hand-declared structs.* The struct declaration is most of
the transcription, so it would remove the smaller half of the work.
