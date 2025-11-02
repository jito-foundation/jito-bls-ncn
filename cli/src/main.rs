use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use jito_bls_ncn_clients::{
    jito_clients::{
        rpc::JitoRpcClient, surf_pool::JitoSurfPoolClient, JitoClient, JitoClientTrait,
    },
    program_clients::meta_bls_ncn_client::setup_test_bls_ncn,
};
use serde_json::json;
use solana_keypair::Pubkey;
use std::fs;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "Jito BLS NCN")]
#[command(about = "Commands to run the Jito BLS NCN", long_about = None)]
struct Cli {
    /// RPC address to connect to
    #[arg(
        short,
        long,
        env = "RPC",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    rpc_url: String,
    #[arg(short, long, env = "Surfpool")]
    surfpool: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// View all vaults for a given public key
    View {
        /// Wallet to query vaults for
        #[arg(short, long, env = "WALLET")]
        wallet: String,
    },
    SurfpoolCreateTestNcn {
        /// Amount of test operators to make
        #[arg(short, long, env = "OPERATOR_COUNT", default_value_t = 3)]
        operator_count: usize,
        /// Amount of test vaults to make
        #[arg(short, long, env = "VAULT_COUNT", default_value_t = 3)]
        vault_count: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    let client = if cli.surfpool {
        JitoSurfPoolClient::new()
    } else {
        JitoRpcClient::new(cli.rpc_url)
    };

    // Match on the subcommand
    match &cli.command {
        Commands::View { wallet } => {
            let wallet_pubkey =
                Pubkey::from_str(wallet).map_err(|e| anyhow!("Could not read wallet: {}", e))?;
            view(&client, &wallet_pubkey).await
        }
        Commands::SurfpoolCreateTestNcn {
            operator_count,
            vault_count,
        } => {
            let mut client = JitoSurfPoolClient::new();
            create_test_ncn(&mut client, *operator_count, *vault_count).await
        }
    }
}

pub async fn view(client: &JitoClient, wallet: &Pubkey) -> Result<()> {
    let account = client.get_account(wallet).await?;
    println!("Wallet: {} ({})", wallet, account.lamports);
    Ok(())
}

pub async fn create_test_ncn(
    client: &mut JitoClient,
    operator_count: usize,
    vault_count: usize,
) -> Result<()> {
    let (ncn, bls_ncn_root) =
        setup_test_bls_ncn(client, operator_count, vault_count, vec![1000]).await?;

    // Build the JSON structure
    let mut operators_json = Vec::new();
    for (i, operator) in bls_ncn_root.test_ncn.operators.iter().enumerate() {
        let bls_keypair = &bls_ncn_root.operator_bls_keypairs[i];

        operators_json.push(json!({
            "operator_pubkey": operator.operator_pubkey.to_string(),
            "bls_keypair": serde_json::from_str::<serde_json::Value>(
                &bls_keypair.to_json().unwrap()
            ).map_err(|e| format!("Failed to parse BLS keypair JSON: {:?}", e)).unwrap()
        }));
    }

    let mut vaults_json = Vec::new();
    for vault in bls_ncn_root.test_ncn.vaults.iter() {
        vaults_json.push(json!({
            "vault_pubkey": vault.vault_pubkey.to_string(),
        }));
    }

    let output = json!({
        "ncn": ncn.to_string(),
        "operators": operators_json,
        "vaults": vaults_json,
    });

    // Write to file
    let json_str = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize JSON: {:?}", e))
        .unwrap();

    fs::write("test_ncn_output.json", json_str)
        .map_err(|e| format!("Failed to write to file: {:?}", e))
        .unwrap();

    println!("Test NCN data written to test_ncn_output.json");

    Ok(())
}
