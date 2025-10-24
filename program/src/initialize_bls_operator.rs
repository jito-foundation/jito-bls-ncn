use jito_bls_ncn_core::utils::JitoAccount;
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
    let [bls_operator, operator, admin, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<InitializeBlsOperatorIxData>(data)? };

    check_system_account(bls_operator, true)?;
    check_system_program(system_program)?;
    check_signer(admin, true)?;

    Operator::check(
        &jito_bls_ncn_core::programs::restaking_sdk::id(),
        operator,
        false,
        Some(admin),
    )?;

    let clock = Clock::get()?;
    let rent = Rent::get()?;

    let (pda, bump, seeds) =
        BlsOperator::create_program_address(program_id, ix_data.bump, *operator.key)?;

    if pda.ne(bls_operator.key) {
        msg!("PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    create_account(
        admin,
        bls_operator,
        system_program,
        program_id,
        &rent,
        BlsOperator::LEN as u64,
        &seeds,
    )?;

    let mut account_data = bls_operator.try_borrow_mut_data()?;
    let account = unsafe { load_account_mut_unchecked::<BlsOperator>(&mut account_data) }?;

    account.initialize(
        operator.key,
        &ix_data.g1,
        &ix_data.g2,
        &ix_data.socket,
        clock.slot,
        bump,
    )?;

    Ok(())
}
