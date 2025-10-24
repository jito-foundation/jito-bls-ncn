//! Restaking SDK - Client-side implementation
//!
//! This module provides client-side SDK functionality for the Jito Restaking program.

use crate::{
    pod::{PodU16, PodU64}, programs::slot_toggle_core::SlotToggle, utils::{check_account, load_account, JitoAccount}
};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::{pubkey, Pubkey};

// ----------------------- CONSTANTS -----------------------

/// Maximum basis points (100%)
pub const MAX_BPS: u16 = 10_000;

/// Default slots per epoch
pub const DEFAULT_SLOTS_PER_EPOCH: u64 = 432_000;

pub fn id() -> Pubkey {
    pubkey!("RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q")
}

// ----------------------- DISCRIMINATORS -----------------------

/// Discriminators for restaking accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum RestakingDiscriminator {
    Config = 1,
    Ncn = 2,
    Operator = 3,
    NcnOperatorState = 4,
    OperatorVaultTicket = 5,
    NcnVaultTicket = 6,
    NcnVaultSlasherTicket = 7,
}

// ----------------------- CONFIG -----------------------

/// The global configuration account for the restaking program
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Config {
    pub discriminator: PodU64,
    pub admin: Pubkey,
    pub vault_program: Pubkey,
    pub ncn_count: PodU64,
    pub operator_count: PodU64,
    pub epoch_length: PodU64,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for Config {
    const DISCRIMINATOR: u64 = RestakingDiscriminator::Config as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"config";
    type SeedInputs = ();

    /// NA ()
    fn seeds(_: Self::SeedInputs) -> Vec<Vec<u8>> {
        vec![Self::SEED.to_vec()]
    }

    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(program_id, data_account.bump, ())?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- NCN -----------------------

/// The NCN manages the operators, vaults, and slashers associated with a network
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Ncn {
    pub discriminator: PodU64,
    pub base: Pubkey,
    pub admin: Pubkey,
    pub operator_admin: Pubkey,
    pub vault_admin: Pubkey,
    pub slasher_admin: Pubkey,
    pub delegate_admin: Pubkey,
    pub metadata_admin: Pubkey,
    pub weight_table_admin: Pubkey,
    pub ncn_program_admin: Pubkey,
    pub index: PodU64,
    pub operator_count: PodU64,
    pub vault_count: PodU64,
    pub slasher_count: PodU64,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for Ncn {
    const DISCRIMINATOR: u64 = RestakingDiscriminator::Ncn as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"ncn";
    type SeedInputs = Pubkey;

    /// base
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let base = inputs;
        vec![Self::SEED.to_vec(), base.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) =
            Self::create_program_address(program_id, data_account.bump, data_account.base)?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- OPERATOR -----------------------

/// The Operator account stores global information for a particular operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Operator {
    pub discriminator: PodU64,
    pub base: Pubkey,
    pub admin: Pubkey,
    pub ncn_admin: Pubkey,
    pub vault_admin: Pubkey,
    pub delegate_admin: Pubkey,
    pub metadata_admin: Pubkey,
    pub voter: Pubkey,
    pub index: PodU64,
    pub ncn_count: PodU64,
    pub vault_count: PodU64,
    pub operator_fee_bps: PodU16,
    pub bump: u8,
    pub reserved_space: [u8; 261],
}

impl JitoAccount for Operator {
    const DISCRIMINATOR: u64 = RestakingDiscriminator::Operator as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"operator";
    type SeedInputs = Pubkey;

    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let base = inputs;
        vec![Self::SEED.to_vec(), base.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) =
            Self::create_program_address(program_id, data_account.bump, data_account.base)?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- NCN OPERATOR STATE -----------------------

/// Tracks the relationship between an NCN and operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NcnOperatorState {
    pub discriminator: PodU64,
    pub ncn: Pubkey,
    pub operator: Pubkey,
    pub index: PodU64,
    pub ncn_opt_in_state: SlotToggle,
    pub operator_opt_in_state: SlotToggle,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for NcnOperatorState {
    const DISCRIMINATOR: u64 = RestakingDiscriminator::NcnOperatorState as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"ncn_operator_state";
    type SeedInputs = (Pubkey, Pubkey);

    /// ncn, operator
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (ncn, operator) = inputs;
        vec![
            Self::SEED.to_vec(),
            ncn.to_bytes().to_vec(),
            operator.to_bytes().to_vec(),
        ]
    }

    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(
            program_id,
            data_account.bump,
            (data_account.ncn, data_account.operator),
        )?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- OPERATOR VAULT TICKET -----------------------

/// Tracks the relationship between an operator and vault
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct OperatorVaultTicket {
    pub discriminator: PodU64,
    pub operator: Pubkey,
    pub vault: Pubkey,
    pub index: PodU64,
    pub state: SlotToggle,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for OperatorVaultTicket {
    const DISCRIMINATOR: u64 = RestakingDiscriminator::OperatorVaultTicket as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"operator_vault_ticket";
    type SeedInputs = (Pubkey, Pubkey);

    /// operator, vault
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (operator, vault) = inputs;
        vec![
            Self::SEED.to_vec(),
            operator.to_bytes().to_vec(),
            vault.to_bytes().to_vec(),
        ]
    }

    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(
            program_id,
            data_account.bump,
            (data_account.operator, data_account.vault),
        )?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- NCN VAULT TICKET -----------------------

/// Tracks the relationship between an NCN and vault
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NcnVaultTicket {
    pub discriminator: PodU64,
    pub ncn: Pubkey,
    pub vault: Pubkey,
    pub index: PodU64,
    pub state: SlotToggle,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for NcnVaultTicket {
    const DISCRIMINATOR: u64 = RestakingDiscriminator::NcnVaultTicket as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"ncn_vault_ticket";
    type SeedInputs = (Pubkey, Pubkey);

    /// ncn, vault
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (ncn, vault) = inputs;
        vec![
            Self::SEED.to_vec(),
            ncn.to_bytes().to_vec(),
            vault.to_bytes().to_vec(),
        ]
    }

    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(
            program_id,
            data_account.bump,
            (data_account.ncn, data_account.vault),
        )?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

// ----------------------- NCN VAULT SLASHER TICKET -----------------------

/// Tracks the relationship between an NCN, vault, and slasher
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct NcnVaultSlasherTicket {
    pub discriminator: PodU64,
    pub ncn: Pubkey,
    pub vault: Pubkey,
    pub slasher: Pubkey,
    pub max_slashable_per_epoch: PodU64,
    pub index: PodU64,
    pub state: SlotToggle,
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoAccount for NcnVaultSlasherTicket {
    const DISCRIMINATOR: u64 = RestakingDiscriminator::NcnVaultSlasherTicket as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"ncn_slasher_ticket";
    type SeedInputs = (Pubkey, Pubkey, Pubkey);

    /// ncn, vault, slasher
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let (ncn, vault, slasher) = inputs;
        vec![
            Self::SEED.to_vec(),
            ncn.to_bytes().to_vec(),
            vault.to_bytes().to_vec(),
            slasher.to_bytes().to_vec(),
        ]
    }

    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) = Self::create_program_address(
            program_id,
            data_account.bump,
            (data_account.ncn, data_account.vault, data_account.slasher),
        )?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}
