#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum Discriminators {
    RollingSnapshot = 0x01,
    BlsOperator = 0x10,
    Config = 0x20,
    Consensus = 0x30,
}

// Compile-time assertions
const _: () = assert!(Discriminators::RollingSnapshot as u64 != 0);
const _: () = assert!(Discriminators::BlsOperator as u64 != 0);
const _: () = assert!(Discriminators::Config as u64 != 0);
const _: () = assert!(Discriminators::Consensus as u64 != 0);
