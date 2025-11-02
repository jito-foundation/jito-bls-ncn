use jito_bls_ncn_core::instructions::RegisterVaultIxData;
use jito_bls_ncn_core::programs::restaking_core::{Ncn, NcnVaultTicket};
use jito_bls_ncn_core::programs::vault_core::{Vault, VaultNcnTicket};
use jito_bls_ncn_core::utils::{load_account, load_account_mut, JitoAccount};
use jito_bls_ncn_core::{
    accounts::{config::Config, rolling_snapshot::RollingSnapshot},
    utils::{check_signer, load_ix_data},
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{clock::Clock, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_register_vault(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [config, rolling_snapshot, restaking_config, ncn, vault, ncn_vault_ticket, vault_ncn_ticket, admin] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<RegisterVaultIxData>(data)? };

    check_signer(admin, false)?;

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

        Ncn::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            ncn,
            false,
        )?;
        Vault::check(&jito_bls_ncn_core::programs::vault_core::id(), vault, false)?;

        VaultNcnTicket::check(
            &jito_bls_ncn_core::programs::vault_core::id(),
            vault_ncn_ticket,
            false,
        )?;
        let vault_ncn_ticket_data = vault_ncn_ticket.try_borrow_data()?;
        let vault_ncn_ticket_account =
            unsafe { load_account::<VaultNcnTicket>(&vault_ncn_ticket_data)? };

        if vault_ncn_ticket_account.ncn.ne(ncn.key) {
            msg!("NCN does not match - VaultNcnTicket");
            return Err(ProgramError::InvalidArgument);
        }
        if vault_ncn_ticket_account.vault.ne(vault.key) {
            msg!("Vault does not match - VaultNcnTicket");
            return Err(ProgramError::InvalidArgument);
        }
        if !vault_ncn_ticket_account
            .state
            .is_active(current_slot, epoch_length)?
        {
            msg!("VaultNCNTicket is not active");
            return Err(ProgramError::InvalidArgument);
        }

        NcnVaultTicket::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            ncn_vault_ticket,
            false,
        )?;

        let ncn_vault_ticket_data = ncn_vault_ticket.try_borrow_data()?;
        let ncn_vault_ticket_account =
            unsafe { load_account::<NcnVaultTicket>(&ncn_vault_ticket_data)? };
        if ncn_vault_ticket_account.ncn.ne(ncn.key) {
            msg!("NCN does not match - NcnVaultTicket");
            return Err(ProgramError::InvalidArgument);
        }
        if ncn_vault_ticket_account.vault.ne(vault.key) {
            msg!("Vault does not match - NcnVaultTicket");
            return Err(ProgramError::InvalidArgument);
        }
        if !ncn_vault_ticket_account
            .state
            .is_active(current_slot, epoch_length)?
        {
            msg!("NcnVaultTicket is not active");
            return Err(ProgramError::InvalidArgument);
        }
    }

    {
        let mut rolling_snapshot_data = rolling_snapshot.try_borrow_mut_data()?;
        let rolling_snapshot_account =
            unsafe { load_account_mut::<RollingSnapshot>(&mut rolling_snapshot_data)? };

        rolling_snapshot_account.add_vault(
            current_slot,
            epoch_length,
            vault.key,
            ix_data.weight_bps.get(),
        )?;
    }

    Ok(())
}
