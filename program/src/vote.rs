use jito_bls_ncn_core::{accounts::rolling_snapshot::RollingSnapshot, bls::solana_bls::{add_g1, did_sign_bitmap, solana_verify_aggregated_signature, sub_g1}, instructions::VoteIxData, utils::{load_account, load_ix_data, JitoAccount}};
// use jito_bls_ncn_core::{
//     instructions::{ReallocRollingSnapshotIxData, VoteIxData},
//     utils::load_ix_data,
// };
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::log::sol_log_compute_units;
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

pub fn process_vote(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [rolling_snapshot] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<VoteIxData>(data)? };

    RollingSnapshot::check(program_id, rolling_snapshot, true)?;
    let rolling_snapshot_data = rolling_snapshot.try_borrow_data()?;
    let rolling_snapshot_account =
        unsafe { load_account::<RollingSnapshot>(&rolling_snapshot_data)? };


    let mut aggregated_g1_nonsigners_pubkey: Option<[u8; 64]> = None;
    let mut non_signers_count: u64 = 0;

    for (index, operator) in rolling_snapshot_account.operators.iter().enumerate() {
        if let Some(operator) = operator.as_ref() {
            let did_sign = did_sign_bitmap(ix_data.operators_bitmap_signed, index).map_err(|e| ProgramError::InsufficientFunds)?;
            msg!("Operator {}", index);
            if !did_sign {
                non_signers_count = non_signers_count.saturating_add(1);
                if let Some(nonsigners) = aggregated_g1_nonsigners_pubkey {
                    aggregated_g1_nonsigners_pubkey = Some(add_g1(&nonsigners, &operator.g1).map_err(|e| ProgramError::InsufficientFunds)?)
                } else {
                    aggregated_g1_nonsigners_pubkey = Some(operator.g1);
                }
            } else {
                //TODO check stake
            }
        } else {
            break;
        }
    }

    if non_signers_count > rolling_snapshot_account.operator_count() as u64 / 3 {
        msg!(
            "Quorum not met: non-signers count ({}) exceeds 1/3 of registered operators ({})",
            non_signers_count,
            rolling_snapshot_account.operator_count()
        );
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let signed_g1 = if let Some(nonsigners) = aggregated_g1_nonsigners_pubkey {
        sub_g1(&rolling_snapshot_account.aggregate_g1, &nonsigners).map_err(|e| ProgramError::InsufficientFunds)?
    } else {
        rolling_snapshot_account.aggregate_g1
    };

    let verified = solana_verify_aggregated_signature(
        &signed_g1,
        &ix_data.aggregated_g2_signed,
        &ix_data.aggregated_g1_signature,
        &ix_data.message,
        None
    ).map_err(|e| ProgramError::InsufficientFunds)?;

    if !verified {
        msg!("Invalid signature");
        return Err(ProgramError::InvalidInstructionData);
    }

    msg!("---------------- CU -----------------");
    sol_log_compute_units();
    msg!("-------------------------------------");

    // return Err(ProgramError::InvalidInstructionData);

    Ok(())
}

// /// Allows an operator to cast a vote on weather status.
// ///
// /// ### Parameters:
// /// - `aggregated_g2`: Aggregated G2 public key in compressed format (64 bytes)
// /// - `aggregated_signature`: Aggregated G1 signature in compressed format (32 bytes)
// /// - `operators_signature_bitmap`: Bitmap indicating which operators signed the vote
// ///
// /// Note: The message used for signature verification is the current vote counter count
// ///
// /// ### Accounts:
// /// 1. `[]` config: NCN configuration account (named `ncn_config` in code)
// /// 2. `[]` ncn: The NCN account
// /// 3. `[]` snapshot: Snapshot containing stakes and operator snapshots
// /// 4. `[]` restaking_config: Restaking configuration account
// /// 5. `[writable]` vote_counter: Vote counter PDA to increment on successful vote
// pub fn process_cast_vote_v2(
//     program_id: &Pubkey,
//     accounts: &[AccountInfo],
//     aggregated_signature: [u8; 64],
//     aggregated_g2: [u8; 128],
//     operators_signature_bitmap: [u8; 32],
//     message: [u8; 32],
// ) -> ProgramResult {
//     let account_info_iter = &mut accounts.iter();
//     let ncn_config = next_account_info(account_info_iter)?;
//     let ncn = next_account_info(account_info_iter)?;
//     let snapshot = next_account_info(account_info_iter)?;
//     let restaking_config = next_account_info(account_info_iter)?;
//     let vote_counter = next_account_info(account_info_iter)?;

