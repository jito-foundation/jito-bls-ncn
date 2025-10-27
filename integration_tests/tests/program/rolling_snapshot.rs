#[cfg(test)]
mod tests {
    use crate::fixtures::fixture::create_test_client;
    use anyhow::Result;
    use jito_bls_ncn_clients::program_clients::bls_ncn_client::{get_rolling_snapshot, initialize_rolling_snapshot};
    use solana_keypair::Keypair;
    use solana_program_test::tokio;
    use solana_signer::Signer;

    #[tokio::test]
    async fn initialize_rolling_snapshot_ok() -> Result<()> {
        let mut client = create_test_client().await?;
        let ncn = Keypair::new();

        initialize_rolling_snapshot(&mut client, &ncn.pubkey()).await?;
        get_rolling_snapshot(&client, &ncn.pubkey()).await?;

        Ok(())
    }
}
