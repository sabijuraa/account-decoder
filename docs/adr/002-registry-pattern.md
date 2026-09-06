# ADR 002: Dispatch by program id

Status: accepted. Implemented in `crates/core/src/registry.rs`.

## Context

A caller with an account has two things: the bytes, and the program that owns
them. Only one of those reliably identifies the layout.

## Decision

The registry is a map from program id to decoder, with separate maps for account
and instruction decoders since a program may have one and not the other. Lookup
is by program id; the bytes are never sniffed to choose a decoder.

Dispatch costs about 44 ns over calling a decoder directly (`cargo bench
--bench decode_path`), which is the number to know before putting it on a hot
path. A caller decoding many accounts from one program should take the decoder
out once with `get_account` and skip the lookup.

## Why not sniff the bytes

Because the bytes are frequently ambiguous, and the ambiguity is not resolvable.
A Token-2022 mint with no extensions is byte-for-byte identical to an SPL Token
mint: same 82 bytes, same fields, no discriminator. Nothing in the data
distinguishes them, and only the account's owner does.

The CLI does offer a "try everything" mode for when a caller genuinely has bytes
and no program id, but it reports *which* decoders accepted them rather than
picking one, because picking one would be a guess.

## Consequences

An unregistered program is `DecodeError::UnknownProgram` rather than a failed
parse, which distinguishes "we do not support this" from "this data is wrong".

The registry holds `Arc<dyn AccountDecoder>`, so decoders are shared rather than
cloned per lookup and the registry is cheap to consult concurrently.

`RegistryBuilder` exists for the common case of assembling a fixed set at
startup; `default_registry()` gives the built-ins, and `empty_registry()` is for
a service that wants only its own.

## What was rejected

*A global registry.* Two components in one process may legitimately want
different decoder sets, and a global makes that impossible to arrange.

*Ordering decoders by priority and taking the first that accepts.* It makes the
result depend on registration order, which is exactly the kind of thing that is
correct in testing and wrong in production.
