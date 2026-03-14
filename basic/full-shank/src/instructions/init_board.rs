use borsh::BorshSerialize;
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use shared::event::emit;

use crate::{
    accounts::BoardConfig,
    errors::PostBoardError,
    events::{BoardEvent, BoardInitialized},
    types::InitBoardArgs,
};

/// Accounts: [admin(sig), config(mut), system_program]
pub fn init_board(
    program_id: &Address,
    accounts: &mut [AccountView],
    args: InitBoardArgs,
) -> ProgramResult {
    let [admin, config, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !admin.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if system_program.address().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (pda, bump) = BoardConfig::derive();

    if config.address().ne(&pda) {
        return Err(ProgramError::InvalidSeeds);
    }

    if config.lamports() > 0 {
        return Err(PostBoardError::BoardAlreadyInitialized.into());
    }

    let mut admin_key = [0u8; 32];
    admin_key.copy_from_slice(admin.address().as_ref());

    let config_data = BoardConfig {
        admin: admin_key,
        post_fee: args.post_fee,
        post_count: 0,
        is_paused: false,
        bump,
    };

    let lamports = Rent::get()?.minimum_balance_unchecked(BoardConfig::SPACE);
    let bump_bytes = &[bump];
    let seeds = [Seed::from(BoardConfig::SEED), Seed::from(bump_bytes)];
    let signers = Signer::from(&seeds);

    CreateAccount {
        from: admin,
        to: config,
        lamports,
        space: BoardConfig::SPACE as u64,
        owner: program_id,
    }
    .invoke_signed(&[signers])?;

    config_data
        .serialize(&mut config.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    emit(&BoardEvent::BoardInitialized(BoardInitialized {
        admin: admin_key,
        post_fee: args.post_fee,
    }))?;

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

    use crate::{accounts::BoardConfig, types::InitBoardArgs, PostBoardInstruction, ID};

    #[test]
    fn init_board() {
        let mut svm = LiteSVM::new();
        let admin = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&admin.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let (config, _) = BoardConfig::derive();

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(admin.pubkey(), true),
                AccountMeta::new(config, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::InitBoard(InitBoardArgs {
                post_fee: 1_000,
            }))
            .unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("logs: {:#?}", result.logs);

        let account = svm.get_account(&config).unwrap();
        let data = BoardConfig::deserialize(&mut account.data.as_ref()).unwrap();

        assert_eq!(data.admin, admin.pubkey().to_bytes());
        assert_eq!(data.post_fee, 1_000);
        assert_eq!(data.post_count, 0);
        assert!(!data.is_paused);
    }

    #[test]
    fn init_board_twice_fails() {
        let mut svm = LiteSVM::new();
        let admin = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&admin.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (config, _) = BoardConfig::derive();

        let build_ix = || Instruction {
            program_id,
            accounts: [
                AccountMeta::new(admin.pubkey(), true),
                AccountMeta::new(config, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::InitBoard(InitBoardArgs { post_fee: 0 }))
                .unwrap(),
        };

        let tx1 = Transaction::new_signed_with_payer(
            &[build_ix()],
            Some(&admin.pubkey()),
            &[&admin],
            svm.latest_blockhash(),
        );
        svm.send_transaction(tx1).unwrap();

        let tx2 = Transaction::new_signed_with_payer(
            &[build_ix()],
            Some(&admin.pubkey()),
            &[&admin],
            svm.latest_blockhash(),
        );
        assert!(svm.send_transaction(tx2).is_err());
    }
}
