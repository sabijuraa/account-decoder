//! Robustness: malformed input must produce an error, never a panic.
//!
//! Every decoder in this workspace is fed bytes it did not expect — truncated,
//! oversized, random, and adversarially shaped — and the only acceptable
//! outcomes are a `DecodeError` or a successful decode. A panic in a decoder is
//! a denial of service for whatever indexer embedded it, because the bytes come
//! from the chain and anyone can put bytes on the chain.
//!
//! These are property tests rather than a fuzzer binary so they run in CI on
//! every push with no extra tooling. `proptest` generates the inputs; the
//! assertion is the absence of a panic, which `catch_unwind` makes explicit
//! rather than relying on the harness noticing.

use std::panic::{catch_unwind, AssertUnwindSafe};

use account_decoder_borsh_util::{read_discriminator, ZeroCopyReader};
use account_decoder_core::{AccountDecoder, InstructionDecoder};
use account_decoder_decoders::{SystemDecoder, Token2022Decoder, TokenDecoder};
use proptest::prelude::*;

/// Run `f` and report whether it panicked, with the input for the failure message.
fn survives<F: FnOnce()>(what: &str, input: &[u8], f: F) {
    let result = catch_unwind(AssertUnwindSafe(f));
    assert!(
        result.is_ok(),
        "{what} panicked on {} bytes: {:02x?}",
        input.len(),
        &input[..input.len().min(64)]
    );
}

/// Every decoder, applied to the same bytes both ways.
fn decode_everything(data: &[u8]) {
    let token = TokenDecoder::new();
    let token_2022 = Token2022Decoder::new();
    let system = SystemDecoder::new();

    survives("TokenDecoder::decode_account", data, || {
        let _ = token.decode_account(data);
    });
    survives("TokenDecoder::decode_instruction", data, || {
        let _ = token.decode_instruction(data);
    });
    survives("Token2022Decoder::decode_account", data, || {
        let _ = token_2022.decode_account(data);
    });
    survives("SystemDecoder::decode_account", data, || {
        let _ = system.decode_account(data);
    });
    survives("SystemDecoder::decode_instruction", data, || {
        let _ = system.decode_instruction(data);
    });
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// Arbitrary bytes of arbitrary length.
    #[test]
    fn decoders_survive_arbitrary_bytes(data in prop::collection::vec(any::<u8>(), 0..512)) {
        decode_everything(&data);
    }

    /// Lengths clustered on the boundaries the decoders branch on: the SPL
    /// mint (82), token account (165) and multisig (355) sizes, and the bytes
    /// either side of each.
    #[test]
    fn decoders_survive_sizes_near_every_boundary(
        len in prop::sample::select(vec![
            0usize, 1, 3, 4, 7, 8, 9,
            81, 82, 83,
            164, 165, 166,
            354, 355, 356,
        ]),
        fill in any::<u8>(),
    ) {
        decode_everything(&vec![fill; len]);
    }

    /// A valid-looking token account whose trailing bytes are noise. The size
    /// check passes, so the decoder commits to a layout and then meets garbage.
    #[test]
    fn a_well_sized_account_full_of_noise_is_handled(
        noise in prop::collection::vec(any::<u8>(), 165..400)
    ) {
        decode_everything(&noise);
    }

    /// The reader is the shared primitive under every decoder, so it is fuzzed
    /// directly: a caller asking for more than remains must be refused, not
    /// allowed to read past the buffer.
    #[test]
    fn the_zero_copy_reader_never_reads_past_its_buffer(
        data in prop::collection::vec(any::<u8>(), 0..256),
        requests in prop::collection::vec(0usize..64, 0..32),
    ) {
        survives("ZeroCopyReader", &data, || {
            let mut reader = ZeroCopyReader::new(&data);
            for n in &requests {
                // Refusing is the correct outcome once the buffer is spent; a
                // success must hand back exactly what was asked for.
                if let Ok(slice) = reader.read_bytes(*n) {
                    assert_eq!(slice.len(), *n);
                }
            }
        });
    }

    /// A borsh string length is attacker-controlled and can claim more bytes
    /// than exist. Reading one must not allocate on that claim.
    #[test]
    fn a_string_length_larger_than_the_buffer_is_refused(
        claimed in any::<u32>(),
        tail in prop::collection::vec(any::<u8>(), 0..32),
    ) {
        let mut data = claimed.to_le_bytes().to_vec();
        data.extend_from_slice(&tail);

        survives("ZeroCopyReader::read_string", &data, || {
            let mut reader = ZeroCopyReader::new(&data);
            if let Ok(s) = reader.read_string() {
                assert!(s.len() <= tail.len(), "a string cannot exceed its buffer");
            }
        });
    }

    /// Discriminator reads on short buffers.
    #[test]
    fn reading_a_discriminator_from_a_short_buffer_is_an_error(
        data in prop::collection::vec(any::<u8>(), 0..16)
    ) {
        survives("read_discriminator", &data, || {
            let result = read_discriminator::<8>(&data);
            if data.len() < 8 {
                assert!(result.is_err(), "8 bytes cannot come from {} ", data.len());
            }
        });
    }
}

/// The specific shapes worth pinning rather than sampling.
#[test]
fn the_pathological_inputs_are_all_errors_not_panics() {
    let token = TokenDecoder::new();
    let system = SystemDecoder::new();

    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("one byte", vec![0xff]),
        ("all zeroes at mint size", vec![0u8; 82]),
        ("all ones at mint size", vec![0xffu8; 82]),
        ("all ones at token account size", vec![0xffu8; 165]),
        // A COption tag of 0xffffffff is neither 0 nor 1.
        ("invalid coption tag", {
            let mut v = vec![0xffu8; 4];
            v.extend_from_slice(&[0u8; 78]);
            v
        }),
        // A System instruction tag far past the last real one.
        ("unknown system tag", 0xdead_beefu32.to_le_bytes().to_vec()),
        // A huge account that is otherwise plausible.
        ("oversized", vec![0u8; 100_000]),
    ];

    for (name, data) in cases {
        let account = catch_unwind(AssertUnwindSafe(|| token.decode_account(&data)));
        assert!(account.is_ok(), "TokenDecoder panicked on {name}");

        let instruction = catch_unwind(AssertUnwindSafe(|| system.decode_instruction(&data)));
        assert!(instruction.is_ok(), "SystemDecoder panicked on {name}");

        println!("{name:32} -> handled ({} bytes)", data.len());
    }
}
