use base64::{prelude::BASE64_STANDARD, Engine};
use borsh::BorshSerialize;
use pinocchio::{error::ProgramError, ProgramResult};

/// Panic if no_alloc is enable
/// Encode base 64 require String allocation
pub fn emit<E: BorshSerialize>(event: &E) -> ProgramResult {
    let event_data = borsh::to_vec(event).map_err(|_| ProgramError::BorshIoError)?;

    pinocchio_log::log!(
        "instruction data: {}",
        BASE64_STANDARD.encode(event_data).as_str()
    );

    Ok(())
}
