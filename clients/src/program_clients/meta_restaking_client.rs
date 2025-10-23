use anyhow::Result;
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::jito_clients::JitoClient;
use crate::program_clients::restaking_client::{NcnRoot, OperatorRoot};
use crate::program_clients::vault_client::VaultRoot;

#[derive(Clone)]
pub struct TestNcn {
    pub ncn_root: NcnRoot,
    pub operators: Vec<OperatorRoot>,
    pub vaults: Vec<VaultRoot>,
}

/// Basic restaking setup with config initialization
/// Initializes both vault and restaking configs
pub async fn setup_restaking<T: JitoClient>(jito_client: &mut T) -> Result<()> {
    // Initialize vault config
    crate::program_clients::vault_client::test_initialize_config(jito_client).await?;

    // Initialize restaking config
    crate::program_clients::restaking_client::test_initialize_config(jito_client).await?;

    Ok(())
}

/// Create a test NCN with full setup
/// Initializes configs and creates an NCN
pub async fn create_test_ncn<T: JitoClient>(jito_client: &mut T) -> Result<TestNcn> {

    // Initialize NCN
    let ncn_root = crate::program_clients::restaking_client::test_initialize_ncn(jito_client).await?;

    Ok(TestNcn {
        ncn_root: ncn_root.clone(),
        operators: vec![],
        vaults: vec![],
    })
}

/// Add multiple operators to a test NCN and warm them up
pub async fn add_operators_to_test_ncn<T: JitoClient>(
    jito_client: &mut T,
    test_ncn: &mut TestNcn,
    operator_count: usize,
    operator_fees_bps: Option<u16>,
) -> Result<()> {
    for _ in 0..operator_count {
        // Initialize operator
        let operator_root = if let Some(fee_bps) = operator_fees_bps {
            // Create operator with custom fee
            let operator_base = Keypair::new();
            let operator_admin = jito_client.keypair().insecure_clone();
            let (operator_pubkey, _) = jito_bls_ncn_core::programs::restaking_sdk::operator_address(&operator_base.pubkey());
            let (config, _) = jito_bls_ncn_core::programs::restaking_sdk::config_address();

            jito_client.test_airdrop(&operator_admin.pubkey(), 1_000_000_000).await?;

            crate::program_clients::restaking_client::initialize_operator(
                jito_client,
                &config,
                &operator_pubkey,
                &operator_admin,
                &operator_base,
                fee_bps,
            ).await?;

            OperatorRoot {
                operator_pubkey,
                operator_admin,
            }
        } else {
            // Use default operator initialization
            crate::program_clients::restaking_client::test_initialize_operator(jito_client).await?
        };

        // Initialize NCN <> Operator relationship
        crate::program_clients::restaking_client::test_initialize_ncn_operator_state(
            jito_client,
            &test_ncn.ncn_root,
            &operator_root,
        ).await?;

        // Warp slot to allow warmup
        jito_client.test_warp_to_slot_incremental(1).await?;

        // Warmup NCN side
        crate::program_clients::restaking_client::test_ncn_warmup_operator(
            jito_client,
            &test_ncn.ncn_root,
            &operator_root.operator_pubkey,
        ).await?;

        // Warmup Operator side
        crate::program_clients::restaking_client::test_operator_warmup_ncn(
            jito_client,
            &operator_root,
            &test_ncn.ncn_root.ncn_pubkey,
        ).await?;

        test_ncn.operators.push(operator_root);
    }

    Ok(())
}

