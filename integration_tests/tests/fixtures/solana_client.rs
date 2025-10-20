use anyhow::{Result, anyhow};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Instruction;


pub async fn send_transaction(
    &mut self,
    instructions: &[Instruction],
    payer: Option<&Pubkey>,
    signers: &[&dyn Signer],
) -> Result<()> {
    // Fetch latest blockhash
    let recent_blockhash = self
        .context
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
    println!(
        "  Recent blockhash: {} (just fetched)",
        msg.recent_blockhash
    );
    println!(
        "  Num required signatures: {}",
        msg.header.num_required_signatures
    );
    println!(
        "  Num readonly signed accounts: {}",
        msg.header.num_readonly_signed_accounts
    );
    println!(
        "  Num readonly unsigned accounts: {}",
        msg.header.num_readonly_unsigned_accounts
    );

    // Account keys
    println!("\n[Account Keys] ({} total)", msg.account_keys.len());
    for (i, key) in msg.account_keys.iter().enumerate() {
        println!("  [{}]: {}", i, key);
    }

    // Instructions
    println!("\n[Instructions] ({} total)", msg.instructions.len());
    for (i, ix) in msg.instructions.iter().enumerate() {
        println!("\n  Instruction #{}:", i);
        println!(
            "    Program: {} (account_keys[{}])",
            msg.account_keys[ix.program_id_index as usize], ix.program_id_index
        );
        println!("    Account indices: {:?}", ix.accounts);
        println!("    Data: {} bytes", ix.data.len());
        println!("    Data (hex): {}", hex::encode(&ix.data));
    }

    println!("\n{}", "=".repeat(60));
    println!("PROCESSING...");
    println!("{}\n", "=".repeat(60));

    // Use simulate instead to get logs
    let simulation = self
        .context
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
    let result = self
        .context
        .banks_client
        .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Processed)
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
