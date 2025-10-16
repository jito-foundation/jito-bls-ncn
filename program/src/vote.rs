use jito_bls_ncn_core::{instructions::{ReallocRollingSnapshotIxData, VoteIxData}, utils::load_ix_data};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    // let [vault, admin, mint, admin_token, vault_token, token_program, system_program] = accounts else {
    //     return Err(ProgramError::NotEnoughAccountKeys);
    // };
    // let ix_data = unsafe { load_ix_data::<VoteIxData>(data)? };

    msg!("Vote");

    Ok(())
}
