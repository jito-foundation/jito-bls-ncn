#![allow(dead_code)]

use anyhow::{Result, anyhow};
use jito_bls_ncn_sdk::id;
use solana_commitment_config::CommitmentLevel;
use solana_keypair::Keypair;
use solana_program::{clock::Clock, program_pack::Pack};
use solana_program_test::{BanksClientError, ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_system_transaction::{create_account, transfer};
use solana_transaction::{Instruction, Transaction};
use spl_associated_token_account_interface::{
    address::get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use hex;
use spl_token_interface::{
    instruction::{initialize_mint2, transfer_checked},
    state::{Account, Mint},
};
use std::fmt::{Debug, Formatter};

pub struct TestBuilder {
    pub context: ProgramTestContext,
}

impl Debug for TestBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestBuilder",)
    }
}

impl TestBuilder {
    pub async fn new() -> Self {
        let mut program_test = ProgramTest::new("jito_bls_ncn_program", id(), None);

        program_test.prefer_bpf(true);

        let context = program_test.start_with_context().await;

        Self { context }
    }

    pub async fn airdrop(&mut self, to: &Pubkey, lamports: u64) -> Result<()> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;
        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                transfer(&self.context.payer, to, lamports, blockhash),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    pub async fn transfer(&mut self, to: &Pubkey, sol: f64) -> Result<()> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;
        let lamports: u64 = (sol / 1_000_000_000.0).round() as u64;

        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                transfer(&self.context.payer, to, lamports, blockhash),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    /// Transfers tokens from the source to the destination
    /// source: the source account - ( not the associated token account )
    /// destination: the destination account - ( not the associated token account )
    pub async fn transfer_token(
        &mut self,
        token_program_id: &Pubkey,
        source: &Keypair,
        destination: &Pubkey,
        mint: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;

        let mint_account_raw = self
            .context
            .banks_client
            .get_account(*mint)
            .await?
            .ok_or(BanksClientError::ClientError("failed to get mint account"))?;
        let mint_account = Mint::unpack(&mint_account_raw.data)?;

        let source_token_account = get_associated_token_address(&source.pubkey(), mint);
        let destination_token_account = get_associated_token_address(&destination, mint);

        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[transfer_checked(
                        &token_program_id,
                        &source_token_account,
                        &mint,
                        &destination_token_account,
                        &source.pubkey(),
                        &[],
                        amount,
                        mint_account.decimals,
                    )
                    .unwrap()],
                    Some(&self.context.payer.pubkey()),
                    &[&source, &self.context.payer],
                    blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    pub async fn get_token_account(&mut self, token_account: &Pubkey) -> Result<Account> {
        let account = self
            .context
            .banks_client
            .get_account(*token_account)
            .await?
            .ok_or(BanksClientError::ClientError("failed to get token account"))?;

        let account_info = Account::unpack(&account.data)
            .map_err(|_e| BanksClientError::ClientError("failed to unpack"))?;

        Ok(account_info)
    }

    pub async fn get_token_mint(&mut self, token_mint: &Pubkey) -> Result<Mint> {
        let account = self
            .context
            .banks_client
            .get_account(*token_mint)
            .await?
            .ok_or(BanksClientError::ClientError("failed to get token account"))?;

        let account_info = Mint::unpack(&account.data)
            .map_err(|_e| BanksClientError::ClientError("failed to unpack"))?;

        Ok(account_info)
    }

    /// Mints tokens to an ATA owned by the `to` address
    pub async fn mint_spl_to(
        &mut self,
        mint: &Pubkey,
        to: &Pubkey,
        amount: u64,
        token_program: &Pubkey,
    ) -> Result<()> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;

        let mint_to_ix = if token_program.eq(&spl_token_interface::id()) {
            vec![
                create_associated_token_account_idempotent(
                    &self.context.payer.pubkey(),
                    to,
                    mint,
                    token_program,
                ),
                spl_token_interface::instruction::mint_to(
                    token_program,
                    mint,
                    &get_associated_token_address(to, mint),
                    &self.context.payer.pubkey(),
                    &[],
                    amount,
                )
                .map_err(|_e| BanksClientError::ClientError("failed to mint to"))?,
            ]
        } else {
            vec![spl_token_interface::instruction::mint_to(
                token_program,
                mint,
                to,
                &self.context.payer.pubkey(),
                &[],
                amount,
            )
            .map_err(|_e| BanksClientError::ClientError("failed to mint to"))?]
        };
        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &mint_to_ix,
                    Some(&self.context.payer.pubkey()),
                    &[&self.context.payer],
                    blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    pub async fn create_mint(&mut self, mint: &Keypair) -> Result<()> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;
        let rent = self.context.banks_client.get_rent().await?;
        let min_rent = rent.minimum_balance(Mint::LEN);

        let create_tx = create_account(
            &self.context.payer,
            &mint,
            blockhash,
            min_rent,
            Mint::LEN as u64,
            &spl_token_interface::id(),
        );

        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                create_tx,
                CommitmentLevel::Processed,
            )
            .await?;

        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[initialize_mint2(
                        &spl_token_interface::id(),
                        &mint.pubkey(),
                        &self.context.payer.pubkey(),
                        None,
                        9,
                    )?],
                    Some(&self.context.payer.pubkey()),
                    &[&self.context.payer],
                    blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;

