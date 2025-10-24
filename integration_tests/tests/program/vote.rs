#[cfg(test)]
mod tests {
    use anyhow::Result;
    use jito_bls_ncn_clients::program_clients::bls_ncn_client::vote;
    use solana_keypair::Keypair;
    use solana_program_test::tokio;
    use solana_signer::Signer;

    use crate::fixtures::fixture::create_test_client;

    #[tokio::test]
    async fn vote_ok() -> Result<()> {
        let mut client = create_test_client().await?;
        let ncn = Keypair::new();

        vote(&mut client, &ncn.pubkey()).await?;

        Ok(())
    }
}
