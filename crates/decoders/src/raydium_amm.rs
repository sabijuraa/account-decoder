//! Raydium AMM v4 pool state.
//!
//! Raydium's constant-product pools are the most-traded venue on Solana, and
//! `AmmInfo` is the account every one of them is described by. It is not an
//! Anchor program: there is no discriminator, the layout is a fixed 752-byte C
//! struct, and every field is little-endian. That makes it a good hand-written
//! counterpart to the generated decoders — the codegen path has nothing to work
//! from here.
//!
//! The offsets below were not taken from documentation. They were confirmed
//! against the live SOL/USDC pool
//! (`58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2`): the mint at offset 400 is
//! wrapped SOL, the one at 432 is USDC, the decimals at 32 and 40 are 9 and 6,
//! and the fee at 144/152 is 25/10000 — Raydium's published 0.25%. The fixture
//! that proves it is committed alongside the tests.

use std::any::Any;

use account_decoder_borsh_util::ZeroCopyReader;
use account_decoder_core::{
    AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities, DecoderIdentity,
    DecoderMetadata, EventKind,
};
use solana_sdk::pubkey::Pubkey;

/// The Raydium AMM v4 program.
pub const RAYDIUM_AMM_V4_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

/// Size of an `AmmInfo` account.
pub const AMM_INFO_SIZE: usize = 752;

/// Where the pubkey block starts. Everything before it is scalar state.
const PUBKEY_BASE: usize = 336;

/// The fee schedule a pool charges.
///
/// Each fee is a numerator over a denominator rather than a rate, because that
/// is how the program stores and applies them; converting to a float here would
/// lose the exactness the pool actually uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmmFees {
    /// Numerator of the minimum separate fee.
    pub min_separate_numerator: u64,
    /// Denominator of the minimum separate fee.
    pub min_separate_denominator: u64,
    /// Numerator of the trade fee.
    pub trade_fee_numerator: u64,
    /// Denominator of the trade fee.
    pub trade_fee_denominator: u64,
    /// Numerator of the PnL fee.
    pub pnl_numerator: u64,
    /// Denominator of the PnL fee.
    pub pnl_denominator: u64,
    /// Numerator of the swap fee.
    pub swap_fee_numerator: u64,
    /// Denominator of the swap fee.
    pub swap_fee_denominator: u64,
}

impl AmmFees {
    /// The trade fee as a fraction, or `None` if the denominator is zero.
    ///
    /// Returning `Option` rather than dividing by zero matters: the fields come
    /// off chain and nothing guarantees a sane denominator.
    pub fn trade_fee_rate(&self) -> Option<f64> {
        (self.trade_fee_denominator != 0)
            .then(|| self.trade_fee_numerator as f64 / self.trade_fee_denominator as f64)
    }

    /// The swap fee as a fraction, or `None` if the denominator is zero.
    pub fn swap_fee_rate(&self) -> Option<f64> {
        (self.swap_fee_denominator != 0)
            .then(|| self.swap_fee_numerator as f64 / self.swap_fee_denominator as f64)
    }
}

/// A Raydium AMM v4 pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmmInfo {
    /// Pool status word. 6 is the ordinary trading state.
    pub status: u64,
    /// PDA nonce for the pool authority.
    pub nonce: u64,
    /// Decimals of the base token.
    pub coin_decimals: u64,
    /// Decimals of the quote token.
    pub pc_decimals: u64,
    /// Pool state word.
    pub state: u64,
    /// Minimum order size on the paired order book.
    pub min_size: u64,
    /// Lot size for the base token.
    pub coin_lot_size: u64,
    /// Lot size for the quote token.
    pub pc_lot_size: u64,
    /// Scaling factor the program uses for its own arithmetic.
    pub sys_decimal_value: u64,
    /// The pool's fee schedule.
    pub fees: AmmFees,
    /// Vault holding the base token.
    pub coin_vault: Pubkey,
    /// Vault holding the quote token.
    pub pc_vault: Pubkey,
    /// Mint of the base token.
    pub coin_mint: Pubkey,
    /// Mint of the quote token.
    pub pc_mint: Pubkey,
    /// Mint of the pool's LP token.
    pub lp_mint: Pubkey,
    /// The pool's open orders account on the order book.
    pub open_orders: Pubkey,
    /// The order book market this pool is paired with.
    pub market: Pubkey,
    /// The order book program.
    pub market_program: Pubkey,
    /// Target orders account.
    pub target_orders: Pubkey,
    /// Withdraw queue account.
    pub withdraw_queue: Pubkey,
    /// LP token vault.
    pub lp_vault: Pubkey,
    /// Pool owner.
    pub owner: Pubkey,
}

impl AmmInfo {
    /// Whether the pool is in its ordinary trading state.
    pub fn is_trading(&self) -> bool {
        self.status == 6
    }
}

