#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Discriminators {
    RollingSnapshot = 0x01,
    BlsOperator = 0x10,
    Config = 0x20,
    Consensus = 0x30,
}
