#![no_std]
#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    default_panic_handler, error::ProgramError, no_allocator, program_entrypoint, AccountView,
    Address, ProgramResult,
};
use pinocchio_pubkey::declare_id;
use shank::ShankInstruction;

use crate::instructions::{close_meme, create_meme};

mod accounts;
mod instructions;

program_entrypoint!(process);
no_allocator!();
default_panic_handler!();

declare_id!("2HXWQuEjgRDbNcMx3X32C1aw4fftVMHyUf9KXYyTiPiD");

#[derive(BorshDeserialize, BorshSerialize, ShankInstruction)]
pub enum CloseAccountInstruction {
    #[account(0, sig, name = "payer")]
    #[account(1, mut, name = "meme")]
    #[account(2, name = "system_program")]
    CreateMeme,

    #[account(0, sig, name = "payer")]
    #[account(1, mut, name = "meme")]
    #[account(2, name = "system_program")]
    CloseMeme,
}

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if !crate::check_id(program_id.as_array()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    match CloseAccountInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        CloseAccountInstruction::CreateMeme => create_meme::process(program_id, accounts),
        CloseAccountInstruction::CloseMeme => close_meme::process(program_id, accounts),
    }
}
