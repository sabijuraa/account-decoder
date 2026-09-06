//! SPL Token-2022 program decoder.
//!
//! Token-2022 extends the original Token program with additional features
//! like transfer hooks, confidential transfers, and metadata.

use account_decoder_core::{
    AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities, DecoderIdentity,
    DecoderMetadata, EventKind,
};
use solana_sdk::pubkey::Pubkey;
use std::any::Any;

use crate::program_ids::TOKEN_2022_PROGRAM_ID;
use crate::token::{AccountState, Mint, TokenAccount};

/// Token-2022 extensions that can be attached to accounts.
#[derive(Debug, Clone, PartialEq)]
pub enum Token2022Extension {
    /// Transfer fee configuration.
    TransferFeeConfig {
        /// Authority that can modify fees.
        transfer_fee_config_authority: Option<Pubkey>,
        /// Authority that can withdraw fees.
        withdraw_withheld_authority: Option<Pubkey>,
        /// Fee basis points (100 = 1%).
        transfer_fee_basis_points: u16,
        /// Maximum fee amount.
        maximum_fee: u64,
    },
    /// Transfer fee state on a token account.
    TransferFeeAmount {
        /// Withheld fees.
        withheld_amount: u64,
    },
    /// Confidential transfer mint configuration.
    ConfidentialTransferMint,
    /// Confidential transfer account state.
    ConfidentialTransferAccount,
    /// Default account state for new accounts.
    DefaultAccountState { state: AccountState },
    /// Immutable owner (cannot change owner after init).
    ImmutableOwner,
    /// Memo required on transfers.
    MemoTransfer {
        require_incoming_transfer_memos: bool,
    },
    /// Non-transferable tokens.
    NonTransferable,
    /// Interest-bearing tokens.
    InterestBearingConfig {
        rate_authority: Option<Pubkey>,
        initialization_timestamp: i64,
        pre_update_average_rate: i16,
        last_update_timestamp: i64,
        current_rate: i16,
    },
    /// CPI guard (prevent certain CPI).
    CpiGuard { lock_cpi: bool },
    /// Permanent delegate.
    PermanentDelegate { delegate: Option<Pubkey> },
    /// Transfer hook configuration.
    TransferHook {
        authority: Option<Pubkey>,
        program_id: Option<Pubkey>,
    },
    /// Metadata pointer.
    MetadataPointer {
        authority: Option<Pubkey>,
        metadata_address: Option<Pubkey>,
    },
    /// Token metadata.
    TokenMetadata {
        update_authority: Option<Pubkey>,
        mint: Pubkey,
        name: String,
        symbol: String,
        uri: String,
    },
    /// Unknown extension type.
    Unknown { extension_type: u16, data: Vec<u8> },
}

/// A Token-2022 mint with extensions.
#[derive(Debug, Clone, PartialEq)]
pub struct Token2022Mint {
    /// Base mint data.
    pub base: Mint,
    /// Extensions attached to this mint.
    pub extensions: Vec<Token2022Extension>,
}

impl DecodedEvent for Token2022Mint {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "Token2022Mint"
    }

    fn program_name(&self) -> &'static str {
        "spl-token-2022"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A Token-2022 token account with extensions.
#[derive(Debug, Clone, PartialEq)]
pub struct Token2022Account {
    /// Base token account data.
    pub base: TokenAccount,
    /// Extensions attached to this account.
    pub extensions: Vec<Token2022Extension>,
}

impl DecodedEvent for Token2022Account {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "Token2022Account"
    }

    fn program_name(&self) -> &'static str {
        "spl-token-2022"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Extension type discriminators.
mod extension_type {
    pub const TRANSFER_FEE_CONFIG: u16 = 1;
    pub const TRANSFER_FEE_AMOUNT: u16 = 2;
    pub const MINT_CLOSE_AUTHORITY: u16 = 3;
    pub const CONFIDENTIAL_TRANSFER_MINT: u16 = 4;
    pub const CONFIDENTIAL_TRANSFER_ACCOUNT: u16 = 5;
    pub const DEFAULT_ACCOUNT_STATE: u16 = 6;
    pub const IMMUTABLE_OWNER: u16 = 7;
    pub const MEMO_TRANSFER: u16 = 8;
    pub const NON_TRANSFERABLE: u16 = 9;
    pub const INTEREST_BEARING_CONFIG: u16 = 10;
    pub const CPI_GUARD: u16 = 11;
    pub const PERMANENT_DELEGATE: u16 = 12;
    pub const NON_TRANSFERABLE_ACCOUNT: u16 = 13;
    pub const TRANSFER_HOOK: u16 = 14;
    pub const TRANSFER_HOOK_ACCOUNT: u16 = 15;
    pub const METADATA_POINTER: u16 = 18;
    pub const TOKEN_METADATA: u16 = 19;
}

