use jito_bls_ncn_core::instructions::SnapshotIxData;
use jito_bls_ncn_core::programs::restaking_core::{Ncn, Operator};
use jito_bls_ncn_core::programs::vault_core::{Vault, VaultOperatorDelegation};
use jito_bls_ncn_core::utils::{check_system_account, load_account, load_account_mut, JitoAccount};
use jito_bls_ncn_core::{
    accounts::{config::Config, rolling_snapshot::RollingSnapshot},
    utils::load_ix_data,
};

use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{clock::Clock, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [config, rolling_snapshot, restaking_config, ncn, operator, vault, vault_operator_delegation] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<SnapshotIxData>(data)? };

    let clock = Clock::get()?;
    let current_slot = clock.slot;

    let epoch_length: u64 = {
        jito_bls_ncn_core::programs::restaking_core::Config::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            restaking_config,
            false,
        )?;
        let restaking_config_data = restaking_config.try_borrow_data()?;
        let restaking_config_account = unsafe {
            load_account::<jito_bls_ncn_core::programs::restaking_core::Config>(
                &restaking_config_data,
            )?
        };
        restaking_config_account.epoch_length.get()
    };

    {
        Config::check(program_id, config, false)?;
        let config_data = config.try_borrow_data()?;
        let config_account = unsafe { load_account::<Config>(&config_data)? };

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
        Vault::check(&jito_bls_ncn_core::programs::vault_core::id(), vault, false)?;

        let vault_data = vault.try_borrow_data()?;
        let vault_account_account = unsafe { load_account::<Vault>(&vault_data)? };

        if vault_account_account.is_update_needed(current_slot, epoch_length)? {
            msg!("Vault account is outdated");
            return Err(ProgramError::InvalidArgument);
        }
    }

    let is_dne = check_system_account(vault_operator_delegation, false).is_ok();

    let (staked, enqueued_for_cooldown, cooling_down) = if is_dne {
        (0, 0, 0)
    } else {
        VaultOperatorDelegation::check(
            &jito_bls_ncn_core::programs::vault_core::id(),
            vault_operator_delegation,
            false,
        )?;
        let vault_operator_delegation_data = vault_operator_delegation.try_borrow_data()?;
        let vault_operator_delegation_account =
            unsafe { load_account::<VaultOperatorDelegation>(&vault_operator_delegation_data)? };

        if vault_operator_delegation_account.vault.ne(vault.key) {
            msg!("Vault does not match - VaultOperatorDelegation");
            return Err(ProgramError::InvalidArgument);
        }
        if vault_operator_delegation_account.operator.ne(operator.key) {
            msg!("Operator does not match - VaultOperatorDelegation");
            return Err(ProgramError::InvalidArgument);
        }

        (
            vault_operator_delegation_account
                .delegation_state
                .staked_amount
                .get(),
            vault_operator_delegation_account
                .delegation_state
                .enqueued_for_cooldown_amount
                .get(),
            vault_operator_delegation_account
                .delegation_state
                .cooling_down_amount
                .get(),
        )
    };

    {
        let mut rolling_snapshot_data = rolling_snapshot.try_borrow_mut_data()?;
        let rolling_snapshot_account =
            unsafe { load_account_mut::<RollingSnapshot>(&mut rolling_snapshot_data)? };

        rolling_snapshot_account.snapshot(
            current_slot,
            epoch_length,
            vault.key,
            ix_data.vault_index.get(),
            operator.key,
            ix_data.operator_index.get(),
            staked,
            enqueued_for_cooldown,
            cooling_down,
        )?;

        msg!("Took snapshot of Operator ({}/{}) and Vault ({}/{}) with staked {}, enqueued for cooldown {}, cooling down {}", ix_data.operator_index.get(), rolling_snapshot_account.operator_count().saturating_sub(1), ix_data.vault_index.get(), rolling_snapshot_account.vault_count().saturating_sub(1), staked, enqueued_for_cooldown, cooling_down);
    }

    Ok(())
}
