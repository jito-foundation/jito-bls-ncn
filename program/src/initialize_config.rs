use jito_bls_ncn_core::instructions::InitializeConfigIxData;
use jito_bls_ncn_core::programs::restaking_core::Ncn;
use jito_bls_ncn_core::utils::{create_or_realloc, load_account, JitoAccount};
use jito_bls_ncn_core::{
    accounts::config::Config,
    utils::{check_signer, check_system_program, load_account_mut_unchecked, load_ix_data},
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{rent::Rent, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_initialize_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [config, ncn, admin, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<InitializeConfigIxData>(data)? };

    check_system_program(system_program)?;
    check_signer(admin, false)?;
    check_signer(payer, true)?;

    {
        Ncn::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            ncn,
            false,
        )?;
        let ncn_data = ncn.try_borrow_data()?;
        let ncn_account = unsafe { load_account::<Ncn>(&ncn_data)? };

        if ncn_account.ncn_program_admin.ne(admin.key) {
            msg!("Operatorn admin mismatch");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let (pda, bump, seeds) = Config::create_program_address(program_id, ix_data.bump, *ncn.key)?;
    if pda.ne(config.key) {
        msg!("PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::get()?;
    create_or_realloc(
        config,
        Config::LEN,
        payer,
        system_program,
        program_id,
        &seeds,
        &rent,
    )?;

    let should_initialize = config.data_len() >= Config::LEN;
    if should_initialize {
        let mut account_data = config.try_borrow_mut_data()?;
        let account = unsafe { load_account_mut_unchecked::<Config>(&mut account_data) }?;

        if account.is_initialized() {
            msg!("Account is already initalized");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        account.initialize(ncn.key, admin.key, bump)?;
    }

    Ok(())
}
