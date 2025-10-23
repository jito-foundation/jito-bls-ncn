//! Restaking SDK - Client-side implementation
//!
//! This module provides client-side SDK functionality for the Jito Restaking program.

use crate::{pod::{PodU16, PodU64}, utils::{JitoDataLen, JitoDiscriminator, JitoInitialized}};
use solana_pubkey::Pubkey;

// ----------------------- CONSTANTS -----------------------

/// Maximum basis points (100%)
pub const MAX_BPS: u16 = 10_000;

/// Default slots per epoch
pub const DEFAULT_SLOTS_PER_EPOCH: u64 = 432_000;

// ----------------------- DISCRIMINATORS -----------------------

/// Discriminators for restaking accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
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

impl JitoDiscriminator for Config {
    const DISCRIMINATOR: u8 = RestakingDiscriminator::Config as u8;
}

impl JitoDataLen for Config {
    const LEN: usize = std::mem::size_of::<Config>();
}

impl JitoInitialized for Config {
    fn is_initialized(&self) -> bool {
        self.bump != 0
    }
}

impl Config {
    pub const DISCRIMINATOR: u8 = RestakingDiscriminator::Config as u8;
    pub const SEED: &'static [u8] = b"config";

    pub fn find_program_address(program_id: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = vec![Self::SEED.to_vec()];
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
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

impl JitoDiscriminator for Ncn {
    const DISCRIMINATOR: u8 = RestakingDiscriminator::Ncn as u8;
}

impl JitoDataLen for Ncn {
    const LEN: usize = std::mem::size_of::<Ncn>();
}

impl JitoInitialized for Ncn {
    fn is_initialized(&self) -> bool {
        self.bump != 0
    }
}

impl Ncn {
    pub const DISCRIMINATOR: u8 = RestakingDiscriminator::Ncn as u8;
    pub const SEED: &'static [u8] = b"ncn";

    pub fn find_program_address(program_id: &Pubkey, base: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = vec![Self::SEED.to_vec(), base.to_bytes().to_vec()];
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
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

impl JitoDiscriminator for Operator {
    const DISCRIMINATOR: u8 = RestakingDiscriminator::Operator as u8;
}

impl JitoDataLen for Operator {
    const LEN: usize = std::mem::size_of::<Operator>();
}

impl JitoInitialized for Operator {
    fn is_initialized(&self) -> bool {
        self.bump != 0
    }
}

impl Operator {
    pub const DISCRIMINATOR: u8 = RestakingDiscriminator::Operator as u8;
    pub const SEED: &'static [u8] = b"operator";

    pub fn find_program_address(program_id: &Pubkey, base: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = vec![Self::SEED.to_vec(), base.to_bytes().to_vec()];
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
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
    pub ncn_opt_in_state: PodU64, // SlotToggle
    pub operator_opt_in_state: PodU64, // SlotToggle
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoDiscriminator for NcnOperatorState {
    const DISCRIMINATOR: u8 = RestakingDiscriminator::NcnOperatorState as u8;
}

impl JitoDataLen for NcnOperatorState {
    const LEN: usize = std::mem::size_of::<NcnOperatorState>();
}

impl JitoInitialized for NcnOperatorState {
    fn is_initialized(&self) -> bool {
        self.bump != 0
    }
}

impl NcnOperatorState {
    pub const DISCRIMINATOR: u8 = RestakingDiscriminator::NcnOperatorState as u8;
    pub const SEED: &'static [u8] = b"ncn_operator_state";

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        operator: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = vec![
            Self::SEED.to_vec(),
            ncn.to_bytes().to_vec(),
            operator.to_bytes().to_vec(),
        ];
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
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
    pub state: PodU64, // SlotToggle
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoDiscriminator for OperatorVaultTicket {
    const DISCRIMINATOR: u8 = RestakingDiscriminator::OperatorVaultTicket as u8;
}

impl JitoDataLen for OperatorVaultTicket {
    const LEN: usize = std::mem::size_of::<OperatorVaultTicket>();
}

impl JitoInitialized for OperatorVaultTicket {
    fn is_initialized(&self) -> bool {
        self.bump != 0
    }
}

impl OperatorVaultTicket {
    pub const DISCRIMINATOR: u8 = RestakingDiscriminator::OperatorVaultTicket as u8;
    pub const SEED: &'static [u8] = b"operator_vault_ticket";

    pub fn find_program_address(
        program_id: &Pubkey,
        operator: &Pubkey,
        vault: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = vec![
            Self::SEED.to_vec(),
            operator.to_bytes().to_vec(),
            vault.to_bytes().to_vec(),
        ];
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
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
    pub state: PodU64, // SlotToggle
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoDiscriminator for NcnVaultTicket {
    const DISCRIMINATOR: u8 = RestakingDiscriminator::NcnVaultTicket as u8;
}

impl JitoDataLen for NcnVaultTicket {
    const LEN: usize = std::mem::size_of::<NcnVaultTicket>();
}

impl JitoInitialized for NcnVaultTicket {
    fn is_initialized(&self) -> bool {
        self.bump != 0
    }
}

impl NcnVaultTicket {
    pub const DISCRIMINATOR: u8 = RestakingDiscriminator::NcnVaultTicket as u8;
    pub const SEED: &'static [u8] = b"ncn_vault_ticket";

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        vault: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = vec![
            Self::SEED.to_vec(),
            ncn.to_bytes().to_vec(),
            vault.to_bytes().to_vec(),
        ];
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
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
    pub state: PodU64, // SlotToggle
    pub bump: u8,
    pub reserved: [u8; 263],
}

impl JitoDiscriminator for NcnVaultSlasherTicket {
    const DISCRIMINATOR: u8 = RestakingDiscriminator::NcnVaultSlasherTicket as u8;
}

impl JitoDataLen for NcnVaultSlasherTicket {
    const LEN: usize = std::mem::size_of::<NcnVaultSlasherTicket>();
}

impl JitoInitialized for NcnVaultSlasherTicket {
    fn is_initialized(&self) -> bool {
        self.bump != 0
    }
}

impl NcnVaultSlasherTicket {
    pub const DISCRIMINATOR: u8 = RestakingDiscriminator::NcnVaultSlasherTicket as u8;
    pub const SEED: &'static [u8] = b"ncn_slasher_ticket";

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        vault: &Pubkey,
        slasher: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = vec![
            Self::SEED.to_vec(),
            ncn.to_bytes().to_vec(),
            vault.to_bytes().to_vec(),
            slasher.to_bytes().to_vec(),
        ];
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }
}
