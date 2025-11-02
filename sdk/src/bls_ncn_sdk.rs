use jito_bls_ncn_core::{
    accounts::*,
    bls::solana_bls_interface::{SolanaBN254G1, SolanaBN254G2, SolanaBN254Keypair},
    instructions::{
        InitializeBlsOperatorIxData, InitializeConfigIxData, InitializeConsensusIxData,
        InitializeRollingSnapshotIxData, RegisterBlsOperatorIxData, RegisterVaultIxData,
        SnapshotIxData, VoteIxData,
    },
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

    //  [config, ncn, admin, payer, system_program]
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

pub fn initialize_consensus_ix(ncn: &Pubkey, payer: &Pubkey) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let (pda, bump, _) = consensus_address(ncn);

    //  [consensus, ncn, payer, system_program]
    let accounts = vec![
        AccountMeta::new(pda, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let ix_data = InitializeConsensusIxData::new(bump);
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

pub fn register_bls_operator_ix(ncn: &Pubkey, operator: &Pubkey, admin: &Pubkey) -> Instruction {
    let program_id = id();

    let (config, _, _) = config_address(ncn);
    let (rolling_snapshot, _, _) = rolling_snapshot_address(ncn);
    let (bls_operator, _, _) = bls_operator_address(operator);

    let (restaking_config, _, _) = crate::restaking_sdk::config_address();
    let (ncn_operator_state, _, _) =
        crate::restaking_sdk::ncn_operator_state_address(ncn, operator);

    // [config, rolling_snapshot, bls_operator, restaking_config, ncn, operator, ncn_operator_state, admin]
    let accounts = vec![
        AccountMeta::new_readonly(config, false),
        AccountMeta::new(rolling_snapshot, false),
        AccountMeta::new_readonly(bls_operator, false),
        AccountMeta::new_readonly(restaking_config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new_readonly(ncn_operator_state, false),
        AccountMeta::new(*admin, true),
    ];

    let ix_data = RegisterBlsOperatorIxData::new();
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}

pub fn register_vault_ix(
    ncn: &Pubkey,
    vault: &Pubkey,
    admin: &Pubkey,
    weight_bps: u16,
) -> Instruction {
    let program_id = id();

    let (config, _, _) = config_address(ncn);
    let (rolling_snapshot, _, _) = rolling_snapshot_address(ncn);

    let (restaking_config, _, _) = crate::restaking_sdk::config_address();
    let (vault_ncn_ticket, _, _) = crate::vault_sdk::vault_ncn_ticket_address(vault, ncn);
    let (ncn_vault_ticket, _, _) = crate::restaking_sdk::ncn_vault_ticket_address(ncn, vault);

    // [config, rolling_snapshot, restaking_config, ncn, vault, ncn_vault_ticket, vault_ncn_ticket, admin]
    let accounts = vec![
        AccountMeta::new_readonly(config, false),
        AccountMeta::new(rolling_snapshot, false),
        AccountMeta::new_readonly(restaking_config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(ncn_vault_ticket, false),
        AccountMeta::new_readonly(vault_ncn_ticket, false),
        AccountMeta::new(*admin, true),
    ];

    let ix_data = RegisterVaultIxData::new(weight_bps);
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}

pub fn snapshot_ix(
    ncn: &Pubkey,
    operator: &Pubkey,
    vault: &Pubkey,
    operator_index: usize,
    vault_index: usize,
) -> Instruction {
    let program_id = id();

    let (config, _, _) = config_address(ncn);
    let (rolling_snapshot, _, _) = rolling_snapshot_address(ncn);

    let (restaking_config, _, _) = crate::restaking_sdk::config_address();
    let (vault_operator_delegation, _, _) =
        crate::vault_sdk::vault_operator_delegation_address(vault, operator);

    // [config, rolling_snapshot, restaking_config, ncn, operator, vault, vault_operator_delegation]
    let accounts = vec![
        AccountMeta::new_readonly(config, false),
        AccountMeta::new(rolling_snapshot, false),
        AccountMeta::new_readonly(restaking_config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(vault_operator_delegation, false),
    ];

    let ix_data = SnapshotIxData::new(operator_index, vault_index);
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}

pub fn vote_ix(
    ncn: &Pubkey,
    aggregated_g1_signature: &SolanaBN254G1,
    aggregated_g2_signed: &SolanaBN254G2,
    operators_bitmap_signed: &[u8; 32],
    raw_message: &[u8; 32],
    consensus_count: u64,
) -> Instruction {
    let program_id = id();
    let (rolling_snapshot, _, _) = rolling_snapshot_address(ncn);
    let (consensus, _, _) = consensus_address(ncn);
    let (restaking_config, _, _) = crate::restaking_sdk::config_address();

    // [rolling_snapshot, consensus, restaking_config, ncn]
    let accounts = vec![
        AccountMeta::new_readonly(rolling_snapshot, false),
        AccountMeta::new(consensus, false),
        AccountMeta::new_readonly(restaking_config, false),
        AccountMeta::new_readonly(*ncn, false),
    ];

    let ix_data = VoteIxData::new(
        aggregated_g1_signature.raw,
        aggregated_g2_signed.raw,
        *operators_bitmap_signed,
        *raw_message,
        consensus_count,
    );
    let ix_data_bytes = unsafe { ix_data.to_bytes() };

    Instruction {
        program_id,
        accounts,
        data: ix_data_bytes.to_vec(),
    }
}
