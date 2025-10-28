use anyhow::{anyhow, Result};
use log::error;
use solana_account::Account;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_epoch_info::EpochInfo;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::{Hash, Transaction};

use crate::jito_clients::{JitoClientTrait, JitoClientType};

// --------------------------- JITO RPC Client -------------------------------
pub struct JitoRpcClient {
    client_type: JitoClientType,
    rpc_client: RpcClient,
    keypair: Keypair,
}

impl JitoRpcClient {
    pub fn new(rpc_url: String) -> Self {
        JitoRpcClient {
            client_type: JitoClientType::RPC,
            rpc_client: RpcClient::new(rpc_url),
            keypair: Keypair::new(),
        }
    }

    pub fn new_with_keypair(rpc_url: String, keypair: Keypair) -> Self {
        JitoRpcClient {
            client_type: JitoClientType::RPC,
            rpc_client: RpcClient::new(rpc_url),
            keypair,
        }
    }
}

impl JitoClientTrait for JitoRpcClient {
    fn get_client_type(&self) -> JitoClientType {
        self.client_type.clone()
    }

    fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    async fn get_account(&self, address: &Pubkey) -> Result<Account> {
        self.rpc_client
            .get_account(address)
            .await
            .map_err(|e| anyhow!("Could not get account for {}: {}", address, e))
    }

    async fn get_balance(&self, address: &Pubkey) -> Result<u64> {
        self.rpc_client
            .get_balance(address)
            .await
            .map_err(|e| anyhow!("Could not get balance for {}: {}", address, e))
    }

    async fn get_recent_blockhash(&self) -> Result<Hash> {
        let blockhash = self.rpc_client.get_latest_blockhash().await?;
        self.rpc_client
            .get_new_latest_blockhash(&blockhash)
            .await
            .map_err(|e| anyhow!("Could not get latest blockhash {}", e))
    }

    async fn send_and_confirm_transaction(
        &self,
        transaction: Transaction,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Option<Signature>> {
        let commitment_config = CommitmentConfig {
            commitment: commitment.unwrap_or(CommitmentLevel::Confirmed),
        };

        self.rpc_client
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &transaction,
                commitment_config,
            )
            .await
            .map(Some)
            .map_err(|e| {
                error!("Error sending transaction: {}", e);
                anyhow!("Error sending transaction: {}", e)
            })
    }

    async fn get_epoch_info(&self) -> Result<EpochInfo> {
        self.rpc_client
            .get_epoch_info()
            .await
            .map_err(|e| anyhow!("Error getting epoch info: {}", e))
    }

    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64> {
        self.rpc_client
            .get_minimum_balance_for_rent_exemption(data_len)
            .await
            .map_err(|e| anyhow!("Error getting minimum balance for rent exemption: {}", e))
    }

    async fn test_warp_to_slot(&mut self, _: u64) -> Result<()> {
        error!("Warp to slot not supported on RPC client");
        Err(anyhow!("Warp to slot not supported"))
    }

    async fn test_warp_to_slot_incremental(&mut self, _: u64) -> Result<()> {
        error!("Warp to slot incremental not supported on RPC client");
        Err(anyhow!("Warp to slot incremental not supported"))
    }

    async fn test_set_account(&mut self, _: &Pubkey, _: &Account) -> Result<()> {
        error!("Set account not supported on RPC client");
        Err(anyhow!("Time travel not supported"))
    }

    async fn test_airdrop(&mut self, address: &Pubkey, lamports: u64) -> Result<()> {
        self.rpc_client
            .request_airdrop(address, lamports)
            .await
            .map_err(|e| anyhow!("Error airdropping: {}", e))?;
        Ok(())
    }
}
