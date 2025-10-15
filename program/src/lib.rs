use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_program::declare_id;
use solana_account_info::AccountInfo;
use solana_program_entrypoint::{ProgramResult, entrypoint};

use jito_bls_ncn_core::instructions::JitoBlsNCNInstructions;

pub mod realloc_rolling_snapshot;

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

declare_id!("3fKQSi6VzzDUJSmeksS8qK6RB3Gs3UoZWtsQD3xagy45");

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "NCN Program Template",
    project_url: "https://jito.network/",
    contacts: "email:team@jito.network",
    policy: "https://github.com/jito-foundation/ncn-program",
    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/jito-foundation/ncn-program"
}

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if *program_id != id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let instruction = JitoBlsNCNInstructions::try_from(instruction_data[0])?;
    match instruction {
        JitoBlsNCNInstructions::ReallocRollingSnapshot => todo!(),
        _ => todo!(),
    }

    Ok(())
}
