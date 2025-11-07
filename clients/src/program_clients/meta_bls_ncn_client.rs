use crate::{
    jito_clients::{JitoClient, JitoClientTrait},
    program_clients::{
        bls_ncn_client::{
            full_snapshot, get_rolling_snapshot, initialize_bls_operator, initialize_config,
            initialize_consensus, initialize_rolling_snapshot, register_bls_operator,
            register_vault, BlsNcnRoot,
        },
        meta_restaking_client::{
            add_delegation_in_test_ncn, add_operators_to_test_ncn, add_vaults_to_test_ncn,
            create_test_ncn, TestNcn,
        },
        restaking_client::{get_epoch_length, NcnRoot, OperatorRoot},
        vault_client::{full_vault_update, VaultRoot},
    },
};
use anyhow::{anyhow, Result};
use jito_bls_ncn_core::{
    bls::solana_bls_interface::SolanaBN254Keypair, constants::BPS_PER_PERCENT, utils::get_epoch,
};
use serde_json::json;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use std::fs;

pub async fn setup_test_bls_ncn(
    client: &mut JitoClient,
    operator_count: usize,
    vault_count: usize,
    operator_delegation_pattern: Vec<u64>,
) -> Result<(Pubkey, BlsNcnRoot)> {
    let mut test_ncn = create_test_ncn(client).await?;
    let ncn = test_ncn.ncn_root.ncn_pubkey;

    let epoch_length = get_epoch_length(client).await?;

    add_operators_to_test_ncn(client, &mut test_ncn, operator_count, None).await?;
    add_vaults_to_test_ncn(client, &mut test_ncn, vault_count, None).await?;
    add_delegation_in_test_ncn(client, &test_ncn, operator_delegation_pattern).await?;

    client
        .test_warp_to_slot_incremental(epoch_length * 2)
        .await?;

    let mut bls_ncn_root = BlsNcnRoot {
        test_ncn: test_ncn.clone(),
        operator_bls_keypairs: Vec::new(),
    };

    initialize_config(client, &ncn).await?;
    initialize_consensus(client, &ncn).await?;
    initialize_rolling_snapshot(client, &ncn).await?;

    let mut operators = Vec::new();
    for operator_root in test_ncn.operators.iter() {
        let bls_keypair = SolanaBN254Keypair::new_unique()
            .map_err(|e| anyhow!("Could not create new bls keypair: {}", e))?;
        bls_ncn_root.operator_bls_keypairs.push(bls_keypair);

        initialize_bls_operator(client, &operator_root.operator_pubkey, &bls_keypair).await?;
        register_bls_operator(client, &ncn, &operator_root.operator_pubkey).await?;

        operators.push(operator_root.operator_pubkey);
    }

    let mut vaults = Vec::new();
    for vault_root in test_ncn.vaults.clone() {
        register_vault(client, &ncn, &vault_root.vault_pubkey, BPS_PER_PERCENT).await?;
        vaults.push(vault_root.vault_pubkey);
    }

    client.test_warp_to_slot_incremental(epoch_length).await?;

    for vault in vaults {
        full_vault_update(client, &vault, &operators).await?;
    }
    full_snapshot(client, &ncn).await?;

    {
        let current_epoch_info = client.get_epoch_info().await?;
        let snapshot = get_rolling_snapshot(client, &ncn).await?;

        let last_snapshot_epoch = get_epoch(snapshot.last_full_snapshot_slot.get(), epoch_length)?;
        let current_epoch = get_epoch(current_epoch_info.absolute_slot, epoch_length)?;

        assert_eq!(epoch_length, current_epoch_info.slots_in_epoch);
        assert_eq!(last_snapshot_epoch, current_epoch);
        assert_ne!(
            snapshot.total_security(current_epoch_info.absolute_slot, epoch_length)?,
            0
        );
    }

    Ok((ncn, bls_ncn_root))
}

