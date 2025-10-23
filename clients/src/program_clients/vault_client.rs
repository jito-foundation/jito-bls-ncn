use jito_bls_ncn_core::{programs::{vault_core::{Config, Vault}, vault_sdk::{add_delegation_ix, close_vault_update_state_tracker_ix, config_address, crank_vault_update_state_tracker_ix, initialize_config_ix, initialize_vault_ix, initialize_vault_ncn_ticket_ix, initialize_vault_operator_delegation_ix, initialize_vault_update_state_tracker_ix, mint_to_ix, update_vault_balance_ix, vault_ncn_ticket_address, vault_operator_delegation_address, vault_update_state_tracker_address, warmup_vault_ncn_ticket_ix, WithdrawalAllocationMethod}}, utils::load_account};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use anyhow::{Result};
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account_interface::address::get_associated_token_address;

use crate::{jito_clients::JitoClient, program_clients::solana_client::{create_ata, create_mint, mint_spl_to}};

pub struct VaultRoot {
    pub vault_pubkey: Pubkey,
    pub vault_admin: Keypair,
    pub mint: Keypair,
}

impl Clone for VaultRoot {
    fn clone(&self) -> Self {
        Self {
            vault_pubkey: self.vault_pubkey,
            vault_admin: self.vault_admin.insecure_clone(),
            mint: self.mint.insecure_clone(),
        }
    }
}

