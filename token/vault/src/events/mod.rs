use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;

mod swap_exact_in_executed;
mod swap_exact_out_executed;
mod vault_created;

pub use swap_exact_in_executed::*;
pub use swap_exact_out_executed::*;
pub use vault_created::*;

#[derive(BorshSerialize, BorshDeserialize, ShankType)]
#[allow(clippy::enum_variant_names)]
pub enum VaultEvent {
    VaultCreated(VaultCreated),
    SwapExactInExecuted(SwapExactInExecuted),
    SwapExactOutExecuted(SwapExactOutExecuted),
}
