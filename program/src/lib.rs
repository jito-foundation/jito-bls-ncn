use solana_program::declare_id;

pub mod realloc_rolling_snapshot;
pub mod vote;

declare_id!("7mk2JdyFKPoPDxkagDuuYSkwhXApeeDUnwvhVdgDoTQ6");

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint {
    use jito_bls_ncn_core::instructions::JitoBlsNCNInstructions;
    use solana_account_info::AccountInfo;
    use solana_msg::msg;
    use solana_program_entrypoint::{entrypoint, ProgramResult};
    use solana_program_error::ProgramError;
    use solana_pubkey::Pubkey;

    use crate::realloc_rolling_snapshot::process_realloc_rolling_snapshot;
    use crate::vote::process_vote;

    #[cfg(not(feature = "no-entrypoint"))]
    use solana_security_txt::security_txt;

    #[cfg(not(feature = "no-entrypoint"))]
    security_txt! {
        // Required fields
        name: "Jito BLS NCN",
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
        if *program_id != Pubkey::new_from_array(crate::id().to_bytes()) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let instruction = JitoBlsNCNInstructions::try_from(instruction_data[0])?;
        match instruction {
            JitoBlsNCNInstructions::ReallocRollingSnapshot => {
                msg!("Reallocating Rolling Snapshot");
                process_realloc_rolling_snapshot(program_id, accounts, instruction_data)
            }
            JitoBlsNCNInstructions::Vote => {
                msg!("Voting");
                process_vote(program_id, accounts, instruction_data)
            }
            _ => {
                msg!("Invalid IX ");
                Err(ProgramError::InvalidInstructionData)
            }
        }
    }
}
