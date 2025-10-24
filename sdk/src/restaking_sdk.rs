use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::{Pubkey, pubkey};

use jito_bls_ncn_core::{programs::restaking_core::{Config, Ncn, NcnOperatorState, NcnVaultSlasherTicket, NcnVaultTicket, Operator, OperatorVaultTicket}, utils::JitoAccount};


// ----------------------- PROGRAM ID -----------------------
// TODO: Replace with actual program ID
pub fn id() -> Pubkey {
    pubkey!("RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q")
}

// ----------------------- ENUMS --------------------------
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcnAdminRole {
    OperatorAdmin = 0,
    VaultAdmin = 1,
    SlasherAdmin = 2,
    DelegateAdmin = 3,
    MetadataAdmin = 4,
    WeightTableAdmin = 5,
    NcnProgramAdmin = 6,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorAdminRole {
    NcnAdmin = 0,
    VaultAdmin = 1,
    VoterAdmin = 2,
    DelegateAdmin = 3,
    MetadataAdmin = 4,
}

// ----------------------- PDA ADDRESSES --------------------------
// TODO: Implement these based on your actual account structures
// These should match the seed derivation logic in your core accounts

pub fn config_address() -> (Pubkey, u8, Vec<Vec<u8>>) {
    Config::offchain_find_program_address(&id(), ())
}

pub fn ncn_address(base: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    Ncn::offchain_find_program_address(&id(), *base)
}

pub fn operator_address(base: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    Operator::offchain_find_program_address(&id(), *base)
}

pub fn ncn_vault_ticket_address(ncn: &Pubkey, vault: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    NcnVaultTicket::offchain_find_program_address(&id(), (*ncn, *vault))
}

pub fn ncn_vault_slasher_ticket_address(
    ncn: &Pubkey,
    vault: &Pubkey,
    slasher: &Pubkey,
) -> (Pubkey, u8, Vec<Vec<u8>>) {
    NcnVaultSlasherTicket::offchain_find_program_address(&id(), (*ncn, *vault, *slasher))
}

pub fn operator_vault_ticket_address(operator: &Pubkey, vault: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    OperatorVaultTicket::offchain_find_program_address(&id(), (*operator, *vault))
}

pub fn ncn_operator_state_address(ncn: &Pubkey, operator: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
    NcnOperatorState::offchain_find_program_address(&id(), (*ncn, *operator))
}

// ----------------------- INSTRUCTION BUILDERS --------------------------

/// Initializes the global configuration
pub fn initialize_config_ix(
    config: &Pubkey,
    admin: &Pubkey,
    vault_program: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(*vault_program, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![0]; // Discriminator for InitializeConfig

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Initializes the NCN
pub fn initialize_ncn_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    admin: &Pubkey,
    base: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new(*ncn, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(*base, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![1]; // Discriminator for InitializeNcn

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Initializes an operator
pub fn initialize_operator_ix(
    config: &Pubkey,
    operator: &Pubkey,
    admin: &Pubkey,
    base: &Pubkey,
    operator_fee_bps: u16,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new(*operator, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(*base, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let mut data = vec![2]; // Discriminator for InitializeOperator
    data.extend_from_slice(&operator_fee_bps.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// The NCN adds support for a vault slasher
pub fn initialize_ncn_vault_slasher_ticket_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    vault: &Pubkey,
    slasher: &Pubkey,
    ncn_vault_ticket: &Pubkey,
    ncn_vault_slasher_ticket: &Pubkey,
    admin: &Pubkey,
    payer: &Pubkey,
    max_slashable_per_epoch: u64,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*ncn, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*slasher, false),
        AccountMeta::new_readonly(*ncn_vault_ticket, false),
        AccountMeta::new(*ncn_vault_slasher_ticket, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let mut data = vec![3]; // Discriminator for InitializeNcnVaultSlasherTicket
    data.extend_from_slice(&max_slashable_per_epoch.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// NCN adds support for receiving delegation from a vault
pub fn initialize_ncn_vault_ticket_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    vault: &Pubkey,
    ncn_vault_ticket: &Pubkey,
    admin: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*ncn, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new(*ncn_vault_ticket, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![4]; // Discriminator for InitializeNcnVaultTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Operator adds support for receiving delegation from a vault
pub fn initialize_operator_vault_ticket_ix(
    config: &Pubkey,
    operator: &Pubkey,
    vault: &Pubkey,
    operator_vault_ticket: &Pubkey,
    admin: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*operator, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new(*operator_vault_ticket, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![5]; // Discriminator for InitializeOperatorVaultTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Initialize NCN operator state
pub fn initialize_ncn_operator_state_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*ncn, false),
        AccountMeta::new(*operator, false),
        AccountMeta::new(*ncn_operator_state, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![6]; // Discriminator for InitializeNcnOperatorState

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Warmup NCN vault ticket
pub fn warmup_ncn_vault_ticket_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    vault: &Pubkey,
    ncn_vault_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new(*ncn_vault_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![7]; // Discriminator for WarmupNcnVaultTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// NCN removes support for receiving delegation from a vault
pub fn cooldown_ncn_vault_ticket_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    vault: &Pubkey,
    ncn_vault_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new(*ncn_vault_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![8]; // Discriminator for CooldownNcnVaultTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// NCN warms up operator
pub fn ncn_warmup_operator_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new(*ncn_operator_state, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![9]; // Discriminator for NcnWarmupOperator

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// NCN cools down operator
pub fn ncn_cooldown_operator_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new(*ncn_operator_state, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![10]; // Discriminator for NcnCooldownOperator

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Operator warms up NCN
pub fn operator_warmup_ncn_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new(*ncn_operator_state, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![11]; // Discriminator for OperatorWarmupNcn

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Operator cools down NCN
pub fn operator_cooldown_ncn_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    ncn_operator_state: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new(*ncn_operator_state, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![12]; // Discriminator for OperatorCooldownNcn

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Warmup NCN vault slasher ticket
pub fn warmup_ncn_vault_slasher_ticket_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    vault: &Pubkey,
    slasher: &Pubkey,
    ncn_vault_ticket: &Pubkey,
    ncn_vault_slasher_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*slasher, false),
        AccountMeta::new_readonly(*ncn_vault_ticket, false),
        AccountMeta::new(*ncn_vault_slasher_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![13]; // Discriminator for WarmupNcnVaultSlasherTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// NCN removes support for a slasher
pub fn cooldown_ncn_vault_slasher_ticket_ix(
    config: &Pubkey,
    ncn: &Pubkey,
    vault: &Pubkey,
    slasher: &Pubkey,
    ncn_vault_slasher_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*slasher, false),
        AccountMeta::new(*ncn_vault_slasher_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![14]; // Discriminator for CooldownNcnVaultSlasherTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Warmup operator vault ticket
pub fn warmup_operator_vault_ticket_ix(
    config: &Pubkey,
    operator: &Pubkey,
    vault: &Pubkey,
    operator_vault_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new(*operator_vault_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![15]; // Discriminator for WarmupOperatorVaultTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Node operator removes support for receiving delegation from a vault
pub fn cooldown_operator_vault_ticket_ix(
    config: &Pubkey,
    operator: &Pubkey,
    vault: &Pubkey,
    operator_vault_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new(*operator_vault_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![16]; // Discriminator for CooldownOperatorVaultTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets the admin for an NCN
pub fn ncn_set_admin_ix(ncn: &Pubkey, old_admin: &Pubkey, new_admin: &Pubkey) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new(*ncn, false),
        AccountMeta::new_readonly(*old_admin, true),
        AccountMeta::new_readonly(*new_admin, true),
    ];

    let data = vec![17]; // Discriminator for NcnSetAdmin

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets a secondary admin for an NCN
pub fn ncn_set_secondary_admin_ix(
    ncn: &Pubkey,
    admin: &Pubkey,
    new_admin: &Pubkey,
    role: NcnAdminRole,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new(*ncn, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new_readonly(*new_admin, false),
    ];

    let mut data = vec![18]; // Discriminator for NcnSetSecondaryAdmin
    data.push(role as u8);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets the admin for a node operator
pub fn operator_set_admin_ix(
    operator: &Pubkey,
    old_admin: &Pubkey,
    new_admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new(*operator, false),
        AccountMeta::new_readonly(*old_admin, true),
        AccountMeta::new_readonly(*new_admin, true),
    ];

    let data = vec![19]; // Discriminator for OperatorSetAdmin

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets a secondary admin for a node operator
pub fn operator_set_secondary_admin_ix(
    operator: &Pubkey,
    admin: &Pubkey,
    new_admin: &Pubkey,
    role: OperatorAdminRole,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new(*operator, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new_readonly(*new_admin, false),
    ];

    let mut data = vec![20]; // Discriminator for OperatorSetSecondaryAdmin
    data.push(role as u8);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets the fee for a node operator
pub fn operator_set_fee_ix(
    config: &Pubkey,
    operator: &Pubkey,
    admin: &Pubkey,
    new_fee_bps: u16,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*operator, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let mut data = vec![21]; // Discriminator for OperatorSetFee
    data.extend_from_slice(&new_fee_bps.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// NCN delegates token account
pub fn ncn_delegate_token_account_ix(
    ncn: &Pubkey,
    delegate_admin: &Pubkey,
    token_mint: &Pubkey,
    token_account: &Pubkey,
    delegate: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*delegate_admin, true),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*delegate, false),
        AccountMeta::new_readonly(*token_program, false),
    ];

    let data = vec![22]; // Discriminator for NcnDelegateTokenAccount

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Operator delegates token account
pub fn operator_delegate_token_account_ix(
    operator: &Pubkey,
    delegate_admin: &Pubkey,
    token_mint: &Pubkey,
    token_account: &Pubkey,
    delegate: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new_readonly(*delegate_admin, true),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*delegate, false),
        AccountMeta::new_readonly(*token_program, false),
    ];

    let data = vec![23]; // Discriminator for OperatorDelegateTokenAccount

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Changes the admin for the config
pub fn set_config_admin_ix(config: &Pubkey, old_admin: &Pubkey, new_admin: &Pubkey) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new_readonly(*old_admin, true),
        AccountMeta::new_readonly(*new_admin, false),
    ];

    let data = vec![24]; // Discriminator for SetConfigAdmin

    Instruction {
        program_id,
        accounts,
        data,
    }
}
