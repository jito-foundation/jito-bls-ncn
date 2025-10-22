use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::{Pubkey, pubkey};

use crate::programs::vault_core::{BurnVault, Config, Vault, VaultNcnSlasherOperatorTicket, VaultNcnSlasherTicket, VaultNcnTicket, VaultOperatorDelegation, VaultStakerWithdrawalTicket, VaultUpdateStateTracker};

// ----------------------- PROGRAM ID -----------------------
pub fn id() -> Pubkey {
    pubkey!("Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8")
}

// ----------------------- ENUMS --------------------------
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigAdminRole {
    FeeAdmin = 0,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultAdminRole {
    DelegationAdmin = 0,
    OperatorAdmin = 1,
    NcnAdmin = 2,
    SlasherAdmin = 3,
    CapacityAdmin = 4,
    FeeWallet = 5,
    MintBurnAdmin = 6,
    DelegateAssetAdmin = 7,
    FeeAdmin = 8,
    MetadataAdmin = 9,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WithdrawalAllocationMethod {
    Greedy = 0,
}

// ----------------------- PDA ADDRESSES --------------------------

pub fn config_address() -> (Pubkey, u8) {
    let (key, bump, _) = Config::find_program_address(&id());
    (key, bump)
}

pub fn burn_vault_address(base: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) = BurnVault::find_program_address(&id(), base);
    (key, bump)
}

pub fn vault_address(base: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) = Vault::find_program_address(&id(), base);
    (key, bump)
}

pub fn vault_operator_delegation_address(vault: &Pubkey, operator: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) =
        VaultOperatorDelegation::find_program_address(
            &id(),
            vault,
            operator,
        );
    (key, bump)
}

pub fn vault_ncn_ticket_address(vault: &Pubkey, ncn: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) =
        VaultNcnTicket::find_program_address(&id(), vault, ncn);
    (key, bump)
}

pub fn vault_ncn_slasher_ticket_address(
    vault: &Pubkey,
    ncn: &Pubkey,
    slasher: &Pubkey,
) -> (Pubkey, u8) {
    let (key, bump, _) =
        VaultNcnSlasherTicket::find_program_address(
            &id(),
            vault,
            ncn,
            slasher,
        );
    (key, bump)
}

pub fn vault_ncn_slasher_operator_ticket_address(
    vault: &Pubkey,
    ncn: &Pubkey,
    slasher: &Pubkey,
    operator: &Pubkey,
    epoch: u64,
) -> (Pubkey, u8) {
    let (key, bump, _) = VaultNcnSlasherOperatorTicket::find_program_address(
        &id(),
        vault,
        ncn,
        slasher,
        operator,
        epoch,
    );
    (key, bump)
}

pub fn vault_staker_withdrawal_ticket_address(vault: &Pubkey, base: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) = VaultStakerWithdrawalTicket::find_program_address(
        &id(),
        vault,
        base,
    );
    (key, bump)
}

pub fn vault_update_state_tracker_address(vault: &Pubkey, ncn_epoch: u64) -> (Pubkey, u8) {
    let (key, bump, _) =
        VaultUpdateStateTracker::find_program_address(
            &id(),
            vault,
            ncn_epoch,
        );
    (key, bump)
}

// ----------------------- INSTRUCTION BUILDERS --------------------------

