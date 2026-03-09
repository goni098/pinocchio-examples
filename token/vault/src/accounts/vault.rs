use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::Address;
use shank::ShankAccount;

use crate::ID;

#[derive(BorshDeserialize, BorshSerialize, ShankAccount)]
pub struct Vault {
    /// Owner of the vault.
    pub owner: [u8; 32],
    /// Random ID used as PDA seed.
    pub vault_id: [u8; 32],
    /// Mint of token A.
    pub token_mint_a: [u8; 32],
    /// Mint of token B.
    pub token_mint_b: [u8; 32],
    /// Vault's token account for mint A (authority = vault PDA).
    pub token_account_a: [u8; 32],
    /// Vault's token account for mint B (authority = vault PDA).
    pub token_account_b: [u8; 32],
    /// Token program for mint A (SPL Token or Token-2022).
    pub token_program_a: [u8; 32],
    /// Token program for mint B (SPL Token or Token-2022).
    pub token_program_b: [u8; 32],
    /// Current reserve of token A.
    pub reserve_a: u64,
    /// Current reserve of token B.
    pub reserve_b: u64,
    /// Constant product k = reserve_a * reserve_b.
    pub k: u128,
    /// PDA bump.
    pub bump: u8,
}

impl Vault {
    pub const SPACE: usize = 32  // owner
        + 32  // vault_id
        + 32  // token_mint_a
        + 32  // token_mint_b
        + 32  // token_account_a
        + 32  // token_account_b
        + 32  // token_program_a
        + 32  // token_program_b
        + 8   // reserve_a
        + 8   // reserve_b
        + 16  // k (u128)
        + 1; // bump

    pub const SEED_PREFIX: &[u8; 5] = b"vault";

    pub fn derive(owner: &Address, vault_id: &[u8; 32]) -> (Address, u8) {
        let prefix: &[u8] = Self::SEED_PREFIX;
        let owner_bytes: &[u8] = owner.as_array();
        let id_bytes: &[u8] = vault_id;
        Address::find_program_address(&[prefix, owner_bytes, id_bytes], &ID.into())
    }
}
