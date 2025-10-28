use anyhow::Result;
use solana_account::Account;
use solana_commitment_config::CommitmentLevel;
use solana_epoch_info::EpochInfo;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::{Hash, Transaction};

pub mod rpc;
pub mod solana_test_program;
pub mod surf_pool;

#[derive(Clone, Debug)]
#[repr(u8)]
pub enum JitoClientType {
    RPC = 0x00,
    Surfpool = 0x01,
    SolanaTestProgram = 0x02,
}

#[allow(async_fn_in_trait)]
pub trait JitoClientTrait {
    fn get_client_type(&self) -> JitoClientType;
    fn keypair(&self) -> &Keypair;

    async fn get_account(&self, address: &Pubkey) -> Result<Account>;
    async fn get_balance(&self, address: &Pubkey) -> Result<u64>;
    async fn get_recent_blockhash(&self) -> Result<Hash>;
    async fn send_and_confirm_transaction(
        &self,
        transaction: Transaction,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Option<Signature>>;
    async fn get_epoch_info(&self) -> Result<EpochInfo>;
    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64>;

    async fn test_warp_to_slot(&mut self, slot: u64) -> Result<()>;
    async fn test_warp_to_slot_incremental(&mut self, slots_to_increment: u64) -> Result<()>;
    async fn test_set_account(&mut self, address: &Pubkey, account: &Account) -> Result<()>;
    async fn test_airdrop(&mut self, address: &Pubkey, lamports: u64) -> Result<()>;
}

#[allow(clippy::large_enum_variant)]
pub enum JitoClient {
    SurfPool(surf_pool::JitoSurfPoolClient),
    Rpc(rpc::JitoRpcClient),
    SolanaTestProgram(solana_test_program::JitoSolanaTestProgramClient),
}

impl JitoClientTrait for JitoClient {
    fn get_client_type(&self) -> JitoClientType {
        match self {
            JitoClient::SurfPool(c) => c.get_client_type(),
            JitoClient::Rpc(c) => c.get_client_type(),
            JitoClient::SolanaTestProgram(c) => c.get_client_type(),
        }
    }

    fn keypair(&self) -> &Keypair {
        match self {
            JitoClient::SurfPool(c) => c.keypair(),
            JitoClient::Rpc(c) => c.keypair(),
            JitoClient::SolanaTestProgram(c) => c.keypair(),
        }
    }

    async fn get_account(&self, address: &Pubkey) -> Result<Account> {
        match self {
            JitoClient::SurfPool(c) => c.get_account(address).await,
            JitoClient::Rpc(c) => c.get_account(address).await,
            JitoClient::SolanaTestProgram(c) => c.get_account(address).await,
        }
    }

    async fn get_balance(&self, address: &Pubkey) -> Result<u64> {
        match self {
            JitoClient::SurfPool(c) => c.get_balance(address).await,
            JitoClient::Rpc(c) => c.get_balance(address).await,
            JitoClient::SolanaTestProgram(c) => c.get_balance(address).await,
        }
    }

    async fn get_recent_blockhash(&self) -> Result<Hash> {
        match self {
            JitoClient::SurfPool(c) => c.get_recent_blockhash().await,
            JitoClient::Rpc(c) => c.get_recent_blockhash().await,
            JitoClient::SolanaTestProgram(c) => c.get_recent_blockhash().await,
        }
    }

    async fn send_and_confirm_transaction(
        &self,
        transaction: Transaction,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Option<Signature>> {
        match self {
            JitoClient::SurfPool(c) => {
                c.send_and_confirm_transaction(transaction, commitment)
                    .await
            }
            JitoClient::Rpc(c) => {
                c.send_and_confirm_transaction(transaction, commitment)
                    .await
            }
            JitoClient::SolanaTestProgram(c) => {
                c.send_and_confirm_transaction(transaction, commitment)
                    .await
            }
        }
    }

    async fn get_epoch_info(&self) -> Result<EpochInfo> {
        match self {
            JitoClient::SurfPool(c) => c.get_epoch_info().await,
            JitoClient::Rpc(c) => c.get_epoch_info().await,
            JitoClient::SolanaTestProgram(c) => c.get_epoch_info().await,
        }
    }

    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64> {
        match self {
            JitoClient::SurfPool(c) => c.get_minimum_balance_for_rent_exemption(data_len).await,
            JitoClient::Rpc(c) => c.get_minimum_balance_for_rent_exemption(data_len).await,
            JitoClient::SolanaTestProgram(c) => {
                c.get_minimum_balance_for_rent_exemption(data_len).await
            }
        }
    }

    async fn test_warp_to_slot(&mut self, slot: u64) -> Result<()> {
        match self {
            JitoClient::SurfPool(c) => c.test_warp_to_slot(slot).await,
            JitoClient::Rpc(c) => c.test_warp_to_slot(slot).await,
            JitoClient::SolanaTestProgram(c) => c.test_warp_to_slot(slot).await,
        }
    }

    async fn test_warp_to_slot_incremental(&mut self, slots_to_increment: u64) -> Result<()> {
        match self {
            JitoClient::SurfPool(c) => c.test_warp_to_slot_incremental(slots_to_increment).await,
            JitoClient::Rpc(c) => c.test_warp_to_slot_incremental(slots_to_increment).await,
            JitoClient::SolanaTestProgram(c) => {
                c.test_warp_to_slot_incremental(slots_to_increment).await
            }
        }
    }

    async fn test_set_account(&mut self, address: &Pubkey, account: &Account) -> Result<()> {
        match self {
            JitoClient::SurfPool(c) => c.test_set_account(address, account).await,
            JitoClient::Rpc(c) => c.test_set_account(address, account).await,
            JitoClient::SolanaTestProgram(c) => c.test_set_account(address, account).await,
        }
    }

    async fn test_airdrop(&mut self, address: &Pubkey, lamports: u64) -> Result<()> {
        match self {
            JitoClient::SurfPool(c) => c.test_airdrop(address, lamports).await,
            JitoClient::Rpc(c) => c.test_airdrop(address, lamports).await,
            JitoClient::SolanaTestProgram(c) => c.test_airdrop(address, lamports).await,
        }
    }
}

// Updated helper function
pub fn surfpool_or_rpc(client_type: &str) -> JitoClient {
    match client_type {
        "surfpool" => JitoClient::SurfPool(surf_pool::JitoSurfPoolClient::new()),
        rpc_url => JitoClient::Rpc(rpc::JitoRpcClient::new(rpc_url.to_string())),
    }
}
