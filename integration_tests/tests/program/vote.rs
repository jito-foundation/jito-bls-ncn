#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use jito_bls_ncn_clients::{jito_clients::JitoClient, program_clients::{
        bls_ncn_client::{initialize_bls_operator, initialize_config, initialize_rolling_snapshot, register_bls_operator, vote, BlsNcnRoot, BlsNcnSignatureRoot},
        meta_restaking_client::{add_operators_to_test_ncn, create_test_ncn},
    }};
    use jito_bls_ncn_core::bls::{solana_bls::{offchain_prepare_vote_data, solana_sign}, solana_bls_interface::{SolanaBN254G1, SolanaBN254G2, SolanaBN254Keypair}};
    use solana_program_test::tokio;

    use crate::fixtures::fixture::create_test_client;

    #[tokio::test]
    async fn test_vote_ok() -> Result<()> {
        let mut client = create_test_client().await?;

        let operator_count = 3;

        let mut ncn_root = create_test_ncn(&mut client).await?;
        add_operators_to_test_ncn(&mut client, &mut ncn_root, operator_count, None).await?;
        client.test_warp_to_slot_incremental(1_000_000).await?;

        let mut bls_ncn_root = BlsNcnRoot {
            test_ncn: ncn_root.clone(),
            operator_bls_keypairs: Vec::new(),
        };

        initialize_config(&client, &ncn_root.ncn_root.ncn_pubkey).await?;
        initialize_rolling_snapshot(&mut client, &ncn_root.ncn_root.ncn_pubkey).await?;


        for operator_root in ncn_root.operators {
            let bls_keypair = SolanaBN254Keypair::new_unique()
                .map_err(|e| anyhow!("Could not create new bls keypair: {}", e))?;
            bls_ncn_root.operator_bls_keypairs.push(bls_keypair);

            initialize_bls_operator(&client, &operator_root.operator_pubkey, &bls_keypair).await?;
            register_bls_operator(&client, &ncn_root.ncn_root.ncn_pubkey, &operator_root.operator_pubkey).await?;
        }


        let test_message = SolanaBN254Keypair::new_unique().map_err(|e| anyhow!("Could not make keypair: {}", e))?;
        let mut bls_ncn_signature_root = BlsNcnSignatureRoot {
            operator_g2_signed: Vec::new(),
            operator_signatures: Vec::new(),
            indexs: Vec::new(),
            message: test_message.private_key.to_vec(),
        };

        for (index, bls_operator) in bls_ncn_root.operator_bls_keypairs.iter().enumerate() {
            let signature = solana_sign(&bls_operator.private_key, &bls_ncn_signature_root.message, None).map_err(|e| anyhow!("Could not sign: {}", e))?;
            let g1_signature = SolanaBN254G1::new(&signature).map_err(|e| anyhow!("Could not make keypair: {}", e))?;
            bls_ncn_signature_root.operator_signatures.push(g1_signature.raw);

            bls_ncn_signature_root.indexs.push(index);

            bls_ncn_signature_root.operator_g2_signed.push(bls_operator.public_key.g2.raw);
        }

        let (aggregated_signature, aggregated_g2, bitmap) = offchain_prepare_vote_data(
            &bls_ncn_signature_root.operator_signatures,
            &bls_ncn_signature_root.operator_g2_signed,
            &bls_ncn_signature_root.indexs,
            operator_count,
        ).map_err(|e| anyhow!("Could not make keypair: {}", e))?;

        let aggregated_g1_signature = SolanaBN254G1::new(&aggregated_signature).map_err(|e| anyhow!("Could not make keypair: {}", e))?;;
        let aggregated_g2_signed = SolanaBN254G2::new(&aggregated_g2).map_err(|e| anyhow!("Could not make keypair: {}", e))?;;

        let message_array: [u8; 32] = bls_ncn_signature_root.message
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Message is not 32 bytes"))?;

        vote(
            &mut client,
            &ncn_root.ncn_root.ncn_pubkey,
            &aggregated_g1_signature,
            &aggregated_g2_signed,
            &bitmap,
            &message_array
        ).await?;

        Ok(())
    }
}
