use core::fmt;
use std::mem::size_of;

use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    bls::solana_bls::verify_g1_g2,
    discriminators::Discriminators,
    pod::{PodOption, PodU64},
    utils::{check_account, JitoDataLen, JitoDiscriminator, JitoInitialized},
};

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BlsOperator {
    pub discriminator: PodOption<u8>,
    /// The bump seed for the PDA
    pub bump: u8,
    /// The operator pubkey
    pub operator: Pubkey,
    /// Admin to change parameters ( Default to Operator Admin )
    pub admin: Pubkey,
    /// The G1 pubkey
    pub g1: [u8; 64],
    /// The G2 pubkey
    pub g2: [u8; 128],
    /// The slot the operator was registered
    pub last_updated: PodU64,
    /// socket for offchain work
    pub socket: [u8; 128],
    /// Reserved for future use
    pub reserved: [u8; 256], // Reserved for future use, must be zeroed
}

impl JitoDiscriminator for BlsOperator {
    const DISCRIMINATOR: u8 = Discriminators::BlsOperator as u8;
}

impl JitoDataLen for BlsOperator {
    const LEN: usize = size_of::<Self>();
}

impl JitoInitialized for BlsOperator {
    fn is_initialized(&self) -> bool {
        if let Some(discriminator) = self.discriminator() {
            *discriminator == Self::DISCRIMINATOR
        } else {
            false
        }
    }
}

impl BlsOperator {
    pub const SEED: &'static [u8] = b"bls_operator";

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        operator: &Pubkey,
        g1: &[u8; 64],
        g2: &[u8; 128],
        socket: &[u8; 128],
        current_slot: u64,
        bump: u8,
    ) -> Result<(), ProgramError> {
        if self.is_initialized() {
            msg!("Already Initialized");
            return Err(ProgramError::InvalidArgument);
        }

        self.discriminator = PodOption::some(Discriminators::BlsOperator as u8);

        self.operator = *operator;
        self.socket = *socket;
        self.bump = bump;
        self.reserved = [0; 256];

        self.update_keys(g1, g2, current_slot)?;

        Ok(())
    }

    pub fn seeds(operator: &Pubkey) -> Vec<Vec<u8>> {
        vec![Self::SEED.to_vec(), operator.to_bytes().to_vec()]
    }

    pub fn offchain_find_program_address(
        program_id: &Pubkey,
        operator: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(operator);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (address, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (address, bump, seeds)
    }

    pub fn create_program_address(
        program_id: &Pubkey,
        operator: &Pubkey,
        bump: u8,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(operator);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let address = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((address, bump, seeds))
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        operator: &Pubkey,
        expect_writable: bool,
        bump: u8,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::create_program_address(program_id, operator, bump)?.0;
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

    pub const fn operator(&self) -> &Pubkey {
        &self.operator
    }

    pub const fn g1(&self) -> &[u8; 64] {
        &self.g1
    }

    pub const fn g2(&self) -> &[u8; 128] {
        &self.g2
    }

    pub const fn socket(&self) -> &[u8; 128] {
        &self.socket
    }

    pub fn slot_registered(&self) -> u64 {
        self.last_updated.into()
    }

    /// Verify that the G1 and G2 keys are related by verifying the pairing
    pub fn verify_keypair(&self) -> Result<(), ProgramError> {
        match verify_g1_g2(self.g1(), self.g2()) {
            Ok(verified) => {
                if verified {
                    Ok(())
                } else {
                    msg!("Not verified");
                    Err(ProgramError::InvalidArgument)
                }
            }
            Err(error) => {
                msg!("Error verifying g1/g2 {}", error);
                Err(ProgramError::InvalidArgument)
            }
        }
    }

    /// Update the BLS keys for this ncn operator account
    pub fn update_keys(
        &mut self,
        new_g1: &[u8; 64],
        new_g2: &[u8; 128],
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        // Update the keys
        self.g1 = *new_g1;
        self.g2 = *new_g2;
        self.last_updated = PodU64::from(current_slot);

        // Check valid keys
        self.verify_keypair()?;

        Ok(())
    }
}

impl Default for BlsOperator {
    fn default() -> Self {
        BlsOperator {
            discriminator: PodOption::none(),
            bump: 0,
            operator: Pubkey::default(),
            admin: Pubkey::default(),
            g1: [0; 64],
            g2: [0; 128],
            last_updated: PodU64::from(0),
            socket: [0; 128],
            reserved: [0; 256],
        }
    }
}

impl fmt::Display for BlsOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n\n----------- NCN Operator Account -------------")?;
        writeln!(f, "  Operator:                     {}", self.operator)?;
        writeln!(f, "  G1 Pubkey:                    {:?}", self.g1)?;
        writeln!(f, "  G2 Pubkey:                    {:?}", self.g2)?;
        writeln!(
            f,
            "  Slot Registered:              {}",
            self.slot_registered()
        )?;

        Ok(())
    }
}
