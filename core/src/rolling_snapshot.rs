use core::fmt;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::{PodU16, PodU64}, AccountDeserialize, Discriminator};
use shank::ShankAccount;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{bls::solana_bls::{add_g1, sub_g1, verify_g1_g2}, bls_operator::BlsOperator, discriminators::Discriminators, loaders::check_load};

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy, Zeroable, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct RollingSnapshot {
    /// The bump seed for the PDA
    pub bump: u8,
    /// The NCN this ncn operator account belongs to
    pub ncn: Pubkey,
    /// Admin to change parameters ( Default to NCN Admin )
    pub admin: Pubkey,
    /// TX Count
    pub tx_count: PodU64,
    /// Aggregate G1
    pub aggregate_g1: [u8; 64],
    /// Reserved for future use
    pub reserved: [u8; 1024],
    /// Operator count
    pub operator_count: PodU16,
    /// Operators
    pub operators: [OperatorEntry; 256],
}

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)] //128 bytes
pub struct OperatorEntry {
    /// The bump seed for the PDA
    pub operator: Pubkey,
    /// The NCN this ncn operator account belongs to
    pub last_updated_slot: PodU64,
    /// TX Count
    pub weight: PodU64,
    /// Aggregate G1
    pub g1: [u8; 64],
    /// Reserved for future use
    pub reserved: [u8; 16],
}

impl Default for OperatorEntry {
    fn default() -> Self {
        Self {
            operator: Pubkey::default(),
            last_updated_slot: 0u64.into(),
            weight: 0u64.into(),
            g1: [0; 64],
            reserved: [0; 16],
        }
    }
}

impl Discriminator for RollingSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::RollingSnapshot as u8;
}

impl RollingSnapshot {
    pub const MAX_OPERATORS: u16 = 256;

    pub fn operator_count(&self) -> u16 {
        self.operator_count.into()
    }

    fn set_operator_count(&mut self, count: u16) -> Result<(), ProgramError>{
        if count > Self::MAX_OPERATORS {
            msg!("Cannot set operator count to more than maximum");
            return Err(ProgramError::InvalidArgument);
        }

        self.operator_count = PodU16::from(count);

        Ok(())
    }

    pub fn add_operator(&mut self, operator: &BlsOperator) -> Result<(), ProgramError>{
        if self.operator_count() >= Self::MAX_OPERATORS {
            msg!("Already at maximum operators");
            return Err(ProgramError::InvalidArgument);
        }

        let duplicate_result = self.operators.iter().find(|operator_entry| operator_entry.operator == *operator.operator());
        if duplicate_result.is_some() {
            msg!("Operator already exists");
            return Err(ProgramError::InvalidArgument);
        }

        self.operators[self.operator_count() as usize] = OperatorEntry {
            operator: *operator.operator(),
            last_updated_slot: 0u64.into(),
            weight: 0u64.into(),
            g1: *operator.g1(),
            reserved: [0; 16],
        };

        match add_g1(&self.aggregate_g1, &operator.g1) {
            Ok(g1) => self.aggregate_g1 = g1,
            Err(err) => {
                msg!("Failed to add operator G1: {}", err);
                return Err(ProgramError::InvalidArgument);
            },
        }

        let new_operator_count = self.operator_count().checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;
        self.set_operator_count(new_operator_count)?;

        Ok(())
    }

    pub fn remove_operator(&mut self, operator: &BlsOperator) -> Result<(), ProgramError>{
        if self.operator_count() == 0 {
            msg!("No operators to remove");
            return Err(ProgramError::InvalidArgument);
        }

        let index = self.operators.iter().position(|op| op.operator == *operator.operator()).ok_or(ProgramError::InvalidArgument)?;

        self.operators[index] = OperatorEntry::default();

        // Shift operators to fill the gap
        for i in index..self.operator_count() as usize - 1 {
            self.operators[i] = self.operators[i + 1];
        }

        match sub_g1(&self.aggregate_g1, &operator.g1) {
            Ok(g1) => self.aggregate_g1 = g1,
            Err(err) => {
                msg!("Failed to remove operator G1: {}", err);
                return Err(ProgramError::InvalidArgument);
            },
        }

        let new_operator_count = self.operator_count().checked_sub(1).ok_or(ProgramError::ArithmeticOverflow)?;
        self.set_operator_count(new_operator_count)?;

        Ok(())
    }
}