/// Decoder for the SPL Token-2022 program.
#[derive(Debug, Clone)]
pub struct Token2022Decoder;

impl Token2022Decoder {
    /// Create a new Token-2022 decoder.
    pub fn new() -> Self {
        Self
    }

    /// Account type byte positions.
    const ACCOUNT_TYPE_OFFSET: usize = 165;

    /// Parse extensions from the extension data.
    /// Whether a TLV region is well formed all the way to its end.
    ///
    /// The account-type byte at offset 165 is one byte of signal, and one byte
    /// is not enough to tell a Token-2022 account from any other program's
    /// account that happens to hold a 1 or a 2 there -- a Raydium pool does.
    /// Requiring the extension region to walk cleanly to its end is the real
    /// check: arbitrary bytes almost never do.
    fn tlv_is_well_formed(data: &[u8]) -> bool {
        let mut offset = 0;
        let mut entries = 0;

        while offset < data.len() {
            // Trailing zero padding is normal once the entries are done.
            if data[offset..].iter().all(|b| *b == 0) {
                break;
            }

            if offset + 4 > data.len() {
                return false;
            }

            let ext_len = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + ext_len > data.len() {
                return false;
            }

            offset += ext_len;
            entries += 1;
        }

        entries > 0
    }

    fn parse_extensions(data: &[u8]) -> Vec<Token2022Extension> {
        let mut extensions = Vec::new();
        let mut offset = 0;

        while offset + 4 <= data.len() {
            // Read extension type (u16) and length (u16)
            let ext_type = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let ext_len = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;

            offset += 4;

            if offset + ext_len > data.len() {
                break;
            }

            let ext_data = &data[offset..offset + ext_len];
            let extension = Self::parse_extension(ext_type, ext_data);
            extensions.push(extension);

            offset += ext_len;

            // Align to 4 bytes
            offset = (offset + 3) & !3;
        }

        extensions
    }

