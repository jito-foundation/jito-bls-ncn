use crate::{errors::BlsNcnProgramError, pod::PodU64, utils::JitoIxData};

#[repr(u64)]
pub enum JitoBlsNCNInstructions {
    InitializeConfig = 0x01,
    InitializeConsensus = 0x02,
    InitializeRollingSnapshot = 0x03,
    InitializeBlsOperator = 0x04,

    RegisterBlsOperator = 0x10,
    RemoveBlsOperator = 0x11,

    Vote = 0x20,
}

impl TryFrom<u64> for JitoBlsNCNInstructions {
    type Error = BlsNcnProgramError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(JitoBlsNCNInstructions::InitializeConfig),
            0x02 => Ok(JitoBlsNCNInstructions::InitializeConsensus),
            0x03 => Ok(JitoBlsNCNInstructions::InitializeRollingSnapshot),
            0x04 => Ok(JitoBlsNCNInstructions::InitializeBlsOperator),
            0x10 => Ok(JitoBlsNCNInstructions::RegisterBlsOperator),
            0x11 => Ok(JitoBlsNCNInstructions::RemoveBlsOperator),
            0x20 => Ok(JitoBlsNCNInstructions::Vote),
            _ => Err(BlsNcnProgramError::InvalidInstruction),
        }
    }
}

// -------------------- INITIALIZE CONFIG ---------------

/// [config, ncn, admin, payer, system_program]
#[repr(C, packed)]
pub struct InitializeConfigIxData {
    pub discriminator: PodU64,
    pub bump: u8,
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
unsafe impl JitoIxData for InitializeConfigIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::InitializeConfig as u64;
    const LEN: usize = size_of::<Self>();

    /// # Safety
    /// Caller must ensure everything is 1 byte aligned
    unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
    }
}

impl InitializeConfigIxData {
    pub fn new(bump: u8) -> Self {
        Self {
            discriminator: PodU64::from(Self::DISCRIMINATOR),
            bump,
        }
    }
}

// -------------------- INITIALIZE CONSENSUS ---------------

/// [config, ncn, admin, payer, system_program]
#[repr(C, packed)]
pub struct InitializeConsensusIxData {
    pub discriminator: PodU64,
    pub bump: u8,
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
unsafe impl JitoIxData for InitializeConsensusIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::InitializeConsensus as u64;
    const LEN: usize = size_of::<Self>();

    /// # Safety
    /// Caller must ensure everything is 1 byte aligned
    unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
    }
}

impl InitializeConsensusIxData {
    pub fn new(bump: u8) -> Self {
        Self {
            discriminator: PodU64::from(Self::DISCRIMINATOR),
            bump,
        }
    }
}


// -------------------- INITIALIZE ROLLING SNAPSHOT ---------------

/// [rolling_snapshot, ncn, payer, system_program]
#[repr(C, packed)]
pub struct InitializeRollingSnapshotIxData {
    pub discriminator: PodU64,
    pub bump: u8,
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
unsafe impl JitoIxData for InitializeRollingSnapshotIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::InitializeRollingSnapshot as u64;
    const LEN: usize = size_of::<Self>();

    /// # Safety
    /// Caller must ensure everything is 1 byte aligned
    unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
    }
}

impl InitializeRollingSnapshotIxData {
    pub fn new(bump: u8) -> Self {
        Self {
            discriminator: PodU64::from(Self::DISCRIMINATOR),
            bump,
        }
    }
}

// -------------------- INITIALIZE BLS OPERATOR ---------------
/// [bls_operator, operator, admin, payer, system_program]
#[repr(C, packed)]
pub struct InitializeBlsOperatorIxData {
    pub discriminator: PodU64,
    pub bump: u8,
    pub g1: [u8; 64],
    pub g2: [u8; 128],
    pub socket: [u8; 128],
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
unsafe impl JitoIxData for InitializeBlsOperatorIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::InitializeBlsOperator as u64;
    const LEN: usize = size_of::<Self>();

    /// # Safety
    /// Caller must ensure everything is 1 byte aligned
    unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
    }
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
}

// -------------------- REGISTER BLS OPERATOR  -----------------------------

#[repr(C, packed)]
pub struct RegisterBlsOperatorIxData {
    pub discriminator: PodU64,
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
unsafe impl JitoIxData for RegisterBlsOperatorIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::RegisterBlsOperator as u64;
    const LEN: usize = size_of::<Self>();

    unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
    }
}

impl RegisterBlsOperatorIxData {
    pub fn new() -> Self {
        Self {
            discriminator: PodU64::from(Self::DISCRIMINATOR),
        }
    }
}

// -------------------- VOTE -----------------------------
#[repr(C, packed)]
pub struct VoteIxData {
    pub discriminator: PodU64,
    pub aggregated_g1_signature: [u8; 64],
    pub aggregated_g2_signed: [u8; 128],
    pub operators_bitmap_signed: [u8; 32],
    pub message: [u8; 32],
    pub consensus_count: PodU64,
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
unsafe impl JitoIxData for VoteIxData {
    const DISCRIMINATOR: u64 = JitoBlsNCNInstructions::Vote as u64;
    const LEN: usize = size_of::<Self>();

    unsafe fn to_bytes(&self) -> &[u8] {
        unsafe { crate::utils::ix_data_to_bytes::<Self>(self) }
    }
}

impl VoteIxData {
    pub fn new(aggregated_g1_signature: [u8; 64], aggregated_g2_signed: [u8; 128], operators_bitmap_signed: [u8; 32], raw_message: [u8; 32], consensus_count: u64) -> Self {

        Self {
            discriminator: PodU64::from(Self::DISCRIMINATOR),
            aggregated_g1_signature,
            aggregated_g2_signed,
            operators_bitmap_signed,
            message: raw_message,
            consensus_count: PodU64::from(consensus_count),
        }
    }
}
