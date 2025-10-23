#[cfg(test)]
mod tests {
    use crate::fixtures::fixture::{create_test_client};
    use anyhow::Result;
    use jito_bls_ncn_clients::{jito_clients::JitoClient, program_clients::bls_ncn_client::realloc_rolling_snapshot};
    use jito_bls_ncn_sdk::realloc_rolling_snapshot_ix;
    use solana_keypair::Keypair;
    use solana_program_test::tokio;
    use solana_signer::Signer;

    #[tokio::test]
    async fn realloc_rolling_snapshot_ok() -> Result<()> {
        let mut client = create_test_client().await?;
        let ncn = Keypair::new();

        realloc_rolling_snapshot(&mut client, &ncn.pubkey()).await?;

        Ok(())
    }
}
