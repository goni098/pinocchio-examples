use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{error::ProgramError, AccountView, ProgramResult};
use pinocchio_token_2022::instructions::TransferChecked;
use shank::ShankType;

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
    let [mint, from_token_account, to_token_account, authority, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate token program is Token-2022
    if token_program.address().ne(&pinocchio_token_2022::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

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

    Ok(())
}

#[cfg(test)]
mod test {}
