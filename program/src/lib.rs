use solana_program::declare_id;

pub mod initialize_bls_operator;
pub mod initialize_config;
pub mod initialize_rolling_snapshot;
pub mod register_bls_operator;
pub mod vote;

declare_id!("3Shbx5RwJtmD4EZHu5XmSkqaTxruikKmEU2Qx4BcccU5");

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint {
    use jito_bls_ncn_core::instructions::JitoBlsNCNInstructions;
    use jito_bls_ncn_core::utils::load_instruction_discriminator;
    use solana_account_info::AccountInfo;
    use solana_msg::msg;
    use solana_program_entrypoint::entrypoint;
    use solana_program_entrypoint::ProgramResult;
    use solana_program_error::ProgramError;
    use solana_pubkey::Pubkey;

    use crate::initialize_bls_operator::process_initialize_bls_operator;
    use crate::initialize_config::process_initialize_config;
    use crate::initialize_rolling_snapshot::process_initialize_rolling_snapshot;
    use crate::register_bls_operator::process_register_bls_operator;
    use crate::vote::process_vote;

    use solana_security_txt::security_txt;

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

    entrypoint!(process_instruction);

    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        if *program_id != Pubkey::new_from_array(crate::id().to_bytes()) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let instruction_u64 = load_instruction_discriminator(instruction_data)?;
        let instruction = JitoBlsNCNInstructions::try_from(instruction_u64)?;
        match instruction {
            JitoBlsNCNInstructions::InitializeConfig => {
                msg!("Initializing Config");
                process_initialize_config(program_id, accounts, instruction_data)
            }
            JitoBlsNCNInstructions::InitializeRollingSnapshot => {
                msg!("Reallocating Rolling Snapshot");
                process_initialize_rolling_snapshot(program_id, accounts, instruction_data)
            }
            JitoBlsNCNInstructions::InitializeBlsOperator => {
                msg!("Initializing BLS Operator");
                process_initialize_bls_operator(program_id, accounts, instruction_data)
            }
            JitoBlsNCNInstructions::RegisterBlsOperator => {
                msg!("Registering BLS Operator");
                process_register_bls_operator(program_id, accounts, instruction_data)
            }
            JitoBlsNCNInstructions::RemoveBlsOperator => todo!(),
            JitoBlsNCNInstructions::Vote => {
                msg!("Voting");
                process_vote(program_id, accounts, instruction_data)
            }
        }
    }
}
