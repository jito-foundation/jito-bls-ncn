#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use jito_bls_ncn_clients::{
        jito_clients::JitoClient,
        program_clients::{
            bls_ncn_client::{get_consensus, vote, BlsNcnRoot, BlsNcnSignatureRoot},
            meta_bls_ncn_client::setup_test_bls_ncn,
        },
    };
    use jito_bls_ncn_core::bls::{
        solana_bls::{offchain_prepare_vote_data, solana_sign},
        solana_bls_interface::{SolanaBN254G1, SolanaBN254G2, SolanaBN254Keypair},
    };
    use solana_program_test::tokio;
    use solana_pubkey::Pubkey;

    use crate::fixtures::fixture::create_test_client;

    /// Setup ballot data with optional message and consensus_count overrides
    async fn setup_ballot(
        client: &JitoClient,
        ncn: &Pubkey,
        message: Option<[u8; 32]>,
        consensus_count: Option<u64>,
    ) -> Result<([u8; 32], u64)> {
        let consensus_count = match consensus_count {
            Some(count) => count,
            None => {
                let consensus_account = get_consensus(client, ncn).await?;
                consensus_account.consensus_count()
            }
        };

        let raw_message = match message {
            Some(msg) => msg,
            None => {
                let test_message = SolanaBN254Keypair::new_unique()
                    .map_err(|e| anyhow!("Could not make keypair: {}", e))?;
                test_message
                    .private_key
                    .as_slice()
                    .try_into()
                    .map_err(|_| anyhow!("Message is not 32 bytes"))?
            }
        };

        Ok((raw_message, consensus_count))
    }

    /// Prepare vote data by collecting signatures from operators (skipping specified indexes)
    fn prepare_vote_signatures(
        bls_ncn_root: &BlsNcnRoot,
        message: &[u8; 32],
        consensus_count: u64,
        indexes_to_skip: &[usize],
    ) -> Result<(SolanaBN254G1, SolanaBN254G2, [u8; 32])> {
        let mut bls_ncn_signature_root = BlsNcnSignatureRoot {
            operator_g2_signed: Vec::new(),
            operator_signatures: Vec::new(),
            indexs: Vec::new(),
            message: message.to_vec(),
        };

        for (index, bls_operator) in bls_ncn_root.operator_bls_keypairs.iter().enumerate() {
            if indexes_to_skip.contains(&index) {
                continue;
            }

            let signature = solana_sign(
                &bls_operator.private_key,
                &bls_ncn_signature_root.message,
                consensus_count,
            )
            .map_err(|e| anyhow!("Could not sign: {}", e))?;

            let g1_signature = SolanaBN254G1::new(&signature)
                .map_err(|e| anyhow!("Could not make G1 signature: {}", e))?;
            bls_ncn_signature_root
                .operator_signatures
                .push(g1_signature.raw);

            bls_ncn_signature_root.indexs.push(index);
            bls_ncn_signature_root
                .operator_g2_signed
                .push(bls_operator.public_key.g2.raw);
        }

        let operator_count = bls_ncn_root.operator_bls_keypairs.len();

        let (aggregated_signature, aggregated_g2, bitmap) = offchain_prepare_vote_data(
            &bls_ncn_signature_root.operator_signatures,
            &bls_ncn_signature_root.operator_g2_signed,
            &bls_ncn_signature_root.indexs,
            operator_count,
        )
        .map_err(|e| anyhow!("Could not prepare vote data: {}", e))?;

        let aggregated_g1_signature = SolanaBN254G1::new(&aggregated_signature)
            .map_err(|e| anyhow!("Could not make aggregated G1: {}", e))?;
        let aggregated_g2_signed = SolanaBN254G2::new(&aggregated_g2)
            .map_err(|e| anyhow!("Could not make aggregated G2: {}", e))?;

        Ok((aggregated_g1_signature, aggregated_g2_signed, bitmap))
    }

    #[tokio::test]
    async fn test_vote_ok() -> Result<()> {
        let mut client = create_test_client().await?;
        let operator_count = 3;
        let vault_count = 1;

        let (ncn, bls_ncn_root) =
            setup_test_bls_ncn(&mut client, operator_count, vault_count, vec![1000]).await?;
        let (raw_message, consensus_count) = setup_ballot(&client, &ncn, None, None).await?;

        let (aggregated_g1_signature, aggregated_g2_signed, bitmap) =
            prepare_vote_signatures(&bls_ncn_root, &raw_message, consensus_count, &[])?;

        vote(
            &mut client,
            &ncn,
            &aggregated_g1_signature,
            &aggregated_g2_signed,
            &bitmap,
            &raw_message,
            consensus_count,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_vote_ok_less_one_signer() -> Result<()> {
        let mut client = create_test_client().await?;
        let operator_count = 3;
        let vault_count = 1;
        let indexes_to_skip = vec![0];

        let (ncn, bls_ncn_root) =
            setup_test_bls_ncn(&mut client, operator_count, vault_count, vec![1000]).await?;
        let (raw_message, consensus_count) = setup_ballot(&client, &ncn, None, None).await?;

        let (aggregated_g1_signature, aggregated_g2_signed, bitmap) = prepare_vote_signatures(
            &bls_ncn_root,
            &raw_message,
            consensus_count,
            &indexes_to_skip,
        )?;

        vote(
            &mut client,
            &ncn,
            &aggregated_g1_signature,
            &aggregated_g2_signed,
            &bitmap,
            &raw_message,
            consensus_count,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_vote_consensus_not_reached() -> Result<()> {
        let mut client = create_test_client().await?;
        let operator_count = 3;
        let vault_count = 1;
        let indexes_to_skip = vec![0, 1];

        let (ncn, bls_ncn_root) =
            setup_test_bls_ncn(&mut client, operator_count, vault_count, vec![1000]).await?;
        let (raw_message, consensus_count) = setup_ballot(&client, &ncn, None, None).await?;

        let (aggregated_g1_signature, aggregated_g2_signed, bitmap) = prepare_vote_signatures(
            &bls_ncn_root,
            &raw_message,
            consensus_count,
            &indexes_to_skip,
        )?;

        let result = vote(
            &mut client,
            &ncn,
            &aggregated_g1_signature,
            &aggregated_g2_signed,
            &bitmap,
            &raw_message,
            consensus_count,
        )
        .await;

        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_vote_wrong_consensus_count() -> Result<()> {
        let mut client = create_test_client().await?;
        let operator_count = 3;
        let vault_count = 1;
        let indexes_to_skip = vec![];

        let (ncn, bls_ncn_root) =
            setup_test_bls_ncn(&mut client, operator_count, vault_count, vec![1000]).await?;
        let (raw_message, consensus_count) = setup_ballot(&client, &ncn, None, Some(10)).await?;

        let (aggregated_g1_signature, aggregated_g2_signed, bitmap) = prepare_vote_signatures(
            &bls_ncn_root,
            &raw_message,
            consensus_count,
            &indexes_to_skip,
        )?;

        let result = vote(
            &mut client,
            &ncn,
            &aggregated_g1_signature,
            &aggregated_g2_signed,
            &bitmap,
            &raw_message,
            consensus_count,
        )
        .await;

        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_votes() -> Result<()> {
        let mut client = create_test_client().await?;
        let operator_count = 5;
        let vault_count = 1;
        let indexes_to_skip = vec![1];

        let (ncn, bls_ncn_root) =
            setup_test_bls_ncn(&mut client, operator_count, vault_count, vec![1000]).await?;

        for _ in 0..5 {
            let (raw_message, consensus_count) = setup_ballot(&client, &ncn, None, None).await?;

            let (aggregated_g1_signature, aggregated_g2_signed, bitmap) = prepare_vote_signatures(
                &bls_ncn_root,
                &raw_message,
                consensus_count,
                &indexes_to_skip,
            )?;

            vote(
                &mut client,
                &ncn,
                &aggregated_g1_signature,
                &aggregated_g2_signed,
                &bitmap,
                &raw_message,
                consensus_count,
            )
            .await?;
        }

        Ok(())
    }

    // #[tokio::test]
    // async fn test_max_voters() -> Result<()> {
    //     let mut client = create_test_client().await?;
    //     let operator_count = u8::MAX;
    //     let vault_count = 1;
    //     let indexes_to_skip = vec![];

    //     let (ncn, bls_ncn_root) = setup_test_bls_ncn(&mut client, operator_count as usize, vault_count, vec![1000]).await?;
    //     let (raw_message, consensus_count) = setup_ballot(&client, &ncn, None, None).await?;

    //     let (aggregated_g1_signature, aggregated_g2_signed, bitmap) = prepare_vote_signatures(
    //         &bls_ncn_root,
    //         &raw_message,
    //         consensus_count,
    //         &indexes_to_skip,
    //     )?;

    //     vote(
    //         &mut client,
    //         &ncn,
    //         &aggregated_g1_signature,
    //         &aggregated_g2_signed,
    //         &bitmap,
    //         &raw_message,
    //         consensus_count,
    //     )
    //     .await?;

    //     // TEST MAX CU
    //     // 256 operators
    //     // Within CU, we can get through 151 non-signers. 256 - 105 = 151 ( not signing )
    //     let one_third = operator_count / 3;
    //     let indexes_to_skip: Vec<usize> = (0..one_third as usize).collect();
    //     let (raw_message, consensus_count) = setup_ballot(&client, &ncn, None, None).await?;
    //     let (aggregated_g1_signature, aggregated_g2_signed, bitmap) = prepare_vote_signatures(
    //         &bls_ncn_root,
    //         &raw_message,
    //         consensus_count,
    //         &indexes_to_skip,
    //     )?;

    //     vote(
    //         &mut client,
    //         &ncn,
    //         &aggregated_g1_signature,
    //         &aggregated_g2_signed,
    //         &bitmap,
    //         &raw_message,
    //         consensus_count,
    //     )
    //     .await?;

    //     // FAIL THRESHOLD
    //     let one_third = operator_count / 3 + 1;
    //     let indexes_to_skip: Vec<usize> = (0..one_third as usize).collect();
    //     let (raw_message, consensus_count) = setup_ballot(&client, &ncn, None, None).await?;
    //     let (aggregated_g1_signature, aggregated_g2_signed, bitmap) = prepare_vote_signatures(
    //         &bls_ncn_root,
    //         &raw_message,
    //         consensus_count,
    //         &indexes_to_skip,
    //     )?;

    //     let result = vote(
    //         &mut client,
    //         &ncn,
    //         &aggregated_g1_signature,
    //         &aggregated_g2_signed,
    //         &bitmap,
    //         &raw_message,
    //         consensus_count,
    //     )
    //     .await;

    //     assert!(result.is_err());

    //     Ok(())
    // }
}
