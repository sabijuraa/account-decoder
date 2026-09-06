//! SPL Token program decoder.
//!
//! Decodes token accounts, mints, and multisig accounts from the SPL Token program.

use account_decoder_borsh_util::ZeroCopyReader;
use account_decoder_core::{
    AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities, DecoderIdentity, DecoderMetadata, EventKind, InstructionDecoder,
};
use solana_sdk::pubkey::Pubkey;
use std::any::Any;

use crate::program_ids::TOKEN_PROGRAM_ID;

/// Discriminators for Token account types.
/// The Token program uses account size + first byte patterns.
const MINT_SIZE: usize = 82;
const TOKEN_ACCOUNT_SIZE: usize = 165;
const MULTISIG_SIZE: usize = 355;

/// A token account holding a balance of tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenAccount {
    /// The mint associated with this account.
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens.
    pub amount: u64,
    /// If set, the delegate can transfer up to `delegated_amount`.
    pub delegate: Option<Pubkey>,
    /// Account state (Uninitialized, Initialized, Frozen).
    pub state: AccountState,
    /// If set, tokens must be transferred to/from this native account as SOL.
    pub is_native: Option<u64>,
    /// Amount delegated to the delegate.
    pub delegated_amount: u64,
    /// Optional authority to close the account.
    pub close_authority: Option<Pubkey>,
}

impl DecodedEvent for TokenAccount {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "TokenAccount"
    }

    fn program_name(&self) -> &'static str {
        "spl-token"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(TOKEN_ACCOUNT_SIZE)
    }
}

/// A token mint defining the token.
#[derive(Debug, Clone, PartialEq)]
pub struct Mint {
    /// Optional authority to mint new tokens.
    pub mint_authority: Option<Pubkey>,
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of decimals.
    pub decimals: u8,
    /// Whether the mint is initialized.
    pub is_initialized: bool,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: Option<Pubkey>,
}

impl DecodedEvent for Mint {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "Mint"
    }

    fn program_name(&self) -> &'static str {
        "spl-token"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(MINT_SIZE)
    }
}

/// A multisig account for multi-party authorization.
#[derive(Debug, Clone, PartialEq)]
pub struct Multisig {
    /// Number of signers required.
    pub m: u8,
    /// Number of valid signers.
    pub n: u8,
    /// Whether the multisig is initialized.
    pub is_initialized: bool,
    /// The signers (up to 11).
    pub signers: Vec<Pubkey>,
}

impl DecodedEvent for Multisig {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "Multisig"
    }

    fn program_name(&self) -> &'static str {
        "spl-token"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(MULTISIG_SIZE)
    }
}

/// Account state for token accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AccountState {
    /// Account is not yet initialized.
    Uninitialized = 0,
    /// Account is initialized and active.
    Initialized = 1,
    /// Account is frozen.
    Frozen = 2,
}

impl AccountState {
    fn from_u8(value: u8) -> Result<Self, DecodeError> {
        match value {
            0 => Ok(AccountState::Uninitialized),
            1 => Ok(AccountState::Initialized),
            2 => Ok(AccountState::Frozen),
            _ => Err(DecodeError::invalid_field(
                "state",
                format!("invalid account state: {value}"),
            )),
        }
    }
}

/// Decoder for the SPL Token program.
#[derive(Debug, Clone)]
pub struct TokenDecoder;

impl TokenDecoder {
    /// Create a new Token decoder.
    pub fn new() -> Self {
        Self
    }

    /// Decode a mint account.
    pub fn decode_mint(data: &[u8]) -> DecodeResult<Mint> {
        if data.len() < MINT_SIZE {
            return Err(DecodeError::insufficient_data(MINT_SIZE, data.len()));
        }

        let mut reader = ZeroCopyReader::new(data);

        // mint_authority: COption<Pubkey> (4 + 32 bytes)
        let mint_authority = Self::read_coption_pubkey(&mut reader)?;

        // supply: u64
        let supply = reader.read_u64().map_err(DecodeError::deserialization)?;

        // decimals: u8
        let decimals = reader.read_u8().map_err(DecodeError::deserialization)?;

        // is_initialized: bool
        let is_initialized = reader.read_bool().map_err(DecodeError::deserialization)?;

        // freeze_authority: COption<Pubkey>
        let freeze_authority = Self::read_coption_pubkey(&mut reader)?;

        Ok(Mint {
            mint_authority,
            supply,
            decimals,
            is_initialized,
            freeze_authority,
        })
    }

