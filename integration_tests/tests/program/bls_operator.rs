#[cfg(test)]
mod tests {
    use jito_bls_ncn_clients::{program_clients::{bls_ncn_client::{initialize_bls_operator, BlsNcnRoot}, meta_restaking_client::{add_delegation_in_test_ncn, add_operators_to_test_ncn, add_vaults_to_test_ncn, create_test_ncn, update_all_vaults_in_test_ncn}}};
    use jito_bls_ncn_core::bls::solana_bls_interface::SolanaBN254Keypair;
    use solana_program_test::tokio;
    use anyhow::{Result, anyhow};

    use crate::fixtures::fixture::create_test_client;

    #[tokio::test]
    async fn test_bls_operator_ok() -> Result<()> {
        let mut client = create_test_client().await?;

        let operator_count = 3;

        let mut ncn_root = create_test_ncn(&mut client).await?;
        add_operators_to_test_ncn(&mut client, &mut ncn_root, operator_count, None).await?;

        let mut bls_ncn_root = BlsNcnRoot {
            test_ncn: ncn_root.clone(),
            operator_keypairs: Vec::new(),
        };

        for operator_root in ncn_root.operators {

            let bls_keypair = SolanaBN254Keypair::new_unique().map_err(|e| anyhow!("Could not create new bls keypair: {}", e))?;
            bls_ncn_root.operator_keypairs.push(bls_keypair);

            initialize_bls_operator(&client, &operator_root.operator_pubkey, &bls_keypair).await?;
        }

        Ok(())
    }
}