pub async fn get_config<T: JitoClient>(
    jito_client: &T,
) -> Result<Config> {
    let (address, _) = config_address();
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<Config>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_vault<T: JitoClient>(
    jito_client: &T,
    vault: &Pubkey,
) -> Result<Vault> {
    let account_raw = jito_client.get_account(&vault).await?;
    let account = unsafe { load_account::<Vault>(&account_raw.data)? };
    Ok(*account)
}

pub async fn test_configure_depositor<T: JitoClient>(
    jito_client: &mut T,
    vault_root: &VaultRoot,
    depositor: &Pubkey,
    amount_to_mint: u64,
) -> Result<()> {
    jito_client.test_airdrop(depositor, 1_000_000_000).await?;
    let vault = get_vault(jito_client, &vault_root.vault_pubkey).await?;
    create_ata(jito_client, depositor, &vault.supported_mint, None).await?;
    create_ata(jito_client, depositor, &vault.vrt_mint, None).await?;
    mint_spl_to(jito_client, &vault.supported_mint, depositor, amount_to_mint, None).await?;

    Ok(())
}

pub async fn get_vault_is_update_needed<T: JitoClient>(jito_client: &mut T, vault: &Pubkey, slot: u64) -> Result<bool> {
    let config = get_config(jito_client).await?;
    let vault = get_vault(jito_client, vault).await?;

    let is_update_needed = vault.is_update_needed(slot, config.epoch_length.into());
    Ok(is_update_needed)
}

pub async fn test_initialize_config<T: JitoClient>(jito_client: &mut T) -> Result<()> {
    let admin = jito_client.keypair().insecure_clone();
    let restaking_program = jito_bls_ncn_core::programs::restaking_sdk::id();
    let (config, _) = config_address();
    initialize_config(jito_client, &config, &admin, &restaking_program, &admin.pubkey(), 100).await?;

    Ok(())
}

    pub async fn initialize_config<T: JitoClient>(
        jito_client: &mut T,
        config: &Pubkey,
        config_admin: &Keypair,
        restaking_program: &Pubkey,
        program_fee_wallet: &Pubkey,
        program_fee_bps: u16,
    ) -> Result<()> {
        let blockhash = jito_client.get_recent_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[initialize_config_ix(
                config,
                &config_admin.pubkey(),
                restaking_program,
                program_fee_wallet,
                program_fee_bps
            )],
            Some(&config_admin.pubkey()),
            &[config_admin],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn test_initialize_vault<T: JitoClient>(
        jito_client: &mut T,
    ) -> Result<VaultRoot> {
        let admin = jito_client.keypair().insecure_clone();
        let base = Keypair::new();
        let vrt_mint = Keypair::new();
        let st_mint = Keypair::new();

        let initialize_token_amount = 1_000_000;
        let fee_bps = 100;
        let decimals = 9;

        let (vault, _) = jito_bls_ncn_core::programs::vault_sdk::vault_address(&base.pubkey());
        let (burn_vault, _) = jito_bls_ncn_core::programs::vault_sdk::burn_vault_address(&base.pubkey());
        let (config, _) = jito_bls_ncn_core::programs::vault_sdk::config_address();

        // Airdrop to vault admin
        jito_client.test_airdrop(&admin.pubkey(), 1_000_000_000).await?;

        // Create mint
        create_mint(jito_client, &st_mint, decimals, None, None, None).await?;

        // Initialize vault
        initialize_vault(
            jito_client,
            &config,
            &vault,
            &burn_vault,
            &vrt_mint,
            &st_mint,
            &admin,
            &base,
            fee_bps,
            fee_bps,
            fee_bps,
            decimals,
            initialize_token_amount,
        ).await?;

        // Create necessary ATAs
        create_ata(jito_client, &admin.pubkey(), &vrt_mint.pubkey(), None).await?;

        Ok(VaultRoot {
            vault_pubkey: vault,
            vault_admin: admin,
            mint: st_mint,
        })
    }

    pub async fn initialize_vault<T: JitoClient>(
        jito_client: &mut T,
        config: &Pubkey,
        vault: &Pubkey,
        burn_vault: &Pubkey,
        vrt_mint: &Keypair,
        st_mint: &Keypair,
        vault_admin: &Keypair,
        vault_base: &Keypair,
        deposit_fee_bps: u16,
        withdrawal_fee_bps: u16,
        reward_fee_bps: u16,
        decimals: u8,
        initialize_token_amount: u64,
    ) -> Result<()> {
        let blockhash = jito_client.get_recent_blockhash().await?;

        let admin_st_token_account = get_associated_token_address(&vault_admin.pubkey(), &st_mint.pubkey());
        let vault_st_token_account = get_associated_token_address(vault, &st_mint.pubkey());

        let burn_vault_vrt_account = get_associated_token_address(&burn_vault, &vrt_mint.pubkey());

        // Create ATAs first
        create_ata(jito_client, vault, &st_mint.pubkey(), None).await?;
        create_ata(jito_client, &vault_admin.pubkey(), &st_mint.pubkey(), None).await?;

        // Mint initial tokens to admin
        mint_spl_to(jito_client, &st_mint.pubkey(), &vault_admin.pubkey(), initialize_token_amount, None).await?;

        let tx = Transaction::new_signed_with_payer(
            &[initialize_vault_ix(
                config,
                vault,
                &vrt_mint.pubkey(),
                &st_mint.pubkey(),
                &admin_st_token_account,
                &vault_st_token_account,
                &burn_vault,
                &burn_vault_vrt_account,
                &vault_admin.pubkey(),
                &vault_base.pubkey(),
                &spl_token_interface::id(),
                &spl_associated_token_account_interface::program::id(),
                deposit_fee_bps,
                withdrawal_fee_bps,
                reward_fee_bps,
                decimals,
                initialize_token_amount,
            )],
            Some(&vault_admin.pubkey()),
            &[vault_admin, vrt_mint, vault_base],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn test_initialize_vault_ncn_ticket<T: JitoClient>(
        jito_client: &mut T,
        vault_root: &VaultRoot,
        ncn: &Pubkey,
    ) -> Result<()> {

        let (config, _) = config_address();
        let (vault_ncn_ticket, _) = vault_ncn_ticket_address(&vault_root.vault_pubkey, ncn);
        let (ncn_vault_ticket, _) = jito_bls_ncn_core::programs::restaking_sdk::ncn_vault_ticket_address(ncn, &vault_root.vault_pubkey);

        initialize_vault_ncn_ticket(
            jito_client,
            &config,
            &vault_root.vault_pubkey,
            ncn,
            &ncn_vault_ticket,
            &vault_ncn_ticket,
            &vault_root.vault_admin,
        ).await?;

        Ok(())
    }

    pub async fn initialize_vault_ncn_ticket<T: JitoClient>(
        jito_client: &mut T,
        config: &Pubkey,
        vault: &Pubkey,
        ncn: &Pubkey,
        ncn_vault_ticket: &Pubkey,
        vault_ncn_ticket: &Pubkey,
        admin: &Keypair,
    ) -> Result<()> {

        let blockhash = jito_client.get_recent_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[initialize_vault_ncn_ticket_ix(
                config,
                vault,
                ncn,
                ncn_vault_ticket,
                vault_ncn_ticket,
                &admin.pubkey(),
                &admin.pubkey(),
            )],
            Some(&admin.pubkey()),
            &[admin],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn test_warmup_vault_ncn_ticket<T: JitoClient>(
        jito_client: &mut T,
        vault_root: &VaultRoot,
        ncn: &Pubkey,
    ) -> Result<()> {
        let (config, _) = config_address();
        let (vault_ncn_ticket, _) = vault_ncn_ticket_address(&vault_root.vault_pubkey, ncn);

        warmup_vault_ncn_ticket(
            jito_client,
            &config,
            &vault_root.vault_pubkey,
            ncn,
            &vault_ncn_ticket,
            &vault_root.vault_admin,
        ).await?;

        Ok(())
    }


    pub async fn warmup_vault_ncn_ticket<T: JitoClient>(
        jito_client: &mut T,
        config: &Pubkey,
        vault: &Pubkey,
        ncn: &Pubkey,
        vault_ncn_ticket: &Pubkey,
        admin: &Keypair,
    ) -> Result<()> {

        let blockhash = jito_client.get_recent_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[warmup_vault_ncn_ticket_ix(
                config,
                vault,
                ncn,
                vault_ncn_ticket,
                &admin.pubkey(),
            )],
            Some(&admin.pubkey()),
            &[admin],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn test_initialize_vault_operator_delegation<T: JitoClient>(
        jito_client: &mut T,
        vault_root: &VaultRoot,
        operator: &Pubkey,
    ) -> Result<()> {

        let (config, _) = config_address();
        let (vault_operator_delegation, _) = vault_operator_delegation_address(&vault_root.vault_pubkey, operator);
        let (operator_vault_ticket, _) = jito_bls_ncn_core::programs::restaking_sdk::operator_vault_ticket_address(operator, &vault_root.vault_pubkey);

        initialize_vault_operator_delegation(
            jito_client,
            &config,
            &vault_root.vault_pubkey,
            operator,
            &operator_vault_ticket,
            &vault_operator_delegation,
            &vault_root.vault_admin,
        ).await?;

        Ok(())
    }

    pub async fn initialize_vault_operator_delegation<T: JitoClient>(
        jito_client: &mut T,
        config: &Pubkey,
        vault: &Pubkey,
        operator: &Pubkey,
        operator_vault_ticket: &Pubkey,
        vault_operator_delegation: &Pubkey,
        admin: &Keypair,
    ) -> Result<()> {

        let blockhash = jito_client.get_recent_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[initialize_vault_operator_delegation_ix(
                config,
                vault,
                operator,
                operator_vault_ticket,
                vault_operator_delegation,
                &admin.pubkey(),
                &admin.pubkey(),
            )],
            Some(&admin.pubkey()),
            &[admin],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn test_add_delegation<T: JitoClient>(
        jito_client: &mut T,
        vault_root: &VaultRoot,
        operator: &Pubkey,
        amount: u64,
    ) -> Result<()> {

        let (config, _) = config_address();
        let (vault_operator_delegation, _) = vault_operator_delegation_address(&vault_root.vault_pubkey, operator);

        add_delegation(
            jito_client,
            &config,
            &vault_root.vault_pubkey,
            operator,
            &vault_operator_delegation,
            &vault_root.vault_admin,
            amount,
        ).await?;

        Ok(())
    }

    pub async fn add_delegation<T: JitoClient>(
        jito_client: &mut T,
        config: &Pubkey,
        vault: &Pubkey,
        operator: &Pubkey,
        vault_operator_delegation: &Pubkey,
        admin: &Keypair,
        amount: u64,
    ) -> Result<()> {

        let blockhash = jito_client.get_recent_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[add_delegation_ix(
                config,
                vault,
                operator,
                vault_operator_delegation,
                &admin.pubkey(),
                amount,
            )],
            Some(&admin.pubkey()),
            &[admin],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn test_mint_to<T: JitoClient>(
        jito_client: &mut T,
        vault_root: &VaultRoot,
        depositor: &Keypair,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        let vault = get_vault(jito_client, &vault_root.vault_pubkey).await?;

        let depositor_token_account = get_associated_token_address(&depositor.pubkey(), &vault.supported_mint);
        let vault_token_account = get_associated_token_address(&vault_root.vault_pubkey, &vault.supported_mint);
        let depositor_vrt_token_account = get_associated_token_address(&depositor.pubkey(), &vault.vrt_mint);
        let vault_fee_token_account = get_associated_token_address(&vault.fee_wallet, &vault.vrt_mint);

        mint_to(
            jito_client,
            &vault_root.vault_pubkey,
            &vault.vrt_mint,
            depositor,
            &depositor_token_account,
            &vault_token_account,
            &depositor_vrt_token_account,
            &vault_fee_token_account,
            amount_in,
            min_amount_out,
        ).await?;

        Ok(())
    }

    pub async fn mint_to<T: JitoClient>(
        jito_client: &mut T,
        vault: &Pubkey,
        vrt_mint: &Pubkey,
        depositor: &Keypair,
        depositor_token_account: &Pubkey,
        vault_token_account: &Pubkey,
        depositor_vrt_token_account: &Pubkey,
        vault_fee_token_account: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {

        let (config, _) = config_address();
        let blockhash = jito_client.get_recent_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[mint_to_ix(
                &config,
                vault,
                vrt_mint,
                &depositor.pubkey(),
                depositor_token_account,
                vault_token_account,
                depositor_vrt_token_account,
                vault_fee_token_account,
                &spl_token_interface::id(),
                None,
                amount_in,
                min_amount_out,
            )],
            Some(&depositor.pubkey()),
            &[depositor],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn full_vault_update<T: JitoClient>(
        jito_client: &mut T,
        vault_pubkey: &Pubkey,
        operators: &[Pubkey],
    ) -> Result<()> {
        let slot = jito_client.get_epoch_info().await?.absolute_slot;

        let config = get_config(jito_client).await?;
        let epoch_length: u64 = config.epoch_length.into();
        let ncn_epoch: u64 = slot / epoch_length;

        let is_update_needed = get_vault_is_update_needed(jito_client, vault_pubkey, slot).await?;
        if !is_update_needed {
            return Ok(());
        }

        let vault_update_state_tracker = vault_update_state_tracker_address(
            vault_pubkey,
            ncn_epoch,
        ).0;

        initialize_vault_update_state_tracker(
            jito_client,
            vault_pubkey,
            &vault_update_state_tracker,
        ).await?;

        for i in 0..operators.len() {
            let operator_index = (i + ncn_epoch as usize) % operators.len();
            let operator = &operators[operator_index];
            let (vault_operator_delegation, _) = vault_operator_delegation_address(vault_pubkey, operator);

            crank_vault_update_state_tracker(
                jito_client,
                vault_pubkey,
                operator,
                &vault_operator_delegation,
                &vault_update_state_tracker,
            ).await?;
        }

        close_vault_update_state_tracker(
            jito_client,
            vault_pubkey,
            &vault_update_state_tracker,
            ncn_epoch,
        ).await?;

        update_vault_balance(jito_client, vault_pubkey).await?;

        Ok(())
    }

    pub async fn do_crank_vault_update_state_tracker<T: JitoClient>(
        jito_client: &mut T,
        vault: &Pubkey,
        operator: &Pubkey,
    ) -> Result<()> {
        let slot = jito_client.get_epoch_info().await?.absolute_slot;
        let config = get_config(jito_client).await?;
        let epoch_length: u64 = config.epoch_length.into();
        let ncn_epoch = slot / epoch_length;

        let (vault_operator_delegation, _) = vault_operator_delegation_address(vault, operator);
        let (vault_update_state_tracker, _) = vault_update_state_tracker_address(vault, ncn_epoch);

        crank_vault_update_state_tracker(
            jito_client,
            vault,
            operator,
            &vault_operator_delegation,
            &vault_update_state_tracker,
        ).await
    }

    pub async fn crank_vault_update_state_tracker<T: JitoClient>(
        jito_client: &mut T,
        vault: &Pubkey,
        operator: &Pubkey,
        vault_operator_delegation: &Pubkey,
        vault_update_state_tracker: &Pubkey,
    ) -> Result<()> {
        let (config, _) = config_address();
        let blockhash = jito_client.get_recent_blockhash().await?;

        let tx = Transaction::new_signed_with_payer(
            &[crank_vault_update_state_tracker_ix(
                &config,
                vault,
                operator,
                vault_operator_delegation,
                vault_update_state_tracker,
            )],
            Some(&jito_client.keypair().pubkey()),
            &[jito_client.keypair()],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn update_vault_balance<T: JitoClient>(
        jito_client: &mut T,
        vault_pubkey: &Pubkey,
    ) -> Result<()> {
        let (config, _) = config_address();
        let blockhash = jito_client.get_recent_blockhash().await?;

        let vault = get_vault(jito_client, vault_pubkey).await?;

        let tx = Transaction::new_signed_with_payer(
            &[update_vault_balance_ix(
                &config,
                vault_pubkey,
                &get_associated_token_address(vault_pubkey, &vault.supported_mint),
                &vault.vrt_mint,
                &get_associated_token_address(&vault.fee_wallet, &vault.vrt_mint),
                &spl_token_interface::id(),
            )],
            Some(&jito_client.keypair().pubkey()),
            &[jito_client.keypair()],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn initialize_vault_update_state_tracker<T: JitoClient>(
        jito_client: &mut T,
        vault_pubkey: &Pubkey,
        vault_update_state_tracker: &Pubkey,
    ) -> Result<()> {
        let (config, _) = config_address();
        let blockhash = jito_client.get_recent_blockhash().await?;

        let tx = Transaction::new_signed_with_payer(
            &[initialize_vault_update_state_tracker_ix(
                &config,
                vault_pubkey,
                vault_update_state_tracker,
                &jito_client.keypair().pubkey(),
                WithdrawalAllocationMethod::Greedy,
            )],
            Some(&jito_client.keypair().pubkey()),
            &[jito_client.keypair()],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }

    pub async fn close_vault_update_state_tracker<T: JitoClient>(
        jito_client: &mut T,
        vault_pubkey: &Pubkey,
        vault_update_state_tracker: &Pubkey,
        ncn_epoch: u64,
    ) -> Result<()> {
        let (config, _) = config_address();
        let blockhash = jito_client.get_recent_blockhash().await?;

        let tx = Transaction::new_signed_with_payer(
            &[close_vault_update_state_tracker_ix(
                &config,
                vault_pubkey,
                vault_update_state_tracker,
                &jito_client.keypair().pubkey(),
                ncn_epoch,
            )],
            Some(&jito_client.keypair().pubkey()),
            &[jito_client.keypair()],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }


// #![allow(dead_code)]

// use std::{fmt, fmt::Debug};

// use anyhow::Result;
// use jito_bytemuck::AccountDeserialize;
// use jito_restaking_core::{
//     ncn_vault_slasher_ticket::NcnVaultSlasherTicket, ncn_vault_ticket::NcnVaultTicket,
//     operator_vault_ticket::OperatorVaultTicket,
// };
// use jito_vault_core::{
//     burn_vault::BurnVault, config::Config, vault::Vault,
//     vault_ncn_slasher_operator_ticket::VaultNcnSlasherOperatorTicket,
//     vault_ncn_slasher_ticket::VaultNcnSlasherTicket, vault_ncn_ticket::VaultNcnTicket,
//     vault_operator_delegation::VaultOperatorDelegation,
//     vault_staker_withdrawal_ticket::VaultStakerWithdrawalTicket,
//     vault_update_state_tracker::VaultUpdateStateTracker,
// };
// use jito_vault_sdk::{
//     instruction::{VaultAdminRole, WithdrawalAllocationMethod},
//     sdk::{
//         add_delegation, cooldown_delegation, initialize_config, initialize_vault,
//         set_deposit_capacity, warmup_vault_ncn_slasher_ticket, warmup_vault_ncn_ticket,
//     },
// };
// use solana_commitment_config::CommitmentLevel;
// use solana_keypair::Keypair;
// use solana_program::{
//     clock::Clock, native_token::sol_str_to_lamports, program_pack::Pack, pubkey::Pubkey, rent::Rent,
// };
// use solana_program_test::{BanksClient, ProgramTestBanksClientExt};
// use solana_pubkey::pubkey;
// use solana_signer::Signer;
// use solana_system_transaction::{create_account, transfer};
// use solana_transaction::Transaction;
// use spl_associated_token_account_interface::{
//     address::get_associated_token_address, instruction::create_associated_token_account_idempotent,
// };
// use spl_token_interface::state::Account as SPLTokenAccount;

// pub const VAULT_PROGRAM_ID: Pubkey = pubkey!("Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8");
// pub const RESTAKING_PROGRAM_ID: Pubkey = pubkey!("RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q");

// pub struct VaultRoot {
//     pub vault_pubkey: Pubkey,
//     pub vault_admin: Keypair,
//     pub mint: Keypair,
// }

// impl Clone for VaultRoot {
//     fn clone(&self) -> Self {
//         Self {
//             vault_pubkey: self.vault_pubkey,
//             vault_admin: self.vault_admin.insecure_clone(),
//             mint: self.mint.insecure_clone(),
//         }
//     }
// }

// impl Debug for VaultRoot {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "VaultRoot {{ vault_pubkey: {}, vault_admin: {:?} }}",
//             self.vault_pubkey, self.vault_admin
//         )
//     }
// }

// #[derive(Debug)]
// #[allow(dead_code)]
// pub struct VaultStakerWithdrawalTicketRoot {
//     pub base: Pubkey,
// }

// pub struct VaultProgramClient {
//     banks_client: BanksClient,
//     payer: Keypair,
// }

// impl VaultProgramClient {
//     pub const fn new(banks_client: BanksClient, payer: Keypair) -> Self {
//         Self {
//             banks_client,
//             payer,
//         }
//     }

//     pub async fn configure_depositor(
//         &mut self,
//         vault_root: &VaultRoot,
//         depositor: &Pubkey,
//         amount_to_mint: u64,
//     ) -> Result<()> {
//         self.airdrop(depositor, 100.0).await?;
//         let vault = self.get_vault(&vault_root.vault_pubkey).await?;
//         self.create_ata(&vault.supported_mint, depositor).await?;
//         self.create_ata(&vault.vrt_mint, depositor).await?;
//         self.mint_spl_to(&vault.supported_mint, depositor, amount_to_mint)
//             .await?;

//         Ok(())
//     }

//     pub async fn get_config(&mut self, account: &Pubkey) -> Result<Config> {
//         let account = self.banks_client.get_account(*account).await?.unwrap();
//         Ok(*Config::try_from_slice_unchecked(account.data.as_slice())?)
//     }

//     pub async fn get_vault(&mut self, account: &Pubkey) -> Result<Vault> {
//         let account = self.banks_client.get_account(*account).await?.unwrap();
//         Ok(*Vault::try_from_slice_unchecked(account.data.as_slice())?)
//     }

//     #[allow(dead_code)]
//     pub async fn get_vault_ncn_ticket(
//         &mut self,
//         vault: &Pubkey,
//         ncn: &Pubkey,
//     ) -> Result<VaultNcnTicket> {
//         let account = VaultNcnTicket::find_program_address(&VAULT_PROGRAM_ID, vault, ncn).0;
//         let account = self.banks_client.get_account(account).await?.unwrap();
//         Ok(*VaultNcnTicket::try_from_slice_unchecked(
//             account.data.as_slice(),
//         )?)
//     }

//     #[allow(dead_code)]
//     pub async fn get_vault_operator_delegation(
//         &mut self,
//         vault: &Pubkey,
//         operator: &Pubkey,
//     ) -> Result<VaultOperatorDelegation> {
//         let account =
//             VaultOperatorDelegation::find_program_address(&VAULT_PROGRAM_ID, vault, operator).0;
//         let account = self.banks_client.get_account(account).await?.unwrap();
//         Ok(*VaultOperatorDelegation::try_from_slice_unchecked(
//             account.data.as_slice(),
//         )?)
//     }

//     #[allow(dead_code)]
//     pub async fn get_vault_staker_withdrawal_ticket(
//         &mut self,
//         vault: &Pubkey,
//         staker: &Pubkey,
//         base: &Pubkey,
//     ) -> Result<VaultStakerWithdrawalTicket> {
//         let account =
//             VaultStakerWithdrawalTicket::find_program_address(&VAULT_PROGRAM_ID, vault, base).0;
//         let account = self.banks_client.get_account(account).await?.unwrap();
//         let withdrawal_ticket =
//             *VaultStakerWithdrawalTicket::try_from_slice_unchecked(account.data.as_slice())?;
//         assert_eq!(withdrawal_ticket.staker, *staker);
//         Ok(withdrawal_ticket)
//     }

//     #[allow(dead_code)]
//     pub async fn get_vault_ncn_slasher_ticket(
//         &mut self,
//         vault: &Pubkey,
//         ncn: &Pubkey,
//         slasher: &Pubkey,
//     ) -> Result<VaultNcnSlasherTicket> {
//         let account =
//             VaultNcnSlasherTicket::find_program_address(&VAULT_PROGRAM_ID, vault, ncn, slasher).0;
//         let account = self.banks_client.get_account(account).await?.unwrap();
//         Ok(*VaultNcnSlasherTicket::try_from_slice_unchecked(
//             account.data.as_slice(),
//         )?)
//     }

//     #[allow(dead_code)]
//     pub async fn get_vault_ncn_slasher_operator_ticket(
//         &mut self,
//         vault: &Pubkey,
//         ncn: &Pubkey,
//         slasher: &Pubkey,
//         operator: &Pubkey,
//         epoch: u64,
//     ) -> Result<VaultNcnSlasherOperatorTicket> {
//         let account = VaultNcnSlasherOperatorTicket::find_program_address(
//             &VAULT_PROGRAM_ID,
//             vault,
//             ncn,
//             slasher,
//             operator,
//             epoch,
//         )
//         .0;
//         let account = self.banks_client.get_account(account).await?.unwrap();
//         Ok(*VaultNcnSlasherOperatorTicket::try_from_slice_unchecked(
//             account.data.as_slice(),
//         )?)
//     }

//     #[allow(dead_code)]
//     pub async fn get_vault_update_state_tracker(
//         &mut self,
//         vault: &Pubkey,
//         epoch: u64,
//     ) -> Result<VaultUpdateStateTracker> {
//         let account =
//             VaultUpdateStateTracker::find_program_address(&VAULT_PROGRAM_ID, vault, epoch).0;
//         let account = self.banks_client.get_account(account).await?.unwrap();
//         Ok(*VaultUpdateStateTracker::try_from_slice_unchecked(
//             account.data.as_slice(),
//         )?)
//     }

//     pub async fn get_vault_is_update_needed(&mut self, vault: &Pubkey, slot: u64) -> Result<bool> {
//         let vault_config = self
//             .get_config(&Config::find_program_address(&VAULT_PROGRAM_ID).0)
//             .await?;
//         let vault_account = self.get_vault(vault).await?;

//         let is_update_needed = vault_account.is_update_needed(slot, vault_config.epoch_length())?;
//         Ok(is_update_needed)
//     }

//     pub async fn do_initialize_config(&mut self) -> Result<Keypair> {
//         let config_admin = Keypair::new();

//         self.airdrop(&config_admin.pubkey(), 1.0).await?;

//         let config_pubkey = Config::find_program_address(&VAULT_PROGRAM_ID).0;
//         self.initialize_config(&config_pubkey, &config_admin, &config_admin.pubkey(), 0)
//             .await?;

//         Ok(config_admin)
//     }

//     pub async fn initialize_config(
//         &mut self,
//         config: &Pubkey,
//         config_admin: &Keypair,
//         program_fee_wallet: &Pubkey,
//         program_fee_bps: u16,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[initialize_config(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 &config_admin.pubkey(),
//                 &RESTAKING_PROGRAM_ID,
//                 program_fee_wallet,
//                 program_fee_bps,
//             )],
//             Some(&config_admin.pubkey()),
//             &[config_admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn setup_config_and_vault(
//         &mut self,
//         deposit_fee_bps: u16,
//         withdrawal_fee_bps: u16,
//         reward_fee_bps: u16,
//     ) -> Result<(Keypair, VaultRoot)> {
//         let config_admin = self.do_initialize_config().await?;
//         let vault_root = self
//             .do_initialize_vault(
//                 deposit_fee_bps,
//                 withdrawal_fee_bps,
//                 reward_fee_bps,
//                 9,
//                 &config_admin.pubkey(),
//                 None,
//             )
//             .await?;

//         Ok((config_admin, vault_root))
//     }

//     pub async fn do_initialize_vault(
//         &mut self,
//         deposit_fee_bps: u16,
//         withdrawal_fee_bps: u16,
//         reward_fee_bps: u16,
//         decimals: u8,
//         program_fee_wallet: &Pubkey,
//         token_mint: Option<Keypair>,
//     ) -> Result<VaultRoot> {
//         let vault_base = Keypair::new();

//         let initialize_token_amount = Vault::DEFAULT_INITIALIZATION_TOKEN_AMOUNT;

//         let vault_pubkey = Vault::find_program_address(&VAULT_PROGRAM_ID, &vault_base.pubkey()).0;

//         let vrt_mint = Keypair::new();
//         let vault_admin = Keypair::new();
//         let token_mint = token_mint.unwrap_or_else(Keypair::new);

//         self.airdrop(&vault_admin.pubkey(), 100.0).await?;

//         let should_create_mint = {
//             let raw_account = self.banks_client.get_account(token_mint.pubkey()).await?;
//             raw_account.is_none()
//         };
//         if should_create_mint {
//             self.create_token_mint(&token_mint, &spl_token_interface::id())
//                 .await?;
//         }

//         self.initialize_vault(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_pubkey,
//             &vrt_mint,
//             &token_mint,
//             &vault_admin,
//             &vault_base,
//             deposit_fee_bps,
//             withdrawal_fee_bps,
//             reward_fee_bps,
//             decimals,
//             initialize_token_amount,
//         )
//         .await?;

//         // for holding the backed asset in the vault
//         self.create_ata(&token_mint.pubkey(), &vault_pubkey).await?;
//         // for holding fees
//         self.create_ata(&vrt_mint.pubkey(), &vault_admin.pubkey())
//             .await?;
//         // for holding program fee
//         self.create_ata(&vrt_mint.pubkey(), program_fee_wallet)
//             .await?;

//         // for holding program fee
//         Ok(VaultRoot {
//             vault_admin,
//             vault_pubkey,
//             mint: token_mint,
//         })
//     }

//     pub async fn do_initialize_vault_ncn_ticket(
//         &mut self,
//         vault_root: &VaultRoot,
//         ncn: &Pubkey,
//     ) -> Result<()> {
//         let vault_ncn_ticket =
//             VaultNcnTicket::find_program_address(&VAULT_PROGRAM_ID, &vault_root.vault_pubkey, ncn)
//                 .0;
//         let ncn_vault_ticket = NcnVaultTicket::find_program_address(
//             &RESTAKING_PROGRAM_ID,
//             ncn,
//             &vault_root.vault_pubkey,
//         )
//         .0;
//         self.initialize_vault_ncn_ticket(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             ncn,
//             &ncn_vault_ticket,
//             &vault_ncn_ticket,
//             &vault_root.vault_admin,
//             &self.payer.insecure_clone(),
//         )
//         .await?;

//         Ok(())
//     }

//     #[allow(dead_code)]
//     pub async fn set_capacity(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         admin: &Keypair,
//         capacity: u64,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[set_deposit_capacity(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 &admin.pubkey(),
//                 capacity,
//             )],
//             Some(&admin.pubkey()),
//             &[&admin],
//             blockhash,
//         ))
//         .await
//     }

//     pub async fn do_warmup_vault_ncn_ticket(
//         &mut self,
//         vault_root: &VaultRoot,
//         ncn: &Pubkey,
//     ) -> Result<()> {
//         let vault_ncn_ticket =
//             VaultNcnTicket::find_program_address(&VAULT_PROGRAM_ID, &vault_root.vault_pubkey, ncn)
//                 .0;

//         self.warmup_vault_ncn_ticket(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             ncn,
//             &vault_ncn_ticket,
//             &vault_root.vault_admin,
//         )
//         .await?;

//         Ok(())
//     }

//     pub async fn warmup_vault_ncn_ticket(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         ncn: &Pubkey,
//         vault_ncn_ticket: &Pubkey,
//         ncn_vault_admin: &Keypair,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[warmup_vault_ncn_ticket(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 ncn,
//                 vault_ncn_ticket,
//                 &ncn_vault_admin.pubkey(),
//             )],
//             Some(&ncn_vault_admin.pubkey()),
//             &[&ncn_vault_admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn setup_vault_ncn_slasher_operator_ticket(
//         &mut self,
//         vault_root: &VaultRoot,
//         ncn_pubkey: &Pubkey,
//         slasher: &Pubkey,
//         operator_pubkey: &Pubkey,
//     ) -> Result<()> {
//         let config = self
//             .get_config(&Config::find_program_address(&VAULT_PROGRAM_ID).0)
//             .await
//             .unwrap();
//         let clock: Clock = self.banks_client.get_sysvar().await?;

//         let vault_ncn_slasher_ticket = VaultNcnSlasherTicket::find_program_address(
//             &VAULT_PROGRAM_ID,
//             &vault_root.vault_pubkey,
//             ncn_pubkey,
//             slasher,
//         )
//         .0;
//         let vault_ncn_slasher_operator_ticket =
//             VaultNcnSlasherOperatorTicket::find_program_address(
//                 &VAULT_PROGRAM_ID,
//                 &vault_root.vault_pubkey,
//                 ncn_pubkey,
//                 slasher,
//                 operator_pubkey,
//                 clock.slot / config.epoch_length(),
//             )
//             .0;
//         self.initialize_vault_ncn_slasher_operator_ticket(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             ncn_pubkey,
//             slasher,
//             operator_pubkey,
//             &vault_ncn_slasher_ticket,
//             &vault_ncn_slasher_operator_ticket,
//             &self.payer.insecure_clone(),
//         )
//         .await
//         .unwrap();

//         Ok(())
//     }

//     pub async fn do_initialize_vault_operator_delegation(
//         &mut self,
//         vault_root: &VaultRoot,
//         operator_pubkey: &Pubkey,
//     ) -> Result<()> {
//         let vault_operator_delegation = VaultOperatorDelegation::find_program_address(
//             &VAULT_PROGRAM_ID,
//             &vault_root.vault_pubkey,
//             operator_pubkey,
//         )
//         .0;
//         let operator_vault_ticket = OperatorVaultTicket::find_program_address(
//             &RESTAKING_PROGRAM_ID,
//             operator_pubkey,
//             &vault_root.vault_pubkey,
//         )
//         .0;
//         self.initialize_vault_operator_delegation(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             operator_pubkey,
//             &operator_vault_ticket,
//             &vault_operator_delegation,
//             &vault_root.vault_admin,
//             &vault_root.vault_admin,
//         )
//         .await?;

//         Ok(())
//     }

//     #[allow(dead_code)]
//     pub async fn do_initialize_vault_ncn_slasher_ticket(
//         &mut self,
//         vault_root: &VaultRoot,
//         ncn_pubkey: &Pubkey,
//         slasher: &Pubkey,
//     ) -> Result<()> {
//         let vault_slasher_ticket_pubkey = VaultNcnSlasherTicket::find_program_address(
//             &VAULT_PROGRAM_ID,
//             &vault_root.vault_pubkey,
//             ncn_pubkey,
//             slasher,
//         )
//         .0;
//         let ncn_slasher_ticket_pubkey = NcnVaultSlasherTicket::find_program_address(
//             &RESTAKING_PROGRAM_ID,
//             ncn_pubkey,
//             &vault_root.vault_pubkey,
//             slasher,
//         )
//         .0;

//         self.initialize_vault_ncn_slasher_ticket(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             ncn_pubkey,
//             slasher,
//             &ncn_slasher_ticket_pubkey,
//             &vault_slasher_ticket_pubkey,
//             &vault_root.vault_admin,
//             &vault_root.vault_admin,
//         )
//         .await?;

//         Ok(())
//     }

//     #[allow(dead_code)]
//     pub async fn do_warmup_vault_ncn_slasher_ticket(
//         &mut self,
//         vault_root: &VaultRoot,
//         ncn_pubkey: &Pubkey,
//         slasher: &Pubkey,
//     ) -> Result<()> {
//         let vault_slasher_ticket_pubkey = VaultNcnSlasherTicket::find_program_address(
//             &VAULT_PROGRAM_ID,
//             &vault_root.vault_pubkey,
//             ncn_pubkey,
//             slasher,
//         )
//         .0;

//         self.warmup_vault_ncn_slasher_ticket(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             ncn_pubkey,
//             slasher,
//             &vault_slasher_ticket_pubkey,
//             &vault_root.vault_admin,
//         )
//         .await?;

//         Ok(())
//     }

//     pub async fn warmup_vault_ncn_slasher_ticket(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         ncn: &Pubkey,
//         slasher: &Pubkey,
//         vault_ncn_slasher_ticket: &Pubkey,
//         admin: &Keypair,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[warmup_vault_ncn_slasher_ticket(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 ncn,
//                 slasher,
//                 vault_ncn_slasher_ticket,
//                 &admin.pubkey(),
//             )],
//             Some(&admin.pubkey()),
//             &[admin],
//             blockhash,
//         ))
//         .await
//     }

//     pub async fn do_add_delegation(
//         &mut self,
//         vault_root: &VaultRoot,
//         operator: &Pubkey,
//         amount: u64,
//     ) -> Result<()> {
//         self.add_delegation(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             operator,
//             &VaultOperatorDelegation::find_program_address(
//                 &VAULT_PROGRAM_ID,
//                 &vault_root.vault_pubkey,
//                 operator,
//             )
//             .0,
//             &vault_root.vault_admin,
//             amount,
//         )
//         .await?;

//         Ok(())
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn initialize_vault(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         vrt_mint: &Keypair,
//         st_mint: &Keypair,
//         vault_admin: &Keypair,
//         vault_base: &Keypair,
//         deposit_fee_bps: u16,
//         withdrawal_fee_bps: u16,
//         reward_fee_bps: u16,
//         decimals: u8,
//         initialize_token_amount: u64,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         let admin_st_token_account =
//             get_associated_token_address(&vault_admin.pubkey(), &st_mint.pubkey());
//         let vault_st_token_account = get_associated_token_address(vault, &st_mint.pubkey());

//         let burn_vault = BurnVault::find_program_address(&VAULT_PROGRAM_ID, &vault_base.pubkey()).0;

//         let burn_vault_vrt_token_account =
//             get_associated_token_address(&burn_vault, &vrt_mint.pubkey());

//         self.create_ata(&st_mint.pubkey(), vault).await?;
//         self.create_ata(&st_mint.pubkey(), &vault_admin.pubkey())
//             .await?;

//         self.mint_spl_to(
//             &st_mint.pubkey(),
//             &vault_admin.pubkey(),
//             initialize_token_amount,
//         )
//         .await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[initialize_vault(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 &vrt_mint.pubkey(),
//                 &st_mint.pubkey(),
//                 &admin_st_token_account,
//                 &vault_st_token_account,
//                 &burn_vault,
//                 &burn_vault_vrt_token_account,
//                 &vault_admin.pubkey(),
//                 &vault_base.pubkey(),
//                 deposit_fee_bps,
//                 withdrawal_fee_bps,
//                 reward_fee_bps,
//                 decimals,
//                 initialize_token_amount,
//             )],
//             Some(&vault_admin.pubkey()),
//             &[&vault_admin, &vrt_mint, &vault_base],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn initialize_vault_ncn_ticket(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         ncn: &Pubkey,
//         ncn_vault_ticket: &Pubkey,
//         vault_ncn_ticket: &Pubkey,
//         admin: &Keypair,
//         payer: &Keypair,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::initialize_vault_ncn_ticket(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 ncn,
//                 ncn_vault_ticket,
//                 vault_ncn_ticket,
//                 &admin.pubkey(),
//                 &payer.pubkey(),
//             )],
//             Some(&payer.pubkey()),
//             &[admin, payer],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn initialize_vault_operator_delegation(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         operator: &Pubkey,
//         operator_vault_ticket: &Pubkey,
//         vault_operator_delegation: &Pubkey,
//         admin: &Keypair,
//         payer: &Keypair,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::initialize_vault_operator_delegation(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 operator,
//                 operator_vault_ticket,
//                 vault_operator_delegation,
//                 &admin.pubkey(),
//                 &payer.pubkey(),
//             )],
//             Some(&payer.pubkey()),
//             &[admin, payer],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn delegate_token_account(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         delegate_asset_admin: &Keypair,
//         token_mint: &Pubkey,
//         token_account: &Pubkey,
//         delegate: &Pubkey,
//         token_program_id: &Pubkey,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::delegate_token_account(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 &delegate_asset_admin.pubkey(),
//                 token_mint,
//                 token_account,
//                 delegate,
//                 token_program_id,
//             )],
//             Some(&self.payer.pubkey()),
//             &[&self.payer, delegate_asset_admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn set_admin(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         old_admin: &Keypair,
//         new_admin: &Keypair,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::set_admin(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 &old_admin.pubkey(),
//                 &new_admin.pubkey(),
//             )],
//             Some(&old_admin.pubkey()),
//             &[old_admin, new_admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn set_secondary_admin(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         admin: &Keypair,
//         new_admin: &Pubkey,
//         role: VaultAdminRole,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::set_secondary_admin(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 &admin.pubkey(),
//                 new_admin,
//                 role,
//             )],
//             Some(&admin.pubkey()),
//             &[admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn set_fees(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         fee_admin: &Keypair,
//         deposit_fee_bps: Option<u16>,
//         withdrawal_fee_bps: Option<u16>,
//         reward_fee_bps: Option<u16>,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::set_fees(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 &fee_admin.pubkey(),
//                 deposit_fee_bps,
//                 withdrawal_fee_bps,
//                 reward_fee_bps,
//             )],
//             Some(&fee_admin.pubkey()),
//             &[fee_admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn set_program_fee(
//         &mut self,
//         config_admin: &Keypair,
//         new_fee_bps: u16,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::set_program_fee(
//                 &VAULT_PROGRAM_ID,
//                 &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//                 &config_admin.pubkey(),
//                 new_fee_bps,
//             )],
//             Some(&config_admin.pubkey()),
//             &[config_admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn do_enqueue_withdrawal(
//         &mut self,
//         vault_root: &VaultRoot,
//         depositor: &Keypair,
//         amount: u64,
//     ) -> Result<VaultStakerWithdrawalTicketRoot> {
//         let vault = self.get_vault(&vault_root.vault_pubkey).await.unwrap();
//         let depositor_vrt_token_account =
//             get_associated_token_address(&depositor.pubkey(), &vault.vrt_mint);

//         let base = Keypair::new();
//         let vault_staker_withdrawal_ticket = VaultStakerWithdrawalTicket::find_program_address(
//             &VAULT_PROGRAM_ID,
//             &vault_root.vault_pubkey,
//             &base.pubkey(),
//         )
//         .0;
//         println!(
//             "vault_staker_withdrawal_ticket: {:?}",
//             vault_staker_withdrawal_ticket
//         );
//         let vault_staker_withdrawal_ticket_token_account =
//             get_associated_token_address(&vault_staker_withdrawal_ticket, &vault.vrt_mint);

//         self.create_ata(&vault.vrt_mint, &vault_staker_withdrawal_ticket)
//             .await?;

//         self.enqueue_withdrawal(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             &vault_staker_withdrawal_ticket,
//             &vault_staker_withdrawal_ticket_token_account,
//             depositor,
//             &depositor_vrt_token_account,
//             &base,
//             amount,
//         )
//         .await?;

//         Ok(VaultStakerWithdrawalTicketRoot {
//             base: base.pubkey(),
//         })
//     }

//     #[allow(dead_code)]
//     pub async fn do_cooldown_delegation(
//         &mut self,
//         vault_root: &VaultRoot,
//         operator: &Pubkey,
//         amount: u64,
//     ) -> Result<()> {
//         self.cooldown_delegation(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             operator,
//             &VaultOperatorDelegation::find_program_address(
//                 &VAULT_PROGRAM_ID,
//                 &vault_root.vault_pubkey,
//                 operator,
//             )
//             .0,
//             &vault_root.vault_admin,
//             amount,
//         )
//         .await
//     }

//     pub async fn cooldown_delegation(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         operator: &Pubkey,
//         vault_operator_delegation: &Pubkey,
//         admin: &Keypair,
//         amount: u64,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[cooldown_delegation(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 operator,
//                 vault_operator_delegation,
//                 &admin.pubkey(),
//                 amount,
//             )],
//             Some(&self.payer.pubkey()),
//             &[&self.payer, admin],
//             blockhash,
//         ))
//         .await
//     }

//     pub async fn do_full_vault_update(
//         &mut self,
//         vault_pubkey: &Pubkey,
//         operators: &[Pubkey],
//     ) -> Result<()> {
//         let slot = self.banks_client.get_sysvar::<Clock>().await?.slot;

//         let config = self
//             .get_config(&Config::find_program_address(&VAULT_PROGRAM_ID).0)
//             .await?;

//         let ncn_epoch = slot / config.epoch_length();

//         let vault_update_state_tracker = VaultUpdateStateTracker::find_program_address(
//             &VAULT_PROGRAM_ID,
//             vault_pubkey,
//             ncn_epoch,
//         )
//         .0;
//         self.initialize_vault_update_state_tracker(vault_pubkey, &vault_update_state_tracker)
//             .await?;

//         for i in 0..operators.len() {
//             let operator_index = (i + ncn_epoch as usize) % operators.len();
//             let operator = &operators[operator_index];
//             self.crank_vault_update_state_tracker(
//                 vault_pubkey,
//                 operator,
//                 &VaultOperatorDelegation::find_program_address(
//                     &VAULT_PROGRAM_ID,
//                     vault_pubkey,
//                     operator,
//                 )
//                 .0,
//                 &vault_update_state_tracker,
//             )
//             .await?;
//         }

//         self.close_vault_update_state_tracker(
//             vault_pubkey,
//             &vault_update_state_tracker,
//             slot / config.epoch_length(),
//         )
//         .await?;

//         self.update_vault_balance(vault_pubkey).await?;

//         Ok(())
//     }

//     #[allow(dead_code)]
//     pub async fn do_crank_vault_update_state_tracker(
//         &mut self,
//         vault: &Pubkey,
//         operator: &Pubkey,
//     ) -> Result<()> {
//         let slot = self.banks_client.get_sysvar::<Clock>().await?.slot;
//         let config = self
//             .get_config(&Config::find_program_address(&VAULT_PROGRAM_ID).0)
//             .await?;
//         let ncn_epoch = slot / config.epoch_length();
//         self.crank_vault_update_state_tracker(
//             vault,
//             operator,
//             &VaultOperatorDelegation::find_program_address(&VAULT_PROGRAM_ID, vault, operator).0,
//             &VaultUpdateStateTracker::find_program_address(&VAULT_PROGRAM_ID, vault, ncn_epoch).0,
//         )
//         .await
//     }

//     pub async fn crank_vault_update_state_tracker(
//         &mut self,
//         vault: &Pubkey,
//         operator: &Pubkey,
//         vault_operator_delegation: &Pubkey,
//         vault_update_state_tracker: &Pubkey,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::crank_vault_update_state_tracker(
//                 &VAULT_PROGRAM_ID,
//                 &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//                 vault,
//                 operator,
//                 vault_operator_delegation,
//                 vault_update_state_tracker,
//             )],
//             Some(&self.payer.pubkey()),
//             &[&self.payer],
//             blockhash,
//         ))
//         .await?;
//         Ok(())
//     }

//     pub async fn update_vault_balance(&mut self, vault_pubkey: &Pubkey) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         let vault = self.get_vault(vault_pubkey).await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::update_vault_balance(
//                 &VAULT_PROGRAM_ID,
//                 &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//                 vault_pubkey,
//                 &get_associated_token_address(vault_pubkey, &vault.supported_mint),
//                 &vault.vrt_mint,
//                 &get_associated_token_address(&vault.fee_wallet, &vault.vrt_mint),
//                 &spl_token_interface::ID,
//             )],
//             Some(&self.payer.pubkey()),
//             &[&self.payer],
//             blockhash,
//         ))
//         .await?;

//         Ok(())
//     }

//     pub async fn initialize_vault_update_state_tracker(
//         &mut self,
//         vault_pubkey: &Pubkey,
//         vault_update_state_tracker: &Pubkey,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::initialize_vault_update_state_tracker(
//                 &VAULT_PROGRAM_ID,
//                 &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//                 vault_pubkey,
//                 vault_update_state_tracker,
//                 &self.payer.pubkey(),
//                 WithdrawalAllocationMethod::Greedy,
//             )],
//             Some(&self.payer.pubkey()),
//             &[&self.payer],
//             blockhash,
//         ))
//         .await?;
//         Ok(())
//     }

//     pub async fn close_vault_update_state_tracker(
//         &mut self,
//         vault_pubkey: &Pubkey,
//         vault_update_state_tracker: &Pubkey,
//         ncn_epoch: u64,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::close_vault_update_state_tracker(
//                 &VAULT_PROGRAM_ID,
//                 &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//                 vault_pubkey,
//                 vault_update_state_tracker,
//                 &self.payer.pubkey(),
//                 ncn_epoch,
//             )],
//             Some(&self.payer.pubkey()),
//             &[&self.payer],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn enqueue_withdrawal(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         vault_staker_withdrawal_ticket: &Pubkey,
//         vault_staker_withdrawal_ticket_token_account: &Pubkey,
//         staker: &Keypair,
//         staker_vrt_token_account: &Pubkey,
//         base: &Keypair,
//         amount: u64,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::enqueue_withdrawal(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 vault_staker_withdrawal_ticket,
//                 vault_staker_withdrawal_ticket_token_account,
//                 &staker.pubkey(),
//                 staker_vrt_token_account,
//                 &base.pubkey(),
//                 None,
//                 amount,
//             )],
//             Some(&staker.pubkey()),
//             &[staker, base],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn do_burn_withdrawal_ticket(
//         &mut self,
//         vault_root: &VaultRoot,
//         staker: &Keypair,
//         vault_staker_withdrawal_ticket_base: &Pubkey,
//         program_fee_wallet: &Pubkey,
//     ) -> Result<()> {
//         let vault = self.get_vault(&vault_root.vault_pubkey).await.unwrap();
//         let vault_staker_withdrawal_ticket = VaultStakerWithdrawalTicket::find_program_address(
//             &VAULT_PROGRAM_ID,
//             &vault_root.vault_pubkey,
//             vault_staker_withdrawal_ticket_base,
//         )
//         .0;

//         self.burn_withdrawal_ticket(
//             &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//             &vault_root.vault_pubkey,
//             &get_associated_token_address(&vault_root.vault_pubkey, &vault.supported_mint),
//             &vault.vrt_mint,
//             &staker.pubkey(),
//             &get_associated_token_address(&staker.pubkey(), &vault.supported_mint),
//             &vault_staker_withdrawal_ticket,
//             &get_associated_token_address(&vault_staker_withdrawal_ticket, &vault.vrt_mint),
//             &get_associated_token_address(&vault.fee_wallet, &vault.vrt_mint),
//             &get_associated_token_address(program_fee_wallet, &vault.vrt_mint),
//         )
//         .await?;

//         Ok(())
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn burn_withdrawal_ticket(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         vault_token_account: &Pubkey,
//         vrt_mint: &Pubkey,
//         staker: &Pubkey,
//         staker_token_account: &Pubkey,
//         vault_staker_withdrawal_ticket: &Pubkey,
//         vault_staker_withdrawal_ticket_token_account: &Pubkey,
//         vault_fee_token_account: &Pubkey,
//         program_fee_vrt_token_account: &Pubkey,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::burn_withdrawal_ticket(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 vault_token_account,
//                 vrt_mint,
//                 staker,
//                 staker_token_account,
//                 vault_staker_withdrawal_ticket,
//                 vault_staker_withdrawal_ticket_token_account,
//                 vault_fee_token_account,
//                 program_fee_vrt_token_account,
//                 None,
//             )],
//             Some(&self.payer.pubkey()),
//             &[&self.payer],
//             blockhash,
//         ))
//         .await
//     }

//     pub async fn add_delegation(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         operator: &Pubkey,
//         vault_operator_delegation: &Pubkey,
//         admin: &Keypair,
//         amount: u64,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[add_delegation(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 operator,
//                 vault_operator_delegation,
//                 &admin.pubkey(),
//                 amount,
//             )],
//             Some(&admin.pubkey()),
//             &[admin],
//             blockhash,
//         ))
//         .await
//     }

//     pub async fn do_mint_to(
//         &mut self,
//         vault_root: &VaultRoot,
//         depositor: &Keypair,
//         amount_in: u64,
//         min_amount_out: u64,
//     ) -> Result<()> {
//         let vault = self.get_vault(&vault_root.vault_pubkey).await.unwrap();
//         self.mint_to(
//             &vault_root.vault_pubkey,
//             &vault.vrt_mint,
//             depositor,
//             &get_associated_token_address(&depositor.pubkey(), &vault.supported_mint),
//             &get_associated_token_address(&vault_root.vault_pubkey, &vault.supported_mint),
//             &get_associated_token_address(&depositor.pubkey(), &vault.vrt_mint),
//             &get_associated_token_address(&vault.fee_wallet, &vault.vrt_mint),
//             None,
//             amount_in,
//             min_amount_out,
//         )
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn mint_to(
//         &mut self,
//         vault: &Pubkey,
//         vrt_mint: &Pubkey,
//         depositor: &Keypair,
//         depositor_token_account: &Pubkey,
//         vault_token_account: &Pubkey,
//         depositor_vrt_token_account: &Pubkey,
//         vault_fee_token_account: &Pubkey,
//         mint_signer: Option<&Keypair>,
//         amount_in: u64,
//         min_amount_out: u64,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         let mut signers = vec![depositor];
//         if let Some(signer) = mint_signer {
//             signers.push(signer);
//         }
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::mint_to(
//                 &VAULT_PROGRAM_ID,
//                 &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//                 vault,
//                 vrt_mint,
//                 &depositor.pubkey(),
//                 depositor_token_account,
//                 vault_token_account,
//                 depositor_vrt_token_account,
//                 vault_fee_token_account,
//                 mint_signer.map(|s| s.pubkey()).as_ref(),
//                 amount_in,
//                 min_amount_out,
//             )],
//             Some(&depositor.pubkey()),
//             &signers,
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn initialize_vault_ncn_slasher_ticket(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         ncn: &Pubkey,
//         slasher: &Pubkey,
//         ncn_slasher_ticket: &Pubkey,
//         vault_slasher_ticket: &Pubkey,
//         admin: &Keypair,
//         payer: &Keypair,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::initialize_vault_ncn_slasher_ticket(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 vault,
//                 ncn,
//                 slasher,
//                 ncn_slasher_ticket,
//                 vault_slasher_ticket,
//                 &admin.pubkey(),
//                 &payer.pubkey(),
//             )],
//             Some(&payer.pubkey()),
//             &[admin, payer],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn initialize_vault_ncn_slasher_operator_ticket(
//         &mut self,
//         config: &Pubkey,
//         vault: &Pubkey,
//         ncn: &Pubkey,
//         slasher: &Pubkey,
//         operator: &Pubkey,
//         vault_ncn_slasher_ticket: &Pubkey,
//         vault_ncn_slasher_operator_ticket: &Pubkey,
//         payer: &Keypair,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[
//                 jito_vault_sdk::sdk::initialize_vault_ncn_slasher_operator_ticket(
//                     &VAULT_PROGRAM_ID,
//                     config,
//                     vault,
//                     ncn,
//                     slasher,
//                     operator,
//                     vault_ncn_slasher_ticket,
//                     vault_ncn_slasher_operator_ticket,
//                     &payer.pubkey(),
//                 ),
//             ],
//             Some(&payer.pubkey()),
//             &[payer],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn create_token_metadata(
//         &mut self,
//         vault: &Pubkey,
//         admin: &Keypair,
//         vrt_mint: &Pubkey,
//         payer: &Keypair,
//         metadata: &Pubkey,
//         name: String,
//         symbol: String,
//         uri: String,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;

//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::create_token_metadata(
//                 &VAULT_PROGRAM_ID,
//                 vault,
//                 &admin.pubkey(),
//                 vrt_mint,
//                 &payer.pubkey(),
//                 metadata,
//                 name,
//                 symbol,
//                 uri,
//             )],
//             Some(&payer.pubkey()),
//             &[admin, payer],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(clippy::too_many_arguments)]
//     pub async fn update_token_metadata(
//         &mut self,
//         vault: &Pubkey,
//         admin: &Keypair,
//         vrt_mint: &Pubkey,
//         metadata: &Pubkey,
//         name: String,
//         symbol: String,
//         uri: String,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::update_token_metadata(
//                 &VAULT_PROGRAM_ID,
//                 vault,
//                 &admin.pubkey(),
//                 vrt_mint,
//                 metadata,
//                 name,
//                 symbol,
//                 uri,
//             )],
//             Some(&self.payer.pubkey()),
//             &[&self.payer, admin],
//             blockhash,
//         ))
//         .await
//     }

//     async fn _process_transaction(&mut self, tx: &Transaction) -> Result<()> {
//         self.banks_client
//             .process_transaction_with_preflight_and_commitment(
//                 tx.clone(),
//                 CommitmentLevel::Processed,
//             )
//             .await?;
//         Ok(())
//     }

//     pub async fn airdrop(&mut self, to: &Pubkey, sol: f64) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         let new_blockhash = self
//             .banks_client
//             .get_new_latest_blockhash(&blockhash)
//             .await
//             .unwrap();

//         let tx = transfer(
//             &self.payer,
//             to,
//             sol_str_to_lamports(&sol.to_string()).unwrap(),
//             new_blockhash,
//         );

//         self.banks_client
//             .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Processed)
//             .await?;
//         Ok(())
//     }

//     pub async fn create_token_mint(
//         &mut self,
//         mint: &Keypair,
//         token_program_id: &Pubkey,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         let rent: Rent = self.banks_client.get_sysvar().await?;

//         let tx = create_account(
//             &self.payer,
//             mint,
//             blockhash,
//             rent.minimum_balance(spl_token_interface::state::Mint::LEN),
//             spl_token_interface::state::Mint::LEN as u64,
//             token_program_id,
//         );

//         self.banks_client
//             .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Processed)
//             .await?;

//         let ixs = vec![spl_token_interface::instruction::initialize_mint2(
//             token_program_id,
//             &mint.pubkey(),
//             &self.payer.pubkey(),
//             None,
//             9,
//         )
//         .unwrap()];
//         self.banks_client
//             .process_transaction_with_preflight_and_commitment(
//                 Transaction::new_signed_with_payer(
//                     &ixs,
//                     Some(&self.payer.pubkey()),
//                     &[&self.payer, mint],
//                     blockhash,
//                 ),
//                 CommitmentLevel::Processed,
//             )
//             .await?;
//         Ok(())
//     }

//     pub async fn create_ata(&mut self, mint: &Pubkey, owner: &Pubkey) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self.banks_client
//             .process_transaction_with_preflight_and_commitment(
//                 Transaction::new_signed_with_payer(
//                     &[create_associated_token_account_idempotent(
//                         &self.payer.pubkey(),
//                         owner,
//                         mint,
//                         &spl_token_interface::id(),
//                     )],
//                     Some(&self.payer.pubkey()),
//                     &[&self.payer],
//                     blockhash,
//                 ),
//                 CommitmentLevel::Processed,
//             )
//             .await?;
//         Ok(())
//     }

//     /// Mints tokens to an ATA owned by the `to` address
//     pub async fn mint_spl_to(&mut self, mint: &Pubkey, to: &Pubkey, amount: u64) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self.banks_client
//             .process_transaction_with_preflight_and_commitment(
//                 Transaction::new_signed_with_payer(
//                     &[
//                         create_associated_token_account_idempotent(
//                             &self.payer.pubkey(),
//                             to,
//                             mint,
//                             &spl_token_interface::id(),
//                         ),
//                         spl_token_interface::instruction::mint_to(
//                             &spl_token_interface::id(),
//                             mint,
//                             &get_associated_token_address(to, mint),
//                             &self.payer.pubkey(),
//                             &[],
//                             amount,
//                         )
//                         .unwrap(),
//                     ],
//                     Some(&self.payer.pubkey()),
//                     &[&self.payer],
//                     blockhash,
//                 ),
//                 CommitmentLevel::Processed,
//             )
//             .await?;

//         Ok(())
//     }

//     #[allow(dead_code)]
//     pub async fn get_reward_fee_token_account(
//         &mut self,
//         vault: &Pubkey,
//     ) -> Result<SPLTokenAccount> {
//         let vault = self.get_vault(vault).await.unwrap();

//         let vault_fee_token_account =
//             get_associated_token_address(&vault.fee_wallet, &vault.vrt_mint);

//         let account = self
//             .banks_client
//             .get_account(vault_fee_token_account)
//             .await
//             .unwrap()
//             .unwrap();

//         Ok(SPLTokenAccount::unpack(&account.data).unwrap())
//     }

//     #[allow(dead_code)]
//     pub async fn create_and_fund_reward_vault(
//         &mut self,
//         vault: &Pubkey,
//         rewarder: &Keypair,
//         amount: u64,
//     ) -> Result<()> {
//         let vault_account = self.get_vault(vault).await.unwrap();

//         let rewarder_token_account =
//             get_associated_token_address(&rewarder.pubkey(), &vault_account.supported_mint);

//         let vault_token_account =
//             get_associated_token_address(vault, &vault_account.supported_mint);

//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self.banks_client
//             .process_transaction_with_preflight_and_commitment(
//                 Transaction::new_signed_with_payer(
//                     &[
//                         create_associated_token_account_idempotent(
//                             &rewarder.pubkey(),
//                             &vault_token_account,
//                             &vault_account.supported_mint,
//                             &spl_token_interface::id(),
//                         ),
//                         spl_token_interface::instruction::transfer(
//                             &spl_token_interface::id(),
//                             &rewarder_token_account,
//                             &vault_token_account,
//                             &rewarder.pubkey(),
//                             &[],
//                             amount,
//                         )
//                         .unwrap(),
//                     ],
//                     Some(&rewarder.pubkey()),
//                     &[&rewarder],
//                     blockhash,
//                 ),
//                 CommitmentLevel::Processed,
//             )
//             .await?;

//         Ok(())
//     }

//     #[allow(dead_code)]
//     pub async fn set_program_fee_wallet(
//         &mut self,
//         program_fee_admin: &Keypair,
//         new_fee_wallet: &Pubkey,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::set_program_fee_wallet(
//                 &VAULT_PROGRAM_ID,
//                 &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//                 &program_fee_admin.pubkey(),
//                 new_fee_wallet,
//             )],
//             Some(&program_fee_admin.pubkey()),
//             &[program_fee_admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn set_is_paused(
//         &mut self,
//         vault: &Pubkey,
//         admin: &Keypair,
//         is_paused: bool,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::set_is_paused(
//                 &VAULT_PROGRAM_ID,
//                 &Config::find_program_address(&VAULT_PROGRAM_ID).0,
//                 vault,
//                 &admin.pubkey(),
//                 is_paused,
//             )],
//             Some(&admin.pubkey()),
//             &[admin],
//             blockhash,
//         ))
//         .await
//     }

//     #[allow(dead_code)]
//     pub async fn set_confsig_admin(
//         &mut self,
//         config: &Pubkey,
//         old_admin: &Keypair,
//         new_admin: &Keypair,
//     ) -> Result<()> {
//         let blockhash = self.banks_client.get_latest_blockhash().await?;
//         self._process_transaction(&Transaction::new_signed_with_payer(
//             &[jito_vault_sdk::sdk::set_config_admin(
//                 &VAULT_PROGRAM_ID,
//                 config,
//                 &old_admin.pubkey(),
//                 &new_admin.pubkey(),
//             )],
//             Some(&old_admin.pubkey()),
//             &[old_admin],
//             blockhash,
//         ))
//         .await
//     }
// }
