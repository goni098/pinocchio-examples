use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};
use shared::event::emit;

use crate::{
    accounts::UserProfile,
    errors::PostBoardError,
    events::{BoardEvent, UsernameUpdated},
    types::{UpdateUsernameArgs, UserStatus},
};

/// Accounts: [authority(sig), profile(mut)]
pub fn update_username(
    _program_id: &Address,
    accounts: &mut [AccountView],
    args: UpdateUsernameArgs,
) -> ProgramResult {
    let [authority, profile] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut profile_data =
        UserProfile::deserialize(&mut profile.try_borrow()?.as_ref())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let mut authority_key = [0u8; 32];
    authority_key.copy_from_slice(authority.address().as_ref());

    if profile_data.authority.ne(&authority_key) {
        return Err(PostBoardError::ProfileNotFound.into());
    }

    if profile_data.status == UserStatus::Suspended {
        return Err(PostBoardError::UserSuspended.into());
    }

    profile_data.username = args.username;

    profile_data
        .serialize(&mut profile.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    emit(&BoardEvent::UsernameUpdated(UsernameUpdated {
        authority: authority_key,
        new_username: args.username,
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
        accounts::UserProfile,
        types::{UpdateUsernameArgs, UserStatus},
        PostBoardInstruction, ID,
    };

    #[test]
    fn update_username() {
        let mut svm = LiteSVM::new();
        let authority = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&authority.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let auth_addr = Address::new_from_array(authority.pubkey().to_bytes());
        let (profile, bump) = UserProfile::derive(&auth_addr);

        let old_username = {
            let mut u = [0u8; 32];
            u[..3].copy_from_slice(b"old");
            u
        };

        svm.set_account(
            profile,
            Account {
                data: borsh::to_vec(&UserProfile {
                    authority: authority.pubkey().to_bytes(),
                    username: old_username,
                    post_count: 0,
                    status: UserStatus::Active,
                    bump,
                })
                .unwrap(),
                lamports: LAMPORTS_PER_SOL,
                owner: ID.into(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        let new_username = {
            let mut u = [0u8; 32];
            u[..3].copy_from_slice(b"new");
            u
        };

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(authority.pubkey(), true),
                AccountMeta::new(profile, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::UpdateUsername(UpdateUsernameArgs {
                username: new_username,
            }))
            .unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("logs: {:#?}", result.logs);

        let account = svm.get_account(&profile).unwrap();
        let data = UserProfile::deserialize(&mut account.data.as_ref()).unwrap();
        assert_eq!(data.username, new_username);
    }

    #[test]
    fn suspended_user_cannot_update_username() {
        let mut svm = LiteSVM::new();
        let authority = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&authority.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let auth_addr = Address::new_from_array(authority.pubkey().to_bytes());
        let (profile, bump) = UserProfile::derive(&auth_addr);

        svm.set_account(
            profile,
            Account {
                data: borsh::to_vec(&UserProfile {
                    authority: authority.pubkey().to_bytes(),
                    username: [0u8; 32],
                    post_count: 0,
                    status: UserStatus::Suspended,
                    bump,
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
                AccountMeta::new(authority.pubkey(), true),
                AccountMeta::new(profile, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::UpdateUsername(UpdateUsernameArgs {
                username: [0u8; 32],
            }))
            .unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&authority.pubkey()),
            &[&authority],
            svm.latest_blockhash(),
        );

        assert!(svm.send_transaction(tx).is_err());
    }
}
