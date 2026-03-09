#![no_std]
#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    default_panic_handler, error::ProgramError, no_allocator, program_entrypoint, AccountView,
    Address, ProgramResult,
};
use pinocchio_pubkey::declare_id;
use shank::ShankInstruction;

use crate::instructions::{
    create_vault, swap_exact_in, swap_exact_out, CreateVaultArgs, SwapExactInArgs, SwapExactOutArgs,
};

mod accounts;
mod events;
pub mod instructions;

program_entrypoint!(process);
no_allocator!();
default_panic_handler!();

declare_id!("9pq4Z25ahsUMFZ9TrKH1yhmibLbGkxxAdbTn7FUJ7reV");

#[derive(ShankInstruction, BorshDeserialize, BorshSerialize)]
pub enum VaultInstruction {
    /// Create a new AMM vault with initial liquidity.
    #[account(0, sig, name = "owner", desc = "Vault owner; pays for account creation and provides initial liquidity")]
    #[account(1, mut, name = "vault", desc = "Vault PDA (seeds: ['vault', owner, vault_id])")]
    #[account(2, mut, name = "user_token_account_a", desc = "Owner's token A account")]
    #[account(3, mut, name = "user_token_account_b", desc = "Owner's token B account")]
    #[account(4, mut, name = "vault_token_account_a", desc = "Vault's token A account (authority = vault PDA)")]
    #[account(5, mut, name = "vault_token_account_b", desc = "Vault's token B account (authority = vault PDA)")]
    #[account(6, name = "token_program_a", desc = "SPL Token or Token-2022 program for mint A")]
    #[account(7, name = "token_program_b", desc = "SPL Token or Token-2022 program for mint B")]
    #[account(8, name = "system_program", desc = "System program")]
    CreateVault(CreateVaultArgs),

    /// Swap an exact amount of tokens in for a calculated amount out.
    #[account(0, sig, name = "user", desc = "Signer and authority of user token accounts")]
    #[account(1, mut, name = "vault", desc = "Vault PDA")]
    #[account(2, mut, name = "user_token_in", desc = "User's source token account")]
    #[account(3, mut, name = "vault_token_in", desc = "Vault's matching source token account")]
    #[account(4, mut, name = "user_token_out", desc = "User's destination token account")]
    #[account(5, mut, name = "vault_token_out", desc = "Vault's matching destination token account")]
    #[account(6, name = "token_program_in", desc = "SPL Token or Token-2022 program for the input token")]
    #[account(7, name = "token_program_out", desc = "SPL Token or Token-2022 program for the output token")]
    SwapExactIn(SwapExactInArgs),

    /// Swap tokens to receive an exact desired output amount.
    #[account(0, sig, name = "user", desc = "Signer and authority of user token accounts")]
    #[account(1, mut, name = "vault", desc = "Vault PDA")]
    #[account(2, mut, name = "user_token_in", desc = "User's source token account")]
    #[account(3, mut, name = "vault_token_in", desc = "Vault's matching source token account")]
    #[account(4, mut, name = "user_token_out", desc = "User's destination token account")]
    #[account(5, mut, name = "vault_token_out", desc = "Vault's matching destination token account")]
    #[account(6, name = "token_program_in", desc = "SPL Token or Token-2022 program for the input token")]
    #[account(7, name = "token_program_out", desc = "SPL Token or Token-2022 program for the output token")]
    SwapExactOut(SwapExactOutArgs),
}

pub fn process(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if !crate::check_id(program_id.as_array()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    match VaultInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        VaultInstruction::CreateVault(args) => create_vault(program_id, accounts, args),
        VaultInstruction::SwapExactIn(args) => swap_exact_in(program_id, accounts, args),
        VaultInstruction::SwapExactOut(args) => swap_exact_out(program_id, accounts, args),
    }
}
