use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    accounts::bls_operator::BlsOperator,
    bls::solana_bls::{add_g1, sub_g1},
    constants::{BPS_PER_PERCENT, DEFAULT_THRESHOLD_BPS},
    discriminators::Discriminators,
    pod::{PodOption, PodU128, PodU16, PodU64},
    utils::{check_account, get_epoch, load_account, JitoAccount},
};

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct RollingSnapshot {
    /// Discriminator for the account
    pub discriminator: PodU64,
    /// The bump seed for the PDA
    pub bump: u8,
    /// The NCN this ncn operator account belongs to
    pub ncn: Pubkey,
    /// If this is Some, each operator counts as one vote as long as they are active and over the consensus_weight_threshold, else it defaults to percentage of total weight
    pub consensus_weight_threshold: PodOption<PodU128>,
    /// BPS of votes or weight/total weight to reach consensus, otherwise a supermajority is needed
    pub consensus_threshold_bps: PodOption<PodU16>,
    /// Aggregate G1
    pub aggregate_g1: [u8; 64],
    /// Reserved for future use
    pub reserved1: [u8; 32],
    /// Last full snapshot slot
    pub last_full_snapshot_slot: PodU64,
    /// Slot when the snapshot starts for the given epoch
    pub start_snapshot_slot: PodOption<PodU64>,
    /// How many vaults are - vaults can only be added or updated after a full snapshot
    pub vault_count: PodU16,
    /// The array of vaults - they will always be consecutive
    pub vaults: [PodOption<VaultEntry>; 300],
    /// Reserved for future use
    pub reserved2: [u8; 32],
    /// How many operators are - operators can only be added or updated after a full snapshot
    pub operator_count: PodU16,
    /// The array of operators - they will always be consecutive
    pub operators: [PodOption<OperatorEntry>; 300],
    /// Reserved for future use
    pub reserved3: [u8; 32],
}

impl JitoAccount for RollingSnapshot {
    const DISCRIMINATOR: u64 = Discriminators::RollingSnapshot as u64;
    const LEN: usize = std::mem::size_of::<Self>();
    const SEED: &'static [u8] = b"rolling_snapshot";
    type SeedInputs = Pubkey;

    /// ncn
    fn seeds(inputs: Self::SeedInputs) -> Vec<Vec<u8>> {
        let ncn = inputs;
        vec![Self::SEED.to_vec(), ncn.to_bytes().to_vec()]
    }

