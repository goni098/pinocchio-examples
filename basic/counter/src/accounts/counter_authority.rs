use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::Address;
use shank::ShankAccount;

use crate::ID;

#[derive(BorshDeserialize, BorshSerialize, ShankAccount)]
pub struct CounterAuthority {
    pub authority: Address,
    pub bump: u8,
    pub count: u64,
}

impl CounterAuthority {
    pub const SPACE: usize = 32  // authority
        + 1  // bump
        + 8; // count

    pub const SEED_PREFIX: &[u8; 17] = b"counter_authority";

    pub fn derive(user: &Address) -> (Address, u8) {
        Address::find_program_address(&[Self::SEED_PREFIX, user.as_array()], &ID.into())
    }
}
