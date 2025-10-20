#[cfg(test)]
mod tests {
    // use crate::fixtures::fixture::TestBuilder;
    use anyhow::Result;
    use jito_bls_ncn_sdk::id;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_program::pubkey::Pubkey;
    use solana_program_test::tokio;

    // #[tokio::test]
    // async fn test_program_ok() -> Result<()> {
    //     // let fixture = TestBuilder::new().await;
    //     let client = RpcClient::new("http://127.0.0.1:8899".to_string());
    //     let program_id: Pubkey = id();

    //     let account = client.get_account(&program_id).await?;

    //     assert!(!account.data.is_empty());

    //     Ok(())
    // }
}
