use solana_account_info::{AccountInfo, MAX_PERMITTED_DATA_INCREASE};
use solana_msg::msg;
use solana_program::{
    program::{invoke, invoke_signed},
    rent::Rent,
};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub trait JitoAccount {
    const DISCRIMINATOR: u64;
    const LEN: usize;
    const SEED: &'static [u8];

    type SeedInputs;

    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>>;
    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>);
    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError>;
    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
        check_admin: Option<&AccountInfo>,
    ) -> Result<(), ProgramError>;

    fn is_initialized(&self) -> bool;
}

pub trait JitoIxData {
    const DISCRIMINATOR: u64;
    const LEN: usize;
}

// pub trait JitoDiscriminator {
//     const DISCRIMINATOR: u64;
// }

// pub trait JitoDataLen {
//     const LEN: usize;
// }

// pub trait JitoInitialized {
//     fn is_initialized(&self) -> bool;
// }

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn load_account<T: JitoAccount>(bytes: &[u8]) -> Result<&T, ProgramError> {
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
pub unsafe fn load_account_unchecked<T: JitoAccount>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn load_account_mut<T: JitoAccount>(bytes: &mut [u8]) -> Result<&mut T, ProgramError> {
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
pub unsafe fn load_account_mut_unchecked<T: JitoAccount>(
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
pub unsafe fn load_ix_data<T: JitoIxData>(bytes: &[u8]) -> Result<&T, ProgramError> {
    if bytes.len() != T::LEN {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn ix_data_to_bytes<T: JitoIxData>(data: &T) -> &[u8] {
    core::slice::from_raw_parts(data as *const T as *const u8, T::LEN)
}

/// # Safety
/// Caller must ensure everything is 1 byte aligned
#[inline(always)]
pub unsafe fn ix_data_to_mut_bytes<T: JitoIxData>(data: &mut T) -> &mut [u8] {
    core::slice::from_raw_parts_mut(data as *mut T as *mut u8, T::LEN)
}

pub fn check_signer(info: &AccountInfo, expect_writable: bool) -> Result<(), ProgramError> {
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

pub fn check_token_program(info: &AccountInfo) -> Result<(), ProgramError> {
    if info.key.ne(&spl_token_interface::id()) {
        msg!("Account is not the token program");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

pub fn check_system_program(info: &AccountInfo) -> Result<(), ProgramError> {
    if info.key.ne(&solana_system_interface::program::id()) {
        msg!("Account is not the system program");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

pub fn check_system_account(info: &AccountInfo, is_writable: bool) -> Result<(), ProgramError> {
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
    expected_discriminator: Option<u64>,
    expect_writable: bool,
    check_admin: Option<&AccountInfo>,
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

        let account_discriminator_slice: [u8; 8] = account.data.borrow()[..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidAccountData)?;
        let account_discriminator = u64::from_le_bytes(account_discriminator_slice);

        if account_discriminator != discriminator {
            msg!("Account u64 discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    if expect_writable && !account.is_writable {
        msg!("Account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    if let Some(admin) = check_admin {
        check_signer(admin, false)?;
    }

    Ok(())
}

/// Creates a new account or initializes an existing account
/// # Arguments
/// * `payer` - The account that will pay for the lamports
/// * `new_account` - The account to create or initialize
/// * `system_program` - The system program account
/// * `program_owner` - The owner of the program
/// * `rent` - The rent sysvar
/// * `space` - The space to allocate
/// * `seeds` - The seeds to use for the PDA
/// # Returns
/// * `ProgramResult` - The result of the operation
#[inline(always)]
pub fn create_account<'a, 'info>(
    payer: &'a AccountInfo<'info>,
    new_account: &'a AccountInfo<'info>,
    system_program: &'a AccountInfo<'info>,
    program_owner: &Pubkey,
    rent: &Rent,
    space: u64,
    seeds: &[Vec<u8>],
) -> ProgramResult {
    let current_lamports = **new_account.try_borrow_lamports()?;
    if current_lamports == 0 {
        // If there are no lamports in the new account, we create it with the create_account instruction
        invoke_signed(
            &solana_system_interface::instruction::create_account(
                payer.key,
                new_account.key,
                rent.minimum_balance(space as usize),
                space,
                program_owner,
            ),
            &[payer.clone(), new_account.clone(), system_program.clone()],
            &[seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )
    } else {
        // someone can transfer lamports to accounts before they're initialized
        // in that case, creating the account won't work.
        // in order to get around it, you need to find the account with enough lamports to be rent exempt,
        // then allocate the required space and set the owner to the current program
        let required_lamports = rent
            .minimum_balance(space as usize)
            .max(1)
            .saturating_sub(current_lamports);
        if required_lamports > 0 {
            invoke(
                &solana_system_interface::instruction::transfer(
                    payer.key,
                    new_account.key,
                    required_lamports,
                ),
                &[payer.clone(), new_account.clone(), system_program.clone()],
            )?;
        }
        // Allocate space.
        invoke_signed(
            &solana_system_interface::instruction::allocate(new_account.key, space),
            &[new_account.clone(), system_program.clone()],
            &[seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
        // Assign to the specified program
        invoke_signed(
            &solana_system_interface::instruction::assign(new_account.key, program_owner),
            &[new_account.clone(), system_program.clone()],
            &[seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )
    }
}

/// Closes the program account
pub fn close_program_account<'a>(
    program_id: &Pubkey,
    account_to_close: &AccountInfo<'a>,
    destination_account: &AccountInfo<'a>,
    system_program: &'a AccountInfo<'a>,
) -> ProgramResult {
    // Check if the account is owned by the program
    if account_to_close.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    **destination_account.lamports.borrow_mut() = destination_account
        .lamports()
        .checked_add(account_to_close.lamports())
        .ok_or(ProgramError::ArithmeticOverflow)?;
    **account_to_close.lamports.borrow_mut() = 0;

    account_to_close.assign(system_program.key);
    account_to_close.resize(0)?;

    Ok(())
}

pub fn realloc<'a, 'info>(
    account: &'a AccountInfo<'info>,
    new_size: usize,
    payer: &'a AccountInfo<'info>,
    rent: &Rent,
) -> ProgramResult {
    let new_minimum_balance = rent.minimum_balance(new_size);

    let lamports_diff = new_minimum_balance.saturating_sub(account.lamports());
    invoke(
        &solana_system_interface::instruction::transfer(payer.key, account.key, lamports_diff),
        &[payer.clone(), account.clone()],
    )?;
    account.resize(new_size)?;
    Ok(())
}

pub fn get_epoch(slot: u64, epoch_length: u64) -> Result<u64, ProgramError> {
    let epoch = slot
        .checked_div(epoch_length)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(epoch)
}

/// Calculate new size for reallocation, capped at target size
/// Returns the minimum of (current_size + MAX_REALLOC_BYTES) and target_size
pub fn get_new_realloc_size(
    current_size: usize,
    target_size: usize,
) -> Result<usize, ProgramError> {
    Ok(current_size
        .checked_add(MAX_PERMITTED_DATA_INCREASE)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .min(target_size))
}

pub fn get_realloc_calls(current_size: usize, target_size: usize) -> Result<usize, ProgramError> {
    let bytes_to_go = target_size.saturating_sub(current_size);
    if bytes_to_go == 0 {
        Ok(0)
    } else {
        let checked_div = bytes_to_go
            .checked_div(MAX_PERMITTED_DATA_INCREASE)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        Ok(checked_div.saturating_add(1))
    }
}
