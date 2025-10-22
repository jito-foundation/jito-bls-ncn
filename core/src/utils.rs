use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub trait JitoDiscriminator {
    const DISCRIMINATOR: u8;
}

pub trait JitoDataLen {
    const LEN: usize;
}

pub trait JitoInitialized {
    fn is_initialized(&self) -> bool;
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn load_account<T: JitoDataLen + JitoInitialized>(
    bytes: &[u8],
) -> Result<&T, ProgramError> {
    load_account_unchecked::<T>(bytes).and_then(|account| {
        if account.is_initialized() {
            Ok(account)
        } else {
            Err(ProgramError::UninitializedAccount)
        }
    })
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn load_account_unchecked<T: JitoDataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn load_account_mut<T: JitoDataLen + JitoInitialized>(
    bytes: &mut [u8],
) -> Result<&mut T, ProgramError> {
    load_account_mut_unchecked::<T>(bytes).and_then(|acc| {
        if acc.is_initialized() {
            Ok(acc)
        } else {
            Err(ProgramError::UninitializedAccount)
        }
    })
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn load_account_mut_unchecked<T: JitoDataLen>(
    bytes: &mut [u8],
) -> Result<&mut T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&mut *(bytes.as_mut_ptr() as *mut T))
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn load_ix_data<T: JitoDataLen>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn to_bytes<T: JitoDataLen>(data: &T) -> &[u8] {
    core::slice::from_raw_parts(data as *const T as *const u8, T::LEN)
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn to_mut_bytes<T: JitoDataLen>(data: &mut T) -> &mut [u8] {
    core::slice::from_raw_parts_mut(data as *mut T as *mut u8, T::LEN)
}

pub fn load_signer(info: &AccountInfo, expect_writable: bool) -> Result<(), ProgramError> {
    if !info.is_signer {
        msg!("Account is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }
    if expect_writable && !info.is_writable {
        msg!("Signer is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

pub fn load_token_program(info: &AccountInfo) -> Result<(), ProgramError> {
    if info.key.ne(&spl_token_interface::id()) {
        msg!("Account is not the token program");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

pub fn load_system_program(info: &AccountInfo) -> Result<(), ProgramError> {
    if info.key.ne(&solana_system_interface::program::id()) {
        msg!("Account is not the system program");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

pub fn load_system_account(info: &AccountInfo, is_writable: bool) -> Result<(), ProgramError> {
    if info.owner.ne(&solana_system_interface::program::id()) {
        msg!("Account is not owned by the system program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !info.data_is_empty() {
        msg!("Account data is not empty");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if is_writable && !info.is_writable {
        msg!("Account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

pub fn check_account(
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

        let account_discriminator_option: u8 = account.data.borrow()[0];
        let account_discriminator: u8 = account.data.borrow()[1];
        if account_discriminator_option == 0 || account_discriminator != discriminator {
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
