#[cfg(test)]
mod tests {
    use anyhow::Result;
    use jito_bls_ncn_sdk::{vote_ix};
    use solana_program_test::tokio;
    use solana_signer::Signer;
    use crate::fixtures::fixture::TestBuilder;

    #[tokio::test]
    async fn test_vote_ok() -> Result<()> {
        let mut fixture = TestBuilder::new().await;

        let admin = fixture.context.payer.insecure_clone();
        let ix = vote_ix(&admin.pubkey());

        fixture.send_transaction(&[ix], None, &[&admin]).await?;

        Ok(())
    }
}
