use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;

#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct SwapExactInExecuted {
    pub vault: [u8; 32],
    pub amount_in: u64,
    pub amount_out: u64,
    pub reserve_a: u64,
    pub reserve_b: u64,
}