    /// Decode a token account.
    pub fn decode_token_account(data: &[u8]) -> DecodeResult<TokenAccount> {
        if data.len() < TOKEN_ACCOUNT_SIZE {
            return Err(DecodeError::insufficient_data(TOKEN_ACCOUNT_SIZE, data.len()));
        }

        let mut reader = ZeroCopyReader::new(data);

        // mint: Pubkey (32 bytes)
        let mint = Pubkey::new_from_array(*reader.read_fixed::<32>()
            .map_err(DecodeError::deserialization)?);

        // owner: Pubkey (32 bytes)
        let owner = Pubkey::new_from_array(*reader.read_fixed::<32>()
            .map_err(DecodeError::deserialization)?);

        // amount: u64
        let amount = reader.read_u64().map_err(DecodeError::deserialization)?;

        // delegate: COption<Pubkey>
        let delegate = Self::read_coption_pubkey(&mut reader)?;

        // state: u8
        let state_byte = reader.read_u8().map_err(DecodeError::deserialization)?;
        let state = AccountState::from_u8(state_byte)?;

        // is_native: COption<u64>
        let is_native = Self::read_coption_u64(&mut reader)?;

        // delegated_amount: u64
        let delegated_amount = reader.read_u64().map_err(DecodeError::deserialization)?;

        // close_authority: COption<Pubkey>
        let close_authority = Self::read_coption_pubkey(&mut reader)?;

        Ok(TokenAccount {
            mint,
            owner,
            amount,
            delegate,
            state,
            is_native,
            delegated_amount,
            close_authority,
        })
    }

    /// Decode a multisig account.
    fn decode_multisig(data: &[u8]) -> DecodeResult<Multisig> {
        if data.len() < MULTISIG_SIZE {
            return Err(DecodeError::insufficient_data(MULTISIG_SIZE, data.len()));
        }

        let mut reader = ZeroCopyReader::new(data);

        let m = reader.read_u8().map_err(DecodeError::deserialization)?;
        let n = reader.read_u8().map_err(DecodeError::deserialization)?;
        let is_initialized = reader.read_bool().map_err(DecodeError::deserialization)?;

        // Read up to 11 signers
        let mut signers = Vec::with_capacity(n as usize);
        for _ in 0..11 {
            let pubkey_bytes = reader.read_fixed::<32>()
                .map_err(DecodeError::deserialization)?;
            let pubkey = Pubkey::new_from_array(*pubkey_bytes);
            if pubkey != Pubkey::default() {
                signers.push(pubkey);
            }
        }

        Ok(Multisig {
            m,
            n,
            is_initialized,
            signers,
        })
    }

    /// Read a COption<Pubkey> (Solana's Option representation).
    fn read_coption_pubkey(reader: &mut ZeroCopyReader) -> DecodeResult<Option<Pubkey>> {
        let tag = reader.read_u32().map_err(DecodeError::deserialization)?;
        let pubkey_bytes = reader.read_fixed::<32>()
            .map_err(DecodeError::deserialization)?;

        Ok(if tag == 0 {
            None
        } else {
            Some(Pubkey::new_from_array(*pubkey_bytes))
        })
    }

    /// Read a COption<u64>.
    fn read_coption_u64(reader: &mut ZeroCopyReader) -> DecodeResult<Option<u64>> {
        let tag = reader.read_u32().map_err(DecodeError::deserialization)?;
        let value = reader.read_u64().map_err(DecodeError::deserialization)?;

        Ok(if tag == 0 { None } else { Some(value) })
    }
}

