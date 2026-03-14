use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use shared::event::emit;

use crate::{
    accounts::{Post, UserProfile},
    errors::PostBoardError,
    events::{BoardEvent, MessagePosted},
    types::{PostMessageArgs, PostStatus, UserStatus},
};

/// Accounts: [author(sig), profile(mut), post(mut), system_program]
pub fn post_message(
    program_id: &Address,
    accounts: &mut [AccountView],
    args: PostMessageArgs,
) -> ProgramResult {
    let [author, profile, post, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !author.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if system_program.address().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut profile_data =
        UserProfile::deserialize(&mut profile.try_borrow()?.as_ref())
            .map_err(|_| ProgramError::InvalidAccountData)?;

    let mut author_key = [0u8; 32];
    author_key.copy_from_slice(author.address().as_ref());

    if profile_data.authority.ne(&author_key) {
        return Err(PostBoardError::ProfileNotFound.into());
    }

    if profile_data.status == UserStatus::Suspended {
        return Err(PostBoardError::UserSuspended.into());
    }

    let post_index = profile_data.post_count;
    let (pda, bump) = Post::derive(author.address(), post_index);

    if post.address().ne(&pda) {
        return Err(ProgramError::InvalidSeeds);
    }

    let lamports = Rent::get()?.minimum_balance_unchecked(Post::SPACE);
    let index_bytes = post_index.to_le_bytes();
    let bump_bytes = &[bump];
    let seeds = [
        Seed::from(Post::SEED),
        Seed::from(author.address().as_ref()),
        Seed::from(index_bytes.as_ref()),
        Seed::from(bump_bytes),
    ];
    let signers = Signer::from(&seeds);

    CreateAccount {
        from: author,
        to: post,
        lamports,
        space: Post::SPACE as u64,
        owner: program_id,
    }
    .invoke_signed(&[signers])?;

    let post_data = Post {
        author: author_key,
        content: args.content,
        reply_to: args.reply_to,
        status: PostStatus::Published,
        post_index,
        bump,
    };

    post_data
        .serialize(&mut post.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    profile_data.post_count = profile_data
        .post_count
        .checked_add(1)
        .ok_or(PostBoardError::NumericalOverflow)?;

    profile_data
        .serialize(&mut profile.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    emit(&BoardEvent::MessagePosted(MessagePosted {
        author: author_key,
        post_index,
        reply_to: args.reply_to,
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
        accounts::{Post, UserProfile},
        types::{PostMessageArgs, PostStatus, UserStatus},
        PostBoardInstruction, ID,
    };

    fn seed_profile(svm: &mut LiteSVM, author: &Keypair) -> Address {
        let author_addr = Address::new_from_array(author.pubkey().to_bytes());
        let (profile, bump) = UserProfile::derive(&author_addr);
        svm.set_account(
            profile,
            Account {
                data: borsh::to_vec(&UserProfile {
                    authority: author.pubkey().to_bytes(),
                    username: {
                        let mut u = [0u8; 32];
                        u[..4].copy_from_slice(b"user");
                        u
                    },
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
        profile
    }

    #[test]
    fn post_top_level_message() {
        let mut svm = LiteSVM::new();
        let author = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&author.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let profile = seed_profile(&mut svm, &author);
        let author_addr = Address::new_from_array(author.pubkey().to_bytes());
        let (post, _) = Post::derive(&author_addr, 0);

        let mut content = [0u8; 128];
        content[..13].copy_from_slice(b"hello, world!");

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(author.pubkey(), true),
                AccountMeta::new(profile, false),
                AccountMeta::new(post, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::PostMessage(PostMessageArgs {
                content,
                reply_to: None,
            }))
            .unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&author.pubkey()),
            &[&author],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("logs: {:#?}", result.logs);

        let post_account = svm.get_account(&post).unwrap();
        let post_data = Post::deserialize(&mut post_account.data.as_ref()).unwrap();

        assert_eq!(post_data.author, author.pubkey().to_bytes());
        assert_eq!(post_data.content, content);
        assert_eq!(post_data.reply_to, None);
        assert_eq!(post_data.post_index, 0);
        assert_eq!(post_data.status, PostStatus::Published);

        let profile_account = svm.get_account(&profile).unwrap();
        let profile_data = UserProfile::deserialize(&mut profile_account.data.as_ref()).unwrap();
        assert_eq!(profile_data.post_count, 1);
    }

    #[test]
    fn post_reply_message() {
        let mut svm = LiteSVM::new();
        let author = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&author.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let profile = seed_profile(&mut svm, &author);
        let author_addr = Address::new_from_array(author.pubkey().to_bytes());
        let (post, _) = Post::derive(&author_addr, 0);

        let parent_post = Keypair::new();
        let reply_to = Some(parent_post.pubkey().to_bytes());

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(author.pubkey(), true),
                AccountMeta::new(profile, false),
                AccountMeta::new(post, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::PostMessage(PostMessageArgs {
                content: [0u8; 128],
                reply_to,
            }))
            .unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&author.pubkey()),
            &[&author],
            svm.latest_blockhash(),
        );

        svm.send_transaction(tx).unwrap();

        let post_account = svm.get_account(&post).unwrap();
        let post_data = Post::deserialize(&mut post_account.data.as_ref()).unwrap();

        assert_eq!(post_data.reply_to, reply_to);
    }
}
