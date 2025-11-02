use anyhow::Result;
use jito_bls_ncn_core::{
    programs::restaking_core::{
        Config, Ncn, NcnOperatorState, NcnVaultTicket, Operator, OperatorVaultTicket,
    },
    utils::load_account,
};
use jito_bls_ncn_sdk::restaking_sdk::{
    config_address, initialize_config_ix, initialize_ncn_ix, initialize_ncn_operator_state_ix,
    initialize_ncn_vault_ticket_ix, initialize_operator_ix, initialize_operator_vault_ticket_ix,
    ncn_address, ncn_cooldown_operator_ix, ncn_operator_state_address, ncn_set_admin_ix,
    ncn_vault_ticket_address, ncn_warmup_operator_ix, operator_address, operator_cooldown_ncn_ix,
    operator_set_fee_ix, operator_vault_ticket_address, operator_warmup_ncn_ix,
    warmup_ncn_vault_ticket_ix, warmup_operator_vault_ticket_ix,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

use crate::jito_clients::{JitoClient, JitoClientTrait};

#[derive(Debug)]
pub struct NcnRoot {
    pub ncn_pubkey: Pubkey,
    pub ncn_admin: Keypair,
}

impl Clone for NcnRoot {
    fn clone(&self) -> Self {
        Self {
            ncn_pubkey: self.ncn_pubkey,
            ncn_admin: self.ncn_admin.insecure_clone(),
        }
    }
}

#[derive(Debug)]
pub struct OperatorRoot {
    pub operator_pubkey: Pubkey,
    pub operator_admin: Keypair,
}

impl Clone for OperatorRoot {
    fn clone(&self) -> Self {
        Self {
            operator_pubkey: self.operator_pubkey,
            operator_admin: self.operator_admin.insecure_clone(),
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

pub async fn get_ncn(jito_client: &JitoClient, ncn: &Pubkey) -> Result<Ncn> {
    let account_raw = jito_client.get_account(ncn).await?;
    let account = unsafe { load_account::<Ncn>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_operator(jito_client: &JitoClient, operator: &Pubkey) -> Result<Operator> {
    let account_raw = jito_client.get_account(operator).await?;
    let account = unsafe { load_account::<Operator>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_ncn_vault_ticket(
    jito_client: &JitoClient,
    ncn: &Pubkey,
    vault: &Pubkey,
) -> Result<NcnVaultTicket> {
    let (address, _, _) = ncn_vault_ticket_address(ncn, vault);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<NcnVaultTicket>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_ncn_operator_state(
    jito_client: &JitoClient,
    ncn: &Pubkey,
    operator: &Pubkey,
) -> Result<NcnOperatorState> {
    let (address, _, _) = ncn_operator_state_address(ncn, operator);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<NcnOperatorState>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_operator_vault_ticket(
    jito_client: &JitoClient,
    operator: &Pubkey,
    vault: &Pubkey,
) -> Result<OperatorVaultTicket> {
    let (address, _, _) = operator_vault_ticket_address(operator, vault);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<OperatorVaultTicket>(&account_raw.data)? };
    Ok(*account)
}

pub async fn test_initialize_config(jito_client: &mut JitoClient) -> Result<Keypair> {
    let restaking_config_admin = Keypair::new();
    let (config, _, _) = config_address();

    jito_client
        .test_airdrop(&restaking_config_admin.pubkey(), 1_000_000_000)
        .await?;
    initialize_config(jito_client, &config, &restaking_config_admin).await?;

    Ok(restaking_config_admin)
}

pub async fn initialize_config(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    config_admin: &Keypair,
) -> Result<()> {
    let vault_program = jito_bls_ncn_sdk::vault_sdk::id();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_config_ix(
            config,
            &config_admin.pubkey(),
            &vault_program,
        )],
        Some(&config_admin.pubkey()),
        &[config_admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_initialize_operator(jito_client: &mut JitoClient) -> Result<OperatorRoot> {
    let operator_base = Keypair::new();
    let operator_admin = jito_client.keypair().insecure_clone();
    let (operator_pubkey, _, _) = operator_address(&operator_base.pubkey());
    let (config, _, _) = config_address();

    jito_client
        .test_airdrop(&operator_admin.pubkey(), 1_000_000_000)
        .await?;

    let fee_bps = 100;

    initialize_operator(
        jito_client,
        &config,
        &operator_pubkey,
        &operator_admin,
        &operator_base,
        fee_bps,
    )
    .await?;

    Ok(OperatorRoot {
        operator_pubkey,
        operator_admin,
    })
}

pub async fn initialize_operator(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    operator: &Pubkey,
    admin: &Keypair,
    base: &Keypair,
    operator_fee_bps: u16,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_operator_ix(
            config,
            operator,
            &admin.pubkey(),
            &base.pubkey(),
            operator_fee_bps,
        )],
        Some(&admin.pubkey()),
        &[admin, base],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_initialize_ncn(jito_client: &mut JitoClient) -> Result<NcnRoot> {
    let ncn_base = Keypair::new();
    let ncn_admin = jito_client.keypair().insecure_clone();
    let (ncn_pubkey, _, _) = ncn_address(&ncn_base.pubkey());
    let (config, _, _) = config_address();

    jito_client
        .test_airdrop(&ncn_admin.pubkey(), 1_000_000_000)
        .await?;

    initialize_ncn(jito_client, &config, &ncn_pubkey, &ncn_admin, &ncn_base).await?;

    Ok(NcnRoot {
        ncn_pubkey,
        ncn_admin,
    })
}

pub async fn initialize_ncn(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    ncn: &Pubkey,
    admin: &Keypair,
    base: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_ncn_ix(
            config,
            ncn,
            &admin.pubkey(),
            &base.pubkey(),
        )],
        Some(&admin.pubkey()),
        &[admin, base],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_initialize_ncn_vault_ticket(
    jito_client: &mut JitoClient,
    ncn_root: &NcnRoot,
    vault: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (ncn_vault_ticket, _, _) = ncn_vault_ticket_address(&ncn_root.ncn_pubkey, vault);

    initialize_ncn_vault_ticket(
        jito_client,
        &config,
        &ncn_root.ncn_pubkey,
        vault,
        &ncn_vault_ticket,
        &ncn_root.ncn_admin,
    )
    .await?;

    Ok(())
}

pub async fn initialize_ncn_vault_ticket(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    ncn: &Pubkey,
    vault: &Pubkey,
    ncn_vault_ticket: &Pubkey,
    admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_ncn_vault_ticket_ix(
            config,
            ncn,
            vault,
            ncn_vault_ticket,
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

pub async fn test_warmup_ncn_vault_ticket(
    jito_client: &mut JitoClient,
    ncn_root: &NcnRoot,
    vault: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (ncn_vault_ticket, _, _) = ncn_vault_ticket_address(&ncn_root.ncn_pubkey, vault);

    warmup_ncn_vault_ticket(
        jito_client,
        &config,
        &ncn_root.ncn_pubkey,
        vault,
        &ncn_vault_ticket,
        &ncn_root.ncn_admin,
    )
    .await?;

    Ok(())
}

pub async fn warmup_ncn_vault_ticket(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    ncn: &Pubkey,
    vault: &Pubkey,
    ncn_vault_ticket: &Pubkey,
    admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[warmup_ncn_vault_ticket_ix(
            config,
            ncn,
            vault,
            ncn_vault_ticket,
            &admin.pubkey(),
        )],
        Some(&admin.pubkey()),
        &[admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_initialize_ncn_operator_state(
    jito_client: &mut JitoClient,
    ncn_root: &NcnRoot,
    operator_root: &OperatorRoot,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (ncn_operator_state, _, _) =
        ncn_operator_state_address(&ncn_root.ncn_pubkey, &operator_root.operator_pubkey);

    initialize_ncn_operator_state(
        jito_client,
        &config,
        &ncn_root.ncn_pubkey,
        &operator_root.operator_pubkey,
        &ncn_operator_state,
        &ncn_root.ncn_admin,
    )
    .await?;

    Ok(())
}

pub async fn initialize_ncn_operator_state(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_ncn_operator_state_ix(
            config,
            ncn,
            operator,
            ncn_operator_state,
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

pub async fn test_ncn_warmup_operator(
    jito_client: &mut JitoClient,
    ncn_root: &NcnRoot,
    operator: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (ncn_operator_state, _, _) = ncn_operator_state_address(&ncn_root.ncn_pubkey, operator);

    ncn_warmup_operator(
        jito_client,
        &config,
        &ncn_root.ncn_pubkey,
        operator,
        &ncn_operator_state,
        &ncn_root.ncn_admin,
    )
    .await?;

    Ok(())
}

pub async fn ncn_warmup_operator(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ncn_warmup_operator_ix(
            config,
            ncn,
            operator,
            ncn_operator_state,
            &admin.pubkey(),
        )],
        Some(&admin.pubkey()),
        &[admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_ncn_cooldown_operator(
    jito_client: &mut JitoClient,
    ncn_root: &NcnRoot,
    operator: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (ncn_operator_state, _, _) = ncn_operator_state_address(&ncn_root.ncn_pubkey, operator);

    ncn_cooldown_operator(
        jito_client,
        &config,
        &ncn_root.ncn_pubkey,
        operator,
        &ncn_operator_state,
        &ncn_root.ncn_admin,
    )
    .await?;

    Ok(())
}

pub async fn ncn_cooldown_operator(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ncn_cooldown_operator_ix(
            config,
            ncn,
            operator,
            ncn_operator_state,
            &admin.pubkey(),
        )],
        Some(&admin.pubkey()),
        &[admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_ncn_set_admin(
    jito_client: &mut JitoClient,
    ncn: &Pubkey,
    old_admin: &Keypair,
    new_admin: &Keypair,
) -> Result<()> {
    ncn_set_admin(jito_client, ncn, old_admin, new_admin).await?;

    Ok(())
}

pub async fn ncn_set_admin(
    jito_client: &mut JitoClient,
    ncn: &Pubkey,
    old_admin: &Keypair,
    new_admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ncn_set_admin_ix(
            ncn,
            &old_admin.pubkey(),
            &new_admin.pubkey(),
        )],
        Some(&old_admin.pubkey()),
        &[old_admin, new_admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_initialize_operator_vault_ticket(
    jito_client: &mut JitoClient,
    operator_root: &OperatorRoot,
    vault: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (operator_vault_ticket, _, _) =
        operator_vault_ticket_address(&operator_root.operator_pubkey, vault);

    initialize_operator_vault_ticket(
        jito_client,
        &config,
        &operator_root.operator_pubkey,
        vault,
        &operator_vault_ticket,
        &operator_root.operator_admin,
        &operator_root.operator_admin,
    )
    .await?;

    Ok(())
}

pub async fn initialize_operator_vault_ticket(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    operator: &Pubkey,
    vault: &Pubkey,
    operator_vault_ticket: &Pubkey,
    admin: &Keypair,
    payer: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_operator_vault_ticket_ix(
            config,
            operator,
            vault,
            operator_vault_ticket,
            &admin.pubkey(),
            &payer.pubkey(),
        )],
        Some(&payer.pubkey()),
        &[admin, payer],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_warmup_operator_vault_ticket(
    jito_client: &mut JitoClient,
    operator_root: &OperatorRoot,
    vault: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (operator_vault_ticket, _, _) =
        operator_vault_ticket_address(&operator_root.operator_pubkey, vault);

    warmup_operator_vault_ticket(
        jito_client,
        &config,
        &operator_root.operator_pubkey,
        vault,
        &operator_vault_ticket,
        &operator_root.operator_admin,
    )
    .await?;

    Ok(())
}

pub async fn warmup_operator_vault_ticket(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    operator: &Pubkey,
    vault: &Pubkey,
    operator_vault_ticket: &Pubkey,
    admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[warmup_operator_vault_ticket_ix(
            config,
            operator,
            vault,
            operator_vault_ticket,
            &admin.pubkey(),
        )],
        Some(&admin.pubkey()),
        &[admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_operator_warmup_ncn(
    jito_client: &mut JitoClient,
    operator_root: &OperatorRoot,
    ncn: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (ncn_operator_state, _, _) =
        ncn_operator_state_address(ncn, &operator_root.operator_pubkey);

    operator_warmup_ncn(
        jito_client,
        &config,
        ncn,
        &operator_root.operator_pubkey,
        &ncn_operator_state,
        &operator_root.operator_admin,
    )
    .await?;

    Ok(())
}

pub async fn operator_warmup_ncn(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[operator_warmup_ncn_ix(
            config,
            ncn,
            operator,
            ncn_operator_state,
            &admin.pubkey(),
        )],
        Some(&admin.pubkey()),
        &[admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_operator_cooldown_ncn(
    jito_client: &mut JitoClient,
    operator_root: &OperatorRoot,
    ncn: &Pubkey,
) -> Result<()> {
    let (config, _, _) = config_address();
    let (ncn_operator_state, _, _) =
        ncn_operator_state_address(ncn, &operator_root.operator_pubkey);

    operator_cooldown_ncn(
        jito_client,
        &config,
        ncn,
        &operator_root.operator_pubkey,
        &ncn_operator_state,
        &operator_root.operator_admin,
    )
    .await?;

    Ok(())
}

pub async fn operator_cooldown_ncn(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Keypair,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[operator_cooldown_ncn_ix(
            config,
            ncn,
            operator,
            ncn_operator_state,
            &admin.pubkey(),
        )],
        Some(&admin.pubkey()),
        &[admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_operator_set_fee(
    jito_client: &mut JitoClient,
    operator_root: &OperatorRoot,
    new_fee_bps: u16,
) -> Result<()> {
    let (config, _, _) = config_address();

    operator_set_fee(
        jito_client,
        &config,
        &operator_root.operator_pubkey,
        &operator_root.operator_admin,
        new_fee_bps,
    )
    .await?;

    Ok(())
}

pub async fn operator_set_fee(
    jito_client: &mut JitoClient,
    config: &Pubkey,
    operator: &Pubkey,
    admin: &Keypair,
    new_fee_bps: u16,
) -> Result<()> {
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[operator_set_fee_ix(
            config,
            operator,
            &admin.pubkey(),
            new_fee_bps,
        )],
        Some(&admin.pubkey()),
        &[admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}
