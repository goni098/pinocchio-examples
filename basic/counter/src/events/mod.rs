use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;

mod increase_counter;
mod increase_counter_authority;
mod init_counter;
mod init_counter_authority;

pub use increase_counter::*;
pub use increase_counter_authority::*;
pub use init_counter::*;
pub use init_counter_authority::*;

#[derive(BorshSerialize, BorshDeserialize, ShankType)]
#[allow(clippy::enum_variant_names)]
pub enum CounterEvent {
    CounterInitialized(CounterInitialized),
    CounterIncreased(CounterIncreased),
    CounterAuthorityInitialized(CounterAuthorityInitialized),
    CounterAuthorityIncreased(CounterAuthorityIncreased),
}
