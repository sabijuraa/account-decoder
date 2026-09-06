//! System program decoder.
//!
//! Decodes native system accounts and nonce accounts.

use account_decoder_borsh_util::ZeroCopyReader;
use account_decoder_core::{
    AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities, DecoderIdentity, DecoderMetadata, EventKind, InstructionDecoder,
};
use solana_sdk::pubkey::Pubkey;
use std::any::Any;

use crate::program_ids::SYSTEM_PROGRAM_ID;

/// Size of a nonce account.
const NONCE_ACCOUNT_SIZE: usize = 80;

/// A basic system account (no special structure).
#[derive(Debug, Clone, PartialEq)]
pub struct SystemAccount {
    /// The account's lamport balance.
    pub lamports: u64,
    /// The raw data (usually empty for basic accounts).
    pub data: Vec<u8>,
}

impl DecodedEvent for SystemAccount {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "SystemAccount"
    }

    fn program_name(&self) -> &'static str {
        "system"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// State of a nonce account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NonceState {
    /// Nonce account is uninitialized.
    Uninitialized = 0,
    /// Nonce account is initialized and active.
    Initialized = 1,
}

impl NonceState {
    fn from_u32(value: u32) -> Result<Self, DecodeError> {
        match value {
            0 => Ok(NonceState::Uninitialized),
            1 => Ok(NonceState::Initialized),
            _ => Err(DecodeError::invalid_field(
                "state",
                format!("invalid nonce state: {value}"),
            )),
        }
    }
}

/// A nonce account used for durable transactions.
#[derive(Debug, Clone, PartialEq)]
pub struct NonceAccount {
    /// Nonce account version (should be 1).
    pub version: u32,
    /// Current state.
    pub state: NonceState,
    /// Authority that can advance the nonce.
    pub authority: Pubkey,
    /// Current nonce value (blockhash).
    pub nonce: Pubkey,
    /// Fee calculator at the time of the nonce.
    pub fee_calculator_lamports_per_signature: u64,
}

impl DecodedEvent for NonceAccount {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "NonceAccount"
    }

    fn program_name(&self) -> &'static str {
        "system"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(NONCE_ACCOUNT_SIZE)
    }
}

/// Decoder for the System program.
#[derive(Debug, Clone)]
pub struct SystemDecoder;

impl SystemDecoder {
    /// Create a new System decoder.
    pub fn new() -> Self {
        Self
    }

    /// Decode a nonce account.
    fn decode_nonce(data: &[u8]) -> DecodeResult<NonceAccount> {
        if data.len() < NONCE_ACCOUNT_SIZE {
            return Err(DecodeError::insufficient_data(NONCE_ACCOUNT_SIZE, data.len()));
        }

        let mut reader = ZeroCopyReader::new(data);

        // Version
        let version = reader.read_u32().map_err(DecodeError::deserialization)?;

        // State
        let state_value = reader.read_u32().map_err(DecodeError::deserialization)?;
        let state = NonceState::from_u32(state_value)?;

        // Authority
        let authority = Pubkey::new_from_array(
            *reader.read_fixed::<32>().map_err(DecodeError::deserialization)?
        );

        // Nonce (blockhash)
        let nonce = Pubkey::new_from_array(
            *reader.read_fixed::<32>().map_err(DecodeError::deserialization)?
        );

        // Fee calculator
        let fee_calculator_lamports_per_signature = reader.read_u64()
            .map_err(DecodeError::deserialization)?;

        Ok(NonceAccount {
            version,
            state,
            authority,
            nonce,
            fee_calculator_lamports_per_signature,
        })
    }
}

impl Default for SystemDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoderIdentity for SystemDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("system", SYSTEM_PROGRAM_ID)
            .with_description("Solana System program decoder")
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities::default()
            .with_zero_copy()
            .with_account_types(vec!["SystemAccount", "NonceAccount"])
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AccountDecoder for SystemDecoder {

    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        // A nonce account is the only System-owned account with a layout.
        if data.len() == NONCE_ACCOUNT_SIZE {
            if let Ok(nonce) = Self::decode_nonce(data) {
                if nonce.version == 1 {
                    return Ok(Box::new(nonce));
                }
            }
        }

        // Otherwise the only System-owned account that means anything is an
        // empty one: a plain lamport holder. Accepting arbitrary bytes here is
        // what made this decoder claim every account it was ever shown --
        // including a 752-byte Raydium pool -- which made registry dispatch and
        // any "which decoder handles this?" search useless.
        if data.is_empty() {
            return Ok(Box::new(SystemAccount {
                lamports: 0,
                data: Vec::new(),
            }));
        }

        Err(DecodeError::invalid_format(format!(
            "{} bytes is neither an empty system account nor a {NONCE_ACCOUNT_SIZE}-byte nonce account",
            data.len()
        )))
    }

    fn can_decode(&self, data: &[u8]) -> bool {
        data.is_empty() || data.len() == NONCE_ACCOUNT_SIZE
    }

}