impl DecodedEvent for AmmInfo {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }

    fn event_type(&self) -> &'static str {
        "AmmInfo"
    }

    fn program_name(&self) -> &'static str {
        "raydium-amm-v4"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(AMM_INFO_SIZE)
    }
}

/// Decoder for Raydium AMM v4 pool accounts.
#[derive(Debug, Clone, Default)]
pub struct RaydiumAmmDecoder;

impl RaydiumAmmDecoder {
    /// Create a new decoder.
    pub fn new() -> Self {
        Self
    }

    /// Read the pubkey at `index` within the pubkey block.
    fn pubkey_at(data: &[u8], index: usize) -> DecodeResult<Pubkey> {
        let offset = PUBKEY_BASE + index * 32;
        let bytes: [u8; 32] = data
            .get(offset..offset + 32)
            .ok_or_else(|| DecodeError::insufficient_data(offset + 32, data.len()))?
            .try_into()
            .map_err(|_| DecodeError::invalid_format("pubkey slice was the wrong length"))?;
        Ok(Pubkey::new_from_array(bytes))
    }

    /// Decode a pool account.
    pub fn decode_amm_info(data: &[u8]) -> DecodeResult<AmmInfo> {
        if data.len() < AMM_INFO_SIZE {
            return Err(DecodeError::insufficient_data(AMM_INFO_SIZE, data.len()));
        }

        let mut reader = ZeroCopyReader::new(data);
        let mut next = || reader.read_u64().map_err(DecodeError::deserialization);

        let status = next()?;
        let nonce = next()?;
        let _order_num = next()?;
        let _depth = next()?;
        let coin_decimals = next()?;
        let pc_decimals = next()?;
        let state = next()?;
        let _reset_flag = next()?;
        let min_size = next()?;
        let _vol_max_cut_ratio = next()?;
        let _amount_wave = next()?;
        let coin_lot_size = next()?;
        let pc_lot_size = next()?;
        let _min_price_multiplier = next()?;
        let _max_price_multiplier = next()?;
        let sys_decimal_value = next()?;

        let fees = AmmFees {
            min_separate_numerator: next()?,
            min_separate_denominator: next()?,
            trade_fee_numerator: next()?,
            trade_fee_denominator: next()?,
            pnl_numerator: next()?,
            pnl_denominator: next()?,
            swap_fee_numerator: next()?,
            swap_fee_denominator: next()?,
        };

        Ok(AmmInfo {
            status,
            nonce,
            coin_decimals,
            pc_decimals,
            state,
            min_size,
            coin_lot_size,
            pc_lot_size,
            sys_decimal_value,
            fees,
            coin_vault: Self::pubkey_at(data, 0)?,
            pc_vault: Self::pubkey_at(data, 1)?,
            coin_mint: Self::pubkey_at(data, 2)?,
            pc_mint: Self::pubkey_at(data, 3)?,
            lp_mint: Self::pubkey_at(data, 4)?,
            open_orders: Self::pubkey_at(data, 5)?,
            market: Self::pubkey_at(data, 6)?,
            market_program: Self::pubkey_at(data, 7)?,
            target_orders: Self::pubkey_at(data, 8)?,
            withdraw_queue: Self::pubkey_at(data, 9)?,
            lp_vault: Self::pubkey_at(data, 10)?,
            owner: Self::pubkey_at(data, 11)?,
        })
    }
}

impl DecoderIdentity for RaydiumAmmDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("raydium-amm-v4", RAYDIUM_AMM_V4_PROGRAM_ID)
            .with_description("Raydium constant-product AMM pool state")
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities::default()
            .with_zero_copy()
            .with_account_types(vec!["AmmInfo"])
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AccountDecoder for RaydiumAmmDecoder {
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        Ok(Box::new(Self::decode_amm_info(data)?))
    }

    fn can_decode(&self, data: &[u8]) -> bool {
        data.len() == AMM_INFO_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_account_is_refused_rather_than_read_past() {
        let decoder = RaydiumAmmDecoder::new();
        for len in [0usize, 1, 335, 400, 751] {
            assert!(
                decoder.decode_account(&vec![0u8; len]).is_err(),
                "{len} bytes is not a pool account"
            );
            assert!(!decoder.can_decode(&vec![0u8; len]));
        }
        assert!(decoder.can_decode(&vec![0u8; AMM_INFO_SIZE]));
    }

    #[test]
    fn a_zero_denominator_does_not_divide_by_zero() {
        // Every field here comes off chain, so nothing guarantees a sane fee.
        let fees = AmmFees {
            min_separate_numerator: 0,
            min_separate_denominator: 0,
            trade_fee_numerator: 25,
            trade_fee_denominator: 0,
            pnl_numerator: 0,
            pnl_denominator: 0,
            swap_fee_numerator: 25,
            swap_fee_denominator: 0,
        };
        assert_eq!(fees.trade_fee_rate(), None);
        assert_eq!(fees.swap_fee_rate(), None);
    }
}
