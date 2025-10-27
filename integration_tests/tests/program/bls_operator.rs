#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use jito_bls_ncn_clients::{
        jito_clients::JitoClient,
        program_clients::{
            bls_ncn_client::{
                get_bls_operator, initialize_bls_operator, initialize_config,
                initialize_rolling_snapshot, register_bls_operator, BlsNcnRoot,
            },
            meta_restaking_client::{add_operators_to_test_ncn, create_test_ncn},
        },
    };
    use jito_bls_ncn_core::bls::solana_bls_interface::SolanaBN254Keypair;
    use solana_program_test::tokio;

    use crate::fixtures::fixture::create_test_client;

    #[tokio::test]
    async fn test_bls_operator_ok() -> Result<()> {
        let mut client = create_test_client().await?;

        let operator_count = 3;

        let mut ncn_root = create_test_ncn(&mut client).await?;
        add_operators_to_test_ncn(&mut client, &mut ncn_root, operator_count, None).await?;
        client.test_warp_to_slot_incremental(1_000_000).await?;

        let mut bls_ncn_root = BlsNcnRoot {
            test_ncn: ncn_root.clone(),
            operator_bls_keypairs: Vec::new(),
        };

        initialize_config(&client, &ncn_root.ncn_root.ncn_pubkey).await?;
        initialize_rolling_snapshot(&mut client, &ncn_root.ncn_root.ncn_pubkey).await?;

        for operator_root in ncn_root.operators {
            let bls_keypair = SolanaBN254Keypair::new_unique()
                .map_err(|e| anyhow!("Could not create new bls keypair: {}", e))?;
            bls_ncn_root.operator_bls_keypairs.push(bls_keypair);

            initialize_bls_operator(&client, &operator_root.operator_pubkey, &bls_keypair).await?;
            register_bls_operator(
                &client,
                &ncn_root.ncn_root.ncn_pubkey,
                &operator_root.operator_pubkey,
            )
            .await?;
            get_bls_operator(&client, &operator_root.operator_pubkey).await?;
        }

        Ok(())
    }
}
