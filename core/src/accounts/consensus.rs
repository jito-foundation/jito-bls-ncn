use core::fmt;

use solana_account_info::AccountInfo;
use solana_msg::msg;
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
pub struct Consensus {
    pub discriminator: PodOption<PodU64>,
    /// The bump seed for the PDA
    pub bump: u8,
    /// The NCN this ncn operator account belongs to
    pub ncn: Pubkey,
    /// The consensus count
    pub consensus_count: PodU64,
    /// Reserved for future use
    pub reserved: [u8; 1024], // Reserved for future use, must be zeroed
}

impl JitoAccount for Consensus {
    const DISCRIMINATOR: u64 = Discriminators::Consensus as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"consensus";
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

        if check_admin.is_some() {
            msg!("No admin in account");
            return Err(ProgramError::InvalidAccountData);
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

impl Consensus {
    pub fn initialize(&mut self, ncn: &Pubkey, bump: u8) -> Result<(), ProgramError> {
        if self.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        self.discriminator = PodOption::some(PodU64::from(Self::DISCRIMINATOR));

        self.ncn = *ncn;
        self.bump = bump;
        self.consensus_count = PodU64::from(0_u64);
        self.reserved = [0; 1024];

        Ok(())
    }

    pub fn discriminator(&self) -> Option<&PodU64> {
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
