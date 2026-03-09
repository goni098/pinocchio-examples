use pinocchio::{cpi::Signer, error::ProgramError, AccountView, Address, ProgramResult};

/// Validate that `program` is either SPL Token or Token-2022.
pub fn validate_token_program(program: &AccountView) -> ProgramResult {
    let addr = program.address();
    if addr.ne(&pinocchio_token_2022::ID) && addr.ne(&pinocchio_token::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Dispatch a token Transfer CPI using either SPL Token or Token-2022.
pub fn transfer_tokens(
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
    token_program_id: &Address,
    signers: &[Signer],
) -> ProgramResult {
    if token_program_id.eq(&pinocchio_token_2022::ID) {
        pinocchio_token_2022::instructions::Transfer {
            from,
            to,
            authority,
            amount,
            token_program: token_program_id,
        }
        .invoke_signed(signers)
    } else {
        pinocchio_token::instructions::Transfer::new(from, to, authority, amount)
            .invoke_signed(signers)
    }
}
