#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use jito_bls_ncn_clients::{
        jito_clients::JitoClientTrait,
        program_clients::{
            bls_ncn_client::{
                get_consensus, initialize_bls_operator, initialize_config, initialize_consensus,
                initialize_rolling_snapshot, register_bls_operator, vote, BlsNcnRoot,
                BlsNcnSignatureRoot,
            },
            meta_restaking_client::{add_operators_to_test_ncn, create_test_ncn},
        },
    };
    use jito_bls_ncn_core::bls::{
        solana_bls::{offchain_prepare_vote_data, solana_sign},
        solana_bls_interface::{SolanaBN254G1, SolanaBN254G2, SolanaBN254Keypair},
    };
    use solana_program_test::tokio;
    use solana_pubkey::Pubkey;

    use crate::fixtures::fixture::create_test_client;

    /// Setup test environment with operators and return all necessary data for voting
    async fn setup_vote_test<T: JitoClientTrait>(
        client: &mut T,
        operator_count: usize,
    ) -> Result<(Pubkey, BlsNcnRoot)> {
        let mut ncn_root = create_test_ncn(client).await?;
        let ncn = ncn_root.ncn_root.ncn_pubkey;

        add_operators_to_test_ncn(client, &mut ncn_root, operator_count, None).await?;
        client.test_warp_to_slot_incremental(1_000_000).await?;

        let mut bls_ncn_root = BlsNcnRoot {
            test_ncn: ncn_root.clone(),
            operator_bls_keypairs: Vec::new(),
        };

        initialize_config(client, &ncn).await?;
        initialize_consensus(client, &ncn).await?;
        initialize_rolling_snapshot(client, &ncn).await?;

        for operator_root in ncn_root.operators {
            let bls_keypair = SolanaBN254Keypair::new_unique()
                .map_err(|e| anyhow!("Could not create new bls keypair: {}", e))?;
            bls_ncn_root.operator_bls_keypairs.push(bls_keypair);

            initialize_bls_operator(client, &operator_root.operator_pubkey, &bls_keypair).await?;
            register_bls_operator(client, &ncn, &operator_root.operator_pubkey).await?;
        }

        Ok((ncn, bls_ncn_root))
    }

    /// Setup ballot data with optional message and consensus_count overrides
    async fn setup_ballot<T: JitoClientTrait>(
        client: &T,
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

        let (ncn, bls_ncn_root) = setup_vote_test(&mut client, operator_count).await?;
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
        let indexes_to_skip = vec![0];

        let (ncn, bls_ncn_root) = setup_vote_test(&mut client, operator_count).await?;
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
        let indexes_to_skip = vec![0, 1];

        let (ncn, bls_ncn_root) = setup_vote_test(&mut client, operator_count).await?;
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
        let indexes_to_skip = vec![];

        let (ncn, bls_ncn_root) = setup_vote_test(&mut client, operator_count).await?;
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
        let indexes_to_skip = vec![1];

        let (ncn, bls_ncn_root) = setup_vote_test(&mut client, operator_count).await?;

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

    #[tokio::test]
    async fn test_max_voters() -> Result<()> {
        let mut client = create_test_client().await?;
        let operator_count = u8::MAX;
        let indexes_to_skip = vec![];

        let (ncn, bls_ncn_root) = setup_vote_test(&mut client, operator_count as usize).await?;
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

        // TEST MAX CU
        // Within CU, we can get through 151 non-signers. 256 - 105 = 151 ( not signing )
        let one_third = operator_count / 3;
        let indexes_to_skip: Vec<usize> = (0..one_third as usize).collect();
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

        // FAIL THRESHOLD
        let one_third = operator_count / 3 + 1;
        let indexes_to_skip: Vec<usize> = (0..one_third as usize).collect();
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
}
