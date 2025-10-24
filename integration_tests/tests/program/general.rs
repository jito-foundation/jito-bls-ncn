#[cfg(test)]
mod tests {
    use crate::fixtures::fixture::{create_test_client};
    use jito_bls_ncn_clients::jito_clients::JitoClient;
    use jito_bls_ncn_sdk::bls_ncn_sdk::id;
    use solana_program::pubkey::Pubkey;
    use solana_program_test::tokio;
    use anyhow::Result;

    #[tokio::test]
    async fn test_program_ok() -> Result<()> {
        let client = create_test_client().await?;
        let program_id: Pubkey = id();

        let account = client
            .get_account(&program_id)
            .await?;

        assert!(!account.data.is_empty());

        Ok(())
    }
}