/// Write the BLS NCN setup to a JSON file
pub fn write_bls_ncn_setup_to_file(
    ncn: &Pubkey,
    bls_ncn_root: &BlsNcnRoot,
    filepath: &str,
) -> Result<()> {
    // Serialize operators
    let mut operators_json = Vec::new();
    for (i, operator_root) in bls_ncn_root.test_ncn.operators.iter().enumerate() {
        let bls_keypair = &bls_ncn_root.operator_bls_keypairs[i];

        operators_json.push(json!({
            "operator_pubkey": operator_root.operator_pubkey.to_string(),
            "operator_admin": operator_root.operator_admin.to_base58_string(),
            "bls_keypair": serde_json::from_str::<serde_json::Value>(
                &bls_keypair.to_json()
                    .map_err(|e| anyhow!("Failed to serialize BLS keypair: {}", e))?
            )?
        }));
    }

    // Serialize vaults
    let mut vaults_json = Vec::new();
    for vault_root in bls_ncn_root.test_ncn.vaults.iter() {
        vaults_json.push(json!({
            "vault_pubkey": vault_root.vault_pubkey.to_string(),
            "vault_admin": vault_root.vault_admin.to_base58_string(),
            "mint": vault_root.mint.to_base58_string(),
        }));
    }

    // Build complete JSON structure
    let output = json!({
        "ncn": ncn.to_string(),
        "ncn_admin": bls_ncn_root.test_ncn.ncn_root.ncn_admin.to_base58_string(),
        "operators": operators_json,
        "vaults": vaults_json,
    });

    // Write to file
    let json_str = serde_json::to_string_pretty(&output)?;
    fs::write(filepath, json_str)?;

    println!("BLS NCN setup written to {}", filepath);

    Ok(())
}

/// Read the BLS NCN setup from a JSON file
pub fn read_bls_ncn_setup_from_file(filepath: &str) -> Result<(Pubkey, BlsNcnRoot)> {
    // Read file
    let json_str = fs::read_to_string(filepath)?;
    let json_obj: serde_json::Value = serde_json::from_str(&json_str)?;

    // Parse NCN
    let ncn = json_obj["ncn"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing ncn field"))?
        .parse::<Pubkey>()?;

    let ncn_admin_base58 = json_obj["ncn_admin"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing ncn_admin field"))?;
    let ncn_admin = Keypair::from_base58_string(ncn_admin_base58);

    // Parse operators
    let mut operators = Vec::new();
    let mut operator_bls_keypairs = Vec::new();

    let operators_array = json_obj["operators"]
        .as_array()
        .ok_or_else(|| anyhow!("Missing or invalid operators array"))?;

    for op in operators_array {
        let operator_pubkey = op["operator_pubkey"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing operator_pubkey"))?
            .parse::<Pubkey>()?;

        let operator_admin_base58 = op["operator_admin"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing operator_admin"))?;
        let operator_admin = Keypair::from_base58_string(operator_admin_base58);

        let bls_keypair_json = serde_json::to_string(&op["bls_keypair"])?;
        let bls_keypair = SolanaBN254Keypair::from_json(&bls_keypair_json)
            .map_err(|e| anyhow!("Failed to parse BLS keypair: {}", e))?;

        operators.push(OperatorRoot {
            operator_pubkey,
            operator_admin,
        });
        operator_bls_keypairs.push(bls_keypair);
    }

    // Parse vaults
    let mut vaults = Vec::new();
    let vaults_array = json_obj["vaults"]
        .as_array()
        .ok_or_else(|| anyhow!("Missing or invalid vaults array"))?;

    for vault in vaults_array {
        let vault_pubkey = vault["vault_pubkey"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing vault_pubkey"))?
            .parse::<Pubkey>()?;

        let vault_admin_base58 = vault["vault_admin"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing vault_admin"))?;
        let vault_admin = Keypair::from_base58_string(vault_admin_base58);

        let mint_base58 = vault["mint"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing mint"))?;
        let mint = Keypair::from_base58_string(mint_base58);

        vaults.push(VaultRoot {
            vault_pubkey,
            vault_admin,
            mint,
        });
    }

    // Build BlsNcnRoot
    let ncn_root = NcnRoot {
        ncn_pubkey: ncn,
        ncn_admin,
    };

    let test_ncn = TestNcn {
        ncn_root,
        operators,
        vaults,
    };

    let bls_ncn_root = BlsNcnRoot {
        test_ncn,
        operator_bls_keypairs,
    };

    println!("BLS NCN setup loaded from {}", filepath);

    Ok((ncn, bls_ncn_root))
}