//     NcnConfig::load(program_id, ncn_config, ncn.key, false)?;
//     Config::load(&jito_restaking_program::id(), restaking_config, false)?;
//     Ncn::load(&jito_restaking_program::id(), ncn, false)?;
//     Snapshot::load(program_id, snapshot, ncn.key, false)?;
//     VoteCounter::load(program_id, vote_counter, ncn.key, true)?;

//     // // Get the current counter value to use as the message for signature verification
//     // let vote_counter_data = vote_counter.data.borrow();
//     // let vote_counter_account = VoteCounter::try_from_slice_unchecked(&vote_counter_data)?;
//     // let current_count = vote_counter_account.count();
//     // let message = current_count.to_le_bytes();
//     // Pad to 32 bytes for signature verification
//     // let mut message_32 = [0u8; 32];
//     // message_32[..8].copy_from_slice(&message);
//     // drop(vote_counter_data);

//     let ncn_epoch_length = {
//         let config_data = restaking_config.data.borrow();
//         let config = Config::try_from_slice_unchecked(&config_data)?;
//         config.epoch_length()
//     };

//     let current_slot = Clock::get()?.slot;

//     let snapshot_data = snapshot.data.borrow();
//     let snapshot = Snapshot::try_from_slice_unchecked(&snapshot_data)?;

//     let operators_registered = snapshot.operators_registered();

//     msg!("Total operators: {}", operators_registered);

//     let slot = Clock::get()?.slot;
//     msg!("Current slot: {}", slot);

//     // Check bitmap size
//     let required_bitmap_bytes = (operators_registered
//         .checked_add(7)
//         .ok_or(ProgramError::ArithmeticOverflow)?)
//         / 8;
//     if operators_signature_bitmap.len() as u64 != required_bitmap_bytes {
//         msg!("Invalid bitmap size");
//         return Err(NCNProgramError::InvalidInputLength.into());
//     }


//     let mut aggregated_nonsigners_pubkey: Option<SolanaBN254G1> = None;
//     let mut non_signers_count: u64 = 0;

//     for (i, operator_snapshot) in snapshot.operator_snapshots().iter().enumerate() {
//         if i >= operators_registered as usize {
//             break;
//         }

//         let byte_index = i / 8;
//         let bit_index = i % 8;
//         let signed = (operators_signature_bitmap[byte_index] >> bit_index) & 1 == 1;

//         if signed {
//             let snapshot_epoch =
//                 get_epoch(operator_snapshot.last_snapshot_slot(), ncn_epoch_length)?;
//             let current_epoch = get_epoch(current_slot, ncn_epoch_length)?;
//             let has_minimum_stake =
//                 operator_snapshot.has_minimum_stake_now(current_epoch, snapshot_epoch)?;
//             if !has_minimum_stake {
//                 msg!(
//                     "The operator {} does not have enough stake to vote",
//                     operator_snapshot.operator()
//                 );
//                 return Err(NCNProgramError::OperatorHasNoMinimumStake.into());
//             }
//         } else {
//             // // Convert bytes to G1Point
//             // let g1_compressed = G1CompressedPoint::from(operator_snapshot.g1_pubkey());
//             // let g1_point = G1Point::try_from(&g1_compressed)
//             //     .map_err(|_| NCNProgramError::G1PointDecompressionError)?;
//             let solana_g1 = SolanaBN254G1::from_compressed(&operator_snapshot.g1_pubkey()).expect("Could not decompress");

//             if aggregated_nonsigners_pubkey.is_none() {
//                 aggregated_nonsigners_pubkey = Some(solana_g1);
//             } else {
//                 // Add this G1 pubkey to the aggregate using G1Point addition
//                 let current = aggregated_nonsigners_pubkey.unwrap();
//                 let sum = add_g1(&current.raw, &solana_g1.raw).expect("Could not add");
//                 aggregated_nonsigners_pubkey = Some(
//                     SolanaBN254G1::new(&sum).expect("Could not cast")
//                 );
//             }

//             non_signers_count = non_signers_count
//                 .checked_add(1)
//                 .ok_or(ProgramError::ArithmeticOverflow)?
//         }

//     }

//     // If non_signers_count is more than 1/3 of registered operators, throw an error because quorum didn't meet
//     if non_signers_count > operators_registered / 3 {
//         msg!(
//             "Quorum not met: non-signers count ({}) exceeds 1/3 of registered operators ({})",
//             non_signers_count,
//             operators_registered
//         );
//         return Err(NCNProgramError::QuorumNotMet.into());
//     }