impl Default for TokenDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoderIdentity for TokenDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("spl-token", TOKEN_PROGRAM_ID)
            .with_description("SPL Token program decoder")
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities::default()
            .with_zero_copy()
            .with_account_types(vec!["TokenAccount", "Mint", "Multisig"])
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AccountDecoder for TokenDecoder {

    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        // Determine account type by size
        // Exact sizes only. SPL Token accounts are fixed-size structs, so
        // anything else is not one -- the previous `len >= 165` fallback made
        // this decoder claim a 752-byte Raydium pool as a token account and
        // report a mint and owner read out of the middle of it. It also
        // contradicted `can_decode`, which was already strict.
        match data.len() {
            MINT_SIZE => Ok(Box::new(Self::decode_mint(data)?)),
            TOKEN_ACCOUNT_SIZE => Ok(Box::new(Self::decode_token_account(data)?)),
            MULTISIG_SIZE => Ok(Box::new(Self::decode_multisig(data)?)),
            len => Err(DecodeError::invalid_format(format!(
                "{len} bytes is not an SPL Token account: expected {MINT_SIZE} (mint), \
                 {TOKEN_ACCOUNT_SIZE} (token account) or {MULTISIG_SIZE} (multisig)"
            ))),
        }
    }

    fn can_decode(&self, data: &[u8]) -> bool {
        matches!(data.len(), MINT_SIZE | TOKEN_ACCOUNT_SIZE | MULTISIG_SIZE)
    }

}

/// Token instruction types.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenInstruction {
    /// Initialize a new mint.
    InitializeMint {
        decimals: u8,
        mint_authority: Pubkey,
        freeze_authority: Option<Pubkey>,
    },
    /// Initialize a new account.
    InitializeAccount,
    /// Initialize a multisig.
    InitializeMultisig { m: u8 },
    /// Transfer tokens.
    Transfer { amount: u64 },
    /// Approve a delegate.
    Approve { amount: u64 },
    /// Revoke delegation.
    Revoke,
    /// Set an authority.
    SetAuthority {
        authority_type: u8,
        new_authority: Option<Pubkey>,
    },
    /// Mint tokens.
    MintTo { amount: u64 },
    /// Burn tokens.
    Burn { amount: u64 },
    /// Close an account.
    CloseAccount,
    /// Freeze an account.
    FreezeAccount,
    /// Thaw an account.
    ThawAccount,
    /// Transfer with checked decimals.
    TransferChecked { amount: u64, decimals: u8 },
    /// Approve with checked decimals.
    ApproveChecked { amount: u64, decimals: u8 },
    /// Mint with checked decimals.
    MintToChecked { amount: u64, decimals: u8 },
    /// Burn with checked decimals.
    BurnChecked { amount: u64, decimals: u8 },
    /// Sync native balance.
    SyncNative,
}

