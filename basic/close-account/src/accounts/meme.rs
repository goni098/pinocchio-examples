use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::Address;

#[derive(BorshDeserialize, BorshSerialize, shank::ShankAccount)]
pub struct Meme {
    #[idl_type("[u8;32]")]
    pub address: Address,
    pub bump: u8,
}

impl Meme {
    pub const SPACE: usize = 1  // bump
        + 8     //count
        + 32; //address

    pub const SEED_PREFIX: &[u8; 4] = b"meme";

    pub fn derive() -> (Address, u8) {
        Address::find_program_address(&[Self::SEED_PREFIX], &crate::ID.into())
    }
}