//     let total_aggregate_g1_pubkey_compressed =
//         G1CompressedPoint::from(snapshot.total_aggregated_g1_pubkey());
//     let total_aggregated_g1_pubkey = G1Point::try_from(&total_aggregate_g1_pubkey_compressed)
//         .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

//     let verified = solana_verify_aggregated_signature(
//         &total_aggregated_g1_pubkey.0,
//         &aggregated_g2,
//         &aggregated_signature,
//         &message,
//         None
//     ).expect("msg");

//     if !verified {
//         msg!("Not Verified");
//         return Err(NCNProgramError::BLSSigningError.into());
//     }

//     // Increment the vote counter PDA after successful signature verification
//     // NOTE: This counter could track anything
//     let mut vote_counter_data = vote_counter.try_borrow_mut_data()?;
//     let vote_counter_account = VoteCounter::try_from_slice_unchecked_mut(&mut vote_counter_data)?;

//     let previous_count = vote_counter_account.count();
//     vote_counter_account.increment()?;
//     let new_count = vote_counter_account.count();

//     msg!(
//         "Vote successfully cast! Counter incremented from {} to {}",
//         previous_count,
//         new_count
//     );

//     // // If there are no non-signers, we should verify the aggregate signature with the total G1
//     // // pubkey because adding to the initial non-signers pubkey would result in error since it is
//     // // initialized to all zeros and this is not a valid point of the curve BN128
//     // if non_signers_count == 0 {
//     //     msg!("All operators signed, verifying aggregate signature with total G1 pubkey");
//     //     aggregated_g2_point
//     //         .verify_aggregated_signature::<Sha256Normalized, &[u8], G1Point>(
//     //             signature,
//     //             &message_32,
//     //             total_aggregated_g1_pubkey,
//     //         )
//     //         .map_err(|_| NCNProgramError::SignatureVerificationFailed)?;
//     // } else {
//     //     msg!("Total non signers: {}", non_signers_count);
//     //     let aggregated_nonsigners_pubkey =
//     //         aggregated_nonsigners_pubkey.ok_or(NCNProgramError::NoNonSignersAggregatedPubkey)?;

//     //     let apk1 = total_aggregated_g1_pubkey
//     //         .checked_add(&aggregated_nonsigners_pubkey.negate())
//     //         .ok_or(NCNProgramError::AltBN128AddError)?;

//     //     msg!("Aggregated non-signers G1 pubkey {:?}", apk1.0);
//     //     msg!("Aggregated G2 pubkey {:?}", aggregated_g2_point.0);

//     //     // One Pairing attempt
//     //     msg!("Verifying aggregate signature one pairing");
//     //     aggregated_g2_point
//     //         .verify_aggregated_signature::<Sha256Normalized, &[u8], G1Point>(
//     //             signature,
//     //             &message_32,
//     //             apk1,
//     //         )
//     //         .map_err(|_| NCNProgramError::SignatureVerificationFailed)?;
//     // }


//     // // Convert aggregated_g2 pubkey to G2Point
//     // let aggregated_g2_compressed_point = G2CompressedPoint::from(aggregated_g2);
//     // let aggregated_g2_point = G2Point::try_from(aggregated_g2_compressed_point)
//     //     .map_err(|_| NCNProgramError::G2PointDecompressionError)?;

//     // // Aggregate the G1 public keys of operators who signed
//     // let mut aggregated_nonsigners_pubkey: Option<G1Point> = None;
//     // let mut non_signers_count: u64 = 0;

//     // for (i, operator_snapshot) in snapshot.operator_snapshots().iter().enumerate() {
//     //     if i >= operators_registered as usize {
//     //         break;
//     //     }

//     //     let byte_index = i / 8;
//     //     let bit_index = i % 8;
//     //     let signed = (operators_signature_bitmap[byte_index] >> bit_index) & 1 == 1;

//     //     if signed {
//     //         let snapshot_epoch =
//     //             get_epoch(operator_snapshot.last_snapshot_slot(), ncn_epoch_length)?;
//     //         let current_epoch = get_epoch(current_slot, ncn_epoch_length)?;
//     //         let has_minimum_stake =
//     //             operator_snapshot.has_minimum_stake_now(current_epoch, snapshot_epoch)?;
//     //         if !has_minimum_stake {
//     //             msg!(
//     //                 "The operator {} does not have enough stake to vote",
//     //                 operator_snapshot.operator()
//     //             );
//     //             return Err(NCNProgramError::OperatorHasNoMinimumStake.into());
//     //         }
//     //     } else {
//     //         // Convert bytes to G1Point
//     //         let g1_compressed = G1CompressedPoint::from(operator_snapshot.g1_pubkey());
//     //         let g1_point = G1Point::try_from(&g1_compressed)
//     //             .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

