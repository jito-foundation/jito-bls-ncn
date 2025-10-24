use jito_bls_ncn_core::{accounts::rolling_snapshot::RollingSnapshot, instructions::ReallocRollingSnapshotIxData, utils::{check_system_account, check_system_program, create_account, get_new_realloc_size, load_account_mut_unchecked, load_ix_data, realloc}};
use jito_bls_ncn_core::utils::JitoAccount;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{rent::Rent, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_realloc_rolling_snapshot(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [rolling_snapshot, ncn, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<ReallocRollingSnapshotIxData>(data)? };

    check_system_program(system_program)?;

    let (pda, _, seeds) = RollingSnapshot::create_program_address(program_id, ix_data.bump, *ncn.key)?;
    if pda.ne(rolling_snapshot.key) {
        msg!("PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::get()?;

    match check_system_account(rolling_snapshot, true) {
        Ok(()) => {
            msg!("Creating account");
            let size = get_new_realloc_size(0, RollingSnapshot::LEN)?;
            create_account(payer, rolling_snapshot, system_program, program_id, &rent, size as u64, &seeds)?;
        }
        Err(_) => {
            if rolling_snapshot.data_len() < RollingSnapshot::LEN {
                msg!("Reallocing account");
                let new_size = get_new_realloc_size(rolling_snapshot.data_len(), RollingSnapshot::LEN)?;

                realloc(rolling_snapshot, new_size, payer, &rent)?;
            }
        }
    }

    let should_initialize = rolling_snapshot.data_len() >= RollingSnapshot::LEN;

    msg!("Should Init: {}", should_initialize);

    if should_initialize {
        let mut account_data = rolling_snapshot.try_borrow_mut_data()?;
        let account = unsafe { load_account_mut_unchecked::<RollingSnapshot>(&mut account_data) }?;

        if !account.is_initialized() {
            account.initialize()?;
        }
    }

    Ok(())
}
