use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;

#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct CounterInitialized {
    pub count: u64,
}
