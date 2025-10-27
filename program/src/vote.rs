use jito_bls_ncn_core::{
    accounts::{consensus::Consensus, rolling_snapshot::RollingSnapshot},
    bls::solana_bls::{add_g1, did_sign_bitmap, solana_verify_aggregated_signature, sub_g1},
    instructions::VoteIxData,
    programs::restaking_core::Ncn,
    utils::{load_account, load_account_mut_unchecked, load_ix_data, JitoAccount},
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::{ProgramError, ProgramResult};
use solana_pubkey::Pubkey;

use crate::errors::BlsNcnError;

pub fn process_vote(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [rolling_snapshot, consensus, ncn] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let ix_data = unsafe { load_ix_data::<VoteIxData>(data)? };

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
    let mut non_signers_count: u64 = 0;

    for (index, operator) in rolling_snapshot_account.operators.iter().enumerate() {
        if let Some(operator) = operator.as_ref() {
            let did_sign = match did_sign_bitmap(ix_data.operators_bitmap_signed, index) {
                Ok(did_sign) => did_sign,
                Err(e) => {
                    msg!("Error checking signature bitmap: {}", e);
                    return Err(BlsNcnError::BitmapCheckFailed.into());
                }
            };

            if !did_sign {
                non_signers_count = non_signers_count
                    .checked_add(1)
                    .ok_or(BlsNcnError::ArithmaticOverflow)?;
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
                //TODO check stake
            }
        } else {
            break;
        }
    }

    let consensus_threshold = (rolling_snapshot_account.operator_count() as u64)
        .checked_div(3)
        .ok_or(BlsNcnError::ArithmaticUnderflow)?;

    if non_signers_count > consensus_threshold {
        msg!(
            "Quorum not met: non-signers count ({}) exceeds 1/3 of registered operators ({})",
            non_signers_count,
            rolling_snapshot_account.operator_count()
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
        msg!("Came to consensus {} times!", account.consensus_count())
    }

    Ok(())
}
