//! Slot toggled state tracker, useful for activations and deactivations of certain features
//! based on slot time.

use std::cmp::Ordering;
use crate::pod::PodU64;
use solana_program::program_error::ProgramError;

/// SlotToggle is a state tracker that allows for activation and deactivation of certain features
/// based on slot time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SlotToggle {
    /// The slot at which the feature was added
    pub slot_added: PodU64,
    /// The slot at which the feature was removed
    pub slot_removed: PodU64,
    pub reserved: [u8; 32],
}

/// The state of the SlotToggle
#[derive(Debug, PartialEq, Eq)]
pub enum SlotToggleState {
    /// The feature is inactive
    Inactive,
    /// The feature is in warm-up state
    WarmUp,
    /// The feature is active
    Active,
    /// The feature is in cooldown state
    Cooldown,
}

impl SlotToggle {
    /// Get the slot at which the feature was added
    pub fn slot_added(&self) -> u64 {
        self.slot_added.into()
    }

    /// Get the slot at which the feature was removed
    pub fn slot_removed(&self) -> u64 {
        self.slot_removed.into()
    }

    /// Check if the feature is active or in cooldown state at the given slot.
    pub fn is_active_or_cooldown(
        &self,
        slot: u64,
        epoch_length: u64,
    ) -> Result<bool, ProgramError> {
        Ok(matches!(
            self.state(slot, epoch_length)?,
            SlotToggleState::Active | SlotToggleState::Cooldown
        ))
    }

    /// Check if the feature is active at the given slot.
    pub fn is_active(&self, slot: u64, epoch_length: u64) -> Result<bool, ProgramError> {
        Ok(matches!(
            self.state(slot, epoch_length)?,
            SlotToggleState::Active
        ))
    }

    /// Get the state of the feature at the given slot.
    /// The state is determined based on the slot time and the epoch length.
    ///
    /// # Arguments
    /// * `slot` - The slot at which the state is being queried
    /// * `epoch_length` - The length of an epoch in slots
    ///
    /// # Returns
    /// * `SlotToggleState` - The state of the feature at the given slot
    pub fn state(&self, slot: u64, epoch_length: u64) -> Result<SlotToggleState, ProgramError> {
        let current_epoch = get_epoch(slot, epoch_length)?;

        let slot_added: u64 = self.slot_added.into();
        let slot_removed: u64 = self.slot_removed.into();

        match slot_added.cmp(&slot_removed) {
            Ordering::Equal => Ok(SlotToggleState::Inactive),
            Ordering::Less => {
                let slot_removed_epoch = get_epoch(slot_removed, epoch_length)?;
                if current_epoch
                    > slot_removed_epoch
                        .checked_add(1)
                        .ok_or(ProgramError::ArithmeticOverflow)?
                {
                    Ok(SlotToggleState::Inactive)
                } else {
                    Ok(SlotToggleState::Cooldown)
                }
            }
            Ordering::Greater => {
                let slot_added_epoch = get_epoch(slot_added, epoch_length)?;
                if current_epoch
                    > slot_added_epoch
                        .checked_add(1)
                        .ok_or(ProgramError::ArithmeticOverflow)?
                {
                    Ok(SlotToggleState::Active)
                } else {
                    Ok(SlotToggleState::WarmUp)
                }
            }
        }
    }
}

/// Helper function to calculate epoch from slot
fn get_epoch(slot: u64, epoch_length: u64) -> Result<u64, ProgramError> {
    slot.checked_div(epoch_length)
        .ok_or(ProgramError::ArithmeticOverflow)
}