//     //         if aggregated_nonsigners_pubkey.is_none() {
//     //             aggregated_nonsigners_pubkey = Some(g1_point);
//     //         } else {
//     //             // Add this G1 pubkey to the aggregate using G1Point addition
//     //             let current = aggregated_nonsigners_pubkey.unwrap();
//     //             aggregated_nonsigners_pubkey = Some(
//     //                 current
//     //                     .checked_add(&g1_point)
//     //                     .ok_or(NCNProgramError::AltBN128AddError)?,
//     //             );
//     //         }

//     //         non_signers_count = non_signers_count
//     //             .checked_add(1)
//     //             .ok_or(ProgramError::ArithmeticOverflow)?
//     //     }
//     // }

//     // // If non_signers_count is more than 1/3 of registered operators, throw an error because quorum didn't meet
//     // if non_signers_count > operators_registered / 3 {
//     //     msg!(
//     //         "Quorum not met: non-signers count ({}) exceeds 1/3 of registered operators ({})",
//     //         non_signers_count,
//     //         operators_registered
//     //     );
//     //     return Err(NCNProgramError::QuorumNotMet.into());
//     // }

//     // let total_aggregate_g1_pubkey_compressed =
//     //     G1CompressedPoint::from(snapshot.total_aggregated_g1_pubkey());
//     // let total_aggregated_g1_pubkey = G1Point::try_from(&total_aggregate_g1_pubkey_compressed)
//     //     .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

//     // let signature_compressed = G1CompressedPoint(aggregated_signature);
//     // let signature = G1Point::try_from(&signature_compressed)
//     //     .map_err(|_| NCNProgramError::G1PointDecompressionError)?;

//     // // If there are no non-signers, we should verify the aggregate signature with the total G1
//     // // pubkey because adding to the initial non-signers pubkey would result in error since it is
//     // // initialized to all zeros and this is not a valid point of the curve BN128
//     // if non_signers_count == 0 {
//     //     msg!("All operators signed, verifying aggregate signature with total G1 pubkey");
//     //     aggregated_g2_point
//     //         .verify_aggregated_signature::<Sha256Normalized, &[u8], G1Point>(
//     //             signature,
//     //             &message_32,
//     //             total_aggregated_g1_pubkey,
//     //         )
//     //         .map_err(|_| NCNProgramError::SignatureVerificationFailed)?;
//     // } else {
//     //     msg!("Total non signers: {}", non_signers_count);
//     //     let aggregated_nonsigners_pubkey =
//     //         aggregated_nonsigners_pubkey.ok_or(NCNProgramError::NoNonSignersAggregatedPubkey)?;

//     //     let apk1 = total_aggregated_g1_pubkey
//     //         .checked_add(&aggregated_nonsigners_pubkey.negate())
//     //         .ok_or(NCNProgramError::AltBN128AddError)?;

//     //     msg!("Aggregated non-signers G1 pubkey {:?}", apk1.0);
//     //     msg!("Aggregated G2 pubkey {:?}", aggregated_g2_point.0);

//     //     // One Pairing attempt
//     //     msg!("Verifying aggregate signature one pairing");
//     //     aggregated_g2_point
//     //         .verify_aggregated_signature::<Sha256Normalized, &[u8], G1Point>(
//     //             signature,
//     //             &message_32,
//     //             apk1,
//     //         )
//     //         .map_err(|_| NCNProgramError::SignatureVerificationFailed)?;
//     // }

//     // // Increment the vote counter PDA after successful signature verification
//     // // NOTE: This counter could track anything
//     // let mut vote_counter_data = vote_counter.try_borrow_mut_data()?;
//     // let vote_counter_account = VoteCounter::try_from_slice_unchecked_mut(&mut vote_counter_data)?;

//     // let previous_count = vote_counter_account.count();
//     // vote_counter_account.increment()?;
//     // let new_count = vote_counter_account.count();

//     // msg!(
//     //     "Vote successfully cast! Counter incremented from {} to {}",
//     //     previous_count,
//     //     new_count
//     // );

//     Ok(())
// }
