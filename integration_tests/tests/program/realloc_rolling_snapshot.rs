#[cfg(test)]
mod tests {
    use crate::fixtures::fixture::TestBuilder;
    use anyhow::Result;
    use jito_bls_ncn_sdk::realloc_rolling_snapshot_ix;
    use solana_program_test::tokio;
    use solana_signer::Signer;

    #[tokio::test]
    async fn realloc_rolling_snapshot_ok() -> Result<()> {
        let mut fixture = TestBuilder::new().await;

        let admin = fixture.context.payer.insecure_clone();
        let ix = realloc_rolling_snapshot_ix(&admin.pubkey());

        fixture.send_transaction(&[ix], None, &[&admin]).await?;

        Ok(())
    }
}
