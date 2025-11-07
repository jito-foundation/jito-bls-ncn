#[cfg(test)]
mod tests {
    use anyhow::Result;
    use jito_bls_ncn_clients::program_clients::{
        bls_ncn_client::{get_config, initialize_config},
        meta_restaking_client::create_test_ncn,
    };
    use solana_program_test::tokio;

    use crate::fixtures::fixture::create_test_client;

    #[tokio::test]
    async fn test_bls_config_ok() -> Result<()> {
        let mut client = create_test_client().await?;
        let ncn_root = create_test_ncn(&mut client).await?;
        let ncn = ncn_root.ncn_root.ncn_pubkey;

        initialize_config(&client, &ncn).await?;
        get_config(&client, &ncn).await?;

        Ok(())
    }
}
