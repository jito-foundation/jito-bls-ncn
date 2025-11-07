use anyhow::{anyhow, Result};
use log::error;
use serde_json::{json, Value};
use solana_account::Account;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcProgramAccountsConfig, RpcSendTransactionConfig},
    rpc_request::RpcRequest,
};
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_epoch_info::EpochInfo;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::{Hash, Transaction};

use crate::jito_clients::{JitoClient, JitoClientTrait, JitoClientType};

// --------------------------- JITO SurfPool Client -------------------------------
pub struct JitoSurfPoolClient {
    client_type: JitoClientType,
    rpc_client: RpcClient,
    keypair: Keypair,
}

impl Default for JitoSurfPoolClient {
    fn default() -> Self {
        JitoSurfPoolClient {
            client_type: JitoClientType::Surfpool,
            rpc_client: RpcClient::new("http://127.0.0.1:8899".to_string()),
            keypair: Keypair::new(),
        }
    }
}

impl JitoSurfPoolClient {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> JitoClient {
        JitoClient::SurfPool(JitoSurfPoolClient::default())
    }

    pub fn new_with_keypair(keypair: Keypair) -> JitoClient {
        JitoClient::SurfPool(JitoSurfPoolClient {
            client_type: JitoClientType::Surfpool,
            rpc_client: RpcClient::new("http://127.0.0.1:8899".to_string()),
            keypair,
        })
    }
}

impl JitoClientTrait for JitoSurfPoolClient {
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

    async fn get_program_accounts_with_config(
        &self,
        address: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(Pubkey, Account)>> {
        self.rpc_client
            .get_program_accounts_with_config(address, config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get program accounts: {}", e))
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
        let rpc_send_tx_config = RpcSendTransactionConfig {
            skip_preflight: true,
            preflight_commitment: None,
            max_retries: None,
            min_context_slot: None,
            encoding: None,
        };

        self.rpc_client
            .send_and_confirm_transaction_with_spinner_and_config(
                &transaction,
                commitment_config,
                rpc_send_tx_config,
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

    async fn test_warp_to_slot(&mut self, slot: u64) -> Result<()> {
        let params = json!([{
            "absoluteSlot": slot
        }]);

        // Make the RPC call
        let _: Value = self
            .rpc_client
            .send(
                RpcRequest::Custom {
                    method: "surfnet_timeTravel",
                },
                params,
            )
            .await
            .map_err(|e| anyhow!("Error warping to slot: {}", e))?;

        Ok(())
    }

    async fn test_warp_to_slot_incremental(&mut self, slots_to_increment: u64) -> Result<()> {
        let current_slot = self.get_epoch_info().await?.absolute_slot;
        let slot_to_warp_to = slots_to_increment.saturating_add(current_slot);
        self.test_warp_to_slot(slot_to_warp_to).await.map_err(|e| {
            anyhow!(
                "Warp to slot incremental {} to {} failed: {}",
                slots_to_increment,
                slot_to_warp_to,
                e
            )
        })
    }

    async fn test_set_account(&mut self, address: &Pubkey, account: &Account) -> Result<()> {
        let update: Value = {
            if account.data.is_empty() {
                json!({
                    "executable": account.executable,
                    "lamports": account.lamports,
                    "owner": account.owner.to_string(),
                })
            } else {
                json!({
                    "data": format!("0x{}", hex::encode(&account.data)),
                    "executable": account.executable,
                    "lamports": account.lamports,
                    "owner": account.owner.to_string(),
                })
            }
        };
        let params = json!([address.to_string(), update]);

        // Make the RPC call
        let _: Value = self
            .rpc_client
            .send(
                RpcRequest::Custom {
                    method: "surfnet_setAccount",
                },
                params,
            )
            .await
            .map_err(|e| anyhow!("Error setting account with surfnet_setAccount: {}", e))?;

        Ok(())
    }

    async fn test_airdrop(&mut self, address: &Pubkey, lamports: u64) -> Result<()> {
        let lamports = match self.get_account(address).await {
            Ok(account) => account.lamports.saturating_add(lamports),
            Err(_) => lamports,
        };

        let update = json!({
            "lamports": lamports,
        });
        let params = json!([address.to_string(), update]);

        // Make the RPC call
        let _: Value = self
            .rpc_client
            .send(
                RpcRequest::Custom {
                    method: "surfnet_setAccount",
                },
                params,
            )
            .await
            .map_err(|e| anyhow!("Error airdropping with surfnet_setAccount: {}", e))?;

        Ok(())
    }
}
