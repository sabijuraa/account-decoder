# ADR 005: One crate to depend on

Status: accepted. Implemented in `crates/sdk/`.

## Context

The workspace is five crates. A caller who wants to decode an account should not
have to know that, or work out which of them holds `DecodeError`.

## Decision

`account-decoder-sdk` re-exports the public surface of the others and is the only
crate a consumer names in `Cargo.toml`. `default_registry()` returns a registry
with the built-ins already in it; `empty_registry()` and `registry_builder()`
are for a service that wants a narrower set.

Built-in decoders and Anchor codegen are features, so an application that only
needs the traits does not compile Raydium or `syn`.

## The prelude includes `DecoderIdentity`

It has to. `DecoderIdentity` is a supertrait of both decoder traits, so a caller
who imported only the prelude could not implement a decoder at all — the trait
they needed was not in scope. That was a real gap, found by writing the
extensibility test in `crates/sdk/tests/extensibility.rs` from the outside, and
it is the reason that test exists: it adds a program the workspace has never
heard of, using nothing but the published API.

## Doc examples are tests

Every example in the SDK docs compiles and runs under `cargo test --doc`. None
is marked `ignore` or `no_run`. An example that does not run is a claim nobody
checks, and several of these were wrong before they were made to run — wrong
error variants, wrong field names, missing imports.

## Consequences

Adding a type to the public API means adding it to the SDK re-exports, which is
a small tax and a useful checkpoint: it makes "is this public?" a decision rather
than an accident of `pub`.

A caller who wants one decoder and no registry can still depend on
`account-decoder-decoders` directly. The SDK is the front door, not a wall.

## What was rejected

*Publishing the five crates as the interface.* It exposes the internal split as
API, so moving a type between crates becomes a breaking change for everyone.

*Folding everything into one crate.* The codegen pulls in `syn`, `quote` and
`prettyplease`; a runtime decoding path should not compile a parser generator.
