#[cfg(test)]
mod tests {
    use jito_bls_ncn_sdk::id;
    use solana_program::pubkey::Pubkey;
    use solana_program_test::tokio;
    use crate::fixtures::fixture::TestBuilder;

    #[tokio::test]
    async fn test_program_ok() {
        let fixture = TestBuilder::new().await;
        let program_id: Pubkey = id();

        let account = fixture
            .context
            .banks_client
            .get_account(program_id)
            .await
            .expect("Could not get program");

        assert!(account.is_some());
        assert!(account.unwrap().data.len() > 0);
    }

}
