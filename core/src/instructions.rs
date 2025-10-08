use crate::errors::BlsNcnProgramError;

#[repr(u8)]
pub enum JitoBlsNCNInstructions {
    // #[account(0, writable, name = "config")]
    // #[account(1, name = "ncn")]
    // #[account(2, name = "ncn_fee_wallet")]
    // #[account(3, signer, name = "ncn_admin")]
    // #[account(4, name = "tie_breaker_admin")]
    // #[account(5, writable, name = "account_payer")]
    // #[account(6, name = "system_program")]
    ReallocRollingSnapshot = 0x01,
}

impl TryFrom<&u8> for JitoBlsNCNInstructions {
    type Error = BlsNcnProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0x01 => Ok(JitoBlsNCNInstructions::ReallocRollingSnapshot),
            _ => Err(BlsNcnProgramError::InvalidInstruction),
        }
    }
}
