use jito_bls_ncn_core::accounts::consensus::Consensus;
use jito_bls_ncn_core::instructions::InitializeConsensusIxData;
use jito_bls_ncn_core::programs::restaking_core::Ncn;
use jito_bls_ncn_core::utils::{
    check_signer, check_system_program, load_account_mut_unchecked, load_ix_data,
};
use jito_bls_ncn_core::utils::{create_or_realloc, JitoAccount};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{rent::Rent, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_initialize_consensus(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [consensus, ncn, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<InitializeConsensusIxData>(data)? };

    check_system_program(system_program)?;
    check_signer(payer, true)?;

    Ncn::check(
        &jito_bls_ncn_core::programs::restaking_core::id(),
        ncn,
        false,
    )?;

    let (pda, bump, seeds) = Consensus::create_program_address(program_id, ix_data.bump, *ncn.key)?;
    if pda.ne(consensus.key) {
        msg!("PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::get()?;
    create_or_realloc(
        consensus,
        Consensus::LEN,
        payer,
        system_program,
        program_id,
        &seeds,
        &rent,
    )?;

    let should_initialize = consensus.data_len() >= Consensus::LEN;
    if should_initialize {
        let mut account_data = consensus.try_borrow_mut_data()?;
        let account = unsafe { load_account_mut_unchecked::<Consensus>(&mut account_data) }?;

        if account.is_initialized() {
            msg!("Account is already initalized");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        account.initialize(ncn.key, bump)?;

        msg!("Initialized Consensus at {}", pda);
    } else {
        msg!(
            "Consensus is at size {}/{}",
            consensus.data_len(),
            Consensus::LEN
        );
    }

    Ok(())
}