/// Add multiple vaults to a test NCN, connecting them to the NCN and all operators
pub async fn add_vaults_to_test_ncn<T: JitoClient>(
    jito_client: &mut T,
    test_ncn: &mut TestNcn,
    vault_count: usize,
    token_mint: Option<Keypair>,
) -> Result<()> {
    const DEPOSIT_FEE_BPS: u16 = 0;
    const WITHDRAWAL_FEE_BPS: u16 = 0;
    const REWARD_FEE_BPS: u16 = 0;
    const DECIMALS: u8 = 9;
    let mint_amount: u64 = 100_000_000 * 10u64.pow(DECIMALS as u32);

    let should_generate = token_mint.is_none();
    let pass_through = if token_mint.is_some() {
        token_mint.unwrap()
    } else {
        Keypair::new()
    };

    for _ in 0..vault_count {
        let pass_through = if should_generate {
            Keypair::new()
        } else {
            pass_through.insecure_clone()
        };

        // Initialize vault with custom parameters
        let vault_root = {
            let admin = jito_client.keypair().insecure_clone();
            let base = Keypair::new();
            let vrt_mint = Keypair::new();
            let st_mint = pass_through;

            let initialize_token_amount = 1_000_000;

            let (vault, _) = jito_bls_ncn_core::programs::vault_sdk::vault_address(&base.pubkey());
            let (burn_vault, _) = jito_bls_ncn_core::programs::vault_sdk::burn_vault_address(&base.pubkey());
            let (config, _) = jito_bls_ncn_core::programs::vault_sdk::config_address();

            // Airdrop to vault admin
            jito_client.test_airdrop(&admin.pubkey(), 1_000_000_000).await?;

            // Create mint
            crate::program_clients::solana_client::create_mint(
                jito_client,
                &st_mint,
                DECIMALS,
                None,
                None,
                None
            ).await?;

            // Initialize vault
            crate::program_clients::vault_client::initialize_vault(
                jito_client,
                &config,
                &vault,
                &burn_vault,
                &vrt_mint,
                &st_mint,
                &admin,
                &base,
                DEPOSIT_FEE_BPS,
                WITHDRAWAL_FEE_BPS,
                REWARD_FEE_BPS,
                DECIMALS,
                initialize_token_amount,
            ).await?;

            // Create necessary ATAs
            crate::program_clients::solana_client::create_ata(
                jito_client,
                &admin.pubkey(),
                &vrt_mint.pubkey(),
                None
            ).await?;

            VaultRoot {
                vault_pubkey: vault,
                vault_admin: admin,
                mint: st_mint,
            }
        };

        // Vault <> NCN relationship
        crate::program_clients::restaking_client::test_initialize_ncn_vault_ticket(
            jito_client,
            &test_ncn.ncn_root,
            &vault_root.vault_pubkey,
        ).await?;

        jito_client.test_warp_to_slot_incremental(1).await?;

        crate::program_clients::restaking_client::test_warmup_ncn_vault_ticket(
            jito_client,
            &test_ncn.ncn_root,
            &vault_root.vault_pubkey,
        ).await?;

        crate::program_clients::vault_client::test_initialize_vault_ncn_ticket(
            jito_client,
            &vault_root,
            &test_ncn.ncn_root.ncn_pubkey,
        ).await?;

        jito_client.test_warp_to_slot_incremental(1).await?;

        crate::program_clients::vault_client::test_warmup_vault_ncn_ticket(
            jito_client,
            &vault_root,
            &test_ncn.ncn_root.ncn_pubkey,
        ).await?;

        // Connect vault to all operators
        for operator_root in test_ncn.operators.iter() {
            // Vault <> Operator relationship
            crate::program_clients::restaking_client::test_initialize_operator_vault_ticket(
                jito_client,
                operator_root,
                &vault_root.vault_pubkey,
            ).await?;

            jito_client.test_warp_to_slot_incremental(1).await?;

            crate::program_clients::restaking_client::test_warmup_operator_vault_ticket(
                jito_client,
                operator_root,
                &vault_root.vault_pubkey,
            ).await?;

            crate::program_clients::vault_client::test_initialize_vault_operator_delegation(
                jito_client,
                &vault_root,
                &operator_root.operator_pubkey,
            ).await?;
        }

        // Configure depositor and mint tokens
        let depositor_keypair = jito_client.keypair().insecure_clone();
        let depositor = depositor_keypair.pubkey();

        crate::program_clients::vault_client::test_configure_depositor(
            jito_client,
            &vault_root,
            &depositor,
            mint_amount,
        ).await?;

        crate::program_clients::vault_client::test_mint_to(
            jito_client,
            &vault_root,
            &depositor_keypair,
            mint_amount,
            mint_amount,
        ).await?;

        test_ncn.vaults.push(vault_root);
    }

    Ok(())
}

/// Add delegations from vaults to operators
pub async fn add_delegation_in_test_ncn<T: JitoClient>(
    jito_client: &mut T,
    test_ncn: &TestNcn,
    delegation_amount: usize,
) -> Result<()> {
    for vault_root in test_ncn.vaults.iter() {
        for operator_root in test_ncn.operators.iter() {
            crate::program_clients::vault_client::test_add_delegation(
                jito_client,
                vault_root,
                &operator_root.operator_pubkey,
                delegation_amount as u64,
            ).await?;
        }
    }

    Ok(())
}

pub async fn update_all_vaults_in_test_ncn<T: JitoClient>(
    jito_client: &mut T,
    test_ncn: &TestNcn,
) -> Result<()> {
    for vault_root in test_ncn.vaults.iter() {
        let operators = test_ncn.operators.iter().map(|op| op.operator_pubkey).collect::<Vec<_>>();
        crate::program_clients::vault_client::full_vault_update(
            jito_client,
            &vault_root.vault_pubkey,
            &operators,
        ).await?;
    }

    Ok(())
}
