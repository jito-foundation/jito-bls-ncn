use anyhow::Result;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_commitment_config::CommitmentLevel;
use solana_keypair::Keypair;
use solana_program_test::ProgramTestContext;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::{Transaction, Instruction};

pub enum JitoClient {
    Rpc {
        client: RpcClient,
        payer: Keypair,
    },
    Test {
        context: ProgramTestContext,
    },
}

impl JitoClient {
    /// Create a new RPC-based client
    pub fn new_rpc(rpc_url: String, payer: Keypair) -> Self {
        Self::Rpc {
            client: RpcClient::new(rpc_url),
            payer,
        }
    }

    /// Create a new test client from ProgramTestContext
    pub fn new_test(context: ProgramTestContext) -> Self {
        Self::Test { context }
    }

    /// Get the payer's pubkey
    pub fn payer(&self) -> Pubkey {
        match self {
            Self::Rpc { payer, .. } => payer.pubkey(),
            Self::Test { context } => context.payer.pubkey(),
        }
    }

    /// Get latest blockhash
    pub async fn get_latest_blockhash(&mut self) -> Result<solana_hash::Hash> {
        match self {
            Self::Rpc { client, .. } => Ok(client.get_latest_blockhash()?),
            Self::Test { context } => Ok(context.banks_client.get_latest_blockhash().await?),
        }
    }

    /// Send and confirm transaction
    pub async fn send_and_confirm_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: &[&dyn Signer],
    ) -> Result<()> {
        let blockhash = self.get_latest_blockhash().await?;

        match self {
            Self::Rpc { client, payer } => {
                let tx = Transaction::new_signed_with_payer(
                    instructions,
                    Some(&payer.pubkey()),
                    signers,
                    blockhash,
                );
                client.send_and_confirm_transaction_with_spinner(&tx)?;
                Ok(())
            }
            Self::Test { context } => {
                let tx = Transaction::new_signed_with_payer(
                    instructions,
                    Some(&context.payer.pubkey()),
                    signers,
                    blockhash,
                );
                context
                    .banks_client
                    .process_transaction_with_preflight_and_commitment(
                        tx,
                        CommitmentLevel::Processed,
                    )
                    .await?;
                Ok(())
            }
        }
    }

    /// Get program accounts with config
    pub async fn get_program_accounts_with_config(
        &mut self,
        program_id: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(Pubkey, solana_account::Account)>> {
        match self {
            Self::Rpc { client, .. } => {
                Ok(client.get_program_accounts_with_config(program_id, config)?)
            }
            Self::Test { context } => {
                // For test context, you'll need to implement filtering logic
                // This is a simplified version - you may need to add more sophisticated filtering
                let accounts = context.banks_client.get_program_accounts(*program_id).await?;

                // Apply filters if present
                if let Some(filters) = config.filters {
                    let filtered: Vec<_> = accounts
                        .into_iter()
                        .filter(|(_, account)| {
                            filters.iter().all(|filter| match filter {
                                RpcFilterType::Memcmp(memcmp) => {
                                    if let Some(offset) = memcmp.offset() {
                                        if let Some(bytes) = memcmp.bytes() {
                                            if offset + bytes.len() <= account.data.len() {
                                                return &account.data[offset..offset + bytes.len()] == bytes.as_slice();
                                            }
                                        }
                                    }
                                    false
                                }
                                RpcFilterType::DataSize(size) => account.data.len() == *size as usize,
                                RpcFilterType::TokenAccountState => true, // Implement if needed
                            })
                        })
                        .collect();
                    Ok(filtered)
                } else {
                    Ok(accounts)
                }
            }
        }
    }

    /// Get account data
    pub async fn get_account(&mut self, pubkey: &Pubkey) -> Result<Option<solana_account::Account>> {
        match self {
            Self::Rpc { client, .. } => Ok(client.get_account(pubkey).ok()),
            Self::Test { context } => Ok(context.banks_client.get_account(*pubkey).await?),
        }
    }
}
