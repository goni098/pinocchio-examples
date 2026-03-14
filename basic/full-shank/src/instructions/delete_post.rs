use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};
use shared::event::emit;

use crate::{
    accounts::{BoardConfig, Post},
    errors::PostBoardError,
    events::{BoardEvent, MessageDeleted},
    types::PostStatus,
};

/// Accounts: [authority(sig), post(mut), refund_to(mut), config?(opt)]
///
/// A post can be deleted by:
/// - Its author (no `config` account required), or
/// - The board admin (`config` account must be provided).
pub fn delete_post(
    _program_id: &Address,
    accounts: &mut [AccountView],
) -> ProgramResult {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let (front, rest) = accounts.split_at_mut(3);
    let [authority, post, refund_to] = front else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut post_data = Post::deserialize(&mut post.try_borrow()?.as_ref())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if post_data.status == PostStatus::Deleted {
        return Err(PostBoardError::PostAlreadyDeleted.into());
    }

    let mut authority_key = [0u8; 32];
    authority_key.copy_from_slice(authority.address().as_ref());

    let is_author = post_data.author == authority_key;

    if !is_author {
        // Admin path: optional `config` account must be present and valid.
        let config = rest.first().ok_or(PostBoardError::NotAuthor)?;
        let config_data = BoardConfig::deserialize(&mut config.try_borrow()?.as_ref())
            .map_err(|_| ProgramError::InvalidAccountData)?;

        if config_data.admin.ne(&authority_key) {
            return Err(PostBoardError::NotAdmin.into());
        }
    }

    post_data.status = PostStatus::Deleted;
    post_data
        .serialize(&mut post.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    refund_to.set_lamports(refund_to.lamports() + post.lamports());
    post.close()?;

    let mut post_key = [0u8; 32];
    post_key.copy_from_slice(post.address().as_ref());

    emit(&BoardEvent::MessageDeleted(MessageDeleted {
        post: post_key,
        deleted_by: authority_key,
    }))?;

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

    use crate::{
        accounts::{BoardConfig, Post},
        types::PostStatus,
        PostBoardInstruction, ID,
    };

    fn seed_post(svm: &mut LiteSVM, author: &Keypair, post_addr: Address) {
        svm.set_account(
            post_addr,
            Account {
                data: borsh::to_vec(&Post {
                    author: author.pubkey().to_bytes(),
                    content: [0u8; 128],
                    reply_to: None,
                    status: PostStatus::Published,
                    post_index: 0,
                    bump: 255,
                })
                .unwrap(),
                lamports: LAMPORTS_PER_SOL,
                owner: ID.into(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
    }

    #[test]
    fn author_deletes_post() {
        let mut svm = LiteSVM::new();
        let author = Keypair::new();
        let program_id = Address::new_from_array(ID);

        svm.add_program_from_file(program_id, "../../target/deploy/full_shank.so")
            .unwrap();
        svm.airdrop(&author.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let author_addr = Address::new_from_array(author.pubkey().to_bytes());
        let (post, _) = Post::derive(&author_addr, 0);
        seed_post(&mut svm, &author, post);

        let recipient = Keypair::new();
        let lamports_before = svm
            .get_account(&recipient.pubkey())
            .map(|a| a.lamports)
            .unwrap_or(0);

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(author.pubkey(), true),
                AccountMeta::new(post, false),
                AccountMeta::new(recipient.pubkey(), false),
                // no config — author path
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::DeletePost).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&author.pubkey()),
            &[&author],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("logs: {:#?}", result.logs);

        assert!(svm.get_account(&post).is_none());
        assert!(
            svm.get_account(&recipient.pubkey())
                .map(|a| a.lamports)
                .unwrap_or(0)
                > lamports_before
        );
    }

    #[test]
    fn admin_deletes_post_with_config() {
        let mut svm = LiteSVM::new();
        let admin = Keypair::new();
        let author = Keypair::new();
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

        // Pre-seed a post by another user.
        let author_addr = Address::new_from_array(author.pubkey().to_bytes());
        let (post, _) = Post::derive(&author_addr, 0);
        seed_post(&mut svm, &author, post);

        let recipient = Keypair::new();

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(admin.pubkey(), true),
                AccountMeta::new(post, false),
                AccountMeta::new(recipient.pubkey(), false),
                AccountMeta::new_readonly(config, false), // optional config for admin path
            ]
            .to_vec(),
            data: borsh::to_vec(&PostBoardInstruction::DeletePost).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            svm.latest_blockhash(),
        );

        svm.send_transaction(tx).unwrap();
        assert!(svm.get_account(&post).is_none());
    }
}