        Ok(())
    }

    pub async fn create_ata(&mut self, mint: &Pubkey, owner: &Pubkey) -> Result<()> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;
        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[create_associated_token_account_idempotent(
                        &self.context.payer.pubkey(),
                        owner,
                        mint,
                        &spl_token_interface::id(),
                    )],
                    Some(&self.context.payer.pubkey()),
                    &[&self.context.payer],
                    blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    pub async fn warp_slot_incremental(&mut self, incremental_slots: u64) -> Result<()> {
        let clock: Clock = self.context.banks_client.get_sysvar().await?;
        self.context
            .warp_to_slot(clock.slot.checked_add(incremental_slots).unwrap())
            .map_err(|_| BanksClientError::ClientError("failed to warp slot"))?;
        Ok(())
    }

    pub async fn warp_to_slot(&mut self, warp_slot: u64) -> Result<()> {
        self.context
            .warp_to_slot(warp_slot)
            .map_err(|_| BanksClientError::ClientError("failed to warp slot"))?;
        Ok(())
    }

    pub async fn get_current_slot(&mut self) -> Result<u64> {
        let clock: Clock = self.context.banks_client.get_sysvar().await?;
        Ok(clock.slot)
    }

    pub async fn send_transaction(
        &mut self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signers: &[&dyn Signer],
    ) -> Result<()> {
        // Fetch latest blockhash
        let recent_blockhash = self.context
            .banks_client
            .get_latest_blockhash()
            .await
            .map_err(|e| anyhow!("failed to get blockhash: {}", e))?;

        // Create transaction
        let tx = Transaction::new_signed_with_payer(
            instructions,
            payer.or(Some(&self.context.payer.pubkey())),
            signers,
            recent_blockhash,
        );

        println!("\n{}", "=".repeat(60));
        println!("SENDING TRANSACTION");
        println!("{}", "=".repeat(60));

        // Signature info
        println!("\n[Signatures] ({} total)", tx.signatures.len());
        for (i, sig) in tx.signatures.iter().enumerate() {
            println!("  {}: {}", i, sig);
        }

        // Message info
        let msg = &tx.message;
        println!("\n[Message]");
        println!("  Recent blockhash: {} (just fetched)", msg.recent_blockhash);
        println!("  Num required signatures: {}", msg.header.num_required_signatures);
        println!("  Num readonly signed accounts: {}", msg.header.num_readonly_signed_accounts);
        println!("  Num readonly unsigned accounts: {}", msg.header.num_readonly_unsigned_accounts);

        // Account keys
        println!("\n[Account Keys] ({} total)", msg.account_keys.len());
        for (i, key) in msg.account_keys.iter().enumerate() {
            println!("  [{}]: {}", i, key);
        }

        // Instructions
        println!("\n[Instructions] ({} total)", msg.instructions.len());
        for (i, ix) in msg.instructions.iter().enumerate() {
            println!("\n  Instruction #{}:", i);
            println!("    Program: {} (account_keys[{}])",
                     msg.account_keys[ix.program_id_index as usize],
                     ix.program_id_index);
            println!("    Account indices: {:?}", ix.accounts);
            println!("    Data: {} bytes", ix.data.len());
            println!("    Data (hex): {}", hex::encode(&ix.data));
        }

        println!("\n{}", "=".repeat(60));
        println!("PROCESSING...");
        println!("{}\n", "=".repeat(60));

        // Use simulate instead to get logs
        let simulation = self.context
            .banks_client
            .simulate_transaction(tx.clone())
            .await
            .map_err(|e| anyhow!("failed to simulate transaction: {}", e))?;

        // Print logs from simulation
        println!("\n{}", "=".repeat(60));
        println!("TRANSACTION LOGS");
        println!("{}", "=".repeat(60));
        if let Some(details) = simulation.simulation_details {
            for log in &details.logs {
                println!("{}", log);
            }
        } else {
            println!("No simulation details available");
        }
        println!("{}\n", "=".repeat(60));

        // Now actually process it
        let result = self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                tx,
                CommitmentLevel::Processed,
            )
            .await;

        match result {
            Ok(_) => {
                println!("\n✅ SUCCESS: Transaction processed successfully\n");
                Ok(())
            }
            Err(e) => {
                eprintln!("\n❌ FAILED: Transaction failed");
                eprintln!("{}", "=".repeat(60));
                eprintln!("Error: {:#?}", e);
                eprintln!("{}\n", "=".repeat(60));
                Err(anyhow!("failed to send transaction: {}", e))
            }
        }
    }
}
