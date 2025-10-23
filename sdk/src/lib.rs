use jito_bls_ncn_core::{
    accounts::*,
    instructions::{ReallocRollingSnapshotIxData, VoteIxData},
};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

// ----------------------- PROGRAM ID -----------------------
pub fn id() -> Pubkey {
    jito_bls_ncn_program::id()
}

// ----------------------- Accounts --------------------------
pub fn bls_operator_address(operator: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) = bls_operator::BlsOperator::offchain_find_program_address(&id(), operator);
    (key, bump)
}

pub fn config_address(ncn: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) = config::Config::offchain_find_program_address(&id(), ncn);
    (key, bump)
}

pub fn consensus_address(ncn: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) = consensus::Consensus::offchain_find_program_address(&id(), ncn);
    (key, bump)
}

pub fn rolling_snapshot_address(ncn: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) =
        rolling_snapshot::RollingSnapshot::offchain_find_program_address(&id(), ncn);
    (key, bump)
}

// --------------------------- Instructions --------------------------

pub fn realloc_rolling_snapshot_ix(ncn: &Pubkey) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    // [ncn, system_program]
    let accounts = vec![
        AccountMeta::new(*ncn, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    let ix_data = ReallocRollingSnapshotIxData::new();
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}

pub fn vote_ix(ncn: &Pubkey) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    // [ncn, system_program]
    let accounts = vec![
        AccountMeta::new(*ncn, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    let ix_data = VoteIxData::new();
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}
