use jito_bls_ncn_core::{
    accounts::*, bls::solana_bls_interface::SolanaBN254Keypair, instructions::{InitializeBlsOperatorIxData, ReallocRollingSnapshotIxData, VoteIxData}, utils::JitoAccount
};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

// ----------------------- PROGRAM ID -----------------------
pub fn id() -> Pubkey {
    jito_bls_ncn_program::id()
}

// ----------------------- Accounts --------------------------
pub fn bls_operator_address(operator: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    bls_operator::BlsOperator::offchain_find_program_address(&id(), *operator)
}

pub fn config_address(ncn: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    config::Config::offchain_find_program_address(&id(), *ncn)
}

pub fn consensus_address(ncn: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    consensus::Consensus::offchain_find_program_address(&id(), *ncn)
}

pub fn rolling_snapshot_address(ncn: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    rolling_snapshot::RollingSnapshot::offchain_find_program_address(&id(), *ncn)
}

// --------------------------- Instructions --------------------------

pub fn initialize_bls_operator_ix(admin: &Pubkey, operator: &Pubkey, bls_keypair: &SolanaBN254Keypair, socket: Option<&[u8; 128]>) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let (pda, bump, _) = bls_operator_address(operator);

    // [bls_operator, operator, admin, system_program]
    let accounts = vec![
        AccountMeta::new(pda, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let g1 = bls_keypair.public_key.g1.raw;
    let g2 = bls_keypair.public_key.g2.raw;
    let socket = socket.unwrap_or(&[0; 128]);

    let ix_data = InitializeBlsOperatorIxData::new(
        bump,
        g1,
        g2,
        *socket,
    );
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}

pub fn realloc_rolling_snapshot_ix(admin: &Pubkey, ncn: &Pubkey) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let (pda, bump, _) = rolling_snapshot_address(ncn);

    //  [rolling_snapshot, ncn, admin, system_program]
    let accounts = vec![
        AccountMeta::new(pda, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let ix_data = ReallocRollingSnapshotIxData::new(
        bump
    );
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
