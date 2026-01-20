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
    increase_counter, increase_counter_authority, init_counter, init_counter_authority,
    InitCounterArgs, InitCounterAuthorityArgs,
};

mod accounts;
mod events;
mod instructions;

program_entrypoint!(process);
no_allocator!();
default_panic_handler!();

declare_id!("8F1XtWR4wTs37nnutBvd2MWpCTfb7XAciFYkw5XHaENj");

#[derive(ShankInstruction, BorshDeserialize, BorshSerialize)]
pub enum CounterInstruction {
    #[account(0, sig, name = "payer")]
    #[account(1, mut, name = "counter")]
    #[account(2, name = "system_program")]
    InitCounter(InitCounterArgs),

    #[account(0, mut, name = "counter")]
    IncreaseCounter,

    #[account(0, sig, name = "payer")]
    #[account(1, mut, name = "counter_authority")]
    #[account(2, name = "system_program")]
    InitCounterAuhthority(InitCounterAuthorityArgs),

    #[account(0, sig, name = "authority")]
    #[account(1, mut, name = "counter_authority")]
    IncreaseCounterAuthority,
}

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if !crate::check_id(program_id.as_array()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    match CounterInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        CounterInstruction::InitCounter(params) => init_counter(program_id, accounts, params),
        CounterInstruction::IncreaseCounter => increase_counter(program_id, accounts),
        CounterInstruction::InitCounterAuhthority(params) => {
            init_counter_authority(program_id, accounts, params)
        }
        CounterInstruction::IncreaseCounterAuthority => {
            increase_counter_authority(program_id, accounts)
        }
    }
}
