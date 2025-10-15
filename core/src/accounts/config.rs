use core::fmt;
use std::mem::size_of;

use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{discriminators::Discriminators, loaders::check_load, pod::PodOption, utils::{DataLen, Discriminator, Initialized}};

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Config {
    pub discriminator: PodOption<u8>,
    /// The bump seed for the PDA
    pub bump: u8,
    /// The NCN this ncn operator account belongs to
    pub ncn: Pubkey,
    /// Reserved for future use
    pub reserved: [u8; 256], // Reserved for future use, must be zeroed
}


impl Discriminator for Config {
    const DISCRIMINATOR: u8 = Discriminators::Config as u8;
}

impl DataLen for Config {
    const LEN: usize = size_of::<Self>();
}

impl Initialized for Config {
    fn is_initialized(&self) -> bool {
        if let Some(discriminator) = self.discriminator() {
            *discriminator == Self::DISCRIMINATOR
        } else {
            false
        }
    }
}

impl Config {
    const SEED: &'static [u8] = b"config";

    pub fn initialize(
        &mut self,
        ncn: &Pubkey,
        bump: u8,
    ) -> Result<(), ProgramError> {

        if self.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        self.ncn = *ncn;
        self.bump = bump;
        self.reserved = [0; 256];

        Ok(())
    }

    pub fn seeds(ncn: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            Self::SEED.to_vec(),
            ncn.to_bytes().to_vec(),
        ]
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
        check_load(
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

    pub const fn ncn(&self) -> &Pubkey {
        &self.ncn
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            discriminator: PodOption::none(),
            bump: 0,
            ncn: Pubkey::default(),
            reserved: [0; 256]
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n\n----------- BLS NCN Config -------------")?;
        writeln!(f, "  NCN:                          {}", self.ncn)?;

        Ok(())
    }
}
