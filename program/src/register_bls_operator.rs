use jito_bls_ncn_core::programs::restaking_core::{
    Config as RestakingConfig, Ncn, NcnOperatorState,
};
use jito_bls_ncn_core::utils::{load_account, load_account_mut, JitoAccount};
use jito_bls_ncn_core::{
    accounts::{bls_operator::BlsOperator, config::Config, rolling_snapshot::RollingSnapshot},
    instructions::RegisterBlsOperatorIxData,
    programs::restaking_core::Operator,
    utils::{check_signer, check_system_program, load_ix_data},
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{clock::Clock, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_register_bls_operator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [config, rolling_snapshot, bls_operator, restaking_config, ncn, operator, ncn_operator_state, admin, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let _ = unsafe { load_ix_data::<RegisterBlsOperatorIxData>(data)? };

    check_system_program(system_program)?;
    check_signer(admin, false)?;

    let clock = Clock::get()?;

    {
        Config::check(program_id, config, false)?;
        let config_data = config.try_borrow_data()?;
        let config_account = unsafe { load_account::<Config>(&config_data)? };

        if config_account.admin.ne(admin.key) {
            msg!("Admin does not match - Config");
            return Err(ProgramError::InvalidArgument);
        }
        if config_account.ncn.ne(ncn.key) {
            msg!("NCN does not match - Config");
            return Err(ProgramError::InvalidArgument);
        }

        RollingSnapshot::check(program_id, rolling_snapshot, true)?;
        let rolling_snapshot_data = rolling_snapshot.try_borrow_data()?;
        let rolling_snapshot_account =
            unsafe { load_account::<RollingSnapshot>(&rolling_snapshot_data)? };

        if rolling_snapshot_account.ncn.ne(ncn.key) {
            msg!("NCN does not match - RollingSnapshot");
            return Err(ProgramError::InvalidArgument);
        }

        BlsOperator::check(program_id, bls_operator, false)?;
        let bls_operator_data = bls_operator.try_borrow_data()?;
        let bls_operator_account = unsafe { load_account::<BlsOperator>(&bls_operator_data)? };
        if bls_operator_account.operator.ne(operator.key) {
            msg!("Operator does not match - BlsOperator");
            return Err(ProgramError::InvalidArgument);
        }

        RestakingConfig::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            restaking_config,
            false,
        )?;
        let restaking_config_data = restaking_config.try_borrow_data()?;
        let restaking_config_account =
            unsafe { load_account::<RestakingConfig>(&restaking_config_data)? };
        let epoch_length: u64 = restaking_config_account.epoch_length.get();

        Ncn::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            ncn,
            false,
        )?;
        Operator::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            operator,
            false,
        )?;

        NcnOperatorState::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            ncn_operator_state,
            false,
        )?;
        let ncn_operator_state_data = ncn_operator_state.try_borrow_data()?;
        let ncn_operator_state_account =
            unsafe { load_account::<NcnOperatorState>(&ncn_operator_state_data)? };

        if ncn_operator_state_account.ncn.ne(ncn.key) {
            msg!("NCN does not match - NcnOperatorState");
            return Err(ProgramError::InvalidArgument);
        }
        if ncn_operator_state_account.operator.ne(operator.key) {
            msg!("Operator does not match - NcnOperatorState");
            return Err(ProgramError::InvalidArgument);
        }
        if !ncn_operator_state_account
            .ncn_opt_in_state
            .is_active(clock.slot, epoch_length)?
        {
            msg!("NCN is not opted in");
            return Err(ProgramError::InvalidArgument);
        }
        if !ncn_operator_state_account
            .operator_opt_in_state
            .is_active(clock.slot, epoch_length)?
        {
            msg!("Operator is not opted in");
            return Err(ProgramError::InvalidArgument);
        }
    }

    {
        let bls_operator_data = bls_operator.try_borrow_data()?;
        let bls_operator_account = unsafe { load_account::<BlsOperator>(&bls_operator_data)? };

        let mut rolling_snapshot_data = rolling_snapshot.try_borrow_mut_data()?;
        let rolling_snapshot_account =
            unsafe { load_account_mut::<RollingSnapshot>(&mut rolling_snapshot_data)? };

        rolling_snapshot_account.add_operator(bls_operator_account)?;
    }

    Ok(())
}
