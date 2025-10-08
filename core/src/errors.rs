use solana_program_error::ProgramError;

#[derive(Clone, PartialEq)]
pub enum BlsNcnProgramError {
    InvalidInstruction,
}

impl From<BlsNcnProgramError> for ProgramError {
    fn from(e: BlsNcnProgramError) -> Self {
        Self::Custom(e as u32)
    }
}
