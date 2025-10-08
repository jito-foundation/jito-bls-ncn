use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_msg::msg;

pub fn check_load(
    program_id: &Pubkey,
    account: &AccountInfo,
    expected_pda: &Pubkey,
    expected_discriminator: Option<u8>,
    expect_writable: bool,
) -> Result<(), ProgramError> {
    if account.owner.ne(program_id) {
        msg!("Account has an invalid owner");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if account.key.ne(expected_pda) {
        msg!("Account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    if let Some(discriminator) = expected_discriminator {
        if account.data_is_empty() {
            msg!("Account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }

        if account.data.borrow()[0].ne(&discriminator) {
            msg!("Account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    if expect_writable && !account.is_writable {
        msg!("Account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}
