use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;

#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct CounterAuthorityIncreased {
    pub new_count: u64,
}
