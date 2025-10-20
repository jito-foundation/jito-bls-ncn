use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    accounts::bls_operator::BlsOperator,
    bls::solana_bls::{add_g1, sub_g1},
    discriminators::Discriminators,
    pod::{PodOption, PodU16, PodU64},
    utils::{check_account, DataLen, Discriminator, Initialized},
};

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RollingSnapshot {
    pub discriminator: PodOption<u8>,
    /// The bump seed for the PDA
    pub bump: u8,
    /// The NCN this ncn operator account belongs to
    pub ncn: Pubkey,
    /// Admin to change parameters ( Default to NCN Admin )
    pub admin: Pubkey,
    /// Aggregate G1
    pub aggregate_g1: [u8; 64],
    /// Reserved for future use
    pub reserved: [u8; 1024],
    /// Operator count
    pub operator_count: PodU16,
    /// Operators
    pub operators: [PodOption<OperatorEntry>; 256],
}

impl Discriminator for RollingSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::RollingSnapshot as u8;
}

impl DataLen for RollingSnapshot {
    const LEN: usize = size_of::<Self>();
}

impl Initialized for RollingSnapshot {
    fn is_initialized(&self) -> bool {
        if let Some(discriminator) = self.discriminator() {
            *discriminator == Self::DISCRIMINATOR
        } else {
            false
        }
    }
}

impl RollingSnapshot {
    pub const MAX_OPERATORS: u16 = 256;
    pub const SEED: &'static [u8] = b"rolling_snapshot";

    pub fn initialize(&mut self) -> Result<(), ProgramError> {
        if self.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        self.discriminator = PodOption::some(Self::DISCRIMINATOR);

        Ok(())
    }

    pub fn seeds(ncn: &Pubkey) -> Vec<Vec<u8>> {
        vec![Self::SEED.to_vec(), ncn.to_bytes().to_vec()]
    }

    pub fn offchain_find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (address, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (address, bump, seeds)
    }

    pub fn create_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        bump: u8,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(ncn);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let address = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((address, bump, seeds))
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn: &Pubkey,
        expect_writable: bool,
        bump: u8,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::create_program_address(program_id, ncn, bump)?.0;
        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    pub fn discriminator(&self) -> Option<&u8> {
        self.discriminator.as_ref()
    }

    pub fn operator_count(&self) -> u16 {
        self.operator_count.into()
    }

    fn set_operator_count(&mut self, count: u16) -> Result<(), ProgramError> {
        if count > Self::MAX_OPERATORS {
            msg!("Cannot set operator count to more than maximum");
            return Err(ProgramError::InvalidArgument);
        }

        self.operator_count = PodU16::from(count);

        Ok(())
    }

    pub fn add_operator(&mut self, operator: &BlsOperator) -> Result<(), ProgramError> {
        if self.operator_count() >= Self::MAX_OPERATORS {
            msg!("Already at maximum operators");
            return Err(ProgramError::InvalidArgument);
        }

        let duplicate_result = self.operators.iter().find(|operator_entry| {
            if let Some(operator_entry) = operator_entry.as_ref() {
                operator_entry.operator == *operator.operator()
            } else {
                false
            }
        });

        if duplicate_result.is_some() {
            msg!("Operator already exists");
            return Err(ProgramError::InvalidArgument);
        }

        self.operators[self.operator_count() as usize] = PodOption::some(OperatorEntry {
            operator: *operator.operator(),
            last_updated_slot: 0u64.into(),
            weight: 0u64.into(),
            g1: *operator.g1(),
            reserved: [0; 16],
        });

        match add_g1(&self.aggregate_g1, &operator.g1) {
            Ok(g1) => self.aggregate_g1 = g1,
            Err(err) => {
                msg!("Failed to add operator G1: {}", err);
                return Err(ProgramError::InvalidArgument);
            }
        }

        let new_operator_count = self
            .operator_count()
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        self.set_operator_count(new_operator_count)?;

        Ok(())
    }

    pub fn check_operator_index(
        &mut self,
        operator: &BlsOperator,
        index: usize,
    ) -> Result<(), ProgramError> {
        if index >= self.operator_count() as usize {
            msg!("Invalid operator index");
            return Err(ProgramError::InvalidArgument);
        }

        if let Some(operator_entry_to_check) = self.operators[index].as_ref() {
            if operator_entry_to_check.operator != *operator.operator() {
                msg!("Operator mismatch");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Operator not found");
            return Err(ProgramError::InvalidArgument);
        }

        Ok(())
    }

    pub fn remove_operator(
        &mut self,
        operator: &BlsOperator,
        index: usize,
    ) -> Result<(), ProgramError> {
        self.check_operator_index(operator, index)?;

        self.operators[index] = PodOption::none();

        // Shift operators to fill the gap
        for i in index..self.operator_count() as usize - 1 {
            self.operators[i] = self.operators[i + 1];
        }

        match sub_g1(&self.aggregate_g1, &operator.g1) {
            Ok(g1) => self.aggregate_g1 = g1,
            Err(err) => {
                msg!("Failed to remove operator G1: {}", err);
                return Err(ProgramError::InvalidArgument);
            }
        }

        let new_operator_count = self
            .operator_count()
            .checked_sub(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        self.set_operator_count(new_operator_count)?;

        Ok(())
    }

    pub fn update_operator_weight(
        &mut self,
        operator: &BlsOperator,
        index: usize,
        weight: u64,
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        self.check_operator_index(operator, index)?;

        if let Some(mut updated_operator) = self.operators[index].copied() {
            updated_operator.weight = PodU64::from(weight);
            updated_operator.last_updated_slot = PodU64::from(current_slot);

            self.operators[index] = PodOption::some(updated_operator);

            Ok(())
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }

    pub fn update_operator_g1(
        &mut self,
        operator: &BlsOperator,
        index: usize,
        g1: &[u8; 64],
    ) -> Result<(), ProgramError> {
        self.check_operator_index(operator, index)?;

        if let Some(mut updated_operator) = self.operators[index].copied() {
            let old_g1 = updated_operator.g1.clone();
            updated_operator.g1 = g1.clone();

            self.operators[index] = PodOption::some(updated_operator);

            self.aggregate_g1 = sub_g1(&self.aggregate_g1, &old_g1).expect("Could not subtract G1");
            self.aggregate_g1 =
                add_g1(&self.aggregate_g1, &updated_operator.g1).expect("Could not add G1");

            Ok(())
        } else {
            Err(ProgramError::InvalidArgument)
        }
    }

    pub fn total_weight(&self, last_valid_slot: u64) -> u64 {
        let mut total_weight: u64 = 0;
        for i in 0..self.operator_count() {
            if let Some(operator) = self.operators[i as usize].as_ref() {
                if operator.last_updated_slot() > last_valid_slot {
                    total_weight = total_weight
                        .checked_add(operator.weight())
                        .expect("Could not add weight");
                }
            }
        }
        total_weight
    }
}

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy)]
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

impl OperatorEntry {
    pub fn last_updated_slot(&self) -> u64 {
        self.last_updated_slot.into()
    }

    pub fn weight(&self) -> u64 {
        self.weight.into()
    }
}
