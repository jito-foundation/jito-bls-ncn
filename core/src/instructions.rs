use crate::{
    errors::BlsNcnProgramError,
    utils::{DataLen, Discriminator},
};

#[repr(u8)]
pub enum JitoBlsNCNInstructions {
    ReallocRollingSnapshot = 0x01,
    Vote = 0x02,
}

impl TryFrom<u8> for JitoBlsNCNInstructions {
    type Error = BlsNcnProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            ReallocRollingSnapshotIxData::DISCRIMINATOR => {
                Ok(JitoBlsNCNInstructions::ReallocRollingSnapshot)
            }
            VoteIxData::DISCRIMINATOR => Ok(JitoBlsNCNInstructions::Vote),
            _ => Err(BlsNcnProgramError::InvalidInstruction),
        }
    }
}

// -------------------- REALLOC ROLLING SNAPSHOT ---------------

// #[account(0, writable, name = "config")]
// #[account(1, name = "ncn")]
// #[account(2, name = "ncn_fee_wallet")]
// #[account(3, signer, name = "ncn_admin")]
// #[account(4, name = "tie_breaker_admin")]
// #[account(5, writable, name = "account_payer")]
// #[account(6, name = "system_program")]
pub struct ReallocRollingSnapshotIxData {
    pub discriminator: u8,
}

impl DataLen for ReallocRollingSnapshotIxData {
    const LEN: usize = size_of::<Self>();
}

impl Discriminator for ReallocRollingSnapshotIxData {
    const DISCRIMINATOR: u8 = JitoBlsNCNInstructions::ReallocRollingSnapshot as u8;
}

impl ReallocRollingSnapshotIxData {
    pub fn new() -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
        }
    }

    pub unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::to_bytes::<Self>(&self) }
    }
}

// -------------------- VOTE -----------------------------

// #[account(0, writable, name = "config")]
// #[account(1, name = "ncn")]
// #[account(2, name = "ncn_fee_wallet")]
// #[account(3, signer, name = "ncn_admin")]
// #[account(4, name = "tie_breaker_admin")]
// #[account(5, writable, name = "account_payer")]
// #[account(6, name = "system_program")]
pub struct VoteIxData {
    pub discriminator: u8,
}

impl DataLen for VoteIxData {
    const LEN: usize = size_of::<Self>();
}

impl Discriminator for VoteIxData {
    const DISCRIMINATOR: u8 = JitoBlsNCNInstructions::Vote as u8;
}

impl VoteIxData {
    pub fn new() -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
        }
    }

    pub unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::to_bytes::<Self>(&self) }
    }
}