/// System program instruction types.
#[derive(Debug, Clone, PartialEq)]
pub enum SystemInstruction {
    /// Create a new account.
    CreateAccount {
        lamports: u64,
        space: u64,
        owner: Pubkey,
    },
    /// Assign account to a program.
    Assign { owner: Pubkey },
    /// Transfer lamports.
    Transfer { lamports: u64 },
    /// Create account with seed.
    CreateAccountWithSeed {
        base: Pubkey,
        seed: String,
        lamports: u64,
        space: u64,
        owner: Pubkey,
    },
    /// Advance nonce account.
    AdvanceNonceAccount,
    /// Withdraw from nonce account.
    WithdrawNonceAccount { lamports: u64 },
    /// Initialize nonce account.
    InitializeNonceAccount { authority: Pubkey },
    /// Authorize nonce account.
    AuthorizeNonceAccount { authority: Pubkey },
    /// Allocate space.
    Allocate { space: u64 },
    /// Allocate with seed.
    AllocateWithSeed {
        base: Pubkey,
        seed: String,
        space: u64,
        owner: Pubkey,
    },
    /// Assign with seed.
    AssignWithSeed {
        base: Pubkey,
        seed: String,
        owner: Pubkey,
    },
    /// Transfer with seed.
    TransferWithSeed {
        lamports: u64,
        from_seed: String,
        from_owner: Pubkey,
    },
    /// Upgrade nonce account.
    UpgradeNonceAccount,
}

