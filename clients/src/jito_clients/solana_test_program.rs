use anyhow::{anyhow, Result};
use solana_account::{Account, AccountSharedData, ReadableAccount};
use solana_commitment_config::CommitmentLevel;
use solana_epoch_info::EpochInfo;
use solana_keypair::Keypair;
use solana_program::clock::Clock;
use solana_program_test::ProgramTestContext;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_system_transaction::transfer;
use solana_transaction::{Hash, Transaction};

use crate::jito_clients::{JitoClientTrait, JitoClientType};

// --------------------------- JITO TEST PROGRAM Client -------------------------------
pub struct JitoSolanaTestProgramClient {
    client_type: JitoClientType,
    context: ProgramTestContext,
}

impl JitoSolanaTestProgramClient {
    pub fn new(context: ProgramTestContext) -> Self {
        JitoSolanaTestProgramClient {
            client_type: JitoClientType::SolanaTestProgram,
            context,
        }
    }
}

impl JitoClientTrait for JitoSolanaTestProgramClient {
    fn get_client_type(&self) -> JitoClientType {
        self.client_type.clone()
    }

    fn keypair(&self) -> &Keypair {
        &self.context.payer
    }

    async fn get_account(&self, address: &Pubkey) -> Result<Account> {
        let result = self
            .context
            .banks_client
            .get_account(*address)
            .await
            .map_err(|e| anyhow!("Could not get account for {}: {}", address, e))?;

        match result {
            Some(account) => Ok(account),
            None => Err(anyhow!("Account not found {}", address)),
        }
    }

    async fn get_balance(&self, address: &Pubkey) -> Result<u64> {
        self.context
            .banks_client
            .get_balance(*address)
            .await
            .map_err(|e| anyhow!("Could not get balance for {}: {}", address, e))
    }

    async fn get_recent_blockhash(&self) -> Result<Hash> {
        self.context
            .banks_client
            .get_latest_blockhash()
            .await
            .map_err(|e| anyhow!("Could not get latest blockhash {}", e))
    }

    async fn send_and_confirm_transaction(
        &self,
        transaction: Transaction,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Option<Signature>> {
        let result = self
            .context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                transaction,
                commitment.unwrap_or(CommitmentLevel::Confirmed),
            )
            .await;

        match result {
            Ok(()) => Ok(None),
            Err(e) => Err(anyhow!("Could not send transaction: {}", e)),
        }
    }

    async fn get_epoch_info(&self) -> Result<EpochInfo> {
        let slots_per_epoch: u64 = self.context.genesis_config().epoch_schedule.slots_per_epoch;
        let clock: Clock = self.context.banks_client.get_sysvar().await?;
        let block_height = self.context.banks_client.get_root_block_height().await?;

        Ok(EpochInfo {
            epoch: clock.epoch,
            slot_index: clock.slot % slots_per_epoch,
            slots_in_epoch: slots_per_epoch,
            absolute_slot: clock.slot,
            block_height,
            transaction_count: None,
        })
    }

    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64> {
        let rent = self
            .context
            .banks_client
            .get_rent()
            .await
            .map_err(|e| anyhow!("Error getting minimum balance for rent exemption: {}", e))?;
        Ok(rent.minimum_balance(data_len))
    }

    async fn test_warp_to_slot(&mut self, slot: u64) -> Result<()> {
        self.context
            .warp_to_slot(slot)
            .map_err(|e| anyhow!("Warp to slot {} failed: {}", slot, e))
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
        let mut new_account =
            AccountSharedData::new(account.lamports, account.data.len(), &account.owner);

        new_account.set_data_from_slice(account.data());
        self.context.set_account(address, &new_account);

        Ok(())
    }

    async fn test_airdrop(&mut self, address: &Pubkey, lamports: u64) -> Result<()> {
        let blockhash = self.get_recent_blockhash().await?;
        let tx = transfer(self.keypair(), address, lamports, blockhash);
        self.send_and_confirm_transaction(tx, None).await?;
        Ok(())
    }
}
