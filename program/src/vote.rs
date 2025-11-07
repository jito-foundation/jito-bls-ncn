use jito_bls_ncn_core::{
    accounts::{consensus::Consensus, rolling_snapshot::RollingSnapshot},
    bls::solana_bls::{add_g1, did_sign_bitmap, solana_verify_aggregated_signature, sub_g1},
    instructions::VoteIxData,
    programs::restaking_core::Ncn,
    utils::{load_account, load_account_mut_unchecked, load_ix_data, JitoAccount},
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program::{clock::Clock, sysvar::Sysvar};
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

use crate::errors::BlsNcnError;

pub fn process_vote(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [rolling_snapshot, consensus, restaking_config, ncn] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<VoteIxData>(data)? };

    let clock = Clock::get()?;
    let current_slot = clock.slot;

    let epoch_length: u64 = {
        jito_bls_ncn_core::programs::restaking_core::Config::check(
            &jito_bls_ncn_core::programs::restaking_core::id(),
            restaking_config,
            false,
        )?;
        let restaking_config_data = restaking_config.try_borrow_data()?;
        let restaking_config_account = unsafe {
            load_account::<jito_bls_ncn_core::programs::restaking_core::Config>(
                &restaking_config_data,
            )?
        };
        restaking_config_account.epoch_length.get()
    };

    Ncn::check(
        &jito_bls_ncn_core::programs::restaking_core::id(),
        ncn,
        false,
    )?;

    let consensus_count = {
        Consensus::check(program_id, consensus, true)?;
        let consensus_data = consensus.try_borrow_data()?;
        let consensus_account = unsafe { load_account::<Consensus>(&consensus_data)? };

        if consensus_account.ncn.ne(ncn.key) {
            msg!("NCN Mismatch");
            return Err(ProgramError::InvalidAccountData);
        }

        let consensus_count = consensus_account.consensus_count.get();
        if consensus_count.ne(&ix_data.consensus_count.get()) {
            msg!("Consensus Count Mismatch");
            return Err(ProgramError::InvalidAccountData);
        }

        consensus_count
    };

    RollingSnapshot::check(program_id, rolling_snapshot, false)?;
    let rolling_snapshot_data = rolling_snapshot.try_borrow_data()?;
    let rolling_snapshot_account =
        unsafe { load_account::<RollingSnapshot>(&rolling_snapshot_data)? };

    if rolling_snapshot_account.ncn.ne(ncn.key) {
        msg!("NCN Mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let mut aggregated_g1_nonsigners_pubkey: Option<[u8; 64]> = None;
    let mut signer_count: u16 = 0;
    let mut weight_tally: u128 = 0;

    for (index, operator) in rolling_snapshot_account.operators.iter().enumerate() {
        if let Some(operator) = operator.as_ref() {
            let did_sign = match did_sign_bitmap(ix_data.operators_bitmap_signed, index) {
                Ok(did_sign) => did_sign,
                Err(e) => {
                    msg!("Error checking signature bitmap: {}", e);
                    return Err(BlsNcnError::BitmapCheckFailed.into());
                }
            };
            let can_sign = operator.can_sign(
                current_slot,
                epoch_length,
                rolling_snapshot_account.consensus_weight_threshold,
            )?;

            if !did_sign || !can_sign {
                if let Some(nonsigners) = aggregated_g1_nonsigners_pubkey {
                    let new_aggregated_g1_nonsigners_pubkey =
                        match add_g1(&nonsigners, &operator.g1) {
                            Ok(new_aggregated_g1_nonsigners_pubkey) => {
                                new_aggregated_g1_nonsigners_pubkey
                            }
                            Err(e) => {
                                msg!("Error adding G1 points: {}", e);
                                return Err(ProgramError::InsufficientFunds);
                            }
                        };
                    aggregated_g1_nonsigners_pubkey = Some(new_aggregated_g1_nonsigners_pubkey);
                } else {
                    aggregated_g1_nonsigners_pubkey = Some(operator.g1);
                }
            } else {
                let operator_weight = operator.total_security(current_slot, epoch_length)?;
                weight_tally = weight_tally
                    .checked_add(operator_weight)
                    .ok_or(ProgramError::ArithmeticOverflow)?;

                signer_count = signer_count
                    .checked_add(1)
                    .ok_or(BlsNcnError::ArithmaticOverflow)?;
            }
        } else {
            break;
        }
    }

    let reached_consensus = rolling_snapshot_account.reached_consensus(
        current_slot,
        epoch_length,
        signer_count,
        weight_tally,
    )?;
    if !reached_consensus {
        msg!(
            "Consensus not reached: signers ({})/({}), weight ({})/({}), weight threshold ({:?}), consensus threshold BPS ({:?})",
            signer_count,
            rolling_snapshot_account.operator_count(),
            weight_tally,
            rolling_snapshot_account.total_security(current_slot, epoch_length)?,
            rolling_snapshot_account.consensus_weight_threshold,
            rolling_snapshot_account.consensus_threshold_bps
        );
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let signed_g1 = if let Some(nonsigners) = aggregated_g1_nonsigners_pubkey {
        match sub_g1(&rolling_snapshot_account.aggregate_g1, &nonsigners) {
            Ok(signed_g1) => signed_g1,
            Err(e) => {
                msg!("Failed to subtract G1 points: {}", e);
                return Err(BlsNcnError::G1SubtractionFailed.into());
            }
        }
    } else {
        rolling_snapshot_account.aggregate_g1
    };

    let verified = match solana_verify_aggregated_signature(
        &signed_g1,
        &ix_data.aggregated_g2_signed,
        &ix_data.aggregated_g1_signature,
        &ix_data.message,
        consensus_count,
    ) {
        Ok(verified) => verified,
        Err(e) => {
            msg!("Failed to verify aggregated signature: {}", e);
            return Err(BlsNcnError::SignatureVerificationFailed.into());
        }
    };

    if !verified {
        msg!("Invalid signature");
        return Err(BlsNcnError::SignatureVerificationFailed.into());
    }

    {
        let mut account_data = consensus.try_borrow_mut_data()?;
        let account = unsafe { load_account_mut_unchecked::<Consensus>(&mut account_data) }?;

        account.increment_consensus_count()?;
        msg!("Came to consensus on {:?} with {}/{} operators and {}/{} stake. Total consensus count: {}", ix_data.message, signer_count, rolling_snapshot_account.operator_count(), weight_tally, rolling_snapshot_account.total_security(current_slot, epoch_length)?, account.consensus_count());
        msg!(
            "Settings: Consensus Threshold: {:?}, Consensus Weight Threshold: {:?}",
            rolling_snapshot_account.consensus_threshold_bps.as_ref(),
            rolling_snapshot_account.consensus_weight_threshold.as_ref()
        );
    }

    Ok(())
}
