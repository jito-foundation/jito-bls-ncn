use solana_program_error::ProgramError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BlsNcnError {
    ArithmaticOverflow = 0x1001,
    ArithmaticUnderflow = 0x1002,
    SignatureVerificationFailed = 0x1003,
    G1AdditionFailed = 0x1004,
    G1SubtractionFailed = 0x1005,
    BitmapCheckFailed = 0x1006,
    QuorumNotMet = 0x1007,
}

impl From<BlsNcnError> for ProgramError {
    fn from(e: BlsNcnError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
