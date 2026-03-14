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
    create_profile, delete_post, init_board, post_message, suspend_profile, update_username,
};
use crate::types::{CreateProfileArgs, InitBoardArgs, PostMessageArgs, UpdateUsernameArgs};

mod accounts;
mod errors;
mod events;
mod instructions;
mod types;

program_entrypoint!(process);
no_allocator!();
default_panic_handler!();

declare_id!("FuLLShankNoteBoardProg111111111111111111111");

/// The PostBoard instruction set.
///
/// This enum exercises every shank account attribute variant:
///   `sig`              → `isSigner: true`
///   `mut`              → `isMut: true`
///   `opt`              → `isOptional: true`
///   `optional_signer`  → `isOptionalSigner: true`
///   `desc = "…"`       → `docs: […]`
#[derive(ShankInstruction, BorshDeserialize, BorshSerialize)]
pub enum PostBoardInstruction {
    /// Initialize the global board configuration (run once by the admin).
    #[account(0, sig, name = "admin", desc = "Board administrator; pays for config creation")]
    #[account(1, mut, name = "config", desc = "Board config PDA (seeds: [\"board\"])")]
    #[account(2, name = "system_program", desc = "System program")]
    InitBoard(InitBoardArgs),

    /// Register a new user profile.
    ///
    /// An optional referrer account may be provided; the program reads it
    /// (but does not write it) to validate referral relationships.
    #[account(0, sig, name = "payer", desc = "Fee payer; becomes the profile authority")]
    #[account(1, mut, name = "profile", desc = "User profile PDA (seeds: [\"profile\", payer])")]
    #[account(2, opt, name = "referrer", desc = "Optional referrer profile — omit if not using referrals")]
    #[account(3, name = "system_program", desc = "System program")]
    CreateProfile(CreateProfileArgs),

    /// Publish a new message (optionally as a reply to an existing post).
    #[account(0, sig, name = "author", desc = "Message author")]
    #[account(1, mut, name = "profile", desc = "Author's profile PDA")]
    #[account(2, mut, name = "post", desc = "New post PDA (seeds: [\"post\", author, post_count])")]
    #[account(3, name = "system_program", desc = "System program")]
    PostMessage(PostMessageArgs),

    /// Update the caller's display username.
    #[account(0, sig, name = "authority", desc = "Profile authority (owner)")]
    #[account(1, mut, name = "profile", desc = "User profile PDA to update")]
    UpdateUsername(UpdateUsernameArgs),

    /// Delete a post and reclaim its lamports.
    ///
    /// Either the author or the board admin can delete a post.
    /// When called by the admin, the optional `config` account must be supplied.
    #[account(0, sig, name = "authority", desc = "Author or admin initiating the deletion")]
    #[account(1, mut, name = "post", desc = "The post PDA to close")]
    #[account(2, mut, name = "refund_to", desc = "Destination for the reclaimed lamports")]
    #[account(3, opt, name = "config", desc = "Board config — required only when the caller is the admin")]
    DeletePost,

    /// Suspend a user profile (admin-only).
    ///
    /// An optional `co_admin` signer may be required by governance rules;
    /// the program accepts but does not mandate it.
    #[account(0, sig, name = "admin", desc = "Board administrator")]
    #[account(1, mut, name = "config", desc = "Board config PDA (validates admin key)")]
    #[account(2, mut, name = "profile", desc = "User profile to suspend")]
    #[account(3, optional_signer, name = "co_admin", desc = "Optional second admin co-signer for governance")]
    SuspendProfile,
}

pub fn process(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if !crate::check_id(program_id.as_array()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    match PostBoardInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        PostBoardInstruction::InitBoard(args) => init_board(program_id, accounts, args),
        PostBoardInstruction::CreateProfile(args) => create_profile(program_id, accounts, args),
        PostBoardInstruction::PostMessage(args) => post_message(program_id, accounts, args),
        PostBoardInstruction::UpdateUsername(args) => update_username(program_id, accounts, args),
        PostBoardInstruction::DeletePost => delete_post(program_id, accounts),
        PostBoardInstruction::SuspendProfile => suspend_profile(program_id, accounts),
    }
}
