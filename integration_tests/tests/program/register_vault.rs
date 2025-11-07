#[cfg(test)]
mod tests {

    use anyhow::{anyhow, Result};
    use jito_bls_ncn_clients::{
        jito_clients::JitoClientTrait,
        program_clients::{
            bls_ncn_client::{
                full_snapshot, get_bls_operator, initialize_bls_operator, initialize_config,
                initialize_rolling_snapshot, register_bls_operator, register_vault, BlsNcnRoot,
            },
            meta_restaking_client::{
                add_delegation_in_test_ncn, add_operators_to_test_ncn, add_vaults_to_test_ncn,
                create_test_ncn,
            },
            vault_client::full_vault_update,
        },
    };
    use jito_bls_ncn_core::{
        bls::solana_bls_interface::SolanaBN254Keypair, constants::BPS_PER_PERCENT,
    };
    use solana_program_test::tokio;
    use solana_pubkey::Pubkey;

    use crate::fixtures::fixture::create_test_client;

    #[tokio::test]
    async fn test_register_vault_ok() -> Result<()> {
        let mut client = create_test_client().await?;

        let operator_count = 3;
        let vault_count = 3;

        let mut test_ncn = create_test_ncn(&mut client).await?;
        add_operators_to_test_ncn(&mut client, &mut test_ncn, operator_count, None).await?;
        add_vaults_to_test_ncn(&mut client, &mut test_ncn, vault_count, None).await?;
        add_delegation_in_test_ncn(&mut client, &test_ncn, vec![1000]).await?;

        client.test_warp_to_slot_incremental(1_000_000).await?;
        let operators = test_ncn
            .operators
            .iter()
            .map(|operator| operator.operator_pubkey)
            .collect::<Vec<Pubkey>>();

        for vault_root in test_ncn.vaults.iter() {
            full_vault_update(&mut client, &vault_root.vault_pubkey, &operators).await?;
        }

        let mut bls_ncn_root = BlsNcnRoot {
            test_ncn: test_ncn.clone(),
            operator_bls_keypairs: Vec::new(),
        };

        initialize_config(&client, &test_ncn.ncn_root.ncn_pubkey).await?;
        initialize_rolling_snapshot(&mut client, &test_ncn.ncn_root.ncn_pubkey).await?;
        full_snapshot(&client, &test_ncn.ncn_root.ncn_pubkey).await?;

        for operator_root in test_ncn.operators {
            let bls_keypair = SolanaBN254Keypair::new_unique()
                .map_err(|e| anyhow!("Could not create new bls keypair: {}", e))?;
            bls_ncn_root.operator_bls_keypairs.push(bls_keypair);

            initialize_bls_operator(&client, &operator_root.operator_pubkey, &bls_keypair).await?;
            register_bls_operator(
                &client,
                &test_ncn.ncn_root.ncn_pubkey,
                &operator_root.operator_pubkey,
            )
            .await?;
            get_bls_operator(&client, &operator_root.operator_pubkey).await?;
        }

        for vault_root in test_ncn.vaults.iter() {
            register_vault(
                &client,
                &test_ncn.ncn_root.ncn_pubkey,
                &vault_root.vault_pubkey,
                BPS_PER_PERCENT,
            )
            .await?;
        }

        Ok(())
    }
}
