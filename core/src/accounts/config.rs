use core::fmt;

use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    discriminators::Discriminators,
    pod::{PodOption, PodU64},
    utils::{check_account, load_account, JitoAccount},
};

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Config {
    pub discriminator: PodOption<PodU64>,
    /// The bump seed for the PDA
    pub bump: u8,
    /// The NCN this ncn operator account belongs to
    pub ncn: Pubkey,
    /// The admin of this the NCN account
    pub admin: Pubkey,
    /// Reserved for future use
    pub reserved: [u8; 256], // Reserved for future use, must be zeroed
}

impl JitoAccount for Config {
    const DISCRIMINATOR: u64 = Discriminators::Config as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"config";
    type SeedInputs = Pubkey;

    /// ncn
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let ncn = inputs;
        vec![Self::SEED.to_vec(), ncn.to_bytes().to_vec()]
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
        check_admin: Option<&AccountInfo>,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) =
            Self::create_program_address(program_id, data_account.bump, data_account.ncn)?;

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
        if let Some(discriminator) = self.discriminator() {
            (*discriminator).get() == Self::DISCRIMINATOR
        } else {
            false
        }
    }
}

impl Config {
    pub fn initialize(
        &mut self,
        ncn: &Pubkey,
        admin: &Pubkey,
        bump: u8,
    ) -> Result<(), ProgramError> {
        if self.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        self.discriminator = PodOption::some(PodU64::from(Self::DISCRIMINATOR));

        self.ncn = *ncn;
        self.admin = *admin;
        self.bump = bump;
        self.reserved = [0; 256];

        Ok(())
    }

    pub fn discriminator(&self) -> Option<&PodU64> {
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
            admin: Pubkey::default(),
            reserved: [0; 256],
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
