use jito_bls_ncn_core::utils::{get_new_realloc_size, load_account, realloc, JitoAccount};
use jito_bls_ncn_core::{
    accounts::bls_operator::BlsOperator,
    instructions::InitializeBlsOperatorIxData,
    programs::restaking_core::Operator,
    utils::{
        check_signer, check_system_account, check_system_program, create_account,
        load_account_mut_unchecked, load_ix_data,
    },
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{clock::Clock, rent::Rent, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_initialize_bls_operator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [bls_operator, operator, admin, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<InitializeBlsOperatorIxData>(data)? };

    check_system_program(system_program)?;
    check_signer(admin, false)?;
    check_signer(payer, true)?;

    {
        Operator::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            operator,
            false,
        )?;
        let operator_data = operator.try_borrow_data()?;
        let operator_account = unsafe { load_account::<Operator>(&operator_data)? };

        if operator_account.admin.ne(admin.key) {
            msg!("Operatorn admin mismatch");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let (pda, bump, seeds) =
        BlsOperator::create_program_address(program_id, ix_data.bump, *operator.key)?;
    if pda.ne(bls_operator.key) {
        msg!("PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::get()?;
    match check_system_account(bls_operator, true) {
        Ok(()) => {
            msg!("Creating account");
            let size = get_new_realloc_size(0, BlsOperator::LEN)?;
            create_account(
                payer,
                bls_operator,
                system_program,
                program_id,
                &rent,
                size as u64,
                &seeds,
            )?;
        }
        Err(_) => {
            if bls_operator.data_len() < BlsOperator::LEN {
                msg!("Reallocing account");
                let new_size = get_new_realloc_size(bls_operator.data_len(), BlsOperator::LEN)?;

                realloc(bls_operator, new_size, payer, &rent)?;
            }
        }
    }

    let should_initialize = bls_operator.data_len() >= BlsOperator::LEN;
    if should_initialize {
        let mut account_data = bls_operator.try_borrow_mut_data()?;
        let account = unsafe { load_account_mut_unchecked::<BlsOperator>(&mut account_data) }?;

        let clock = Clock::get()?;

        if !account.is_initialized() {
            account.initialize(
                operator.key,
                &ix_data.g1,
                &ix_data.g2,
                &ix_data.socket,
                clock.slot,
                bump,
            )?;
        }
    }

    Ok(())
}
