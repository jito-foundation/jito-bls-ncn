// use jito_bls_ncn_core::{instructions::ReallocRollingSnapshotIxData, utils::load_ix_data};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramResult;
use solana_pubkey::Pubkey;

pub fn process_realloc_rolling_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    // let [vault, admin, mint, admin_token, vault_token, token_program, system_program] = accounts else {
    //     return Err(ProgramError::NotEnoughAccountKeys);
    // };
    // let ix_data = unsafe { load_ix_data::<ReallocRollingSnapshotIxData>(data)? };

    msg!("Realloc");
    msg!("Program ID: {}", program_id);
    msg!("Accounts: {}", accounts.len());
    msg!("Data: {:?}", data);
    Ok(())
}
