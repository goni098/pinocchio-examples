use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::{accounts::CounterAuthority, ID};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Params {
    pub count: u64,
}

pub fn process(program_id: &Address, accounts: &[AccountView], params: Params) -> ProgramResult {
    if program_id.as_array().ne(&ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let [payer, counter_authority, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, bump) = CounterAuthority::derive(payer.address());

    if counter_authority.address().ne(&pda) {
        return Err(ProgramError::InvalidSeeds);
    }

    if counter_authority.lamports().ne(&0) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if system_program.address().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    };

    let counter_data = CounterAuthority {
        bump,
        count: params.count,
        authority: payer.address().clone(),
    };

    let account_span = CounterAuthority::SPACE;
    let lamports_required = Rent::get()?.minimum_balance_unchecked(account_span);

    let bump_bytes = &[bump];
    let seeds = [
        Seed::from(CounterAuthority::SEED_PREFIX),
        Seed::from(payer.address().as_array()),
        Seed::from(bump_bytes),
    ];

    let signers = Signer::from(&seeds);

    CreateAccount {
        from: payer,
        to: counter_authority,
        lamports: lamports_required,
        space: account_span as u64,
        owner: program_id,
    }
    .invoke_signed(&[signers])?;

    counter_data
        .serialize(&mut counter_authority.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(())
}
#[cfg(test)]
mod test {
    extern crate std;

    use borsh::BorshDeserialize;
    use litesvm::LiteSVM;
    use pinocchio::Address;
    use solana_sdk::{
        instruction::Instruction, message::AccountMeta, native_token::LAMPORTS_PER_SOL,
        signature::Keypair, signer::Signer, transaction::Transaction,
    };

    use crate::{accounts::CounterAuthority, CounterInstruction, ID};

    #[test]
    fn init_counter_authority() {
        let mut svm = LiteSVM::new();

        let payer = Keypair::new();

        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/counter.so")
            .unwrap();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let (counter, _) = CounterAuthority::derive(&payer.pubkey());

        let ix_data = CounterInstruction::InitCounterAuhthority(super::Params { count: 19 });

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(counter, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&ix_data).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();

        std::println!("Program executed successfully!");
        std::println!("Transaction logs: {:#?}", result.logs);

        let counter = svm.get_account(&counter).unwrap();

        let counter_data = CounterAuthority::deserialize(&mut counter.data.as_ref()).unwrap();

        assert_eq!(counter_data.count, 19);
        assert_eq!(counter_data.authority, payer.pubkey());
    }
}
