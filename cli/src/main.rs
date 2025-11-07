use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use jito_bls_ncn_clients::{
    jito_clients::{
        rpc::JitoRpcClient, surf_pool::JitoSurfPoolClient, JitoClient, JitoClientTrait,
    },
    program_clients::meta_bls_ncn_client::setup_test_bls_ncn,
};
use jito_bls_ncn_core::bls::solana_bls_interface::{
    SolanaBN254Keypair, TestBlsNcn, TestBlsOperator, TestBlsOrchestrator, TestBlsVault,
};
use solana_keypair::Pubkey;
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

    // Create test operators
    let mut test_operators = Vec::new();
    for (i, operator) in bls_ncn_root.test_ncn.operators.iter().enumerate() {
        let bls_keypair = bls_ncn_root.operator_bls_keypairs[i];
        test_operators.push(TestBlsOperator::new(bls_keypair, operator.operator_pubkey));
    }

    // Create test vaults
    let mut test_vaults = Vec::new();
    for vault in bls_ncn_root.test_ncn.vaults.iter() {
        test_vaults.push(TestBlsVault::new(vault.vault_pubkey));
    }

    // Create orchestrator
    let orchestrator_bls_keypair =
        SolanaBN254Keypair::new_unique().map_err(|e| anyhow!("Could not create keypair: {}", e))?;
    let bls_orchestrator = TestBlsOrchestrator::new(orchestrator_bls_keypair);

    // Create TestBlsStruct
    let test_bls_struct = TestBlsNcn::new(
        "http://127.0.0.1:8899".to_string(),
        client.keypair().insecure_clone(),
        ncn,
        bls_orchestrator,
        test_operators,
        test_vaults,
    );

    // Write to file
    test_bls_struct
        .to_json_file("test_ncn_output.json")
        .map_err(|e| anyhow!("Could not write test NCN json file: {}", e))?;

    println!("Test NCN data written to test_ncn_output.json");

    Ok(())
}
