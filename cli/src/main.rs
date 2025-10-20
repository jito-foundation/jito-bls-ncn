use anyhow::{anyhow, Ok, Result};
use clap::{Parser, Subcommand};
use solana_client::rpc_client::RpcClient;
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
    rpc: String,

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
}

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    let rpc_client = RpcClient::new(cli.rpc.clone());

    // Match on the subcommand
    match &cli.command {
        Commands::View { wallet } => {
            let wallet_pubkey =
                Pubkey::from_str(wallet).map_err(|e| anyhow!("Could not read wallet: {}", e))?;

            view(&rpc_client, &wallet_pubkey)
        }
    }
}

pub fn view(_rpc_client: &RpcClient, wallet: &Pubkey) -> Result<()> {
    println!("Wallet: {}", wallet);

    Ok(())
}
