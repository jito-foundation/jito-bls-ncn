use jito_bls_ncn_core::{accounts::{bls_operator::BlsOperator, config::Config, consensus::Consensus, rolling_snapshot::RollingSnapshot}, utils::load_account};
use jito_bls_ncn_sdk::{bls_operator_address, config_address, consensus_address, realloc_rolling_snapshot_ix, rolling_snapshot_address, vote_ix};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use anyhow::Result;
use solana_signer::Signer;
use solana_transaction::Transaction;

use crate::jito_clients::JitoClient;

#[derive(Debug)]
pub struct BlsNcnRoot {
    pub ncn_pubkey: Pubkey,
    pub ncn_admin: Keypair,
}

impl Clone for BlsNcnRoot {
    fn clone(&self) -> Self {
        Self {
            ncn_pubkey: self.ncn_pubkey,
            ncn_admin: self.ncn_admin.insecure_clone(),
        }
    }
}

pub async fn get_bls_operator<T: JitoClient>(
    jito_client: &T,
    operator: &Pubkey,
) -> Result<BlsOperator> {
    let (address, _) = bls_operator_address(operator);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<BlsOperator>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_config<T: JitoClient>(
    jito_client: &T,
    ncn: &Pubkey,
) -> Result<Config> {
    let (address, _) = config_address(ncn);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<Config>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_consensus<T: JitoClient>(
    jito_client: &T,
    ncn: &Pubkey,
) -> Result<Consensus> {
    let (address, _) = consensus_address(ncn);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<Consensus>(&account_raw.data)? };
    Ok(*account)
}

pub async fn get_rolling_snapshot<T: JitoClient>(
    jito_client: &T,
    ncn: &Pubkey,
) -> Result<RollingSnapshot> {
    let (address, _) = rolling_snapshot_address(ncn);
    let account_raw = jito_client.get_account(&address).await?;
    let account = unsafe { load_account::<RollingSnapshot>(&account_raw.data)? };
    Ok(*account)
}

pub async fn test_realloc_rolling_snapshot<T: JitoClient>(
    jito_client: &mut T,
    ncn: &Pubkey,
) -> Result<()> {
    realloc_rolling_snapshot(jito_client, ncn).await?;
    Ok(())
}

pub async fn realloc_rolling_snapshot<T: JitoClient>(
    jito_client: &mut T,
    ncn: &Pubkey,
) -> Result<()> {
    let payer = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[realloc_rolling_snapshot_ix(ncn)],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn test_vote<T: JitoClient>(
    jito_client: &mut T,
    ncn: &Pubkey,
) -> Result<()> {
    vote(jito_client, ncn).await?;
    Ok(())
}

pub async fn vote<T: JitoClient>(
    jito_client: &mut T,
    ncn: &Pubkey,
) -> Result<()> {
    let payer = jito_client.keypair().insecure_clone();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[vote_ix(ncn)],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}
