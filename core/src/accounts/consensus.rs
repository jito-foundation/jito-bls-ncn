use core::fmt;
use std::mem::size_of;

use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    discriminators::Discriminators,
    pod::{PodOption, PodU64},
    utils::{check_account, DataLen, Discriminator, Initialized},
};

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Consensus {
    pub discriminator: PodOption<u8>,
    /// The bump seed for the PDA
    pub bump: u8,
    /// The NCN this ncn operator account belongs to
    pub ncn: Pubkey,
    /// The consensus count
    pub consensus_count: PodU64,
    /// Reserved for future use
    pub reserved: [u8; 1024], // Reserved for future use, must be zeroed
}

impl Discriminator for Consensus {
    const DISCRIMINATOR: u8 = Discriminators::Consensus as u8;
}

impl DataLen for Consensus {
    const LEN: usize = size_of::<Self>();
}

impl Initialized for Consensus {
    fn is_initialized(&self) -> bool {
        if let Some(discriminator) = self.discriminator() {
            *discriminator == Self::DISCRIMINATOR
        } else {
            false
        }
    }
}

impl Consensus {
    pub const SEED: &'static [u8] = b"consensus";

    pub fn initialize(&mut self, ncn: &Pubkey, bump: u8) -> Result<(), ProgramError> {
        if self.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        self.ncn = *ncn;
        self.bump = bump;
        self.consensus_count = PodU64::from(0_u64);
        self.reserved = [0; 1024];

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

    pub const fn ncn(&self) -> &Pubkey {
        &self.ncn
    }
}

impl Default for Consensus {
    fn default() -> Self {
        Consensus {
            discriminator: PodOption::none(),
            bump: 0,
            ncn: Pubkey::default(),
            consensus_count: PodU64::from(0_u64),
            reserved: [0; 1024],
        }
    }
}

impl fmt::Display for Consensus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n\n----------- BLS NCN Consensus -------------")?;
        writeln!(f, "  NCN:                          {}", self.ncn)?;

        Ok(())
    }
}
