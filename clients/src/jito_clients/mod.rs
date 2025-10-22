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

pub trait JitoClient {
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
