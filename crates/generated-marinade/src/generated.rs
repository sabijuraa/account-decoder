//! Generated decoder for #program_name v#version
//!
//! This file was automatically generated. Do not edit manually.
use account_decoder_core::{
    AccountDecoder, DecodeError, DecodeResult, DecodedEvent, DecoderCapabilities,
    DecoderIdentity, DecoderMetadata, EventKind, InstructionDecoder,
};
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use std::any::Any;
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct SplitStakeAccountInfo {
    pub account: Pubkey,
    pub index: u32,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct U64ValueChange {
    pub old: u64,
    pub new: u64,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct U32ValueChange {
    pub old: u32,
    pub new: u32,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct FeeValueChange {
    pub old: Fee,
    pub new: Fee,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct FeeCentsValueChange {
    pub old: FeeCents,
    pub new: FeeCents,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct PubkeyValueChange {
    pub old: Pubkey,
    pub new: Pubkey,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct BoolValueChange {
    pub old: bool,
    pub new: bool,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ChangeAuthorityData {
    pub admin: Option<Pubkey>,
    pub validator_manager: Option<Pubkey>,
    pub operational_sol_account: Option<Pubkey>,
    pub treasury_msol_account: Option<Pubkey>,
    pub pause_authority: Option<Pubkey>,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ConfigLpParams {
    pub min_fee: Option<Fee>,
    pub max_fee: Option<Fee>,
    pub liquidity_target: Option<u64>,
    pub treasury_cut: Option<Fee>,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ConfigMarinadeParams {
    pub rewards_fee: Option<Fee>,
    pub slots_for_stake_delta: Option<u64>,
    pub min_stake: Option<u64>,
    pub min_deposit: Option<u64>,
    pub min_withdraw: Option<u64>,
    pub staking_sol_cap: Option<u64>,
    pub liquidity_sol_cap: Option<u64>,
    pub withdraw_stake_account_enabled: Option<bool>,
    pub delayed_unstake_fee: Option<FeeCents>,
    pub withdraw_stake_account_fee: Option<FeeCents>,
    pub max_stake_moved_per_epoch: Option<Fee>,
    pub deposit_sol_fee: Option<FeeCents>,
    pub deposit_stake_account_fee: Option<FeeCents>,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct InitializeData {
    pub admin_authority: Pubkey,
    pub validator_manager_authority: Pubkey,
    pub min_stake: u64,
    pub rewards_fee: Fee,
    pub liq_pool: LiqPoolInitializeData,
    pub additional_stake_record_space: u32,
    pub additional_validator_record_space: u32,
    pub slots_for_stake_delta: u64,
    pub pause_authority: Pubkey,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct LiqPoolInitializeData {
    pub lp_liquidity_target: u64,
    pub lp_max_fee: Fee,
    pub lp_min_fee: Fee,
    pub lp_treasury_cut: Fee,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct Fee {
    pub basis_points: u32,
}
///FeeCents, same as Fee but / 1_000_000 instead of 10_000
///1 FeeCent = 0.0001%, 10_000 FeeCent = 1%, 1_000_000 FeeCent = 100%
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct FeeCents {
    pub bp_cents: u32,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct LiqPool {
    pub lp_mint: Pubkey,
    pub lp_mint_authority_bump_seed: u8,
    pub sol_leg_bump_seed: u8,
    pub msol_leg_authority_bump_seed: u8,
    pub msol_leg: Pubkey,
    ///Liquidity target. If the Liquidity reach this amount, the fee reaches lp_min_discount_fee
    pub lp_liquidity_target: u64,
    ///Liquidity pool max fee
    pub lp_max_fee: Fee,
    ///SOL/mSOL Liquidity pool min fee
    pub lp_min_fee: Fee,
    ///Treasury cut
    pub treasury_cut: Fee,
    pub lp_supply: u64,
    pub lent_from_sol_leg: u64,
    pub liquidity_sol_cap: u64,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct List {
    pub account: Pubkey,
    pub item_size: u32,
    pub count: u32,
    pub reserved1: Pubkey,
    pub reserved2: u32,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct StakeRecord {
    pub stake_account: Pubkey,
    pub last_update_delegated_lamports: u64,
    pub last_update_epoch: u64,
    pub is_emergency_unstaking: bool,
    pub last_update_status: StakeStatus,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct StakeList {}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct StakeSystem {
    pub stake_list: List,
    pub delayed_unstake_cooling_down: u64,
    pub stake_deposit_bump_seed: u8,
    pub stake_withdraw_bump_seed: u8,
    ///set by admin, how much slots before the end of the epoch, stake-delta can start
    pub slots_for_stake_delta: u64,
    ///Marks the start of stake-delta operations, meaning that if somebody starts a delayed-unstake ticket
    ///after this var is set with epoch_num the ticket will have epoch_created = current_epoch+1
    ///(the user must wait one more epoch, because their unstake-delta will be execute in this epoch)
    pub last_stake_delta_epoch: u64,
    pub min_stake: u64,
    ///can be set by validator-manager-auth to allow a second run of stake-delta to stake late stakers in the last minute of the epoch
    ///so we maximize user's rewards
    pub extra_stake_delta_runs: u32,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ValidatorRecord {
    ///Validator vote pubkey
    pub validator_account: Pubkey,
    ///Validator total balance in lamports
    pub active_balance: u64,
    pub score: u32,
    pub last_stake_delta_epoch: u64,
    pub duplication_flag_bump_seed: u8,
    pub delinquent_upgrader_active_balance: u64,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ValidatorList {}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ValidatorSystem {
    pub validator_list: List,
    pub manager_authority: Pubkey,
    pub total_validator_score: u32,
    ///sum of all active lamports staked
    pub total_active_balance: u64,
    ///DEPRECATED, no longer used
    pub auto_add_validator_enabled: u8,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub enum DelinquentUpgraderState {
    IteratingStakes,
    IteratingValidators,
    Done,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub enum StakeStatus {
    Unknown,
    Active,
    Deactivating,
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct TicketAccountData {
    pub state_address: Pubkey,
    pub beneficiary: Pubkey,
    pub lamports_amount: u64,
    pub created_epoch: u64,
}
impl DecodedEvent for TicketAccountData {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }
    fn event_type(&self) -> &'static str {
        "TicketAccountData"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct State {
    pub msol_mint: Pubkey,
    pub admin_authority: Pubkey,
    pub operational_sol_account: Pubkey,
    pub treasury_msol_account: Pubkey,
    pub reserve_bump_seed: u8,
    pub msol_mint_authority_bump_seed: u8,
    pub rent_exempt_for_token_acc: u64,
    pub reward_fee: Fee,
    pub stake_system: StakeSystem,
    pub validator_system: ValidatorSystem,
    pub liq_pool: LiqPool,
    pub available_reserve_balance: u64,
    pub msol_supply: u64,
    pub msol_price: u64,
    ///count tickets for delayed-unstake
    pub circulating_ticket_count: u64,
    ///total lamports amount of generated and not claimed yet tickets
    pub circulating_ticket_balance: u64,
    pub lent_from_reserve: u64,
    pub min_deposit: u64,
    pub min_withdraw: u64,
    pub staking_sol_cap: u64,
    pub emergency_cooling_down: u64,
    ///emergency pause
    pub pause_authority: Pubkey,
    pub paused: bool,
    pub delayed_unstake_fee: FeeCents,
    pub withdraw_stake_account_fee: FeeCents,
    pub withdraw_stake_account_enabled: bool,
    pub last_stake_move_epoch: u64,
    pub stake_moved: u64,
    pub max_stake_moved_per_epoch: Fee,
    pub delinquent_upgrader: DelinquentUpgraderState,
    pub deposit_sol_fee: FeeCents,
    pub deposit_stake_account_fee: FeeCents,
}
impl DecodedEvent for State {
    fn event_kind(&self) -> EventKind {
        EventKind::Account
    }
    fn event_type(&self) -> &'static str {
        "State"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
/// Account decoder for #program_name.
#[derive(Debug, Clone)]
pub struct MarinadeFinanceAccountDecoder {
    program_id: Pubkey,
}
impl MarinadeFinanceAccountDecoder {
    /// Create a new decoder with the given program ID.
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }
}
impl DecoderIdentity for MarinadeFinanceAccountDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("marinade_finance", self.program_id)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl AccountDecoder for MarinadeFinanceAccountDecoder {
    fn decode_account(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        if data.len() < 8 {
            return Err(DecodeError::insufficient_data(8, data.len()));
        }
        let discriminator: [u8; 8] = data[..8].try_into().unwrap();
        match discriminator {
            [133u8, 77u8, 18u8, 98u8, 211u8, 1u8, 231u8, 3u8] => {
                let mut cursor = &data[8..];
                let value = TicketAccountData::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [216u8, 146u8, 107u8, 94u8, 104u8, 75u8, 182u8, 177u8] => {
                let mut cursor = &data[8..];
                let value = State::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            _ => Err(DecodeError::unknown_discriminator(&discriminator)),
        }
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct InitializeInstruction {
    pub data: InitializeData,
}
impl DecodedEvent for InitializeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "initialize"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ChangeAuthorityInstruction {
    pub data: ChangeAuthorityData,
}
impl DecodedEvent for ChangeAuthorityInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "changeAuthority"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct AddValidatorInstruction {
    pub score: u32,
}
impl DecodedEvent for AddValidatorInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "addValidator"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RemoveValidatorInstruction {
    pub index: u32,
    pub validator_vote: Pubkey,
}
impl DecodedEvent for RemoveValidatorInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "removeValidator"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct SetValidatorScoreInstruction {
    pub index: u32,
    pub validator_vote: Pubkey,
    pub score: u32,
}
impl DecodedEvent for SetValidatorScoreInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "setValidatorScore"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ConfigValidatorSystemInstruction {
    pub extra_runs: u32,
}
impl DecodedEvent for ConfigValidatorSystemInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "configValidatorSystem"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct DepositInstruction {
    pub lamports: u64,
}
impl DecodedEvent for DepositInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "deposit"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct DepositStakeAccountInstruction {
    pub validator_index: u32,
}
impl DecodedEvent for DepositStakeAccountInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "depositStakeAccount"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct LiquidUnstakeInstruction {
    pub msol_amount: u64,
}
impl DecodedEvent for LiquidUnstakeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "liquidUnstake"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct AddLiquidityInstruction {
    pub lamports: u64,
}
impl DecodedEvent for AddLiquidityInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "addLiquidity"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct RemoveLiquidityInstruction {
    pub tokens: u64,
}
impl DecodedEvent for RemoveLiquidityInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "removeLiquidity"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ConfigLpInstruction {
    pub params: ConfigLpParams,
}
impl DecodedEvent for ConfigLpInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "configLp"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ConfigMarinadeInstruction {
    pub params: ConfigMarinadeParams,
}
impl DecodedEvent for ConfigMarinadeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "configMarinade"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct OrderUnstakeInstruction {
    pub msol_amount: u64,
}
impl DecodedEvent for OrderUnstakeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "orderUnstake"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ClaimInstruction {}
impl DecodedEvent for ClaimInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "claim"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct StakeReserveInstruction {
    pub validator_index: u32,
}
impl DecodedEvent for StakeReserveInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "stakeReserve"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct UpdateActiveInstruction {
    pub stake_index: u32,
    pub validator_index: u32,
}
impl DecodedEvent for UpdateActiveInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "updateActive"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct UpdateDeactivatedInstruction {
    pub stake_index: u32,
    pub validator_index: u32,
}
impl DecodedEvent for UpdateDeactivatedInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "updateDeactivated"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct DeactivateStakeInstruction {
    pub stake_index: u32,
    pub validator_index: u32,
}
impl DecodedEvent for DeactivateStakeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "deactivateStake"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct EmergencyUnstakeInstruction {
    pub stake_index: u32,
    pub validator_index: u32,
}
impl DecodedEvent for EmergencyUnstakeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "emergencyUnstake"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct PartialUnstakeInstruction {
    pub stake_index: u32,
    pub validator_index: u32,
    pub desired_unstake_amount: u64,
}
impl DecodedEvent for PartialUnstakeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "partialUnstake"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct MergeStakesInstruction {
    pub destination_stake_index: u32,
    pub source_stake_index: u32,
    pub validator_index: u32,
}
impl DecodedEvent for MergeStakesInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "mergeStakes"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct CreateCanonicalStakeInstruction {
    pub source_stake_index: u32,
    pub validator_index: u32,
}
impl DecodedEvent for CreateCanonicalStakeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "createCanonicalStake"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct PauseInstruction {}
impl DecodedEvent for PauseInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "pause"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ResumeInstruction {}
impl DecodedEvent for ResumeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "resume"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct WithdrawStakeAccountInstruction {
    pub stake_index: u32,
    pub validator_index: u32,
    pub msol_amount: u64,
    pub beneficiary: Pubkey,
}
impl DecodedEvent for WithdrawStakeAccountInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "withdrawStakeAccount"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ReallocValidatorListInstruction {
    pub capacity: u32,
}
impl DecodedEvent for ReallocValidatorListInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "reallocValidatorList"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct ReallocStakeListInstruction {
    pub capacity: u32,
}
impl DecodedEvent for ReallocStakeListInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "reallocStakeList"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[derive(BorshDeserialize, Debug, Clone, PartialEq)]
pub struct FinalizeDelinquentUpgradeInstruction {
    pub max_validators: u32,
}
impl DecodedEvent for FinalizeDelinquentUpgradeInstruction {
    fn event_kind(&self) -> EventKind {
        EventKind::Instruction
    }
    fn event_type(&self) -> &'static str {
        "finalizeDelinquentUpgrade"
    }
    fn program_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
/// Instruction decoder for #program_name.
#[derive(Debug, Clone)]
pub struct MarinadeFinanceInstructionDecoder {
    program_id: Pubkey,
}
impl MarinadeFinanceInstructionDecoder {
    /// Create a new decoder with the given program ID.
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }
}
impl DecoderIdentity for MarinadeFinanceInstructionDecoder {
    fn metadata(&self) -> DecoderMetadata {
        DecoderMetadata::new("marinade_finance", self.program_id)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl InstructionDecoder for MarinadeFinanceInstructionDecoder {
    fn decode_instruction(&self, data: &[u8]) -> DecodeResult<Box<dyn DecodedEvent>> {
        if data.len() < 8 {
            return Err(DecodeError::insufficient_data(8, data.len()));
        }
        let discriminator: [u8; 8] = data[..8].try_into().unwrap();
        match discriminator {
            [175u8, 175u8, 109u8, 31u8, 13u8, 152u8, 155u8, 237u8] => {
                let mut cursor = &data[8..];
                let value = InitializeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [50u8, 106u8, 66u8, 104u8, 99u8, 118u8, 145u8, 88u8] => {
                let mut cursor = &data[8..];
                let value = ChangeAuthorityInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [250u8, 113u8, 53u8, 54u8, 141u8, 117u8, 215u8, 185u8] => {
                let mut cursor = &data[8..];
                let value = AddValidatorInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [25u8, 96u8, 211u8, 155u8, 161u8, 14u8, 168u8, 188u8] => {
                let mut cursor = &data[8..];
                let value = RemoveValidatorInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [101u8, 41u8, 206u8, 33u8, 216u8, 111u8, 25u8, 78u8] => {
                let mut cursor = &data[8..];
                let value = SetValidatorScoreInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [27u8, 90u8, 97u8, 209u8, 17u8, 115u8, 7u8, 40u8] => {
                let mut cursor = &data[8..];
                let value = ConfigValidatorSystemInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [242u8, 35u8, 198u8, 137u8, 82u8, 225u8, 242u8, 182u8] => {
                let mut cursor = &data[8..];
                let value = DepositInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [110u8, 130u8, 115u8, 41u8, 164u8, 102u8, 2u8, 59u8] => {
                let mut cursor = &data[8..];
                let value = DepositStakeAccountInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [30u8, 30u8, 119u8, 240u8, 191u8, 227u8, 12u8, 16u8] => {
                let mut cursor = &data[8..];
                let value = LiquidUnstakeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [181u8, 157u8, 89u8, 67u8, 143u8, 182u8, 52u8, 72u8] => {
                let mut cursor = &data[8..];
                let value = AddLiquidityInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [80u8, 85u8, 209u8, 72u8, 24u8, 206u8, 177u8, 108u8] => {
                let mut cursor = &data[8..];
                let value = RemoveLiquidityInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [10u8, 24u8, 168u8, 119u8, 86u8, 48u8, 225u8, 17u8] => {
                let mut cursor = &data[8..];
                let value = ConfigLpInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [67u8, 3u8, 34u8, 114u8, 190u8, 185u8, 17u8, 62u8] => {
                let mut cursor = &data[8..];
                let value = ConfigMarinadeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [97u8, 167u8, 144u8, 107u8, 117u8, 190u8, 128u8, 36u8] => {
                let mut cursor = &data[8..];
                let value = OrderUnstakeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [62u8, 198u8, 214u8, 193u8, 213u8, 159u8, 108u8, 210u8] => {
                let mut cursor = &data[8..];
                let value = ClaimInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [87u8, 217u8, 23u8, 179u8, 205u8, 25u8, 113u8, 129u8] => {
                let mut cursor = &data[8..];
                let value = StakeReserveInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [4u8, 67u8, 81u8, 64u8, 136u8, 245u8, 93u8, 152u8] => {
                let mut cursor = &data[8..];
                let value = UpdateActiveInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [16u8, 232u8, 131u8, 115u8, 156u8, 100u8, 239u8, 50u8] => {
                let mut cursor = &data[8..];
                let value = UpdateDeactivatedInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [165u8, 158u8, 229u8, 97u8, 168u8, 220u8, 187u8, 225u8] => {
                let mut cursor = &data[8..];
                let value = DeactivateStakeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [123u8, 69u8, 168u8, 195u8, 183u8, 213u8, 199u8, 214u8] => {
                let mut cursor = &data[8..];
                let value = EmergencyUnstakeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [55u8, 241u8, 205u8, 221u8, 45u8, 114u8, 205u8, 163u8] => {
                let mut cursor = &data[8..];
                let value = PartialUnstakeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [216u8, 36u8, 141u8, 225u8, 243u8, 78u8, 125u8, 237u8] => {
                let mut cursor = &data[8..];
                let value = MergeStakesInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [109u8, 2u8, 136u8, 224u8, 147u8, 51u8, 182u8, 232u8] => {
                let mut cursor = &data[8..];
                let value = CreateCanonicalStakeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [211u8, 22u8, 221u8, 251u8, 74u8, 121u8, 193u8, 47u8] => {
                let mut cursor = &data[8..];
                let value = PauseInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [1u8, 166u8, 51u8, 170u8, 127u8, 32u8, 141u8, 206u8] => {
                let mut cursor = &data[8..];
                let value = ResumeInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [211u8, 85u8, 184u8, 65u8, 183u8, 177u8, 233u8, 217u8] => {
                let mut cursor = &data[8..];
                let value = WithdrawStakeAccountInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [215u8, 59u8, 218u8, 133u8, 93u8, 138u8, 60u8, 123u8] => {
                let mut cursor = &data[8..];
                let value = ReallocValidatorListInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [12u8, 36u8, 124u8, 27u8, 128u8, 96u8, 85u8, 199u8] => {
                let mut cursor = &data[8..];
                let value = ReallocStakeListInstruction::deserialize(&mut cursor)?;
                Ok(Box::new(value))
            }
            [173u8, 8u8, 90u8, 193u8, 222u8, 52u8, 169u8, 144u8] => {
                let mut cursor = &data[8..];
                let value = FinalizeDelinquentUpgradeInstruction::deserialize(
                    &mut cursor,
                )?;
                Ok(Box::new(value))
            }
            _ => Err(DecodeError::unknown_discriminator(&discriminator)),
        }
    }
}
