use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

use crate::{accounts::Counter, ID};

pub fn process(program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
    if program_id.as_array().ne(&ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let [counter] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if counter.lamports().eq(&0) {
        return Err(ProgramError::UninitializedAccount);
    }

    let (pda, _) = Counter::derive();

    if counter.address().ne(&pda) {
        return Err(ProgramError::InvalidSeeds);
    }

    let mut counter_data = Counter::try_from_slice(&counter.try_borrow()?)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    counter_data.count += 1;

    counter_data
        .serialize(&mut counter.try_borrow_mut()?.as_mut())
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
        account::Account,
        message::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    };

    use crate::{accounts::Counter, CounterInstruction, ID};

    #[test]
    fn increase_counter() {
        let mut svm = LiteSVM::new();

        let payer = Keypair::new();

        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/counter.so")
            .unwrap();

        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let (counter, _) = Counter::derive();

        svm.set_account(
            counter,
            Account {
                data: borsh::to_vec(&Counter {
                    count: 19,
                    bump: 254,
                })
                .unwrap(),
                executable: false,
                lamports: LAMPORTS_PER_SOL,
                owner: ID.into(),
                rent_epoch: 0,
            },
        )
        .unwrap();

        let ix_data = CounterInstruction::IncreaseCounter;

        let ix = Instruction {
            program_id,
            accounts: [AccountMeta::new(counter, false)].to_vec(),
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

        let counter_data = Counter::deserialize(&mut counter.data.as_ref()).unwrap();

        assert_eq!(counter_data.count, 20);
    }
}
