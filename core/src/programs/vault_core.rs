//! Vault SDK - Client-side implementation
//!
//! This module provides client-side SDK functionality for the Jito Vault program.

use crate::{pod::{PodU16, PodU64}, programs::slot_toggle_core::SlotToggle, utils::{check_account, get_epoch, load_account, JitoAccount}};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

// ----------------------- CONSTANTS -----------------------

/// Maximum basis points (100%)
pub const MAX_BPS: u16 = 10_000;

/// Default slots per epoch
pub const DEFAULT_SLOTS_PER_EPOCH: u64 = 432_000;

// ----------------------- DISCRIMINATORS -----------------------

/// Discriminators for vault accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum VaultDiscriminator {
    Config = 1,
    Vault = 2,
    VaultNcnTicket = 3,
    VaultOperatorDelegation = 4,
    VaultNcnSlasherTicket = 5,
    VaultNcnSlasherOperatorTicket = 6,
    VaultStakerWithdrawalTicket = 7,
    VaultUpdateStateTracker = 8,
}

// ----------------------- CONFIG -----------------------

/// The vault configuration account for the vault program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Config {
    pub discriminator: PodU64,
    pub admin: Pubkey,
    pub restaking_program: Pubkey,
    pub epoch_length: PodU64,
    pub num_vaults: PodU64,
    pub deposit_withdrawal_fee_cap_bps: PodU16,
    pub fee_rate_of_change_bps: PodU16,
    pub fee_bump_bps: PodU16,
    pub program_fee_bps: PodU16,
    pub program_fee_wallet: Pubkey,
    pub fee_admin: Pubkey,
    pub bump: u8,
    pub reserved: [u8; 229],
}

impl JitoAccount for Config {
    const DISCRIMINATOR: u64 = VaultDiscriminator::Config as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"config";
    type SeedInputs = ();

    /// NA ()
    fn seeds(_: Self::SeedInputs) -> Vec<Vec<u8>> {
        vec![Self::SEED.to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(program_id: &Pubkey, account: &AccountInfo, expect_writable: bool, check_admin: Option<&AccountInfo>) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, ())?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
            check_admin,
        )?;

