use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use jito_bls_ncn_clients::jito_clients::{
    rpc::JitoRpcClient, surf_pool::JitoSurfPoolClient, JitoClient, JitoClientTrait,
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
    SurfpoolCreateTestNcn {},
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    let client = if cli.surfpool {
        JitoClient::SurfPool(JitoSurfPoolClient::new())
    } else {
        JitoClient::Rpc(JitoRpcClient::new(cli.rpc_url))
    };

    // Match on the subcommand
    match &cli.command {
        Commands::View { wallet } => {
            let wallet_pubkey =
                Pubkey::from_str(wallet).map_err(|e| anyhow!("Could not read wallet: {}", e))?;
            view(&client, &wallet_pubkey).await
        }
        Commands::SurfpoolCreateTestNcn {} => {
            todo!()
        }
    }
}

pub async fn view(client: &JitoClient, wallet: &Pubkey) -> Result<()> {
    let account = client.get_account(wallet).await?;
    println!("Wallet: {} ({})", wallet, account.lamports);
    Ok(())
}
