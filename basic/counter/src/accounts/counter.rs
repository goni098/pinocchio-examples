use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::Address;
use shank::ShankAccount;

use crate::ID;

#[derive(BorshDeserialize, BorshSerialize, ShankAccount)]
pub struct Counter {
    pub bump: u8,
    pub count: u64,
}

impl Counter {
    pub const SPACE: usize = 1  // bump
        + 8; //count

    pub const SEED_PREFIX: &[u8; 7] = b"counter";

    pub fn derive() -> (Address, u8) {
        Address::find_program_address(&[Self::SEED_PREFIX], &ID.into())
    }
}