    fn offchain_find_program_address(
        program_id: &Pubkey,
        inputs: Self::SeedInputs,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(inputs);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    fn create_program_address(
        program_id: &Pubkey,
        bump: u8,
        inputs: Self::SeedInputs,
    ) -> Result<(Pubkey, u8, Vec<Vec<u8>>), ProgramError> {
        let mut seeds = Self::seeds(inputs);
        seeds.push(vec![bump]);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let pda = Pubkey::create_program_address(&seeds_iter, program_id)?;
        Ok((pda, bump, seeds))
    }

    fn check(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let data = account.data.borrow();
        let data_account = unsafe { load_account::<Self>(&data)? };
        let (expected_pda, _, _) =
            Self::create_program_address(program_id, data_account.bump, data_account.ncn)?;

        check_account(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.discriminator.get() == Self::DISCRIMINATOR
    }
}

impl RollingSnapshot {
    pub const MAX_OPERATORS: u16 = 256;
    pub const MAX_VAULTS: u16 = 256;

    pub fn initialize(&mut self, ncn: &Pubkey, bump: u8) -> Result<(), ProgramError> {
        if self.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        self.discriminator = PodU64::from(Self::DISCRIMINATOR);
        self.bump = bump;
        self.ncn = *ncn;
        self.consensus_threshold_bps = PodOption::none();
        self.consensus_weight_threshold = PodOption::none();
        self.aggregate_g1 = [0u8; 64];
        self.last_full_snapshot_slot = PodU64::from(0);
        self.start_snapshot_slot = PodOption::none();
        self.reserved1 = [0u8; 32];
        self.vault_count = PodU16::from(0);
        self.vaults = [PodOption::none(); 300];
        self.reserved2 = [0u8; 32];
        self.operator_count = PodU16::from(0);
        self.operators = [PodOption::none(); 300];
        self.reserved3 = [0u8; 32];

        Ok(())
    }

    pub fn discriminator(&self) -> u64 {
        self.discriminator.get()
    }

    pub fn vault_count(&self) -> u16 {
        self.vault_count.into()
    }

    pub fn operator_count(&self) -> u16 {
        self.operator_count.into()
    }

    pub fn snapshot_completed(
        &self,
        current_slot: u64,
        epoch_length: u64,
    ) -> Result<bool, ProgramError> {
        let current_epoch = get_epoch(current_slot, epoch_length)?;
        let last_full_snapshot_epoch = get_epoch(self.last_full_snapshot_slot.get(), epoch_length)?;

        Ok(current_epoch.eq(&last_full_snapshot_epoch))
    }

    pub fn needs_snapshot(
        &self,
        current_slot: u64,
        epoch_length: u64,
    ) -> Result<bool, ProgramError> {
        Ok(!self.snapshot_completed(current_slot, epoch_length)?)
    }

    pub fn can_update_operators_or_vaults(
        &self,
        current_slot: u64,
        epoch_length: u64,
    ) -> Result<bool, ProgramError> {
        if self.operator_count() == 0 || self.vault_count() == 0 {
            return Ok(true);
        }

        self.snapshot_completed(current_slot, epoch_length)
    }

    fn update_operator_or_vault_count(
        &mut self,
        current_slot: u64,
        epoch_length: u64,
        new_vault_count: Option<u16>,
        new_operator_count: Option<u16>,
    ) -> Result<(), ProgramError> {
        // Sanity check
        if !self.can_update_operators_or_vaults(current_slot, epoch_length)? {
            msg!("Cannot update snapshot");
        }

        let new_operator_count = new_operator_count.unwrap_or(self.operator_count());
        let new_vault_count = new_vault_count.unwrap_or(self.vault_count());

        self.operator_count = PodU16::from(new_operator_count);
        self.vault_count = PodU16::from(new_vault_count);
        self.last_full_snapshot_slot = PodU64::from(current_slot);

        // Clear operator snapshots - so if operators or vaults are removed and the state
        // cannot be snapshotted, old data will not be used
        if self.operator_count() == 0 || self.vault_count() == 0 {
            for i in 0..self.operator_count() {
                if let Some(operator) = self.operators[i as usize].as_mut() {
                    operator.clear_operator_weight();
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn snapshot(
        &mut self,
        current_slot: u64,
        epoch_length: u64,
        vault: &Pubkey,
        vault_index: u16,
        operator: &Pubkey,
        operator_index: u16,
        staked: u64,
        enqueued_for_cooldown: u64,
        cooling_down: u64,
    ) -> Result<(), ProgramError> {
        if self.snapshot_completed(current_slot, epoch_length)? {
            msg!("Already up to date");
            return Err(ProgramError::InvalidArgument);
        }

        self.check_operator_index(operator, operator_index as usize)?;
        self.check_vault_index(vault, vault_index as usize)?;

        if self.start_snapshot_slot.is_none() {
            self.start_snapshot_slot = PodOption::some(PodU64::from(current_slot))
        }

        let vault_count = self.vault_count();
        let vault_entry = self.vaults[vault_index as usize]
            .as_ref()
            .ok_or(ProgramError::InvalidArgument)?;
        let vault_weight_bps = vault_entry.weight_bps.get();

        let weight_staked = (staked as u128)
            .checked_mul(vault_weight_bps as u128)
            .ok_or(ProgramError::InvalidArgument)?;
        let weight_enqueued_for_cooldown = (enqueued_for_cooldown as u128)
            .checked_mul(vault_weight_bps as u128)
            .ok_or(ProgramError::InvalidArgument)?;
        let weight_cooling_down = (cooling_down as u128)
            .checked_mul(vault_weight_bps as u128)
            .ok_or(ProgramError::InvalidArgument)?;

        {
            let operator = self.operators[operator_index as usize]
                .as_mut()
                .ok_or(ProgramError::InvalidArgument)?;
            operator.snapshot(
                current_slot,
                epoch_length,
                vault_index,
                vault_count,
                weight_staked,
                weight_enqueued_for_cooldown,
                weight_cooling_down,
            )?;
        }

        // Snapshotting complete
        if self.are_all_operators_snapshotted(current_slot, epoch_length)? {
            self.start_snapshot_slot = PodOption::none();
            self.last_full_snapshot_slot = PodU64::from(current_slot);
        }

        Ok(())
    }

    pub fn add_vault(
        &mut self,
        current_slot: u64,
        epoch_length: u64,
        vault: &Pubkey,
        weight_bps: u16,
    ) -> Result<(), ProgramError> {
        if !self.can_update_operators_or_vaults(current_slot, epoch_length)? {
            msg!("Cannot update");
            return Err(ProgramError::InvalidArgument);
        }

        if self.vault_count() >= Self::MAX_VAULTS {
            msg!("Already at maximum vaults");
            return Err(ProgramError::InvalidArgument);
        }

        let duplicate_result = self.vaults.iter().find(|entry| {
            if let Some(entry) = entry.as_ref() {
                entry.vault.eq(vault)
            } else {
                false
            }
        });

        if duplicate_result.is_some() {
            msg!("Vault already exists");
            return Err(ProgramError::InvalidArgument);
        }

        let vault_entry = VaultEntry {
            vault: *vault,
            weight_bps: PodU16::from(weight_bps),
        };
        self.vaults[self.vault_count() as usize] = PodOption::some(vault_entry);

        let vaults_plus_one = self
            .vault_count()
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        self.update_operator_or_vault_count(
            current_slot,
            epoch_length,
            Some(vaults_plus_one),
            None,
        )?;

        Ok(())
    }

    pub fn check_vault_index(&self, vault: &Pubkey, index: usize) -> Result<(), ProgramError> {
        if index >= self.vault_count() as usize {
            msg!("Invalid vault index");
            return Err(ProgramError::InvalidArgument);
        }

        if let Some(vault_entry_to_check) = self.vaults[index].as_ref() {
            if vault_entry_to_check.vault.ne(vault) {
                msg!("Vault mismatch");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Vault not found");
            return Err(ProgramError::InvalidArgument);
        }

        Ok(())
    }

    pub fn remove_vault(
        &mut self,
        current_slot: u64,
        epoch_length: u64,
        vault: &Pubkey,
        index: usize,
    ) -> Result<(), ProgramError> {
        if !self.can_update_operators_or_vaults(current_slot, epoch_length)? {
            msg!("Cannot update");
            return Err(ProgramError::InvalidArgument);
        }

        self.check_vault_index(vault, index)?;

        {
            // Wipe the old VaultEntry
            self.vaults[index] = PodOption::some(VaultEntry::default());
            self.vaults[index] = PodOption::none();
        }

        // Shift vaults to fill the gap
        let end_index: usize = self.vault_count().saturating_sub(1) as usize;
        for i in index..end_index {
            let i_plus_one = i.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;
            self.vaults[i] = self.vaults[i_plus_one];
        }

        let vaults_minus_one = self
            .vault_count()
            .checked_sub(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        self.update_operator_or_vault_count(
            current_slot,
            epoch_length,
            Some(vaults_minus_one),
            None,
        )?;

        Ok(())
    }

    pub fn update_vault_weight_bps(
        &mut self,
        current_slot: u64,
        epoch_length: u64,
        vault: &Pubkey,
        index: usize,
        weight_bps: u16,
    ) -> Result<(), ProgramError> {
        if !self.can_update_operators_or_vaults(current_slot, epoch_length)? {
            msg!("Cannot update");
            return Err(ProgramError::InvalidArgument);
        }

        self.check_vault_index(vault, index)?;

        let vault = self.vaults[index]
            .as_mut()
            .ok_or(ProgramError::InvalidArgument)?;
        vault.weight_bps = PodU16::from(weight_bps);

        Ok(())
    }

    // OPERATORS
    pub fn add_operator(
        &mut self,
        current_slot: u64,
        epoch_length: u64,
        operator: &BlsOperator,
    ) -> Result<(), ProgramError> {
        if !self.can_update_operators_or_vaults(current_slot, epoch_length)? {
            msg!("Cannot update");
            return Err(ProgramError::InvalidArgument);
        }

        if self.operator_count() >= Self::MAX_OPERATORS {
            msg!("Already at maximum operators");
            return Err(ProgramError::InvalidArgument);
        }

        let duplicate_result = self.operators.iter().find(|operator_entry| {
            if let Some(operator_entry) = operator_entry.as_ref() {
                operator_entry.operator.eq(operator.operator())
            } else {
                false
            }
        });

        if duplicate_result.is_some() {
            msg!("Operator already exists");
            return Err(ProgramError::InvalidArgument);
        }

        self.operators[self.operator_count() as usize] = PodOption::some(OperatorEntry {
            operator: *operator.operator(),
            last_full_snapshot_slot: 0u64.into(),
            g1: *operator.g1(),
            reserved: [0; 16],
            weight_staked: 0u128.into(),
            weight_enqueued_for_cooldown: 0u128.into(),
            weight_cooling_down: 0u128.into(),
            snapshot_start_slot: PodOption::none(),
            snapshot_vault_index: PodOption::none(),
            snapshot_weight_staked: PodOption::none(),
            snapshot_weight_enqueued_for_cooldown: PodOption::none(),
            snapshot_weight_cooling_down: PodOption::none(),
        });

        match add_g1(&self.aggregate_g1, &operator.g1) {
            Ok(g1) => self.aggregate_g1 = g1,
            Err(err) => {
                msg!("Failed to add operator G1: {}", err);
                return Err(ProgramError::InvalidArgument);
            }
        }

        let operators_plus_one = self
            .operator_count()
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        self.update_operator_or_vault_count(
            current_slot,
            epoch_length,
            None,
            Some(operators_plus_one),
        )?;

        Ok(())
    }

    pub fn check_operator_index(
        &mut self,
        operator: &Pubkey,
        index: usize,
    ) -> Result<(), ProgramError> {
        if index >= self.operator_count() as usize {
            msg!("Invalid operator index");
            return Err(ProgramError::InvalidArgument);
        }

        if let Some(operator_entry_to_check) = self.operators[index].as_ref() {
            if operator_entry_to_check.operator.ne(operator) {
                msg!("Operator mismatch");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Operator not found");
            return Err(ProgramError::InvalidArgument);
        }

        Ok(())
    }

    pub fn remove_operator(
        &mut self,
        current_slot: u64,
        epoch_length: u64,
        operator: &Pubkey,
        index: usize,
    ) -> Result<(), ProgramError> {
        if !self.can_update_operators_or_vaults(current_slot, epoch_length)? {
            msg!("Cannot update");
            return Err(ProgramError::InvalidArgument);
        }

        self.check_operator_index(operator, index)?;

        let operator_entry_to_remove = self.operators[index]
            .copied()
            .ok_or(ProgramError::InvalidArgument)?;

        {
            // Wipe the old OperatorEntry
            self.operators[index] = PodOption::some(OperatorEntry::default()); // Wipe
            self.operators[index] = PodOption::none();
        }

        // Shift operators to fill the gap
        let end_index: usize = self.operator_count().saturating_sub(1) as usize;
        for i in index..end_index {
            let i_plus_one = i.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;
            self.operators[i] = self.operators[i_plus_one];
        }

        match sub_g1(&self.aggregate_g1, &operator_entry_to_remove.g1) {
            Ok(g1) => self.aggregate_g1 = g1,
            Err(err) => {
                msg!("Failed to remove operator G1: {}", err);
                return Err(ProgramError::InvalidArgument);
            }
        }

        let operators_minus_one = self
            .operator_count()
            .checked_sub(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        self.update_operator_or_vault_count(
            current_slot,
            epoch_length,
            None,
            Some(operators_minus_one),
        )?;

        Ok(())
    }

    /// This does not need to wait for the snapshot, it can be changed immediately.
    pub fn update_operator_g1(
        &mut self,
        operator: &Pubkey,
        index: usize,
        g1: &[u8; 64],
    ) -> Result<(), ProgramError> {
        self.check_operator_index(operator, index)?;

        let operator = self.operators[index]
            .as_mut()
            .ok_or(ProgramError::InvalidArgument)?;
        let old_g1 = operator.g1;
        operator.g1 = *g1;
        self.aggregate_g1 = sub_g1(&self.aggregate_g1, &old_g1).expect("Could not subtract G1");
        self.aggregate_g1 = add_g1(&self.aggregate_g1, &operator.g1).expect("Could not add G1");

        Ok(())
    }

    // Cannot add/remove vaults until all operators are updated
    pub fn are_all_operators_snapshotted(
        &self,
        current_slot: u64,
        epoch_length: u64,
    ) -> Result<bool, ProgramError> {
        for i in 0..self.operator_count() {
            if let Some(operator) = self.operators[i as usize].as_ref() {
                if !operator.snapshot_completed(current_slot, epoch_length)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn total_security(
        &self,
        current_slot: u64,
        epoch_length: u64,
    ) -> Result<u128, ProgramError> {
        let mut total_security: u128 = 0;
        for i in 0..self.operator_count() {
            if let Some(operator) = self.operators[i as usize].as_ref() {
                let operator_security = operator.total_security(current_slot, epoch_length)?;
                total_security = total_security.saturating_add(operator_security);
            }
        }
        Ok(total_security)
    }

    pub fn reached_consensus(
        &self,
        current_slot: u64,
        epoch_length: u64,
        signer_count: u16,
        weight_tally: u128,
    ) -> Result<bool, ProgramError> {
        let default_threshold = PodU16::from(DEFAULT_THRESHOLD_BPS);
        let consensus_threshold_bps = self
            .consensus_threshold_bps
            .as_ref()
            .unwrap_or(&default_threshold)
            .get();

        let (votes, total) = if self.consensus_weight_threshold.is_some() {
            // Each operator has a weight of 1
            let votes: u128 = signer_count as u128;
            let total: u128 = self.operator_count() as u128;

            let votes_bps = votes
                .checked_mul(BPS_PER_PERCENT as u128)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            let total_bps = total
                .checked_mul(BPS_PER_PERCENT as u128)
                .ok_or(ProgramError::ArithmeticOverflow)?;

            (votes_bps, total_bps)
        } else {
            (
                weight_tally,
                self.total_security(current_slot, epoch_length)?,
            )
        };

        if total == 0 {
            msg!("Total cannot be zero");
            return Err(ProgramError::InvalidArgument);
        }

        let total_threshold = total
            .checked_mul(consensus_threshold_bps as u128)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        let threshold = total_threshold
            .checked_div(BPS_PER_PERCENT as u128)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(votes.ge(&threshold))
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VaultEntry {
    /// The vault
    pub vault: Pubkey,
    /// The weight of the underlaying
    pub weight_bps: PodU16,
}

impl Default for VaultEntry {
    fn default() -> Self {
        Self {
            vault: Pubkey::default(),
            weight_bps: PodU16::from(0),
        }
    }
}

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy)]
#[repr(C)] //128 bytes
pub struct OperatorEntry {
    /// The operator
    pub operator: Pubkey,
    /// Last full updated slot
    pub last_full_snapshot_slot: PodU64,
    /// Valid weight
    pub weight_staked: PodU128,
    /// Valid enqueued weight
    pub weight_enqueued_for_cooldown: PodU128,
    /// Valid cooling down weight
    pub weight_cooling_down: PodU128,
    /// G1
    pub g1: [u8; 64],
    /// Reserved for future use
    pub reserved: [u8; 16],
    /// Snapshot Tracking
    pub snapshot_start_slot: PodOption<PodU64>,
    /// Snapshot current vault index
    pub snapshot_vault_index: PodOption<PodU16>,
    /// Snapshot current weight staked
    pub snapshot_weight_staked: PodOption<PodU128>,
    /// Snapshot current weight enqueued for cool down
    pub snapshot_weight_enqueued_for_cooldown: PodOption<PodU128>,
    /// Snapshot current weight cooling down
    pub snapshot_weight_cooling_down: PodOption<PodU128>,
}

impl Default for OperatorEntry {
    fn default() -> Self {
        Self {
            operator: Pubkey::default(),
            last_full_snapshot_slot: 0u64.into(),
            weight_staked: 0u128.into(),
            weight_enqueued_for_cooldown: 0u128.into(),
            weight_cooling_down: 0u128.into(),
            g1: [0; 64],
            reserved: [0; 16],
            snapshot_start_slot: PodOption::none(),
            snapshot_vault_index: PodOption::none(),
            snapshot_weight_staked: PodOption::none(),
            snapshot_weight_enqueued_for_cooldown: PodOption::none(),
            snapshot_weight_cooling_down: PodOption::none(),
        }
    }
}

impl OperatorEntry {
    pub fn last_full_snapshot_slot(&self) -> u64 {
        self.last_full_snapshot_slot.into()
    }

    pub fn clear_operator_weight(&mut self) {
        self.clear_last_state();
        self.weight_staked = PodU128::from(0);
        self.weight_enqueued_for_cooldown = PodU128::from(0);
        self.weight_cooling_down = PodU128::from(0);
    }

    pub fn clear_last_state(&mut self) {
        self.snapshot_start_slot = PodOption::none();
        self.snapshot_vault_index = PodOption::none();
        self.snapshot_weight_staked = PodOption::none();
        self.snapshot_weight_enqueued_for_cooldown = PodOption::none();
        self.snapshot_weight_cooling_down = PodOption::none();
    }

    pub fn new_last_state(&mut self, current_slot: u64) {
        self.snapshot_start_slot = PodOption::some(PodU64::from(current_slot));
        self.snapshot_vault_index = PodOption::some(PodU16::from(0));
        self.snapshot_weight_staked = PodOption::some(PodU128::from(0));
        self.snapshot_weight_enqueued_for_cooldown = PodOption::some(PodU128::from(0));
        self.snapshot_weight_cooling_down = PodOption::some(PodU128::from(0));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn snapshot(
        &mut self,
        current_slot: u64,
        epoch_length: u64,
        vault_index: u16,
        vault_count: u16,
        weight_staked: u128,
        weight_enqueued_for_cooldown: u128,
        weight_cooling_down: u128,
    ) -> Result<(), ProgramError> {
        if self.snapshot_completed(current_slot, epoch_length)? {
            msg!("Operator {} is up to date", self.operator);
            return Err(ProgramError::InvalidInstructionData);
        }

        // Initialize or reset snapshot state if starting new epoch
        if let Some(snapshot_start_slot) = self.snapshot_start_slot.as_ref() {
            let last_snapshot_epoch = get_epoch(snapshot_start_slot.get(), epoch_length)?;
            let current_epoch = get_epoch(current_slot, epoch_length)?;
            if current_epoch.ne(&last_snapshot_epoch) {
                self.new_last_state(current_slot);
            }
        } else {
            self.new_last_state(current_slot);
        }

        // Accumulate weights from this vault
        let current_vault_index = self
            .snapshot_vault_index
            .as_ref()
            .ok_or(ProgramError::InvalidInstructionData)?
            .get();

        if current_vault_index.ne(&vault_index) {
            msg!(
                "Incorrect Index: expected {}, got {}",
                current_vault_index,
                vault_index
            );
            return Err(ProgramError::InvalidInstructionData);
        }

        // Accumulate weights from this vault
        let current_weight_staked = self
            .snapshot_weight_staked
            .as_ref()
            .ok_or(ProgramError::InvalidInstructionData)?
            .get();
        let current_weight_enqueued = self
            .snapshot_weight_enqueued_for_cooldown
            .as_ref()
            .ok_or(ProgramError::InvalidInstructionData)?
            .get();
        let current_weight_cooling = self
            .snapshot_weight_cooling_down
            .as_ref()
            .ok_or(ProgramError::InvalidInstructionData)?
            .get();

        let new_weight_staked = current_weight_staked
            .checked_add(weight_staked)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        let new_weight_enqueued = current_weight_enqueued
            .checked_add(weight_enqueued_for_cooldown)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        let new_weight_cooling = current_weight_cooling
            .checked_add(weight_cooling_down)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Update accumulated weights
        self.snapshot_weight_staked = PodOption::some(PodU128::from(new_weight_staked));
        self.snapshot_weight_enqueued_for_cooldown =
            PodOption::some(PodU128::from(new_weight_enqueued));
        self.snapshot_weight_cooling_down = PodOption::some(PodU128::from(new_weight_cooling));

        // Increment vault index for next call
        let next_vault_index = vault_index
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        self.snapshot_vault_index = PodOption::some(PodU16::from(next_vault_index));

        // Check if we've processed all vaults
        if next_vault_index >= vault_count {
            // Finalize the snapshot by copying accumulated values to main fields
            self.weight_staked = PodU128::from(new_weight_staked);
            self.weight_enqueued_for_cooldown = PodU128::from(new_weight_enqueued);
            self.weight_cooling_down = PodU128::from(new_weight_cooling);
            self.last_full_snapshot_slot = PodU64::from(current_slot);

            // Clear snapshot state
            self.clear_last_state();
        }

        Ok(())
    }

    pub fn snapshot_completed(
        &self,
        current_slot: u64,
        epoch_length: u64,
    ) -> Result<bool, ProgramError> {
        let last_full_snapshot_epoch = get_epoch(self.last_full_snapshot_slot(), epoch_length)?;
        let current_epoch = get_epoch(current_slot, epoch_length)?;
        Ok(last_full_snapshot_epoch.eq(&current_epoch))
    }

    pub fn total_security(
        &self,
        current_slot: u64,
        epoch_length: u64,
    ) -> Result<u128, ProgramError> {
        let last_full_snapshot_slot = get_epoch(self.last_full_snapshot_slot(), epoch_length)?;
        let current_epoch = get_epoch(current_slot, epoch_length)?;
        let epoch_diff = current_epoch.saturating_sub(last_full_snapshot_slot);

        match epoch_diff {
            0 => {
                // All stake is still active
                let normal_stake = self.weight_staked.get();
                let cooling_down_stake = self
                    .weight_enqueued_for_cooldown
                    .get()
                    .saturating_add(self.weight_cooling_down.get());
                let total_security = normal_stake.saturating_add(cooling_down_stake);
                Ok(total_security)
            }
            1 => {
                // Stake in active cooldown has now been deactivated
                let normal_stake = self.weight_staked.get();
                let cooling_down_stake = self.weight_enqueued_for_cooldown.get();
                let total_security = normal_stake.saturating_add(cooling_down_stake);
                Ok(total_security)
            }
            2 => {
                // All of the stake could be in "cooling down" at this point, but still active
                let normal_stake = self.weight_staked.get();
                let cooling_down_stake = 0; // Cooling down stake
                let total_security = normal_stake.saturating_add(cooling_down_stake);
                Ok(total_security)
            }
            _ => Ok(0), // Its been too long since the last update, operator is considered inactive
        }
    }

    pub fn can_sign(
        &self,
        current_slot: u64,
        epoch_length: u64,
        consensus_weight_threshold: PodOption<PodU128>,
    ) -> Result<bool, ProgramError> {
        if let Some(consensus_weight_threshold) = consensus_weight_threshold.as_ref() {
            let total_security = self.total_security(current_slot, epoch_length)?;
            Ok(total_security.ge(&consensus_weight_threshold.get()))
        } else {
            Ok(true)
        }
    }
}
