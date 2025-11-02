use anyhow::{anyhow, Result};
use jito_bls_ncn_core::{
    programs::vault_core::{Config, Vault, VaultUpdateStateTracker},
    utils::{get_epoch, load_account},
};
use jito_bls_ncn_sdk::vault_sdk::{
    add_delegation_ix, close_vault_update_state_tracker_ix, config_address,
    crank_vault_update_state_tracker_ix, initialize_config_ix, initialize_vault_ix,
    initialize_vault_ncn_ticket_ix, initialize_vault_operator_delegation_ix,
    initialize_vault_update_state_tracker_ix, mint_to_ix, update_vault_balance_ix,
    vault_ncn_ticket_address, vault_operator_delegation_address,
    vault_update_state_tracker_address, warmup_vault_ncn_ticket_ix, WithdrawalAllocationMethod,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account_interface::address::get_associated_token_address;

use crate::{
    jito_clients::{JitoClient, JitoClientTrait},
    program_clients::solana_client::{create_ata, create_mint, mint_spl_to},
};

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

pub async fn get_config(jito_client: &JitoClient) -> Result<Config> {
    let (address, _, _) = config_address();
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<Config>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_epoch_length(jito_client: &JitoClient) -> Result<u64> {
    let config = get_config(jito_client).await?;
    Ok(config.epoch_length.get())
}

pub async fn get_vault(jito_client: &JitoClient, vault: &Pubkey) -> Result<Vault> {
    let account_raw = jito_client.get_account(vault).await?;
    let account = unsafe { load_account::<Vault>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_vault_update_state_tracker(
    jito_client: &JitoClient,
    vault: &Pubkey,
) -> Result<VaultUpdateStateTracker> {
    let account_raw = jito_client.get_account(vault).await?;
    let account = unsafe { load_account::<VaultUpdateStateTracker>(&account_raw.data)? };
    Ok(*account)
}

pub async fn test_configure_depositor(
    jito_client: &mut JitoClient,
    vault_root: &VaultRoot,
    depositor: &Pubkey,
    amount_to_mint: u64,
) -> Result<()> {
    jito_client.test_airdrop(depositor, 1_000_000_000).await?;
    let vault = get_vault(jito_client, &vault_root.vault_pubkey).await?;
    create_ata(jito_client, depositor, &vault.supported_mint, None).await?;
    create_ata(jito_client, depositor, &vault.vrt_mint, None).await?;
    mint_spl_to(
        jito_client,
        &vault.supported_mint,
        depositor,
        amount_to_mint,
        None,
    )
    .await?;

    Ok(())
}

pub async fn get_vault_is_update_needed(
    jito_client: &mut JitoClient,
    vault: &Pubkey,
    slot: u64,
) -> Result<bool> {
    let config = get_config(jito_client).await?;
    let vault = get_vault(jito_client, vault).await?;

    vault
        .is_update_needed(slot, config.epoch_length.into())
        .map_err(|e| anyhow!("Could not get is update needed: {}", e))
}

pub async fn test_initialize_config(jito_client: &mut JitoClient) -> Result<()> {
    let admin = jito_client.keypair().insecure_clone();
    let restaking_program = jito_bls_ncn_sdk::restaking_sdk::id();
    let (config, _, _) = config_address();
    initialize_config(
        jito_client,
        &config,
        &admin,
        &restaking_program,
        &admin.pubkey(),
        100,
    )
    .await?;

    Ok(())
}

pub async fn initialize_config(
    jito_client: &mut JitoClient,
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
            program_fee_bps,
        )],
        Some(&config_admin.pubkey()),
        &[config_admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_initialize_vault(jito_client: &mut JitoClient) -> Result<VaultRoot> {
    let admin = jito_client.keypair().insecure_clone();
    let base = Keypair::new();
    let vrt_mint = Keypair::new();
    let st_mint = Keypair::new();

    let initialize_token_amount = 1_000_000;
    let fee_bps = 100;
    let decimals = 9;

    let (vault, _, _) = jito_bls_ncn_sdk::vault_sdk::vault_address(&base.pubkey());
    let (burn_vault, _, _) = jito_bls_ncn_sdk::vault_sdk::burn_vault_address(&base.pubkey());
    let (config, _, _) = jito_bls_ncn_sdk::vault_sdk::config_address();

    // Airdrop to vault admin
    jito_client
        .test_airdrop(&admin.pubkey(), 1_000_000_000)
        .await?;

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
    )
    .await?;

    // Create necessary ATAs
    create_ata(jito_client, &admin.pubkey(), &vrt_mint.pubkey(), None).await?;

    Ok(VaultRoot {
        vault_pubkey: vault,
        vault_admin: admin,
        mint: st_mint,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn initialize_vault(
    jito_client: &mut JitoClient,
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

    let admin_st_token_account =
        get_associated_token_address(&vault_admin.pubkey(), &st_mint.pubkey());
    let vault_st_token_account = get_associated_token_address(vault, &st_mint.pubkey());

    let burn_vault_vrt_account = get_associated_token_address(burn_vault, &vrt_mint.pubkey());

    // Create ATAs first
    create_ata(jito_client, vault, &st_mint.pubkey(), None).await?;
    create_ata(jito_client, &vault_admin.pubkey(), &st_mint.pubkey(), None).await?;

    // Mint initial tokens to admin
    mint_spl_to(
        jito_client,
        &st_mint.pubkey(),
        &vault_admin.pubkey(),
        initialize_token_amount,
        None,
    )
    .await?;

    let tx = Transaction::new_signed_with_payer(
        &[initialize_vault_ix(
            config,
            vault,
            &vrt_mint.pubkey(),
            &st_mint.pubkey(),
            &admin_st_token_account,
            &vault_st_token_account,
            burn_vault,
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

pub async fn test_initialize_vault_ncn_ticket(
    jito_client: &mut JitoClient,
    vault_root: &VaultRoot,
    ncn: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (vault_ncn_ticket, _, _) = vault_ncn_ticket_address(&vault_root.vault_pubkey, ncn);
    let (ncn_vault_ticket, _, _) =
        jito_bls_ncn_sdk::restaking_sdk::ncn_vault_ticket_address(ncn, &vault_root.vault_pubkey);

    initialize_vault_ncn_ticket(
        jito_client,
        &config,
        &vault_root.vault_pubkey,
        ncn,
        &ncn_vault_ticket,
        &vault_ncn_ticket,
        &vault_root.vault_admin,
    )
    .await?;

    Ok(())
}

pub async fn initialize_vault_ncn_ticket(
    jito_client: &mut JitoClient,
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

pub async fn test_warmup_vault_ncn_ticket(
    jito_client: &mut JitoClient,
    vault_root: &VaultRoot,
    ncn: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (vault_ncn_ticket, _, _) = vault_ncn_ticket_address(&vault_root.vault_pubkey, ncn);

    warmup_vault_ncn_ticket(
        jito_client,
        &config,
        &vault_root.vault_pubkey,
        ncn,
        &vault_ncn_ticket,
        &vault_root.vault_admin,
    )
    .await?;

    Ok(())
}

pub async fn warmup_vault_ncn_ticket(
    jito_client: &mut JitoClient,
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

pub async fn test_initialize_vault_operator_delegation(
    jito_client: &mut JitoClient,
    vault_root: &VaultRoot,
    operator: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (vault_operator_delegation, _, _) =
        vault_operator_delegation_address(&vault_root.vault_pubkey, operator);
    let (operator_vault_ticket, _, _) =
        jito_bls_ncn_sdk::restaking_sdk::operator_vault_ticket_address(
            operator,
            &vault_root.vault_pubkey,
        );

    initialize_vault_operator_delegation(
        jito_client,
        &config,
        &vault_root.vault_pubkey,
        operator,
        &operator_vault_ticket,
        &vault_operator_delegation,
        &vault_root.vault_admin,
    )
    .await?;

    Ok(())
}

pub async fn initialize_vault_operator_delegation(
    jito_client: &mut JitoClient,
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

pub async fn test_add_delegation(
    jito_client: &mut JitoClient,
    vault_root: &VaultRoot,
    operator: &Pubkey,
    amount: u64,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (vault_operator_delegation, _, _) =
        vault_operator_delegation_address(&vault_root.vault_pubkey, operator);

    if amount == 0 {
        return Ok(());
    }

    add_delegation(
        jito_client,
        &config,
        &vault_root.vault_pubkey,
        operator,
        &vault_operator_delegation,
        &vault_root.vault_admin,
        amount,
    )
    .await?;

    Ok(())
}

pub async fn add_delegation(
    jito_client: &mut JitoClient,
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

pub async fn test_mint_to(
    jito_client: &mut JitoClient,
    vault_root: &VaultRoot,
    depositor: &Keypair,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<()> {
    let vault = get_vault(jito_client, &vault_root.vault_pubkey).await?;

    let depositor_token_account =
        get_associated_token_address(&depositor.pubkey(), &vault.supported_mint);
    let vault_token_account =
        get_associated_token_address(&vault_root.vault_pubkey, &vault.supported_mint);
    let depositor_vrt_token_account =
        get_associated_token_address(&depositor.pubkey(), &vault.vrt_mint);
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
    )
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn mint_to(
    jito_client: &mut JitoClient,
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
    let (config, _, _) = config_address();
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

pub async fn full_vault_update(
    jito_client: &mut JitoClient,
    vault: &Pubkey,
    operators: &[Pubkey],
) -> Result<()> {
    let current_slot = jito_client.get_epoch_info().await?.absolute_slot;
    let epoch_length: u64 = get_epoch_length(jito_client).await?;
    let ncn_epoch: u64 = get_epoch(current_slot, epoch_length)?;

    let is_update_needed = get_vault_is_update_needed(jito_client, vault, current_slot).await?;
    if !is_update_needed {
        return Ok(());
    }

    let vault_update_state_tracker = vault_update_state_tracker_address(vault, ncn_epoch).0;

    let vault_update_state_tracker_account_dne = get_vault_update_state_tracker(jito_client, vault)
        .await
        .is_err();
    if vault_update_state_tracker_account_dne {
        initialize_vault_update_state_tracker(jito_client, vault, &vault_update_state_tracker)
            .await?;
    }

    for i in 0..operators.len() {
        let operator_index = (i + ncn_epoch as usize) % operators.len();
        let operator = &operators[operator_index];
        let (vault_operator_delegation, _, _) = vault_operator_delegation_address(vault, operator);

        crank_vault_update_state_tracker(
            jito_client,
            vault,
            operator,
            &vault_operator_delegation,
            &vault_update_state_tracker,
        )
        .await?;
    }

    close_vault_update_state_tracker(jito_client, vault, &vault_update_state_tracker, ncn_epoch)
        .await?;

    update_vault_balance(jito_client, vault).await?;

    Ok(())
}

pub async fn do_crank_vault_update_state_tracker(
    jito_client: &mut JitoClient,
    vault: &Pubkey,
    operator: &Pubkey,
) -> Result<()> {
    let slot = jito_client.get_epoch_info().await?.absolute_slot;
    let config = get_config(jito_client).await?;
    let epoch_length: u64 = config.epoch_length.into();
    let ncn_epoch = slot / epoch_length;

    let (vault_operator_delegation, _, _) = vault_operator_delegation_address(vault, operator);
    let (vault_update_state_tracker, _, _) = vault_update_state_tracker_address(vault, ncn_epoch);

    crank_vault_update_state_tracker(
        jito_client,
        vault,
        operator,
        &vault_operator_delegation,
        &vault_update_state_tracker,
    )
    .await
}

pub async fn crank_vault_update_state_tracker(
    jito_client: &mut JitoClient,
    vault: &Pubkey,
    operator: &Pubkey,
    vault_operator_delegation: &Pubkey,
    vault_update_state_tracker: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
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

pub async fn update_vault_balance(
    jito_client: &mut JitoClient,
    vault_pubkey: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
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

pub async fn initialize_vault_update_state_tracker(
    jito_client: &mut JitoClient,
    vault_pubkey: &Pubkey,
    vault_update_state_tracker: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
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

pub async fn close_vault_update_state_tracker(
    jito_client: &mut JitoClient,
    vault_pubkey: &Pubkey,
    vault_update_state_tracker: &Pubkey,
    ncn_epoch: u64,
) -> Result<()> {
    let (config, _, _) = config_address();
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