    /// Parse a single extension.
    fn parse_extension(ext_type: u16, data: &[u8]) -> Token2022Extension {
        /// Token-2022 stores an absent pubkey as 32 zero bytes rather than as a
        /// tagged option, so all-zero means None.
        fn optional_pubkey(data: &[u8], offset: usize) -> Option<Pubkey> {
            let bytes: [u8; 32] = data.get(offset..offset + 32)?.try_into().ok()?;
            (bytes != [0u8; 32]).then(|| Pubkey::new_from_array(bytes))
        }

        fn read_u64(data: &[u8], offset: usize) -> u64 {
            data.get(offset..offset + 8)
                .and_then(|b| b.try_into().ok())
                .map(u64::from_le_bytes)
                .unwrap_or(0)
        }

        fn read_u16(data: &[u8], offset: usize) -> u16 {
            data.get(offset..offset + 2)
                .and_then(|b| b.try_into().ok())
                .map(u16::from_le_bytes)
                .unwrap_or(0)
        }

        fn unknown(ext_type: u16, data: &[u8]) -> Token2022Extension {
            Token2022Extension::Unknown {
                extension_type: ext_type,
                data: data.to_vec(),
            }
        }

        /// Read a borsh string: a u32 length then that many bytes.
        ///
        /// The length is attacker-controlled, so a claim longer than the buffer
        /// ends the parse rather than allocating on it.
        fn read_string(data: &[u8], cursor: &mut usize) -> Option<String> {
            let len = read_u32(data, *cursor)? as usize;
            *cursor += 4;
            let bytes = data.get(*cursor..*cursor + len)?;
            *cursor += len;
            String::from_utf8(bytes.to_vec()).ok()
        }

        fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
            data.get(offset..offset + 4)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_le_bytes)
        }

        /// TokenMetadata: update authority, mint, then three strings.
        fn parse_token_metadata(data: &[u8]) -> Option<Token2022Extension> {
            let update_authority = {
                let bytes: [u8; 32] = data.get(0..32)?.try_into().ok()?;
                (bytes != [0u8; 32]).then(|| Pubkey::new_from_array(bytes))
            };
            let mint = Pubkey::new_from_array(data.get(32..64)?.try_into().ok()?);

            let mut cursor = 64;
            let name = read_string(data, &mut cursor)?;
            let symbol = read_string(data, &mut cursor)?;
            let uri = read_string(data, &mut cursor)?;

            Some(Token2022Extension::TokenMetadata {
                update_authority,
                mint,
                name,
                symbol,
                uri,
            })
        }

        match ext_type {
            extension_type::TRANSFER_FEE_AMOUNT => {
                if data.len() >= 8 {
                    let withheld_amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
                    Token2022Extension::TransferFeeAmount { withheld_amount }
                } else {
                    Token2022Extension::Unknown {
                        extension_type: ext_type,
                        data: data.to_vec(),
                    }
                }
            }
            extension_type::TRANSFER_FEE_CONFIG => {
                // authority(32) + withdraw authority(32) + withheld(8)
                // + older TransferFee(18) + newer TransferFee(18).
                // A TransferFee is epoch(8) + maximum_fee(8) + basis_points(2),
                // and the *newer* one is what applies from the next epoch, so
                // that is the pair reported here.
                const NEWER_FEE_OFFSET: usize = 32 + 32 + 8 + 18;
                if data.len() >= NEWER_FEE_OFFSET + 18 {
                    let maximum_fee = read_u64(data, NEWER_FEE_OFFSET + 8);
                    let transfer_fee_basis_points = read_u16(data, NEWER_FEE_OFFSET + 16);
                    Token2022Extension::TransferFeeConfig {
                        transfer_fee_config_authority: optional_pubkey(data, 0),
                        withdraw_withheld_authority: optional_pubkey(data, 32),
                        transfer_fee_basis_points,
                        maximum_fee,
                    }
                } else {
                    unknown(ext_type, data)
                }
            }
            extension_type::MINT_CLOSE_AUTHORITY => {
                // A single OptionalNonZeroPubkey. There is no variant for it, so
                // it is reported by type with its bytes intact rather than
                // silently discarded.
                unknown(ext_type, data)
            }
            extension_type::PERMANENT_DELEGATE => Token2022Extension::PermanentDelegate {
                delegate: optional_pubkey(data, 0),
            },
            extension_type::INTEREST_BEARING_CONFIG => {
                const LEN: usize = 32 + 8 + 2 + 8 + 2;
                if data.len() >= LEN {
                    Token2022Extension::InterestBearingConfig {
                        rate_authority: optional_pubkey(data, 0),
                        initialization_timestamp: read_u64(data, 32) as i64,
                        pre_update_average_rate: read_u16(data, 40) as i16,
                        last_update_timestamp: read_u64(data, 42) as i64,
                        current_rate: read_u16(data, 50) as i16,
                    }
                } else {
                    unknown(ext_type, data)
                }
            }
            extension_type::TRANSFER_HOOK => {
                if data.len() >= 64 {
                    Token2022Extension::TransferHook {
                        authority: optional_pubkey(data, 0),
                        program_id: optional_pubkey(data, 32),
                    }
                } else {
                    unknown(ext_type, data)
                }
            }
            extension_type::METADATA_POINTER => {
                if data.len() >= 64 {
                    Token2022Extension::MetadataPointer {
                        authority: optional_pubkey(data, 0),
                        metadata_address: optional_pubkey(data, 32),
                    }
                } else {
                    unknown(ext_type, data)
                }
            }
            extension_type::CONFIDENTIAL_TRANSFER_MINT => {
                Token2022Extension::ConfidentialTransferMint
            }
            extension_type::CONFIDENTIAL_TRANSFER_ACCOUNT => {
                Token2022Extension::ConfidentialTransferAccount
            }
            // A non-transferable *account* marker carries no payload; the mint
            // side is NON_TRANSFERABLE above.
            extension_type::NON_TRANSFERABLE_ACCOUNT => Token2022Extension::NonTransferable,
            extension_type::TOKEN_METADATA => {
                parse_token_metadata(data).unwrap_or_else(|| unknown(ext_type, data))
            }
            extension_type::TRANSFER_HOOK_ACCOUNT => {
                // A single "transferring" flag, with no variant to carry it.
                unknown(ext_type, data)
            }
            extension_type::IMMUTABLE_OWNER => Token2022Extension::ImmutableOwner,
            extension_type::NON_TRANSFERABLE => Token2022Extension::NonTransferable,
            extension_type::MEMO_TRANSFER => {
                let require = data.first().copied().unwrap_or(0) != 0;
                Token2022Extension::MemoTransfer {
                    require_incoming_transfer_memos: require,
                }
            }
            extension_type::CPI_GUARD => {
                let lock = data.first().copied().unwrap_or(0) != 0;
                Token2022Extension::CpiGuard { lock_cpi: lock }
            }
            extension_type::DEFAULT_ACCOUNT_STATE => {
                let state_byte = data.first().copied().unwrap_or(0);
                let state = match state_byte {
                    0 => AccountState::Uninitialized,
                    1 => AccountState::Initialized,
                    2 => AccountState::Frozen,
                    _ => AccountState::Uninitialized,
                };
                Token2022Extension::DefaultAccountState { state }
            }
            _ => Token2022Extension::Unknown {
                extension_type: ext_type,
                data: data.to_vec(),
            },
        }
    }
}

