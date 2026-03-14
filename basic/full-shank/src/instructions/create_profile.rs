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
    accounts::UserProfile,
    errors::PostBoardError,
    events::{BoardEvent, ProfileCreated},
    types::{CreateProfileArgs, UserStatus},
};

/// Accounts: [payer(sig), profile(mut), referrer?(opt), system_program]
pub fn create_profile(
    program_id: &Address,
    accounts: &mut [AccountView],
    args: CreateProfileArgs,
) -> ProgramResult {
    // system_program is always last; referrer is optional in between
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let (front, rest) = accounts.split_at_mut(2);
    let [payer, profile] = front else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let system_program = rest.last().ok_or(ProgramError::NotEnoughAccountKeys)?;

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if system_program.address().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (pda, bump) = UserProfile::derive(payer.address());

    if profile.address().ne(&pda) {
        return Err(ProgramError::InvalidSeeds);
    }

    if profile.lamports() > 0 {
        return Err(PostBoardError::ProfileAlreadyExists.into());
    }

    let mut authority_key = [0u8; 32];
    authority_key.copy_from_slice(payer.address().as_ref());

    let profile_data = UserProfile {
        authority: authority_key,
        username: args.username,
        post_count: 0,
        status: UserStatus::Active,
        bump,
    };

    let lamports = Rent::get()?.minimum_balance_unchecked(UserProfile::SPACE);
    let bump_bytes = &[bump];
    let seeds = [
        Seed::from(UserProfile::SEED),
        Seed::from(payer.address().as_ref()),
        Seed::from(bump_bytes),
    ];
    let signers = Signer::from(&seeds);

    CreateAccount {
        from: payer,
        to: profile,
        lamports,
        space: UserProfile::SPACE as u64,
        owner: program_id,
    }
    .invoke_signed(&[signers])?;

    profile_data
        .serialize(&mut profile.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    emit(&BoardEvent::ProfileCreated(ProfileCreated {
        authority: authority_key,
        username: args.username,
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

    use crate::{accounts::UserProfile, types::CreateProfileArgs, PostBoardInstruction, ID};

    #[test]
    fn create_profile() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let payer_addr = Address::new_from_array(payer.pubkey().to_bytes());
        let (profile, _) = UserProfile::derive(&payer_addr);

        let mut username = [0u8; 32];
        username[..4].copy_from_slice(b"goni");

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(profile, false),
                // no referrer — system_program is last
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::CreateProfile(CreateProfileArgs {
                username,
            }))
            .unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("logs: {:#?}", result.logs);

        let account = svm.get_account(&profile).unwrap();
        let data = UserProfile::deserialize(&mut account.data.as_ref()).unwrap();

        assert_eq!(data.authority, payer.pubkey().to_bytes());
        assert_eq!(data.username, username);
        assert_eq!(data.post_count, 0);
    }

    #[test]
    fn create_profile_with_referrer() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let referrer = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();
        svm.airdrop(&referrer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        // Create the referrer's profile first.
        let referrer_addr = Address::new_from_array(referrer.pubkey().to_bytes());
        let (referrer_profile, _) = UserProfile::derive(&referrer_addr);

        let mut ref_username = [0u8; 32];
        ref_username[..3].copy_from_slice(b"ref");

        let ix_ref = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(referrer.pubkey(), true),
                AccountMeta::new(referrer_profile, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::CreateProfile(CreateProfileArgs {
                username: ref_username,
            }))
            .unwrap(),
        };
        let tx_ref = Transaction::new_signed_with_payer(
            &[ix_ref],
            Some(&referrer.pubkey()),
            &[&referrer],
            svm.latest_blockhash(),
        );
        svm.send_transaction(tx_ref).unwrap();

        // Create the new user's profile, passing the referrer as the optional account.
        let payer_addr = Address::new_from_array(payer.pubkey().to_bytes());
        let (profile, _) = UserProfile::derive(&payer_addr);

        let mut username = [0u8; 32];
        username[..4].copy_from_slice(b"newb");

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(profile, false),
                AccountMeta::new_readonly(referrer_profile, false), // optional referrer
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::CreateProfile(CreateProfileArgs {
                username,
            }))
            .unwrap(),
        };
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("logs: {:#?}", result.logs);

        let account = svm.get_account(&profile).unwrap();
        let data = UserProfile::deserialize(&mut account.data.as_ref()).unwrap();
        assert_eq!(data.username, username);
    }
}
