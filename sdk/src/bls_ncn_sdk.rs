use jito_bls_ncn_core::{
    accounts::*,
    bls::solana_bls_interface::SolanaBN254Keypair,
    instructions::{InitializeBlsOperatorIxData, InitializeConfigIxData, InitializeRollingSnapshotIxData, VoteIxData},
    utils::{JitoAccount, JitoIxData},
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
pub fn initialize_config_ix(ncn: &Pubkey, admin: &Pubkey, payer: &Pubkey) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let (pda, bump, _) = config_address(ncn);

    //  [rolling_snapshot, ncn, admin, payer, system_program]
    let accounts = vec![
        AccountMeta::new(pda, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let ix_data = InitializeConfigIxData::new(bump);
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}

pub fn initialize_rolling_snapshot_ix(ncn: &Pubkey, payer: &Pubkey) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let (pda, bump, _) = rolling_snapshot_address(ncn);

    //  [rolling_snapshot, ncn, admin, payer, system_program]
    let accounts = vec![
        AccountMeta::new(pda, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let ix_data = InitializeRollingSnapshotIxData::new(bump);
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}

pub fn initialize_bls_operator_ix(
    operator: &Pubkey,
    admin: &Pubkey,
    payer: &Pubkey,
    bls_keypair: &SolanaBN254Keypair,
    socket: Option<&[u8; 128]>,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let (pda, bump, _) = bls_operator_address(operator);

    // [bls_operator, operator, admin, payer, system_program]
    let accounts = vec![
        AccountMeta::new(pda, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let g1 = bls_keypair.public_key.g1.raw;
    let g2 = bls_keypair.public_key.g2.raw;
    let socket = socket.unwrap_or(&[0; 128]);

    let ix_data = InitializeBlsOperatorIxData::new(bump, g1, g2, *socket);
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

    let ix_data = VoteIxData::new([0; 64]);
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}
