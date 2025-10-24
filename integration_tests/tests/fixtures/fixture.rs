#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Result;
use jito_bls_ncn_clients::jito_clients::JitoClient;
use jito_bls_ncn_sdk::bls_ncn_sdk::id;
use solana_program_test::ProgramTest;
use solana_signer::Signer;

// ===========================================================================
// Feature-based imports and type definitions
// ===========================================================================

#[cfg(feature = "surfpool")]
use jito_bls_ncn_clients::jito_clients::surf_pool::JitoSurfPoolClient;

#[cfg(feature = "test-program")]
use jito_bls_ncn_clients::jito_clients::solana_test_program::JitoSolanaTestProgramClient;

// Priority: test-program > surfpool
#[cfg(feature = "test-program")]
pub type TestClient = JitoSolanaTestProgramClient;

#[cfg(all(feature = "surfpool", not(feature = "test-program")))]
pub type TestClient = JitoSurfPoolClient;

#[cfg(not(any(feature = "surfpool", feature = "test-program")))]
compile_error!("Either 'surfpool' or 'test-program' feature must be enabled");

// ===========================================================================
// Client creation
// ===========================================================================

pub async fn create_test_client() -> Result<TestClient> {
    #[cfg(feature = "test-program")]
    {
        create_test_program_client().await
    }

    #[cfg(all(feature = "surfpool", not(feature = "test-program")))]
    {
        create_surfpool_client().await
    }
}

// ===========================================================================
// Implementation details
// ===========================================================================

#[cfg(feature = "test-program")]
async fn create_test_program_client() -> Result<JitoSolanaTestProgramClient> {
    // Setup BPF directory for loading programs
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let programs_dir = std::path::PathBuf::from(manifest_dir)
        .join("tests/bins")
        .canonicalize()
        .expect("Failed to find tests/bins directory");
    std::env::set_var("BPF_OUT_DIR", &programs_dir);

    // Configure and start program test
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(true);

    // Add required programs
    program_test.add_program("jito_bls_ncn_program", id(), None);
    program_test.add_program(
        "jito_vault_program",
        jito_bls_ncn_sdk::vault_sdk::id(),
        None,
    );
    program_test.add_program(
        "jito_restaking_program",
        jito_bls_ncn_sdk::restaking_sdk::id(),
        None,
    );

    // Initialize client and setup
    let context = program_test.start_with_context().await;
    let mut client = JitoSolanaTestProgramClient::new(context);

    jito_bls_ncn_clients::program_clients::meta_restaking_client::setup_restaking(&mut client)
        .await?;

    Ok(client)
}

#[cfg(feature = "surfpool")]
async fn create_surfpool_client() -> Result<JitoSurfPoolClient> {
    let mut client = JitoSurfPoolClient::new();
    let payer = client.keypair();

    // Airdrop SOL for testing
    client
        .test_airdrop(&payer.pubkey(), 100_000_000_000)
        .await?;

    Ok(client)
}