/// Initializes global configuration
pub fn initialize_config_ix(
    config: &Pubkey,
    admin: &Pubkey,
    restaking_program: &Pubkey,
    program_fee_wallet: &Pubkey,
    program_fee_bps: u16,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(*restaking_program, false),
        AccountMeta::new_readonly(*program_fee_wallet, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    let mut data = vec![0]; // Discriminator for InitializeConfig
    data.extend_from_slice(&program_fee_bps.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Initializes the vault
#[allow(clippy::too_many_arguments)]
pub fn initialize_vault_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vrt_mint: &Pubkey,
    st_mint: &Pubkey,
    admin_st_token_account: &Pubkey,
    vault_st_token_account: &Pubkey,
    burn_vault: &Pubkey,
    burn_vault_vrt_token_account: &Pubkey,
    admin: &Pubkey,
    base: &Pubkey,
    token_program: &Pubkey,
    associated_token_program: &Pubkey,
    deposit_fee_bps: u16,
    withdrawal_fee_bps: u16,
    reward_fee_bps: u16,
    decimals: u8,
    initialize_token_amount: u64,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vrt_mint, true),
        AccountMeta::new_readonly(*st_mint, false),
        AccountMeta::new(*admin_st_token_account, false),
        AccountMeta::new(*vault_st_token_account, false),
        AccountMeta::new_readonly(*burn_vault, false),
        AccountMeta::new(*burn_vault_vrt_token_account, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(*base, true),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(*token_program, false),
        AccountMeta::new_readonly(*associated_token_program, false),
    ];

    let mut data = vec![1]; // Discriminator for InitializeVault
    data.extend_from_slice(&deposit_fee_bps.to_le_bytes());
    data.extend_from_slice(&withdrawal_fee_bps.to_le_bytes());
    data.extend_from_slice(&reward_fee_bps.to_le_bytes());
    data.push(decimals);
    data.extend_from_slice(&initialize_token_amount.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Initializes a vault with an already-created VRT mint
pub fn initialize_vault_with_mint_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vrt_mint: &Pubkey,
    st_mint: &Pubkey,
    admin_st_token_account: &Pubkey,
    vault_st_token_account: &Pubkey,
    burn_vault: &Pubkey,
    burn_vault_vrt_token_account: &Pubkey,
    admin: &Pubkey,
    base: &Pubkey,
    token_program: &Pubkey,
    associated_token_program: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vrt_mint, false),
        AccountMeta::new_readonly(*st_mint, false),
        AccountMeta::new(*admin_st_token_account, false),
        AccountMeta::new(*vault_st_token_account, false),
        AccountMeta::new_readonly(*burn_vault, false),
        AccountMeta::new(*burn_vault_vrt_token_account, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(*base, true),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(*token_program, false),
        AccountMeta::new_readonly(*associated_token_program, false),
    ];

    let data = vec![2]; // Discriminator for InitializeVaultWithMint

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Vault adds support for an operator
pub fn initialize_vault_operator_delegation_ix(
    config: &Pubkey,
    vault: &Pubkey,
    operator: &Pubkey,
    operator_vault_ticket: &Pubkey,
    vault_operator_delegation: &Pubkey,
    admin: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new(*operator, false),
        AccountMeta::new_readonly(*operator_vault_ticket, false),
        AccountMeta::new(*vault_operator_delegation, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![3]; // Discriminator for InitializeVaultOperatorDelegation

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Vault adds support for the NCN
pub fn initialize_vault_ncn_ticket_ix(
    config: &Pubkey,
    vault: &Pubkey,
    ncn: &Pubkey,
    ncn_vault_ticket: &Pubkey,
    vault_ncn_ticket: &Pubkey,
    admin: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*ncn_vault_ticket, false),
        AccountMeta::new(*vault_ncn_ticket, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![4]; // Discriminator for InitializeVaultNcnTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Initializes the account which keeps track of how much an operator has been slashed
#[allow(clippy::too_many_arguments)]
pub fn initialize_vault_ncn_slasher_operator_ticket_ix(
    config: &Pubkey,
    vault: &Pubkey,
    ncn: &Pubkey,
    slasher: &Pubkey,
    operator: &Pubkey,
    vault_ncn_slasher_ticket: &Pubkey,
    vault_ncn_slasher_operator_ticket: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*slasher, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new_readonly(*vault_ncn_slasher_ticket, false),
        AccountMeta::new(*vault_ncn_slasher_operator_ticket, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![5]; // Discriminator for InitializeVaultNcnSlasherOperatorTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Registers a slasher with the vault
#[allow(clippy::too_many_arguments)]
pub fn initialize_vault_ncn_slasher_ticket_ix(
    config: &Pubkey,
    vault: &Pubkey,
    ncn: &Pubkey,
    slasher: &Pubkey,
    ncn_slasher_ticket: &Pubkey,
    vault_slasher_ticket: &Pubkey,
    admin: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*slasher, false),
        AccountMeta::new_readonly(*ncn_slasher_ticket, false),
        AccountMeta::new(*vault_slasher_ticket, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let data = vec![6]; // Discriminator for InitializeVaultNcnSlasherTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Warmup vault NCN ticket
pub fn warmup_vault_ncn_ticket_ix(
    config: &Pubkey,
    vault: &Pubkey,
    ncn: &Pubkey,
    vault_ncn_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new(*vault_ncn_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![7]; // Discriminator for WarmupVaultNcnTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Vault removes support for an NCN
pub fn cooldown_vault_ncn_ticket_ix(
    config: &Pubkey,
    vault: &Pubkey,
    ncn: &Pubkey,
    vault_ncn_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new(*vault_ncn_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![8]; // Discriminator for CooldownVaultNcnTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Warmup vault NCN slasher ticket
pub fn warmup_vault_ncn_slasher_ticket_ix(
    config: &Pubkey,
    vault: &Pubkey,
    ncn: &Pubkey,
    slasher: &Pubkey,
    vault_slasher_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*slasher, false),
        AccountMeta::new(*vault_slasher_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![9]; // Discriminator for WarmupVaultNcnSlasherTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Cooldown vault NCN slasher ticket
pub fn cooldown_vault_ncn_slasher_ticket_ix(
    config: &Pubkey,
    vault: &Pubkey,
    ncn: &Pubkey,
    slasher: &Pubkey,
    vault_ncn_slasher_ticket: &Pubkey,
    admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*ncn, false),
        AccountMeta::new_readonly(*slasher, false),
        AccountMeta::new(*vault_ncn_slasher_ticket, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let data = vec![10]; // Discriminator for CooldownVaultNcnSlasherTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Mints VRT by depositing tokens into the vault
#[allow(clippy::too_many_arguments)]
pub fn mint_to_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vrt_mint: &Pubkey,
    depositor: &Pubkey,
    depositor_token_account: &Pubkey,
    vault_token_account: &Pubkey,
    depositor_vrt_token_account: &Pubkey,
    vault_fee_token_account: &Pubkey,
    token_program: &Pubkey,
    mint_signer: Option<&Pubkey>,
    amount_in: u64,
    min_amount_out: u64,
) -> Instruction {
    let program_id = id();

    let mut accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vrt_mint, false),
        AccountMeta::new(*depositor, true),
        AccountMeta::new(*depositor_token_account, false),
        AccountMeta::new(*vault_token_account, false),
        AccountMeta::new(*depositor_vrt_token_account, false),
        AccountMeta::new(*vault_fee_token_account, false),
        AccountMeta::new_readonly(*token_program, false),
    ];

    if let Some(signer) = mint_signer {
        accounts.push(AccountMeta::new_readonly(*signer, true));
    }

    let mut data = vec![11]; // Discriminator for MintTo
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Enqueues a withdrawal of VRT tokens
#[allow(clippy::too_many_arguments)]
pub fn enqueue_withdrawal_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vault_staker_withdrawal_ticket: &Pubkey,
    vault_staker_withdrawal_ticket_token_account: &Pubkey,
    staker: &Pubkey,
    staker_vrt_token_account: &Pubkey,
    base: &Pubkey,
    token_program: &Pubkey,
    burn_signer: Option<&Pubkey>,
    amount: u64,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let mut accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vault_staker_withdrawal_ticket, false),
        AccountMeta::new(*vault_staker_withdrawal_ticket_token_account, false),
        AccountMeta::new(*staker, true),
        AccountMeta::new(*staker_vrt_token_account, false),
        AccountMeta::new_readonly(*base, true),
        AccountMeta::new_readonly(*token_program, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    if let Some(signer) = burn_signer {
        accounts.push(AccountMeta::new_readonly(*signer, true));
    }

    let mut data = vec![12]; // Discriminator for EnqueueWithdrawal
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Change withdrawal ticket owner
pub fn change_withdrawal_ticket_owner_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vault_staker_withdrawal_ticket: &Pubkey,
    old_owner: &Pubkey,
    new_owner: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new(*vault_staker_withdrawal_ticket, false),
        AccountMeta::new_readonly(*old_owner, true),
        AccountMeta::new_readonly(*new_owner, false),
    ];

    let data = vec![13]; // Discriminator for ChangeWithdrawalTicketOwner

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Burns the withdrawal ticket, returning funds to the staker
#[allow(clippy::too_many_arguments)]
pub fn burn_withdrawal_ticket_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vault_token_account: &Pubkey,
    vrt_mint: &Pubkey,
    staker: &Pubkey,
    staker_token_account: &Pubkey,
    vault_staker_withdrawal_ticket: &Pubkey,
    vault_staker_withdrawal_ticket_token_account: &Pubkey,
    vault_fee_token_account: &Pubkey,
    program_fee_token_account: &Pubkey,
    token_program: &Pubkey,
    burn_signer: Option<&Pubkey>,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let mut accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vault_token_account, false),
        AccountMeta::new(*vrt_mint, false),
        AccountMeta::new(*staker, false),
        AccountMeta::new(*staker_token_account, false),
        AccountMeta::new(*vault_staker_withdrawal_ticket, false),
        AccountMeta::new(*vault_staker_withdrawal_ticket_token_account, false),
        AccountMeta::new(*vault_fee_token_account, false),
        AccountMeta::new(*program_fee_token_account, false),
        AccountMeta::new_readonly(*token_program, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    if let Some(signer) = burn_signer {
        accounts.push(AccountMeta::new_readonly(*signer, true));
    }

    let data = vec![14]; // Discriminator for BurnWithdrawalTicket

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets the max tokens that can be deposited into the VRT
pub fn set_deposit_capacity_ix(
    config: &Pubkey,
    vault: &Pubkey,
    admin: &Pubkey,
    amount: u64,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let mut data = vec![15]; // Discriminator for SetDepositCapacity
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets the fees for depositing and withdrawing
pub fn set_fees_ix(
    config: &Pubkey,
    vault: &Pubkey,
    admin: &Pubkey,
    deposit_fee_bps: Option<u16>,
    withdrawal_fee_bps: Option<u16>,
    reward_fee_bps: Option<u16>,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let mut data = vec![16]; // Discriminator for SetFees

    // Encode Option<u16> as 1 byte flag + 2 bytes value
    if let Some(fee) = deposit_fee_bps {
        data.push(1);
        data.extend_from_slice(&fee.to_le_bytes());
    } else {
        data.push(0);
    }

    if let Some(fee) = withdrawal_fee_bps {
        data.push(1);
        data.extend_from_slice(&fee.to_le_bytes());
    } else {
        data.push(0);
    }

    if let Some(fee) = reward_fee_bps {
        data.push(1);
        data.extend_from_slice(&fee.to_le_bytes());
    } else {
        data.push(0);
    }

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets the program fee for the vault program
pub fn set_program_fee_ix(config: &Pubkey, admin: &Pubkey, new_fee_bps: u16) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let mut data = vec![17]; // Discriminator for SetProgramFee
    data.extend_from_slice(&new_fee_bps.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets the program fee wallet for the vault program
pub fn set_program_fee_wallet_ix(
    config: &Pubkey,
    program_fee_admin: &Pubkey,
    new_fee_wallet: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new_readonly(*program_fee_admin, true),
        AccountMeta::new_readonly(*new_fee_wallet, false),
    ];

    let data = vec![18]; // Discriminator for SetProgramFeeWallet

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Sets `is_paused`
pub fn set_is_paused_ix(
    config: &Pubkey,
    vault: &Pubkey,
    admin: &Pubkey,
    is_paused: bool,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let mut data = vec![19]; // Discriminator for SetIsPaused
    data.push(is_paused as u8);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Delegate the token account to a third party
pub fn delegate_token_account_ix(
    config: &Pubkey,
    vault: &Pubkey,
    delegate_asset_admin: &Pubkey,
    token_mint: &Pubkey,
    token_account: &Pubkey,
    delegate: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*delegate_asset_admin, true),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*delegate, false),
        AccountMeta::new_readonly(*token_program, false),
    ];

    let data = vec![20]; // Discriminator for DelegateTokenAccount

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Revoke Delegate of the token account
pub fn revoke_delegate_token_account_ix(
    config: &Pubkey,
    vault: &Pubkey,
    delegate_asset_admin: &Pubkey,
    token_mint: &Pubkey,
    token_account: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*delegate_asset_admin, true),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*token_program, false),
    ];

    let data = vec![21]; // Discriminator for RevokeDelegateTokenAccount

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Changes the signer for vault admin
pub fn set_admin_ix(
    config: &Pubkey,
    vault: &Pubkey,
    old_admin: &Pubkey,
    new_admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*old_admin, true),
        AccountMeta::new_readonly(*new_admin, true),
    ];

    let data = vec![22]; // Discriminator for SetAdmin

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Changes the signer for vault delegation
pub fn set_secondary_admin_ix(
    config: &Pubkey,
    vault: &Pubkey,
    admin: &Pubkey,
    new_admin: &Pubkey,
    role: VaultAdminRole,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new_readonly(*new_admin, false),
    ];

    let mut data = vec![23]; // Discriminator for SetSecondaryAdmin
    data.push(role as u8);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Delegates a token amount to a specific node operator
pub fn add_delegation_ix(
    config: &Pubkey,
    vault: &Pubkey,
    operator: &Pubkey,
    vault_operator_delegation: &Pubkey,
    admin: &Pubkey,
    amount: u64,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new(*vault_operator_delegation, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let mut data = vec![24]; // Discriminator for AddDelegation
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Cools down delegation
pub fn cooldown_delegation_ix(
    config: &Pubkey,
    vault: &Pubkey,
    operator: &Pubkey,
    vault_operator_delegation: &Pubkey,
    admin: &Pubkey,
    amount: u64,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new(*vault_operator_delegation, false),
        AccountMeta::new_readonly(*admin, true),
    ];

    let mut data = vec![25]; // Discriminator for CooldownDelegation
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Updates vault balance
pub fn update_vault_balance_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vault_token_account: &Pubkey,
    vrt_mint: &Pubkey,
    vault_fee_token_account: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*vault_token_account, false),
        AccountMeta::new(*vrt_mint, false),
        AccountMeta::new(*vault_fee_token_account, false),
        AccountMeta::new_readonly(*token_program, false),
    ];

    let data = vec![26]; // Discriminator for UpdateVaultBalance

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Starts updating the vault
pub fn initialize_vault_update_state_tracker_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vault_update_state_tracker: &Pubkey,
    payer: &Pubkey,
    withdrawal_allocation_method: WithdrawalAllocationMethod,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vault_update_state_tracker, false),
        AccountMeta::new(*payer, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    let mut data = vec![27]; // Discriminator for InitializeVaultUpdateStateTracker
    data.push(withdrawal_allocation_method as u8);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Shall be called on every vault_operator_delegation
pub fn crank_vault_update_state_tracker_ix(
    config: &Pubkey,
    vault: &Pubkey,
    operator: &Pubkey,
    vault_operator_delegation: &Pubkey,
    vault_update_state_tracker: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new_readonly(*operator, false),
        AccountMeta::new(*vault_operator_delegation, false),
        AccountMeta::new(*vault_update_state_tracker, false),
    ];

    let data = vec![28]; // Discriminator for CrankVaultUpdateStateTracker

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Closes vault update state tracker
pub fn close_vault_update_state_tracker_ix(
    config: &Pubkey,
    vault: &Pubkey,
    vault_update_state_tracker: &Pubkey,
    payer: &Pubkey,
    ncn_epoch: u64,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vault_update_state_tracker, false),
        AccountMeta::new(*payer, true),
    ];

    let mut data = vec![29]; // Discriminator for CloseVaultUpdateStateTracker
    data.extend_from_slice(&ncn_epoch.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Creates token metadata for the vault VRT
pub fn create_token_metadata_ix(
    vault: &Pubkey,
    admin: &Pubkey,
    vrt_mint: &Pubkey,
    payer: &Pubkey,
    metadata: &Pubkey,
    mpl_token_metadata_program: &Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    let program_id = id();
    let system_program = solana_system_interface::program::id();

    let accounts = vec![
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new_readonly(*vrt_mint, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new(*metadata, false),
        AccountMeta::new_readonly(*mpl_token_metadata_program, false),
        AccountMeta::new_readonly(system_program, false),
    ];

    let mut data = vec![30]; // Discriminator for CreateTokenMetadata

    // Serialize string as length prefix + bytes
    let name_bytes = name.as_bytes();
    data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(name_bytes);

    let symbol_bytes = symbol.as_bytes();
    data.extend_from_slice(&(symbol_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(symbol_bytes);

    let uri_bytes = uri.as_bytes();
    data.extend_from_slice(&(uri_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(uri_bytes);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Updates token metadata for the vault VRT
pub fn update_token_metadata_ix(
    vault: &Pubkey,
    admin: &Pubkey,
    vrt_mint: &Pubkey,
    metadata: &Pubkey,
    mpl_token_metadata_program: &Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*vault, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new_readonly(*vrt_mint, false),
        AccountMeta::new(*metadata, false),
        AccountMeta::new_readonly(*mpl_token_metadata_program, false),
    ];

    let mut data = vec![31]; // Discriminator for UpdateTokenMetadata

    // Serialize string as length prefix + bytes
    let name_bytes = name.as_bytes();
    data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(name_bytes);

    let symbol_bytes = symbol.as_bytes();
    data.extend_from_slice(&(symbol_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(symbol_bytes);

    let uri_bytes = uri.as_bytes();
    data.extend_from_slice(&(uri_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(uri_bytes);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Changes the admin for the config
pub fn set_config_admin_ix(
    config: &Pubkey,
    old_admin: &Pubkey,
    new_admin: &Pubkey,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new(*config, false),
        AccountMeta::new_readonly(*old_admin, true),
        AccountMeta::new_readonly(*new_admin, false),
    ];

    let data = vec![32]; // Discriminator for SetConfigAdmin

    Instruction {
        program_id,
        accounts,
        data,
    }
}

/// Changes the secondary admin for the config
pub fn set_config_secondary_admin_ix(
    config: &Pubkey,
    admin: &Pubkey,
    new_admin: &Pubkey,
    role: ConfigAdminRole,
) -> Instruction {
    let program_id = id();

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new_readonly(*new_admin, false),
    ];

    let mut data = vec![33]; // Discriminator for SetConfigSecondaryAdmin
    data.push(role as u8);

    Instruction {
        program_id,
        accounts,
        data,
    }
}
