use shank::ShankInstruction;

#[rustfmt::skip]
#[derive(Debug, ShankInstruction)]
pub enum JitoBlsNCNInstructions {

    // ---------------------------------------------------- //
    //                         GLOBAL                       //
    // ---------------------------------------------------- //
    /// Initialize the config account for the NCN program
    /// Sets up the basic program parameters
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "ncn_fee_wallet")]
    #[account(3, signer, name = "ncn_admin")]
    #[account(4, name = "tie_breaker_admin")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    InitializeConfig {
        /// Number of epochs before voting is considered stalled
        epochs_before_stall: u64,
        /// Number of epochs after consensus before accounts can be closed
        epochs_after_consensus_before_close: u64,
        /// Number of slots after consensus where voting is still valid
        valid_slots_after_consensus: u64,
        /// Minimum stake for a validator to be considered valid
        minimum_stake: u128,
        /// NCN fee basis points (bps) for the NCN program
        ncn_fee_bps: u16,
    },
}
