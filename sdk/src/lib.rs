use solana_pubkey::Pubkey;
use jito_bls_ncn_core::accounts::*;

// pub mod accounts {
//     pub use jito_bls_ncn_core::accounts::*;
// }

// pub mod instructions {
//     pub use jito_bls_ncn_core::instructions::*;
// }

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

pub fn rolling_snapsshot_address(ncn: &Pubkey) -> (Pubkey, u8) {
    let (key, bump, _) = rolling_snapshot::RollingSnapshot::offchain_find_program_address(&id(), ncn);
    (key, bump)
}
