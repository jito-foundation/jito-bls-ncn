use jito_bls_ncn_core::utils::{check_signer, create_or_realloc, JitoAccount};
use jito_bls_ncn_core::{
    accounts::rolling_snapshot::RollingSnapshot,
    instructions::InitializeRollingSnapshotIxData,
    utils::{check_system_program, load_account_mut_unchecked, load_ix_data},
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{rent::Rent, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_initialize_rolling_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [rolling_snapshot, ncn, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<InitializeRollingSnapshotIxData>(data)? };

    check_system_program(system_program)?;
    check_signer(payer, true)?;

    let (pda, bump, seeds) =
        RollingSnapshot::create_program_address(program_id, ix_data.bump, *ncn.key)?;
    if pda.ne(rolling_snapshot.key) {
        msg!("PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::get()?;
    create_or_realloc(
        rolling_snapshot,
        RollingSnapshot::LEN,
        payer,
        system_program,
        program_id,
        &seeds,
        &rent,
    )?;

    let should_initialize = rolling_snapshot.data_len() >= RollingSnapshot::LEN;
    if should_initialize {
        let mut account_data = rolling_snapshot.try_borrow_mut_data()?;
        let account = unsafe { load_account_mut_unchecked::<RollingSnapshot>(&mut account_data) }?;

        if account.is_initialized() {
            msg!("Account is already initalized");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        account.initialize(bump)?;
    }

    Ok(())
}
