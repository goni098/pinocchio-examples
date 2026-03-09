use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;

#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct VaultCreated {
    pub vault: [u8; 32],
    pub owner: [u8; 32],
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub k: u128,
}
