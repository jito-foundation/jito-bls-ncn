#[cfg(test)]
mod tests {
    use anyhow::Result;
    use jito_bls_ncn_sdk::id;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_program::pubkey::Pubkey;
    use solana_program_test::tokio;

    #[tokio::test]
    async fn test_program_ok() -> Result<()> {
        let program_id: Pubkey = id();
        let rpc_client = RpcClient::new("http://localhost:8899".to_string());

        let account = rpc_client.get_account(&program_id).await?;

        assert!(!account.data.is_empty());
        Ok(())
    }
}
