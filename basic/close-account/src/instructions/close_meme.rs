use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

pub fn process(_program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
    let [payer, meme, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let leaving_lamports = 0;
    let taking_lamports = meme.lamports() - leaving_lamports;

    meme.set_lamports(leaving_lamports);
    payer.set_lamports(payer.lamports() + taking_lamports);

    meme.resize(0)?;

    unsafe {
        meme.assign(system_program.address());
    }

    Ok(())
}

#[cfg(test)]
mod test {
    extern crate std;

    use litesvm::LiteSVM;
    use pinocchio::Address;
    use solana_sdk::{
        account::Account, instruction::Instruction, message::AccountMeta,
        native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer,
        transaction::Transaction,
    };

    use crate::{accounts::Meme, CloseAccountInstruction, ID};

    #[test]
    fn close_meme() {
        let mut svm = LiteSVM::new();

        let payer = Keypair::new();

        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/close_account.so")
            .unwrap();

        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let (meme, bump) = Meme::derive();

        svm.set_account(
            meme,
            Account {
                data: borsh::to_vec(&Meme {
                    bump,
                    address: meme,
                })
                .unwrap(),
                executable: false,
                lamports: LAMPORTS_PER_SOL,
                owner: ID.into(),
                rent_epoch: 0,
            },
        )
        .unwrap();

        let ix_data = CloseAccountInstruction::CloseMeme;

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(meme, false),
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

        let meme = svm.get_account(&meme);

        assert!(meme.is_none());
    }
}
