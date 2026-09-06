# ADR 001: Splitting identity from decoding

Status: accepted. Implemented in `crates/core/src/decoder.rs`.

## Context

A decoder has to do two unrelated things: say what it is, and decode bytes. The
registry needs the first before it ever asks for the second — to list what is
registered, to report which program a decoder serves, to answer "what handles
this?" — and an admin view needs it for decoders that are never invoked at all.

## Decision

Three traits rather than one.

`DecoderIdentity` carries metadata and capabilities, plus `as_any` for
downcasting. `AccountDecoder` and `InstructionDecoder` each require it and add
one method. `ProgramDecoder` is a blanket impl over anything implementing both,
so a decoder that covers a whole program gets that for free rather than writing a
third impl.

The split means a decoder can be listed, described and dispatched to without
being run, and a program that only produces accounts implements only the account
half.

## `can_decode` is part of the contract

Every decoder answers `can_decode` alongside `decode_account`, and the two must
agree: **a decoder must never successfully decode data its own `can_decode`
rejects.**

This is not a nicety. Solana accounts mostly carry no type tag, so length and
discriminator are all a decoder has to go on, and a decoder that accepts
"at least 165 bytes" will confidently decode a 752-byte pool from another program
and report a mint and an owner read out of the middle of it. Every decoder in
this workspace had that bug in some form; `crates/decoders/tests/decoder_discipline.rs`
now holds all of them to the rule against real accounts from other programs.

## Consequences

Downcasting is how a caller recovers the concrete type: `decode_account` returns
`Box<dyn DecodedEvent>`, and `TypedEvent::downcast_ref` gets back to the struct.
That costs a vtable hop and gives the registry a uniform return type, which is the
trade the registry exists to make.

`DecoderCapabilities` is optional with a default, so a minimal decoder implements
two methods. The generated decoders populate it from the IDL, which means a
generated decoder describes itself as well as a hand-written one.

## What was rejected

*One trait with both methods.* Programs that only produce accounts would have to
implement an instruction method that errors, and the registry would have no way
to tell a real instruction decoder from a stub.

*An enum of known programs.* Adding a program would mean editing the core, which
is the thing this design exists to avoid.
