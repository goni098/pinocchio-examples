#![no_std]
#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{entrypoint, error::ProgramError, AccountView, Address, ProgramResult};
use pinocchio_pubkey::declare_id;
use shank::ShankInstruction;

use crate::instructions::{create_mint, transfer_mint, CreateMintArgs, TransferMintArgs};

mod instructions;

entrypoint!(process);

declare_id!("8F1XtWR4wTs37nnutBvd2MWpCTfb7XAciFYkw5XHaENj");

#[derive(ShankInstruction, BorshDeserialize, BorshSerialize)]
pub enum BasicMintInstruction {
    /// Create a new mint with token metadata
    #[account(0, sig, name = "payer", desc = "The payer for the mint creation")]
    #[account(1, mut, name = "mint", desc = "The mint account (PDA)")]
    #[account(2, name = "token_program", desc = "Token-2022 program")]
    #[account(3, name = "system_program", desc = "System program")]
    CreateMint(CreateMintArgs),

    /// Transfer tokens between token accounts
    #[account(0, mut, name = "from_token_account", desc = "Source token account")]
    #[account(1, name = "mint", desc = "The token mint")]
    #[account(2, mut, name = "to_token_account", desc = "Destination token account")]
    #[account(3, sig, name = "authority", desc = "Authority/owner of the source token account")]
    #[account(4, name = "token_program", desc = "Token-2022 program")]
    TransferMint(TransferMintArgs),
}

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if !crate::check_id(program_id.as_array()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    match BasicMintInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        BasicMintInstruction::CreateMint(args) => create_mint(program_id, accounts, args),
        BasicMintInstruction::TransferMint(args) => transfer_mint(program_id, accounts, args),
    }
}
