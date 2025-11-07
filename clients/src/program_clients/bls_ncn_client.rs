use anyhow::{anyhow, Result};
use jito_bls_ncn_core::{
    accounts::{
        bls_operator::BlsOperator, config::Config, consensus::Consensus,
        rolling_snapshot::RollingSnapshot,
    },
    bls::solana_bls_interface::{SolanaBN254G1, SolanaBN254G2, SolanaBN254Keypair},
    utils::{get_realloc_calls, load_account, JitoAccount},
};
use jito_bls_ncn_sdk::bls_ncn_sdk::{
    bls_operator_address, config_address, consensus_address, initialize_bls_operator_ix,
    initialize_config_ix, initialize_consensus_ix, initialize_rolling_snapshot_ix,
    register_bls_operator_ix, register_vault_ix, rolling_snapshot_address, snapshot_ix, vote_ix,
};
use log::error;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

use crate::{
    jito_clients::{JitoClient, JitoClientTrait},
    program_clients::{
        meta_restaking_client::TestNcn, restaking_client::get_epoch_length, vault_client::get_vault,
    },
};

pub struct BlsNcnRoot {
    pub test_ncn: TestNcn,
    pub operator_bls_keypairs: Vec<SolanaBN254Keypair>,
}

pub struct BlsNcnSignatureRoot {
    pub operator_g2_signed: Vec<[u8; 128]>,
    pub operator_signatures: Vec<[u8; 64]>,
    pub indexs: Vec<usize>,
    pub message: Vec<u8>,
}

pub async fn get_bls_operator(jito_client: &JitoClient, operator: &Pubkey) -> Result<BlsOperator> {
    let (address, _, _) = bls_operator_address(operator);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<BlsOperator>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_config(jito_client: &JitoClient, ncn: &Pubkey) -> Result<Config> {
    let (address, _, _) = config_address(ncn);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<Config>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_consensus(jito_client: &JitoClient, ncn: &Pubkey) -> Result<Consensus> {
    let (address, _, _) = consensus_address(ncn);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<Consensus>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_rolling_snapshot(
    jito_client: &JitoClient,
    ncn: &Pubkey,
) -> Result<RollingSnapshot> {
    let (address, _, _) = rolling_snapshot_address(ncn);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<RollingSnapshot>(&account_raw.data)? };
    Ok(*account)
}

pub async fn initialize_config(jito_client: &JitoClient, ncn: &Pubkey) -> Result<()> {
    let payer = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_config_ix(ncn, &payer.pubkey(), &payer.pubkey())],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn initialize_consensus(jito_client: &JitoClient, ncn: &Pubkey) -> Result<()> {
    let payer = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_consensus_ix(ncn, &payer.pubkey())],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn initialize_rolling_snapshot(jito_client: &mut JitoClient, ncn: &Pubkey) -> Result<()> {
    let (pda, _, _) = rolling_snapshot_address(ncn);
    let current_size = match jito_client.get_account(&pda).await {
        Ok(account) => account.data.len(),
        Err(_) => 0,
    };

    let realloc_calls = get_realloc_calls(current_size, RollingSnapshot::LEN)?;
    for _ in 0..realloc_calls {
        let payer = jito_client.keypair().insecure_clone();
        let blockhash = jito_client.get_recent_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[initialize_rolling_snapshot_ix(ncn, &payer.pubkey())],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );

        jito_client.send_and_confirm_transaction(tx, None).await?;
        jito_client.test_warp_to_slot_incremental(30).await?;
    }

    Ok(())
}

pub async fn initialize_bls_operator(
    jito_client: &JitoClient,
    operator: &Pubkey,
    bls_keypair: &SolanaBN254Keypair,
) -> Result<()> {
    let payer = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[initialize_bls_operator_ix(
            operator,
            &payer.pubkey(),
            &payer.pubkey(),
            bls_keypair,
            None,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn register_bls_operator(
    jito_client: &JitoClient,
    ncn: &Pubkey,
    operator: &Pubkey,
) -> Result<()> {
    let admin = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[register_bls_operator_ix(ncn, operator, &admin.pubkey())],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn register_vault(
    jito_client: &JitoClient,
    ncn: &Pubkey,
    vault: &Pubkey,
    weight_bps: u16,
) -> Result<()> {
    let admin = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[register_vault_ix(ncn, vault, &admin.pubkey(), weight_bps)],
        Some(&admin.pubkey()),
        &[&admin], // Not a needed sig
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn snapshot(
    jito_client: &JitoClient,
    ncn: &Pubkey,
    operator: &Pubkey,
    vault: &Pubkey,
    operator_index: usize,
    vault_index: usize,
) -> Result<()> {
    let admin = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[snapshot_ix(
            ncn,
            operator,
            vault,
            operator_index,
            vault_index,
        )],
        Some(&admin.pubkey()),
        &[&admin], // Not a needed sig
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn full_snapshot(jito_client: &JitoClient, ncn: &Pubkey) -> Result<()> {
    let current_slot = jito_client.get_epoch_info().await?.absolute_slot;
    let epoch_length = get_epoch_length(jito_client).await?;
    let rolling_snapshot = get_rolling_snapshot(jito_client, ncn).await?;

    for (operator_index, operator_entry) in rolling_snapshot.operators.iter().enumerate() {
        if let Some(operator_entry) = operator_entry.as_ref() {
            for (vault_index, vault_entry) in rolling_snapshot.vaults.iter().enumerate() {
                if let Some(vault_entry) = vault_entry.as_ref() {
                    let vault_account = get_vault(jito_client, &vault_entry.vault).await?;
                    if vault_account.is_update_needed(current_slot, epoch_length)? {
                        error!("Vault update needed for vault: {}", vault_entry.vault);
                        return Err(anyhow!(
                            "Vault update needed for vault: {}",
                            vault_entry.vault
                        ));
                    }

                    let result = snapshot(
                        jito_client,
                        ncn,
                        &operator_entry.operator,
                        &vault_entry.vault,
                        operator_index,
                        vault_index,
                    )
                    .await;

                    if let Err(err) = result {
                        error!("Error snapshotting operator: {}", err);
                    }
                }
            }
        }
    }

    Ok(())
}

// aggregated_g1_signature: SolanaBN254G1,
// aggregated_g2_signed: SolanaBN254G2,
// operators_bitmap_signed: [u8; 32],
// message: [u8; 32],
pub async fn vote(
    jito_client: &mut JitoClient,
    ncn: &Pubkey,
    aggregated_g1_signature: &SolanaBN254G1,
    aggregated_g2_signed: &SolanaBN254G2,
    operators_bitmap_signed: &[u8; 32],
    raw_message: &[u8; 32],
    consensus_count: u64,
) -> Result<()> {
    let payer = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            vote_ix(
                ncn,
                aggregated_g1_signature,
                aggregated_g2_signed,
                operators_bitmap_signed,
                raw_message,
                consensus_count,
            ),
        ],
        Some(&payer.pubkey()),
        &[&payer], // Not a needed sig
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}
