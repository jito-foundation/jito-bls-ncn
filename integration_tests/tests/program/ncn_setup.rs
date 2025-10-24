#[cfg(test)]
mod tests {
    use anyhow::Result;
    use jito_bls_ncn_clients::{
        jito_clients::JitoClient,
        program_clients::meta_restaking_client::{
            add_delegation_in_test_ncn, add_operators_to_test_ncn, add_vaults_to_test_ncn,
            create_test_ncn, update_all_vaults_in_test_ncn,
        },
    };
    use solana_program_test::tokio;

    use crate::fixtures::fixture::create_test_client;

    #[tokio::test]
    async fn test_can_setup_ncn() -> Result<()> {
        let mut client = create_test_client().await?;

        let operator_count = 3;
        let vault_count = 2;

        let mut ncn_root = create_test_ncn(&mut client).await?;
        add_operators_to_test_ncn(&mut client, &mut ncn_root, operator_count, None).await?;
        add_vaults_to_test_ncn(&mut client, &mut ncn_root, vault_count, None).await?;
        add_delegation_in_test_ncn(&mut client, &ncn_root, 1000).await?;

        client.test_warp_to_slot_incremental(1_000_000).await?;
        update_all_vaults_in_test_ncn(&mut client, &ncn_root).await?;

        Ok(())
    }
}
