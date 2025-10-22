use anyhow::{anyhow, Result};
use solana_commitment_config::CommitmentLevel;
use solana_keypair::Keypair;
use solana_program::program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_system_transaction::create_account;
use solana_transaction::Transaction;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_token_interface::state::{Account as TokenAccount, Mint};

use crate::jito_clients::JitoClient;

pub async fn transfer<T: JitoClient>(jito_client: &T, to: &Pubkey, lamports: u64) -> Result<()> {
    let payer = jito_client.keypair();
    let blockhash = jito_client.get_recent_blockhash().await?;
    let tx = solana_system_transaction::transfer(payer, to, lamports, blockhash);

    jito_client.send_and_confirm_transaction(tx, None).await?;

    Ok(())
}

pub async fn transfer_token<T: JitoClient>(
    jito_client: &T,
    destination: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    token_program: Option<Pubkey>,
) -> Result<()> {
    let token_program = token_program.unwrap_or(spl_token_interface::id());
    let payer = jito_client.keypair();

    let mint_account = get_mint(jito_client, mint).await?;

    let source_token_account = get_associated_token_address(&payer.pubkey(), mint);
    let destination_token_account = get_associated_token_address(destination, mint);

    let blockhash = jito_client.get_recent_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &payer.pubkey(),
                destination,
                mint,
                &token_program,
            ),
            spl_token_interface::instruction::transfer_checked(
                &token_program,
                &source_token_account,
                mint,
                &destination_token_account,
                &payer.pubkey(),
                &[],
                amount,
                mint_account.decimals,
            ).map_err(|e| anyhow!("Could not create transfer checked ix: {}", e))?
        ],
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(())
}

pub async fn get_mint<T: JitoClient>(jito_client: &T, mint: &Pubkey) -> Result<Mint> {
    let mint_account_raw = jito_client.get_account(mint).await?;
    let mint_account = Mint::unpack(&mint_account_raw.data)?;
    Ok(mint_account)
}

pub async fn get_associated_token_account<T: JitoClient>(
    jito_client: &T,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<TokenAccount> {
    let ata = get_associated_token_address(owner, mint);
    get_token_account(jito_client, &ata).await
}

pub async fn get_token_account<T: JitoClient>(
    jito_client: &T,
    token: &Pubkey,
) -> Result<TokenAccount> {
    let token_account_raw = jito_client.get_account(token).await?;
    let token_account = TokenAccount::unpack(&token_account_raw.data)?;

    Ok(token_account)
}

pub async fn create_mint<T: JitoClient>(
    jito_client: &T,
    mint: &Keypair,
    decimals: u8,
    mint_authority: Option<&Pubkey>,
    freeze_authority: Option<&Pubkey>,
    token_program: Option<Pubkey>,
) -> Result<()> {
    let token_program = token_program.unwrap_or(spl_token_interface::id());
    let payer = jito_client.keypair();
    let payer_pubkey = payer.pubkey();
    let mint_authority = mint_authority.unwrap_or(&payer_pubkey);

    let blockhash = jito_client.get_recent_blockhash().await?;
    let min_rent = jito_client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await?;

    let create_tx = create_account(
        payer,
        mint,
        blockhash,
        min_rent,
        Mint::LEN as u64,
        &token_program,
    );

    jito_client
        .send_and_confirm_transaction(create_tx, Some(CommitmentLevel::Processed))
        .await?;

    let blockhash = jito_client.get_recent_blockhash().await?;

    jito_client
        .send_and_confirm_transaction(
            Transaction::new_signed_with_payer(
                &[spl_token_interface::instruction::initialize_mint2(
                    &token_program,
                    &mint.pubkey(),
                    mint_authority,
                    freeze_authority,
                    decimals,
                )
                .map_err(|e| anyhow!("Could not create initialize_mint2 ix: {}", e))?],
                Some(&payer.pubkey()),
                &[payer],
                blockhash,
            ),
            Some(CommitmentLevel::Processed),
        )
        .await?;

    Ok(())
}

/// Mints tokens to an ATA owned by the `to` address
pub async fn mint_spl_to<T: JitoClient>(
    jito_client: &T,
    mint: &Pubkey,
    to: &Pubkey,
    amount: u64,
    token_program: Option<Pubkey>,
) -> Result<()> {
    let token_program = token_program.unwrap_or(spl_token_interface::id());
    let payer = jito_client.keypair();
    let blockhash = jito_client.get_recent_blockhash().await?;

    let mint_to_ixs = vec![
        spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            to,
            mint,
            &token_program,
        ),
        spl_token_interface::instruction::mint_to(
            &token_program,
            mint,
            &get_associated_token_address(to, mint),
            &payer.pubkey(),
            &[],
            amount,
        )
        .map_err(|e| anyhow!("Could not mint to: {}", e))?,
    ];

    jito_client
        .send_and_confirm_transaction(
            Transaction::new_signed_with_payer(
                &mint_to_ixs,
                Some(&payer.pubkey()),
                &[&payer],
                blockhash,
            ),
            Some(CommitmentLevel::Processed),
        )
        .await?;

    Ok(())
}

pub async fn create_ata<T: JitoClient>(
    jito_client: &T,
    wallet: &Pubkey,
    mint: &Pubkey,
    token_program: Option<Pubkey>,
) -> Result<Pubkey> {
    let token_program = token_program.unwrap_or(spl_token_interface::id());
    let payer = jito_client.keypair();

    let ata = get_associated_token_address(wallet, mint);

    let blockhash = jito_client.get_recent_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account_interface::instruction::create_associated_token_account(
                &payer.pubkey(),
                wallet,
                mint,
                &token_program,
            ),
        ],
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    jito_client.send_and_confirm_transaction(tx, None).await?;
    Ok(ata)
}