impl DecodedEvent for SystemInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }

    fn event_type(&self) -> &'static str {
        match self {
            SystemInstruction::CreateAccount { .. } => "CreateAccount",
            SystemInstruction::Assign { .. } => "Assign",
            SystemInstruction::Transfer { .. } => "Transfer",
            SystemInstruction::CreateAccountWithSeed { .. } => "CreateAccountWithSeed",
            SystemInstruction::AdvanceNonceAccount => "AdvanceNonceAccount",
            SystemInstruction::WithdrawNonceAccount { .. } => "WithdrawNonceAccount",
            SystemInstruction::InitializeNonceAccount { .. } => "InitializeNonceAccount",
            SystemInstruction::AuthorizeNonceAccount { .. } => "AuthorizeNonceAccount",
            SystemInstruction::Allocate { .. } => "Allocate",
            SystemInstruction::AllocateWithSeed { .. } => "AllocateWithSeed",
            SystemInstruction::AssignWithSeed { .. } => "AssignWithSeed",
            SystemInstruction::TransferWithSeed { .. } => "TransferWithSeed",
            SystemInstruction::UpgradeNonceAccount => "UpgradeNonceAccount",
        }
    }

    fn program_name(&self) -> &'static str {
        "system"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl InstructionDecoder for SystemDecoder {

    fn decode_instruction(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        /// Read the next 32 bytes as a pubkey.
        fn read_pubkey(reader: &mut ZeroCopyReader<'_>) -> DecodeResult<Pubkey> {
            Ok(Pubkey::new_from_array(
                *reader
                    .read_fixed::<32>()
                    .map_err(DecodeError::deserialization)?,
            ))
        }

        if data.len() < 4 {
            return Err(DecodeError::insufficient_data(4, data.len()));
        }

        let mut reader = ZeroCopyReader::new(data);
        let discriminator = reader.read_u32().map_err(DecodeError::deserialization)?;

        let instruction = match discriminator {
            0 => {
                // CreateAccount
                let lamports = reader.read_u64().map_err(DecodeError::deserialization)?;
                let space = reader.read_u64().map_err(DecodeError::deserialization)?;
                let owner = Pubkey::new_from_array(
                    *reader.read_fixed::<32>().map_err(DecodeError::deserialization)?
                );
                SystemInstruction::CreateAccount {
                    lamports,
                    space,
                    owner,
                }
            }
            1 => {
                // Assign
                let owner = Pubkey::new_from_array(
                    *reader.read_fixed::<32>().map_err(DecodeError::deserialization)?
                );
                SystemInstruction::Assign { owner }
            }
            2 => {
                // Transfer
                let lamports = reader.read_u64().map_err(DecodeError::deserialization)?;
                SystemInstruction::Transfer { lamports }
            }
            3 => {
                // CreateAccountWithSeed. The seed is a borsh String, so it is a
                // u32 length followed by that many bytes -- everything after it
                // is at a variable offset, which is why these four instructions
                // cannot be read with fixed offsets like the others.
                let base = read_pubkey(&mut reader)?;
                let seed = reader.read_string().map_err(DecodeError::deserialization)?;
                let lamports = reader.read_u64().map_err(DecodeError::deserialization)?;
                let space = reader.read_u64().map_err(DecodeError::deserialization)?;
                let owner = read_pubkey(&mut reader)?;
                SystemInstruction::CreateAccountWithSeed {
                    base,
                    seed,
                    lamports,
                    space,
                    owner,
                }
            }
            4 => SystemInstruction::AdvanceNonceAccount,
            5 => {
                let lamports = reader.read_u64().map_err(DecodeError::deserialization)?;
                SystemInstruction::WithdrawNonceAccount { lamports }
            }
            6 => {
                let authority = Pubkey::new_from_array(
                    *reader.read_fixed::<32>().map_err(DecodeError::deserialization)?
                );
                SystemInstruction::InitializeNonceAccount { authority }
            }
            7 => {
                let authority = Pubkey::new_from_array(
                    *reader.read_fixed::<32>().map_err(DecodeError::deserialization)?
                );
                SystemInstruction::AuthorizeNonceAccount { authority }
            }
            8 => {
                let space = reader.read_u64().map_err(DecodeError::deserialization)?;
                SystemInstruction::Allocate { space }
            }
            9 => {
                let base = read_pubkey(&mut reader)?;
                let seed = reader.read_string().map_err(DecodeError::deserialization)?;
                let space = reader.read_u64().map_err(DecodeError::deserialization)?;
                let owner = read_pubkey(&mut reader)?;
                SystemInstruction::AllocateWithSeed {
                    base,
                    seed,
                    space,
                    owner,
                }
            }
            10 => {
                let base = read_pubkey(&mut reader)?;
                let seed = reader.read_string().map_err(DecodeError::deserialization)?;
                let owner = read_pubkey(&mut reader)?;
                SystemInstruction::AssignWithSeed { base, seed, owner }
            }
            11 => {
                // TransferWithSeed puts the lamports first, unlike the other
                // three; the seed describes the *source* account.
                let lamports = reader.read_u64().map_err(DecodeError::deserialization)?;
                let from_seed = reader.read_string().map_err(DecodeError::deserialization)?;
                let from_owner = read_pubkey(&mut reader)?;
                SystemInstruction::TransferWithSeed {
                    lamports,
                    from_seed,
                    from_owner,
                }
            }
            12 => SystemInstruction::UpgradeNonceAccount,
            _ => return Err(DecodeError::unknown_discriminator(&discriminator.to_le_bytes())),
        };

        Ok(Box::new(instruction))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;

    /// Encode a System instruction the way the runtime does: a u32 tag followed
    /// by borsh-serialised fields.
    fn encode(tag: u32, fields: &[u8]) -> Vec<u8> {
        let mut out = tag.to_le_bytes().to_vec();
        out.extend_from_slice(fields);
        out
    }

    fn borsh_string(value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        value.serialize(&mut out).expect("string serialises");
        out
    }

    #[test]
    fn the_with_seed_instructions_decode() {
        // These four have a variable-length seed in the middle, so they cannot
        // be read at fixed offsets. All four used to fall through to
        // "unknown discriminator" despite having variants defined for them.
        let decoder = SystemDecoder::new();
        let base = Pubkey::new_from_array([3u8; 32]);
        let owner = Pubkey::new_from_array([4u8; 32]);

        let mut fields = base.to_bytes().to_vec();
        fields.extend_from_slice(&borsh_string("my-seed"));
        fields.extend_from_slice(&1_000u64.to_le_bytes());
        fields.extend_from_slice(&165u64.to_le_bytes());
        fields.extend_from_slice(&owner.to_bytes());
        let event = decoder
            .decode_instruction(&encode(3, &fields))
            .expect("CreateAccountWithSeed decodes");
        let decoded = event
            .as_any()
            .downcast_ref::<SystemInstruction>()
            .expect("a system instruction");
        assert_eq!(
            decoded,
            &SystemInstruction::CreateAccountWithSeed {
                base,
                seed: "my-seed".to_string(),
                lamports: 1_000,
                space: 165,
                owner,
            }
        );

        let mut fields = base.to_bytes().to_vec();
        fields.extend_from_slice(&borsh_string("alloc"));
        fields.extend_from_slice(&99u64.to_le_bytes());
        fields.extend_from_slice(&owner.to_bytes());
        let event = decoder
            .decode_instruction(&encode(9, &fields))
            .expect("AllocateWithSeed decodes");
        assert_eq!(event.event_type(), "AllocateWithSeed");

        let mut fields = base.to_bytes().to_vec();
        fields.extend_from_slice(&borsh_string("assign"));
        fields.extend_from_slice(&owner.to_bytes());
        let event = decoder
            .decode_instruction(&encode(10, &fields))
            .expect("AssignWithSeed decodes");
        assert_eq!(event.event_type(), "AssignWithSeed");

        // TransferWithSeed puts lamports first, which is the detail most easily
        // got wrong by copying the shape of the other three.
        let mut fields = 7_500u64.to_le_bytes().to_vec();
        fields.extend_from_slice(&borsh_string("from"));
        fields.extend_from_slice(&owner.to_bytes());
        let event = decoder
            .decode_instruction(&encode(11, &fields))
            .expect("TransferWithSeed decodes");
        let decoded = event
            .as_any()
            .downcast_ref::<SystemInstruction>()
            .expect("a system instruction");
        assert_eq!(
            decoded,
            &SystemInstruction::TransferWithSeed {
                lamports: 7_500,
                from_seed: "from".to_string(),
                from_owner: owner,
            }
        );
    }

    #[test]
    fn a_seed_longer_than_the_buffer_is_an_error_not_a_panic() {
        // The length prefix is attacker-controlled. Claiming a 4GB seed in a
        // 40-byte instruction must not allocate or read out of bounds.
        let decoder = SystemDecoder::new();
        let mut fields = [3u8; 32].to_vec();
        fields.extend_from_slice(&u32::MAX.to_le_bytes());
        fields.extend_from_slice(b"short");

        let result = decoder.decode_instruction(&encode(3, &fields));
        assert!(result.is_err(), "an impossible seed length must be rejected");
    }
    use account_decoder_core::TypedEvent;

    #[test]
    fn test_decode_nonce_account() {
        let mut data = vec![0u8; NONCE_ACCOUNT_SIZE];

        // Version: 1
        data[0..4].copy_from_slice(&1u32.to_le_bytes());

        // State: Initialized (1)
        data[4..8].copy_from_slice(&1u32.to_le_bytes());

        // Authority: 32 bytes of 1s
        data[8..40].fill(1);

        // Nonce: 32 bytes of 2s
        data[40..72].fill(2);

        // Fee: 5000
        data[72..80].copy_from_slice(&5000u64.to_le_bytes());

        let decoder = SystemDecoder::new();
        let event = decoder.decode_account(&data).unwrap();

        let nonce = event.downcast_ref::<NonceAccount>().unwrap();
        assert_eq!(nonce.version, 1);
        assert_eq!(nonce.state, NonceState::Initialized);
        assert_eq!(nonce.fee_calculator_lamports_per_signature, 5000);
    }

    #[test]
    fn test_decode_transfer_instruction() {
        let mut data = vec![0u8; 12];
        data[0..4].copy_from_slice(&2u32.to_le_bytes()); // Transfer discriminator
        data[4..12].copy_from_slice(&1_000_000u64.to_le_bytes()); // lamports

        let decoder = SystemDecoder::new();
        let event = decoder.decode_instruction(&data).unwrap();

        assert_eq!(event.event_type(), "Transfer");

        let instr = event.downcast_ref::<SystemInstruction>().unwrap();
        assert!(matches!(
            instr,
            SystemInstruction::Transfer { lamports: 1_000_000 }
        ));
    }
}
