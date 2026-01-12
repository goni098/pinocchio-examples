// #![no_std]
#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{entrypoint, error::ProgramError, AccountView, Address, ProgramResult};
use pinocchio_pubkey::declare_id;
use shank::ShankInstruction;

use crate::instructions::{
    increase_counter, increase_couter_authority, init_counter, init_counter_authority,
};

mod accounts;
mod error;
mod instructions;

entrypoint!(process);

declare_id!("8F1XtWR4wTs37nnutBvd2MWpCTfb7XAciFYkw5XHaENj");

#[derive(ShankInstruction, BorshDeserialize, BorshSerialize)]
pub enum CounterInstruction {
    #[account(0, sig, name = "payer")]
    #[account(1, mut, name = "counter")]
    #[account(2, name = "system_program")]
    InitCounter(init_counter::Params),

    #[account(0, mut, name = "counter")]
    IncreaseCounter,

    #[account(0, sig, name = "payer")]
    #[account(1, mut, name = "counter_authority")]
    #[account(2, name = "system_program")]
    InitCounterAuhthority(init_counter_authority::Params),

    #[account(0, sig, name = "authority")]
    #[account(1, mut, name = "counter_authority")]
    IncreaseCounterAuthority,
}

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    match CounterInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        CounterInstruction::InitCounter(params) => {
            init_counter::process(program_id, accounts, params)
        }
        CounterInstruction::IncreaseCounter => increase_counter::process(program_id, accounts),
        CounterInstruction::InitCounterAuhthority(params) => {
            init_counter_authority::process(program_id, accounts, params)
        }
        CounterInstruction::IncreaseCounterAuthority => {
            increase_couter_authority::process(program_id, accounts)
        }
    }
}
