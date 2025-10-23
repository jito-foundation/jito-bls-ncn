#![allow(dead_code)]

use anyhow::{anyhow, Result};
use hex;
use jito_bls_ncn_clients::jito_clients::JitoClient;
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
use spl_token_interface::{
    instruction::{initialize_mint2, transfer_checked},
    state::{Account, Mint},
};
use std::fmt::{Debug, Formatter};

// Import the concrete client types
#[cfg(feature = "surfpool")]
use jito_bls_ncn_clients::jito_clients::surf_pool::JitoSurfPoolClient;
#[cfg(feature = "test-program")]
use jito_bls_ncn_clients::jito_clients::solana_test_program::JitoSolanaTestProgramClient;

// Define the type alias based on features
#[cfg(feature = "surfpool")]
pub type TestClient = JitoSurfPoolClient;
#[cfg(feature = "test-program")]
pub type TestClient = JitoSolanaTestProgramClient;

pub async fn create_test_client() -> Result<TestClient> {
    #[cfg(feature = "test-program")]
    {
        // Since this file is in integration_tests/tests/fixtures/
        // the bins directory is at ../bins relative to this file
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR not set");

        let programs_dir = std::path::PathBuf::from(manifest_dir)
            .join("tests/bins")
            .canonicalize()
            .expect("Failed to find tests/bins directory");

        std::env::set_var("BPF_OUT_DIR", &programs_dir);

        let mut program_test = ProgramTest::default();

        // Add programs - they'll be loaded from BPF_OUT_DIR
        program_test.add_program(
            "jito_bls_ncn_program",
            id(),
            None,
        );

        program_test.add_program(
            "jito_vault_program",
            jito_bls_ncn_core::programs::vault_sdk::id(),
            None,
        );

        program_test.add_program(
            "jito_restaking_program",
            jito_bls_ncn_core::programs::restaking_sdk::id(),
            None,
        );

        program_test.prefer_bpf(true);

        let context = program_test.start_with_context().await;

        let mut client = JitoSolanaTestProgramClient::new(context);

        jito_bls_ncn_clients::program_clients::meta_restaking_client::setup_restaking(&mut client).await?;

        Ok(client)
    }

    #[cfg(feature = "surfpool")]
    {
       let mut client = JitoSurfPoolClient::new();

       let payer = client.keypair();
       client.test_airdrop(&payer.pubkey(), 100_000_000_000).await?;

       Ok(client)
    }
}
