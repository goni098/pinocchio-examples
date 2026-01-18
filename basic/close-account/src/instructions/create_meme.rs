use borsh::BorshSerialize;
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::accounts::Meme;

pub fn process(program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
    let [payer, meme, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, bump) = Meme::derive();

    if meme.address().ne(&pda) {
        return Err(ProgramError::InvalidSeeds);
    }

    if meme.lamports().ne(&0) || meme.data_len().ne(&0) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if system_program.address().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    };

    let meme_data = Meme { bump, address: pda };

    let account_span = Meme::SPACE;
    let lamports_required = Rent::get()?.minimum_balance_unchecked(account_span);

    let bump_bytes = &[bump];
    let seeds = [Seed::from(Meme::SEED_PREFIX), Seed::from(bump_bytes)];

    let signers = Signer::from(&seeds);

    CreateAccount {
        from: payer,
        to: meme,
        lamports: lamports_required,
        space: account_span as u64,
        owner: program_id,
    }
    .invoke_signed(&[signers])?;

    meme_data
        .serialize(&mut meme.try_borrow_mut()?.as_mut())
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

    use crate::{accounts::Meme, CloseAccountInstruction, ID};

    #[test]
    fn create_meme() {
        let mut svm = LiteSVM::new();

        let payer = Keypair::new();

        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/close_account.so")
            .unwrap();

        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let (meme_addr, bump) = Meme::derive();

        let ix_data = CloseAccountInstruction::CreateMeme;

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(meme_addr, false),
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

        let meme = svm.get_account(&meme_addr).unwrap();

        let meme_data = Meme::deserialize(&mut meme.data.as_ref()).unwrap();

        assert_eq!(meme_data.bump, bump);
        assert_eq!(meme_data.address, meme_addr)
    }
}