impl Default for Token2022Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoderIdentity for Token2022Decoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("spl-token-2022", TOKEN_2022_PROGRAM_ID)
            .with_description("SPL Token-2022 program decoder with extensions support")
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities::default()
            .with_zero_copy()
            .with_account_types(vec!["Token2022Mint", "Token2022Account"])
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AccountDecoder for Token2022Decoder {
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        // Token-2022 accounts are at least 165 bytes (token account size)
        // with optional extensions after

        if data.len() < 82 {
            return Err(DecodeError::insufficient_data(82, data.len()));
        }

        // Check if this is a mint (82 bytes base) or token account (165 bytes base)
        // Token-2022 uses an account type byte at offset 165 for extended accounts

        if data.len() == 82 {
            // Standard mint without extensions
            let mint = crate::token::TokenDecoder::decode_mint(data)?;
            return Ok(Box::new(Token2022Mint {
                base: mint,
                extensions: vec![],
            }));
        }

        // A plain token account is exactly 165 bytes with no type byte.
        if data.len() == Self::ACCOUNT_TYPE_OFFSET {
            let base = crate::token::TokenDecoder::decode_token_account(data)?;
            return Ok(Box::new(Token2022Account {
                base,
                extensions: vec![],
            }));
        }

        if data.len() > Self::ACCOUNT_TYPE_OFFSET {
            let account_type = data[Self::ACCOUNT_TYPE_OFFSET];
            let tlv = &data[Self::ACCOUNT_TYPE_OFFSET + 1..];

            if !matches!(account_type, 1 | 2) || !Self::tlv_is_well_formed(tlv) {
                return Err(DecodeError::invalid_format(format!(
                    "{} bytes with account type {account_type} is not a Token-2022 account: \
                     the extension region does not parse",
                    data.len()
                )));
            }

            match account_type {
                1 => {
                    // Mint with extensions
                    let base_data = &data[..82];
                    let base = crate::token::TokenDecoder::decode_mint(base_data)?;

                    // Extensions start after base + padding
                    let ext_start = 82 + 83; // base + padding to 165
                    let extensions = if data.len() > ext_start {
                        Self::parse_extensions(&data[ext_start..])
                    } else {
                        vec![]
                    };

                    Ok(Box::new(Token2022Mint { base, extensions }))
                }
                2 => {
                    // Token account with extensions
                    let base_data = &data[..165];
                    let base = crate::token::TokenDecoder::decode_token_account(base_data)?;

                    // Extensions start after the account type byte
                    let ext_start = Self::ACCOUNT_TYPE_OFFSET + 1;
                    let extensions = if data.len() > ext_start {
                        Self::parse_extensions(&data[ext_start..])
                    } else {
                        vec![]
                    };

                    Ok(Box::new(Token2022Account { base, extensions }))
                }
                _ => {
                    // Try to decode as standard token account
                    let base = crate::token::TokenDecoder::decode_token_account(data)?;
                    Ok(Box::new(Token2022Account {
                        base,
                        extensions: vec![],
                    }))
                }
            }
        } else {
            // Not enough data for extended format, try standard
            if data.len() >= 165 {
                let base = crate::token::TokenDecoder::decode_token_account(data)?;
                Ok(Box::new(Token2022Account {
                    base,
                    extensions: vec![],
                }))
            } else if data.len() >= 82 {
                let base = crate::token::TokenDecoder::decode_mint(data)?;
                Ok(Box::new(Token2022Mint {
                    base,
                    extensions: vec![],
                }))
            } else {
                Err(DecodeError::insufficient_data(82, data.len()))
            }
        }
    }

    fn can_decode(&self, data: &[u8]) -> bool {
        data.len() >= 82
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a TLV extension blob: type(u16) + length(u16) + payload.
    fn tlv(ext_type: u16, payload: &[u8]) -> Vec<u8> {
        let mut out = ext_type.to_le_bytes().to_vec();
        out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn another_programs_account_is_not_claimed_as_token_2022() {
        // The regression this guards: a 752-byte Raydium pool has a 2 at offset
        // 165, so an account-type check alone accepted it and reported a mint
        // and owner read out of the middle of a liquidity pool.
        let decoder = Token2022Decoder::new();
        let mut pool = vec![0u8; 752];
        pool[165] = 2;
        for (i, byte) in pool.iter_mut().enumerate().skip(166) {
            *byte = (i % 251) as u8; // arbitrary, non-TLV-shaped
        }

        assert!(
            decoder.decode_account(&pool).is_err(),
            "an account whose extension region does not parse is not Token-2022"
        );
    }

    #[test]
    fn a_well_formed_extension_region_is_accepted() {
        let mut account = vec![0u8; 165];
        account[164] = 1; // plausible base
        account.push(2); // account type: Account
        account.extend_from_slice(&tlv(7, &[])); // ImmutableOwner
        assert!(Token2022Decoder::tlv_is_well_formed(&account[166..]));
    }

    #[test]
    fn a_transfer_fee_config_reports_the_fee_that_will_apply() {
        // The newer of the two fee records is the one that takes effect, and
        // reading the older one would understate a fee increase.
        let authority = Pubkey::new_from_array([1u8; 32]);
        let withdrawer = Pubkey::new_from_array([2u8; 32]);

        let mut payload = authority.to_bytes().to_vec();
        payload.extend_from_slice(&withdrawer.to_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes()); // withheld
                                                        // older: epoch 10, max 1_000, 50 bps
        payload.extend_from_slice(&10u64.to_le_bytes());
        payload.extend_from_slice(&1_000u64.to_le_bytes());
        payload.extend_from_slice(&50u16.to_le_bytes());
        // newer: epoch 11, max 5_000, 250 bps
        payload.extend_from_slice(&11u64.to_le_bytes());
        payload.extend_from_slice(&5_000u64.to_le_bytes());
        payload.extend_from_slice(&250u16.to_le_bytes());

        let parsed = Token2022Decoder::parse_extension(1, &payload);
        assert_eq!(
            parsed,
            Token2022Extension::TransferFeeConfig {
                transfer_fee_config_authority: Some(authority),
                withdraw_withheld_authority: Some(withdrawer),
                transfer_fee_basis_points: 250,
                maximum_fee: 5_000,
            }
        );
    }

    #[test]
    fn an_all_zero_authority_is_absent_rather_than_the_zero_pubkey() {
        // Token-2022 encodes "no authority" as 32 zero bytes, not as a tagged
        // option. Reporting Pubkey::default() would name a real address nobody
        // controls as the permanent delegate.
        let parsed = Token2022Decoder::parse_extension(12, &[0u8; 32]);
        assert_eq!(
            parsed,
            Token2022Extension::PermanentDelegate { delegate: None }
        );

        let delegate = Pubkey::new_from_array([9u8; 32]);
        let parsed = Token2022Decoder::parse_extension(12, &delegate.to_bytes());
        assert_eq!(
            parsed,
            Token2022Extension::PermanentDelegate {
                delegate: Some(delegate)
            }
        );
    }

    #[test]
    fn token_metadata_is_read_including_its_strings() {
        let authority = Pubkey::new_from_array([1u8; 32]);
        let mint = Pubkey::new_from_array([2u8; 32]);

        let mut payload = authority.to_bytes().to_vec();
        payload.extend_from_slice(&mint.to_bytes());
        for value in ["PayPal USD", "PYUSD", "https://example.invalid/pyusd.json"] {
            payload.extend_from_slice(&(value.len() as u32).to_le_bytes());
            payload.extend_from_slice(value.as_bytes());
        }

        assert_eq!(
            Token2022Decoder::parse_extension(19, &payload),
            Token2022Extension::TokenMetadata {
                update_authority: Some(authority),
                mint,
                name: "PayPal USD".to_string(),
                symbol: "PYUSD".to_string(),
                uri: "https://example.invalid/pyusd.json".to_string(),
            }
        );
    }

    #[test]
    fn a_metadata_string_longer_than_its_buffer_is_refused() {
        // The length prefix comes off chain. Claiming four gigabytes of name in
        // a hundred-byte extension must fall back rather than allocate.
        let mut payload = [0u8; 64].to_vec();
        payload.extend_from_slice(&u32::MAX.to_le_bytes());
        payload.extend_from_slice(b"short");

        assert!(
            matches!(
                Token2022Decoder::parse_extension(19, &payload),
                Token2022Extension::Unknown {
                    extension_type: 19,
                    ..
                }
            ),
            "an impossible string length keeps the raw bytes instead"
        );
    }

    #[test]
    fn a_metadata_pointer_reports_both_halves() {
        let authority = Pubkey::new_from_array([4u8; 32]);
        let metadata = Pubkey::new_from_array([5u8; 32]);
        let mut payload = authority.to_bytes().to_vec();
        payload.extend_from_slice(&metadata.to_bytes());

        assert_eq!(
            Token2022Decoder::parse_extension(18, &payload),
            Token2022Extension::MetadataPointer {
                authority: Some(authority),
                metadata_address: Some(metadata),
            }
        );
    }

    #[test]
    fn a_truncated_extension_is_reported_rather_than_guessed_at() {
        // Half a transfer fee config must not be read as a whole one with
        // zeroes in the missing half.
        let parsed = Token2022Decoder::parse_extension(1, &[7u8; 40]);
        assert!(
            matches!(
                parsed,
                Token2022Extension::Unknown {
                    extension_type: 1,
                    ..
                }
            ),
            "a short payload should keep its bytes rather than decode partially"
        );
    }

    #[test]
    fn extensions_are_walked_in_sequence_and_stop_at_a_bad_length() {
        // A length field that runs past the buffer is attacker-controlled;
        // parsing must stop rather than read out of bounds.
        let mut blob = tlv(7, &[]); // ImmutableOwner
        blob.extend_from_slice(&tlv(9, &[])); // NonTransferable
        let good = Token2022Decoder::parse_extensions(&blob);
        assert_eq!(good.len(), 2);
        assert_eq!(good[0], Token2022Extension::ImmutableOwner);
        assert_eq!(good[1], Token2022Extension::NonTransferable);

        let mut truncated = tlv(7, &[]);
        truncated.extend_from_slice(&1u16.to_le_bytes());
        truncated.extend_from_slice(&u16::MAX.to_le_bytes()); // claims 65535 bytes
        truncated.extend_from_slice(&[0u8; 4]);
        let parsed = Token2022Decoder::parse_extensions(&truncated);
        assert_eq!(
            parsed.len(),
            1,
            "the impossible length ends the walk instead of over-reading"
        );
    }
    use account_decoder_core::TypedEvent;

    #[test]
    fn test_decode_simple_mint() {
        // Create a minimal mint account data
        let mut data = vec![0u8; 82];

        // Set is_initialized: true
        data[45] = 1;
        data[44] = 6; // decimals

        let decoder = Token2022Decoder::new();
        let event = decoder.decode_account(&data).unwrap();

        let mint = event.downcast_ref::<Token2022Mint>().unwrap();
        assert_eq!(mint.base.decimals, 6);
        assert!(mint.extensions.is_empty());
    }
}
