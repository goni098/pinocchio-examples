use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};
use pinocchio_token_2022::instructions::TransferChecked;
use shank::ShankType;
use solana_address::Address;

use super::MINT_SEED_PREFIX;
use crate::ID;

#[derive(BorshDeserialize, BorshSerialize, ShankType)]
pub struct TransferMintArgs {
    /// Amount of tokens to transfer
    pub amount: u64,
    /// Decimals of the token
    pub decimals: u8,
}

pub fn transfer_mint(
    _program_id: &pinocchio::Address,
    accounts: &[AccountView],
    args: TransferMintArgs,
) -> ProgramResult {
    let [from_token_account, mint, to_token_account, authority, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate token program is Token-2022
    if token_program.address().ne(&pinocchio_token_2022::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Check if authority is a signer or if we need to use PDA signing
    if authority.is_signer() {
        // Direct transfer with signer authority
        TransferChecked {
            from: from_token_account,
            mint,
            to: to_token_account,
            authority,
            amount: args.amount,
            decimals: args.decimals,
            token_program: token_program.address(),
        }
        .invoke()?;
    } else {
        // PDA transfer - authority is the mint PDA
        let (pda, bump) =
            Address::find_program_address(&[MINT_SEED_PREFIX], &Address::new_from_array(ID));

        if authority.address().as_array().ne(pda.as_array()) {
            return Err(ProgramError::InvalidSeeds);
        }

        let bump_bytes = &[bump];
        let seeds = [
            Seed::from(MINT_SEED_PREFIX.as_slice()),
            Seed::from(bump_bytes),
        ];
        let signers = Signer::from(&seeds);

        TransferChecked {
            from: from_token_account,
            mint,
            to: to_token_account,
            authority,
            amount: args.amount,
            decimals: args.decimals,
            token_program: token_program.address(),
        }
        .invoke_signed(&[signers])?;
    }

    Ok(())
}