        if let Some(admin) = check_admin {
            if admin.key != &data_account.admin {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- BURN VAULT -----------------------

/// Empty PDA to send tokens to "burn"
pub struct BurnVault {}

impl JitoAccount for BurnVault {
    const DISCRIMINATOR: u64 = u64::MAX;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"burn_vault";
    type SeedInputs = Pubkey;

    /// base
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let base = inputs;
        vec![Self::SEED.to_vec(), base.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(_: &Pubkey, _: &AccountInfo, _: bool, _: Option<&AccountInfo>) -> Result<(), ProgramError> {
        msg!("Burn Vault is symbolic, not a data account");
        Err(ProgramError::InvalidAccountData)
    }

    fn is_initialized(&self) -> bool {
        false
    }
}

// ----------------------- DELEGATION STATE -----------------------

/// Tracks the delegation state for an operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct DelegationState {
    /// The amount of stake that is currently active on the operator
    pub staked_amount: PodU64,

    /// Any stake that was deactivated in the current epoch
    pub enqueued_for_cooldown_amount: PodU64,

    /// Any stake that was deactivated in the previous epoch,
    /// to be available for re-delegation in the current epoch + 1
    pub cooling_down_amount: PodU64,

    pub reserved: [u8; 256],
}

// ----------------------- VAULT NCN SLASHER OPERATOR TICKET -----------------------

/// Ticket tracking the amount an operator has been slashed by a slasher
/// for a given NCN and vault for a given epoch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VaultNcnSlasherOperatorTicket {
    pub discriminator: PodU64,
    pub vault: Pubkey,
    pub ncn: Pubkey,
    pub slasher: Pubkey,
    pub operator: Pubkey,
    pub epoch: PodU64,
    pub slashed: PodU64,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for VaultNcnSlasherOperatorTicket {
    const DISCRIMINATOR: u64 = VaultDiscriminator::VaultNcnSlasherOperatorTicket as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"vault_ncn_slasher_operator";
    type SeedInputs = (Pubkey, Pubkey, Pubkey, Pubkey, u64);

    /// vault, ncn, slasher, operator, epoch
    /// Note: check if epoch should be `le`
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (vault, ncn, slasher, operator, epoch) = inputs;
        vec![Self::SEED.to_vec(), vault.to_bytes().to_vec(), ncn.to_bytes().to_vec(), slasher.to_bytes().to_vec(), operator.to_bytes().to_vec(), epoch.to_le_bytes().to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(program_id: &Pubkey, account: &AccountInfo, expect_writable: bool, check_admin: Option<&AccountInfo>) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, (data_account.vault, data_account.ncn, data_account.slasher, data_account.operator, data_account.epoch.get()))?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
            check_admin,
        )?;

        if let Some(_) = check_admin {
            msg!("No admin in account");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}


// ----------------------- VAULT NCN SLASHER TICKET -----------------------

/// Ticket tracking the relationship between a vault, NCN, and slasher
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VaultNcnSlasherTicket {
    pub discriminator: PodU64,
    pub vault: Pubkey,
    pub ncn: Pubkey,
    pub slasher: Pubkey,
    pub max_slashable_per_epoch: PodU64,
    pub index: PodU64,
    pub state: SlotToggle,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for VaultNcnSlasherTicket {
    const DISCRIMINATOR: u64 = VaultDiscriminator::VaultNcnSlasherTicket as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"vault_slasher_ticket";
    type SeedInputs = (Pubkey, Pubkey, Pubkey);

    /// vault, ncn, slasher
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (vault, ncn, slasher) = inputs;
        vec![Self::SEED.to_vec(), vault.to_bytes().to_vec(), ncn.to_bytes().to_vec(), slasher.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(program_id: &Pubkey, account: &AccountInfo, expect_writable: bool, check_admin: Option<&AccountInfo>) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, (data_account.vault, data_account.ncn, data_account.slasher))?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
            check_admin,
        )?;

        if let Some(_) = check_admin {
            msg!("No admin in account");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- VAULT NCN TICKET -----------------------

/// Ticket tracking the relationship between a vault and NCN
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VaultNcnTicket {
    pub discriminator: PodU64,
    pub vault: Pubkey,
    pub ncn: Pubkey,
    pub index: PodU64,
    pub state: SlotToggle,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for VaultNcnTicket {
    const DISCRIMINATOR: u64 = VaultDiscriminator::VaultNcnTicket as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"vault_ncn_ticket";
    type SeedInputs = (Pubkey, Pubkey);

    /// vault, ncn
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (vault, ncn) = inputs;
        vec![Self::SEED.to_vec(), vault.to_bytes().to_vec(), ncn.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(program_id: &Pubkey, account: &AccountInfo, expect_writable: bool, check_admin: Option<&AccountInfo>) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, (data_account.vault, data_account.ncn))?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
            check_admin,
        )?;

        if let Some(_) = check_admin {
            msg!("No admin in account");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- VAULT OPERATOR DELEGATION -----------------------

/// Tracks delegation between a vault and operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VaultOperatorDelegation {
    pub discriminator: PodU64,
    pub vault: Pubkey,
    pub operator: Pubkey,
    pub delegation_state: DelegationState,
    pub last_update_slot: PodU64,
    pub index: PodU64,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for VaultOperatorDelegation {
    const DISCRIMINATOR: u64 = VaultDiscriminator::VaultOperatorDelegation as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"vault_operator_delegation";
    type SeedInputs = (Pubkey, Pubkey);

    /// vault, operator
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (vault, operator) = inputs;
        vec![Self::SEED.to_vec(), vault.to_bytes().to_vec(), operator.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(program_id: &Pubkey, account: &AccountInfo, expect_writable: bool, check_admin: Option<&AccountInfo>) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, (data_account.vault, data_account.operator))?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
            check_admin,
        )?;

        if let Some(_) = check_admin {
            msg!("No admin in account");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- VAULT STAKER WITHDRAWAL TICKET -----------------------

/// Ticket for tracking staker withdrawal requests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VaultStakerWithdrawalTicket {
    pub discriminator: PodU64,
    pub vault: Pubkey,
    pub staker: Pubkey,
    pub base: Pubkey,
    pub vrt_amount: PodU64,
    pub slot_unstaked: PodU64,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for VaultStakerWithdrawalTicket {
    const DISCRIMINATOR: u64 = VaultDiscriminator::VaultStakerWithdrawalTicket as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"vault_staker_withdrawal_ticket";
    type SeedInputs = (Pubkey, Pubkey);

    /// vault, base
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (vault, base) = inputs;
        vec![Self::SEED.to_vec(), vault.to_bytes().to_vec(), base.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(program_id: &Pubkey, account: &AccountInfo, expect_writable: bool, check_admin: Option<&AccountInfo>) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, (data_account.vault, data_account.base))?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
            check_admin,
        )?;

        if let Some(_) = check_admin {
            msg!("No admin in account");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- VAULT UPDATE STATE TRACKER -----------------------

/// Tracks the state updates for a vault
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VaultUpdateStateTracker {
    pub discriminator: PodU64,
    pub vault: Pubkey,
    pub ncn_epoch: PodU64,
    pub last_updated_index: PodU64,
    pub delegation_state: DelegationState,
    pub withdrawal_allocation_method: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for VaultUpdateStateTracker {
    const DISCRIMINATOR: u64 = VaultDiscriminator::VaultUpdateStateTracker as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"vault_update_state_tracker";
    type SeedInputs = (Pubkey, u64);

    /// vault, ncn_epoch
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (vault, ncn_epoch) = inputs;
        vec![Self::SEED.to_vec(), vault.to_bytes().to_vec(), ncn_epoch.to_le_bytes().to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(_: &Pubkey, _: &AccountInfo, _: bool, _: Option<&AccountInfo>) -> Result<(), ProgramError> {

        msg!("No bump in account");
        Err(ProgramError::InvalidAccountData)

        // let data = account.data.borrow();
        // let data_account = unsafe { load_account::<Self>(&data)? };
        // let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, (data_account.vault, data_account.ncn_epoch.get()))?;

        // check_account(
        //     program_id,
        //     account,
        //     &expected_pda,
        //     Some(Self::DISCRIMINATOR),
        //     expect_writable,
        //     check_admin,
        // )?;

        // if let Some(_) = check_admin {
        //     msg!("No admin in account");
        //     return Err(ProgramError::InvalidAccountData);
        // }

        // Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- VAULT -----------------------

/// The main vault account
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Vault {
    pub discriminator: PodU64,
    pub base: Pubkey,
    pub vrt_mint: Pubkey,
    pub supported_mint: Pubkey,
    pub vrt_supply: PodU64,
    pub tokens_deposited: PodU64,
    pub deposit_capacity: PodU64,
    pub delegation_state: DelegationState,
    pub additional_assets_need_unstaking: PodU64,
    pub vrt_enqueued_for_cooldown_amount: PodU64,
    pub vrt_cooling_down_amount: PodU64,
    pub vrt_ready_to_claim_amount: PodU64,
    pub admin: Pubkey,
    pub delegation_admin: Pubkey,
    pub operator_admin: Pubkey,
    pub ncn_admin: Pubkey,
    pub slasher_admin: Pubkey,
    pub capacity_admin: Pubkey,
    pub fee_admin: Pubkey,
    pub delegate_asset_admin: Pubkey,
    pub fee_wallet: Pubkey,
    pub mint_burn_admin: Pubkey,
    pub metadata_admin: Pubkey,
    pub vault_index: PodU64,
    pub ncn_count: PodU64,
    pub operator_count: PodU64,
    pub slasher_count: PodU64,
    pub last_fee_change_slot: PodU64,
    pub last_full_state_update_slot: PodU64,
    pub deposit_fee_bps: PodU16,
    pub withdrawal_fee_bps: PodU16,
    pub next_withdrawal_fee_bps: PodU16,
    pub reward_fee_bps: PodU16,
    pub program_fee_bps: PodU16,
    pub bump: u8,
    pub is_paused: u8,
    pub last_start_state_update_slot: PodU64,
    pub reserved: [u8; 251],
}

impl JitoAccount for Vault {
    const DISCRIMINATOR: u64 = VaultDiscriminator::Vault as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"vault";
    type SeedInputs = Pubkey;

    /// base
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let base = inputs;
        vec![Self::SEED.to_vec(), base.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(program_id: &Pubkey, inputs: Self::SeedInputs) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(program_id: &Pubkey, bump: u8, inputs: Self::SeedInputs) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(program_id: &Pubkey, account: &AccountInfo, expect_writable: bool, check_admin: Option<&AccountInfo>) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, data_account.base)?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
            check_admin
        )?;

        if let Some(admin) = check_admin {
            if admin.key != &data_account.admin {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

impl Vault {
    pub fn get_epoch(slot: u64, epoch_length: u64) -> Result<u64, ProgramError> {
        get_epoch(slot, epoch_length)
    }

    pub fn is_update_needed(&self, slot: u64, epoch_length: u64) -> Result<bool, ProgramError> {
        let last_updated_epoch = Self::get_epoch(self.last_full_state_update_slot.into(), epoch_length)?;
        let current_epoch = Self::get_epoch(slot, epoch_length)?;

        Ok(last_updated_epoch < current_epoch)
    }
}