impl DecodedEvent for TokenInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }

    fn event_type(&self) -> &'static str {
        match self {
            TokenInstruction::InitializeMint { .. } => "InitializeMint",
            TokenInstruction::InitializeAccount => "InitializeAccount",
            TokenInstruction::InitializeMultisig { .. } => "InitializeMultisig",
            TokenInstruction::Transfer { .. } => "Transfer",
            TokenInstruction::Approve { .. } => "Approve",
            TokenInstruction::Revoke => "Revoke",
            TokenInstruction::SetAuthority { .. } => "SetAuthority",
            TokenInstruction::MintTo { .. } => "MintTo",
            TokenInstruction::Burn { .. } => "Burn",
            TokenInstruction::CloseAccount => "CloseAccount",
            TokenInstruction::FreezeAccount => "FreezeAccount",
            TokenInstruction::ThawAccount => "ThawAccount",
            TokenInstruction::TransferChecked { .. } => "TransferChecked",
            TokenInstruction::ApproveChecked { .. } => "ApproveChecked",
            TokenInstruction::MintToChecked { .. } => "MintToChecked",
            TokenInstruction::BurnChecked { .. } => "BurnChecked",
            TokenInstruction::SyncNative => "SyncNative",
        }
    }

    fn program_name(&self) -> &'static str {
        "spl-token"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl InstructionDecoder for TokenDecoder {

    fn decode_instruction(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        if data.is_empty() {
            return Err(DecodeError::insufficient_data(1, 0));
        }

        let mut reader = ZeroCopyReader::new(data);
        let discriminator = reader.read_u8().map_err(DecodeError::deserialization)?;

        let instruction = match discriminator {
            0 => {
                // InitializeMint
                let decimals = reader.read_u8().map_err(DecodeError::deserialization)?;
                let mint_authority = Pubkey::new_from_array(
                    *reader.read_fixed::<32>().map_err(DecodeError::deserialization)?
                );
                let freeze_authority = Self::read_coption_pubkey(&mut reader)?;

                TokenInstruction::InitializeMint {
                    decimals,
                    mint_authority,
                    freeze_authority,
                }
            }
            1 => TokenInstruction::InitializeAccount,
            2 => {
                let m = reader.read_u8().map_err(DecodeError::deserialization)?;
                TokenInstruction::InitializeMultisig { m }
            }
            3 => {
                let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
                TokenInstruction::Transfer { amount }
            }
            4 => {
                let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
                TokenInstruction::Approve { amount }
            }
            5 => TokenInstruction::Revoke,
            6 => {
                let authority_type = reader.read_u8().map_err(DecodeError::deserialization)?;
                let new_authority = Self::read_coption_pubkey(&mut reader)?;
                TokenInstruction::SetAuthority {
                    authority_type,
                    new_authority,
                }
            }
            7 => {
                let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
                TokenInstruction::MintTo { amount }
            }
            8 => {
                let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
                TokenInstruction::Burn { amount }
            }
            9 => TokenInstruction::CloseAccount,
            10 => TokenInstruction::FreezeAccount,
            11 => TokenInstruction::ThawAccount,
            12 => {
                let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
                let decimals = reader.read_u8().map_err(DecodeError::deserialization)?;
                TokenInstruction::TransferChecked { amount, decimals }
            }
            13 => {
                let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
                let decimals = reader.read_u8().map_err(DecodeError::deserialization)?;
                TokenInstruction::ApproveChecked { amount, decimals }
            }
            14 => {
                let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
                let decimals = reader.read_u8().map_err(DecodeError::deserialization)?;
                TokenInstruction::MintToChecked { amount, decimals }
            }
            15 => {
                let amount = reader.read_u64().map_err(DecodeError::deserialization)?;
                let decimals = reader.read_u8().map_err(DecodeError::deserialization)?;
                TokenInstruction::BurnChecked { amount, decimals }
            }
            17 => TokenInstruction::SyncNative,
            _ => return Err(DecodeError::unknown_discriminator(&[discriminator])),
        };

        Ok(Box::new(instruction))
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_three_real_sizes_are_accepted() {
        // The sizes are the whole type tag here: SPL Token accounts carry no
        // discriminator, so length is all a decoder has to go on. Accepting
        // anything larger means decoding another program's account as a token
        // account and reporting whatever happened to sit at those offsets.
        let decoder = TokenDecoder::new();

        for len in [MINT_SIZE, TOKEN_ACCOUNT_SIZE, MULTISIG_SIZE] {
            assert!(decoder.can_decode(&vec![0u8; len]), "{len} is a real size");
        }

        for len in [0usize, 81, 83, 164, 166, 354, 356, 752, 866, 10_000] {
            let data = vec![0u8; len];
            assert!(!decoder.can_decode(&data), "{len} is not an SPL Token size");
            assert!(
                decoder.decode_account(&data).is_err(),
                "{len} bytes must not decode as a token account"
            );
        }
    }
    use account_decoder_core::TypedEvent;

    #[test]
    fn test_decode_mint() {
        // Create a minimal mint account data
        let mut data = vec![0u8; MINT_SIZE];

        // Set mint authority: None (tag = 0)
        data[0..4].copy_from_slice(&0u32.to_le_bytes());
        // Skip 32 bytes for pubkey

        // Supply: 1000
        data[36..44].copy_from_slice(&1000u64.to_le_bytes());

        // Decimals: 9
        data[44] = 9;

        // is_initialized: true
        data[45] = 1;

        // freeze_authority: None
        data[46..50].copy_from_slice(&0u32.to_le_bytes());

        let decoder = TokenDecoder::new();
        let event = decoder.decode_account(&data).unwrap();

        let mint = event.downcast_ref::<Mint>().unwrap();
        assert_eq!(mint.supply, 1000);
        assert_eq!(mint.decimals, 9);
        assert!(mint.is_initialized);
        assert!(mint.mint_authority.is_none());
    }

    #[test]
    fn test_decode_transfer_instruction() {
        let mut data = vec![0u8; 9];
        data[0] = 3; // Transfer discriminator
        data[1..9].copy_from_slice(&100u64.to_le_bytes()); // amount

        let decoder = TokenDecoder::new();
        let event = decoder.decode_instruction(&data).unwrap();

        assert_eq!(event.event_type(), "Transfer");

        let instr = event.downcast_ref::<TokenInstruction>().unwrap();
        assert!(matches!(instr, TokenInstruction::Transfer { amount: 100 }));
    }
}
