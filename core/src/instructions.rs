use crate::{
    errors::BlsNcnProgramError, pod::PodU64, utils::JitoIxData,
};

#[repr(u64)]
pub enum JitoBlsNCNInstructions {
    InitializeBlsOperator = 0x03,
    ReallocRollingSnapshot = 0x01,
    Vote = 0x02,
}

impl TryFrom<u64> for JitoBlsNCNInstructions {
    type Error = BlsNcnProgramError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0x03 => Ok(JitoBlsNCNInstructions::InitializeBlsOperator),
            0x01 => Ok(JitoBlsNCNInstructions::ReallocRollingSnapshot),
            0x02 => Ok(JitoBlsNCNInstructions::Vote),
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
#[repr(C, packed)]
pub struct InitializeBlsOperatorIxData {
    pub discriminator: PodU64,
    pub bump: u8,
    pub g1: [u8; 64],
    pub g2: [u8; 128],
    pub socket: [u8; 128],
}

impl JitoIxData for InitializeBlsOperatorIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::InitializeBlsOperator as u64;
    const LEN: usize = size_of::<Self>();
}

impl InitializeBlsOperatorIxData {
    pub fn new(bump: u8, g1: [u8; 64], g2: [u8; 128], socket: [u8; 128]) -> Self {
        Self {
            discriminator: PodU64::from(Self::DISCRIMINATOR),
            bump,
            g1,
            g2,
            socket,
        }
    }

    /// # Safety
    /// Caller must ensure everything is 1 byte aligned
    pub unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
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
#[repr(C, packed)]
pub struct ReallocRollingSnapshotIxData {
    pub discriminator: PodU64,
    pub bump: u8,
}

impl JitoIxData for ReallocRollingSnapshotIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::ReallocRollingSnapshot as u64;
    const LEN: usize = size_of::<Self>();
}

impl ReallocRollingSnapshotIxData {
    pub fn new(bump: u8) -> Self {
        Self {
            discriminator: PodU64::from(Self::DISCRIMINATOR),
            bump,
        }
    }

    /// # Safety
    /// Caller must ensure everything is 1 byte aligned
    pub unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
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
#[repr(C, packed)]
pub struct VoteIxData {
    pub discriminator: u64,
}

impl JitoIxData for VoteIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::Vote as u64;
    const LEN: usize = size_of::<Self>();
}

impl VoteIxData {
    pub fn new() -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
        }
    }

    /// # Safety
    /// Caller must ensure everything is 1 byte aligned
    pub unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
    }
}
