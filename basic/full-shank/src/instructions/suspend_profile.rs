use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};
use shared::event::emit;

use crate::{
    accounts::{BoardConfig, UserProfile},
    errors::PostBoardError,
    events::{BoardEvent, ProfileSuspended},
    types::UserStatus,
};

/// Accounts: [admin(sig), config(mut), profile(mut), co_admin?(optional_signer)]
pub fn suspend_profile(
    _program_id: &Address,
    accounts: &mut [AccountView],
) -> ProgramResult {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let (front, _rest) = accounts.split_at_mut(3);
    let [admin, config, profile] = front else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !admin.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let config_data = BoardConfig::deserialize(&mut config.try_borrow()?.as_ref())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let mut admin_key = [0u8; 32];
    admin_key.copy_from_slice(admin.address().as_ref());

    if config_data.admin.ne(&admin_key) {
        return Err(PostBoardError::NotAdmin.into());
    }

    let mut profile_data =
        UserProfile::deserialize(&mut profile.try_borrow()?.as_ref())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    profile_data.status = UserStatus::Suspended;

    profile_data
        .serialize(&mut profile.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let profile_authority = profile_data.authority;

    emit(&BoardEvent::ProfileSuspended(ProfileSuspended {
        profile_authority,
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
        account::Account, instruction::Instruction, message::AccountMeta,
        native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer,
        transaction::Transaction,
    };

    use crate::{
        accounts::{BoardConfig, UserProfile},
        types::UserStatus,
        PostBoardInstruction, ID,
    };

    #[test]
    fn admin_suspends_profile() {
        let mut svm = LiteSVM::new();
        let admin = Keypair::new();
        let user = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&admin.pubkey(), LAMPORTS_PER_SOL).unwrap();

        // Pre-seed board config.
        let (config, config_bump) = BoardConfig::derive();
        svm.set_account(
            config,
            Account {
                data: borsh::to_vec(&BoardConfig {
                    admin: admin.pubkey().to_bytes(),
                    post_fee: 0,
                    post_count: 0,
                    is_paused: false,
                    bump: config_bump,
                })
                .unwrap(),
                lamports: LAMPORTS_PER_SOL,
                owner: ID.into(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // Pre-seed an active user profile.
        let user_addr = Address::new_from_array(user.pubkey().to_bytes());
        let (profile, profile_bump) = UserProfile::derive(&user_addr);
        svm.set_account(
            profile,
            Account {
                data: borsh::to_vec(&UserProfile {
                    authority: user.pubkey().to_bytes(),
                    username: [0u8; 32],
                    post_count: 0,
                    status: UserStatus::Active,
                    bump: profile_bump,
                })
                .unwrap(),
                lamports: LAMPORTS_PER_SOL,
                owner: ID.into(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(admin.pubkey(), true),
                AccountMeta::new(config, false),
                AccountMeta::new(profile, false),
                // no co_admin (optional signer)
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::SuspendProfile).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("logs: {:#?}", result.logs);

        let account = svm.get_account(&profile).unwrap();
        let data = UserProfile::deserialize(&mut account.data.as_ref()).unwrap();
        assert_eq!(data.status, UserStatus::Suspended);
    }

    #[test]
    fn non_admin_cannot_suspend() {
        let mut svm = LiteSVM::new();
        let admin = Keypair::new();
        let attacker = Keypair::new();
        let user = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&attacker.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let (config, config_bump) = BoardConfig::derive();
        svm.set_account(
            config,
            Account {
                data: borsh::to_vec(&BoardConfig {
                    admin: admin.pubkey().to_bytes(), // real admin, not attacker
                    post_fee: 0,
                    post_count: 0,
                    is_paused: false,
                    bump: config_bump,
                })
                .unwrap(),
                lamports: LAMPORTS_PER_SOL,
                owner: ID.into(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        let user_addr = Address::new_from_array(user.pubkey().to_bytes());
        let (profile, profile_bump) = UserProfile::derive(&user_addr);
        svm.set_account(
            profile,
            Account {
                data: borsh::to_vec(&UserProfile {
                    authority: user.pubkey().to_bytes(),
                    username: [0u8; 32],
                    post_count: 0,
                    status: UserStatus::Active,
                    bump: profile_bump,
                })
                .unwrap(),
                lamports: LAMPORTS_PER_SOL,
                owner: ID.into(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(attacker.pubkey(), true), // attacker, not admin
                AccountMeta::new(config, false),
                AccountMeta::new(profile, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::SuspendProfile).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&attacker.pubkey()),
            &[&attacker],
            svm.latest_blockhash(),
        );

        assert!(svm.send_transaction(tx).is_err());
    }
}
