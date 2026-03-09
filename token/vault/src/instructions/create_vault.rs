use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use shank::ShankType;
use shared::{
    event::emit,
    token::{transfer_tokens, validate_token_program},
};

use crate::{
    accounts::Vault,
    events::{VaultCreated, VaultEvent},
};

#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct CreateVaultArgs {
    /// Random pubkey used as vault identifier and PDA seed.
    pub vault_id: [u8; 32],
    /// Mint address of token A.
    pub token_mint_a: [u8; 32],
    /// Mint address of token B.
    pub token_mint_b: [u8; 32],
    /// Initial liquidity amount for token A.
    pub amount_a: u64,
    /// Initial liquidity amount for token B.
    pub amount_b: u64,
}

/// Create a new AMM vault and seed it with initial liquidity.
///
/// Token A and token B may use different token programs (SPL Token vs Token-2022).
///
/// ### Accounts:
///   0. `[SIGNER, WRITE]` owner — pays for vault creation and provides liquidity
///   1. `[WRITE]`         vault — the vault PDA (seeds: ["vault", owner, vault_id])
///   2. `[WRITE]`         user_token_account_a — owner's token A account
///   3. `[WRITE]`         user_token_account_b — owner's token B account
///   4. `[WRITE]`         vault_token_account_a — vault's token A account (authority = vault PDA)
///   5. `[WRITE]`         vault_token_account_b — vault's token B account (authority = vault PDA)
///   6. `[]`              token_program_a — SPL Token or Token-2022 for mint A
///   7. `[]`              token_program_b — SPL Token or Token-2022 for mint B
///   8. `[]`              system_program
pub fn create_vault(
    program_id: &Address,
    accounts: &mut [AccountView],
    args: CreateVaultArgs,
) -> ProgramResult {
    let [owner, vault, user_token_a, user_token_b, vault_token_a, vault_token_b, token_program_a, token_program_b, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !owner.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if vault.lamports().ne(&0) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if system_program.address().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    validate_token_program(token_program_a)?;
    validate_token_program(token_program_b)?;

    if args.amount_a == 0 || args.amount_b == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    let (pda, bump) = Vault::derive(owner.address(), &args.vault_id);
    if vault.address().ne(&pda) {
        return Err(ProgramError::InvalidSeeds);
    }

    // Create the vault account via system program.
    let account_span = Vault::SPACE;
    let lamports_required = Rent::get()?.minimum_balance_unchecked(account_span);

    let bump_bytes = &[bump];
    let seeds = [
        Seed::from(Vault::SEED_PREFIX),
        Seed::from(owner.address().as_array()),
        Seed::from(&args.vault_id),
        Seed::from(bump_bytes),
    ];
    let signers = Signer::from(&seeds);

    CreateAccount {
        from: owner,
        to: vault,
        lamports: lamports_required,
        space: account_span as u64,
        owner: program_id,
    }
    .invoke_signed(&[signers])?;

    // Transfer initial liquidity using each token's respective program.
    transfer_tokens(
        user_token_a,
        vault_token_a,
        owner,
        args.amount_a,
        token_program_a.address(),
        &[],
    )?;

    transfer_tokens(
        user_token_b,
        vault_token_b,
        owner,
        args.amount_b,
        token_program_b.address(),
        &[],
    )?;

    let k = (args.amount_a as u128)
        .checked_mul(args.amount_b as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let vault_data = Vault {
        owner: owner.address().to_bytes(),
        vault_id: args.vault_id,
        token_mint_a: args.token_mint_a,
        token_mint_b: args.token_mint_b,
        token_account_a: vault_token_a.address().to_bytes(),
        token_account_b: vault_token_b.address().to_bytes(),
        token_program_a: token_program_a.address().to_bytes(),
        token_program_b: token_program_b.address().to_bytes(),
        reserve_a: args.amount_a,
        reserve_b: args.amount_b,
        k,
        bump,
    };

    vault_data
        .serialize(&mut vault.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    emit(&VaultEvent::VaultCreated(VaultCreated {
        vault: vault.address().to_bytes(),
        owner: owner.address().to_bytes(),
        reserve_a: args.amount_a,
        reserve_b: args.amount_b,
        k,
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

    use crate::{accounts::Vault, VaultInstruction, ID};

    /// Build a raw SPL Token account (165 bytes).
    fn spl_token_account(mint: [u8; 32], owner: [u8; 32], amount: u64) -> [u8; 165] {
        let mut data = [0u8; 165];
        data[0..32].copy_from_slice(&mint);
        data[32..64].copy_from_slice(&owner);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1; // state = Initialized
        data
    }

    /// Build a raw Token-2022 token account (165 base bytes + 1 account-type byte).
    fn token_2022_account(mint: [u8; 32], owner: [u8; 32], amount: u64) -> [u8; 166] {
        let mut data = [0u8; 166];
        data[0..32].copy_from_slice(&mint);
        data[32..64].copy_from_slice(&owner);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1; // state = Initialized
        data[165] = 2; // account_type = TokenAccount
        data
    }

    fn spl_token_id() -> Address {
        pinocchio_token::ID
    }

    fn token_2022_id() -> Address {
        pinocchio_token_2022::ID
    }

    /// Both tokens use SPL Token.
    #[test]
    fn create_vault_spl_spl() {
        let mut svm = LiteSVM::new();

        let owner = Keypair::new();
        let vault_id = Keypair::new().pubkey();
        let mint_a = Keypair::new().pubkey();
        let mint_b = Keypair::new().pubkey();

        let program_id = Address::new_from_array(ID);
        svm.add_program_from_file(program_id, "../../target/deploy/vault.so")
            .unwrap();
        svm.airdrop(&owner.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (vault_pda, _) = Vault::derive(&owner.pubkey(), &vault_id.to_bytes());

        let user_token_a = Keypair::new();
        let user_token_b = Keypair::new();
        let vault_token_a = Keypair::new();
        let vault_token_b = Keypair::new();

        let spl = spl_token_id();

        for (kp, mint, owner_bytes, amount) in [
            (
                &user_token_a,
                mint_a.to_bytes(),
                owner.pubkey().to_bytes(),
                1000u64,
            ),
            (
                &user_token_b,
                mint_b.to_bytes(),
                owner.pubkey().to_bytes(),
                500,
            ),
            (&vault_token_a, mint_a.to_bytes(), vault_pda.to_bytes(), 0),
            (&vault_token_b, mint_b.to_bytes(), vault_pda.to_bytes(), 0),
        ] {
            svm.set_account(
                kp.pubkey(),
                solana_sdk::account::Account {
                    data: spl_token_account(mint, owner_bytes, amount).to_vec(),
                    lamports: LAMPORTS_PER_SOL,
                    owner: spl,
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
        }

        let ix_data = VaultInstruction::CreateVault(super::CreateVaultArgs {
            vault_id: vault_id.to_bytes(),
            token_mint_a: mint_a.to_bytes(),
            token_mint_b: mint_b.to_bytes(),
            amount_a: 1000,
            amount_b: 500,
        });

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(user_token_a.pubkey(), false),
                AccountMeta::new(user_token_b.pubkey(), false),
                AccountMeta::new(vault_token_a.pubkey(), false),
                AccountMeta::new(vault_token_b.pubkey(), false),
                AccountMeta::new_readonly(spl, false),
                AccountMeta::new_readonly(spl, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&ix_data).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&owner.pubkey()),
            &[&owner],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("CreateVault (SPL+SPL) logs: {:#?}", result.logs);

        let vault_account = svm.get_account(&vault_pda).unwrap();
        let vault_data = Vault::deserialize(&mut vault_account.data.as_ref()).unwrap();
        assert_eq!(vault_data.reserve_a, 1000);
        assert_eq!(vault_data.reserve_b, 500);
        assert_eq!(vault_data.k, 500_000);
        assert_eq!(vault_data.token_program_a, spl.to_bytes());
        assert_eq!(vault_data.token_program_b, spl.to_bytes());

        let vta = svm.get_account(&vault_token_a.pubkey()).unwrap();
        let vtb = svm.get_account(&vault_token_b.pubkey()).unwrap();
        assert_eq!(
            u64::from_le_bytes(vta.data[64..72].try_into().unwrap()),
            1000
        );
        assert_eq!(
            u64::from_le_bytes(vtb.data[64..72].try_into().unwrap()),
            500
        );

        let uta = svm.get_account(&user_token_a.pubkey()).unwrap();
        assert_eq!(u64::from_le_bytes(uta.data[64..72].try_into().unwrap()), 0);
    }

    /// Token A uses SPL Token, token B uses Token-2022.
    #[test]
    fn create_vault_spl_token2022() {
        let mut svm = LiteSVM::new();

        let owner = Keypair::new();
        let vault_id = Keypair::new().pubkey();
        let mint_a = Keypair::new().pubkey();
        let mint_b = Keypair::new().pubkey();

        let program_id = Address::new_from_array(ID);
        svm.add_program_from_file(program_id, "../../target/deploy/vault.so")
            .unwrap();
        svm.airdrop(&owner.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (vault_pda, _) = Vault::derive(&owner.pubkey(), &vault_id.to_bytes());

        let user_token_a = Keypair::new();
        let user_token_b = Keypair::new();
        let vault_token_a = Keypair::new();
        let vault_token_b = Keypair::new();

        let spl = spl_token_id();
        let t22 = token_2022_id();

        // Token A: SPL Token accounts.
        for (kp, owner_bytes, amount) in [
            (&user_token_a, owner.pubkey().to_bytes(), 1000u64),
            (&vault_token_a, vault_pda.to_bytes(), 0),
        ] {
            svm.set_account(
                kp.pubkey(),
                solana_sdk::account::Account {
                    data: spl_token_account(mint_a.to_bytes(), owner_bytes, amount).to_vec(),
                    lamports: LAMPORTS_PER_SOL,
                    owner: spl,
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
        }

        // Token B: Token-2022 accounts.
        for (kp, owner_bytes, amount) in [
            (&user_token_b, owner.pubkey().to_bytes(), 500u64),
            (&vault_token_b, vault_pda.to_bytes(), 0),
        ] {
            svm.set_account(
                kp.pubkey(),
                solana_sdk::account::Account {
                    data: token_2022_account(mint_b.to_bytes(), owner_bytes, amount).to_vec(),
                    lamports: LAMPORTS_PER_SOL,
                    owner: t22,
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
        }

        let ix_data = VaultInstruction::CreateVault(super::CreateVaultArgs {
            vault_id: vault_id.to_bytes(),
            token_mint_a: mint_a.to_bytes(),
            token_mint_b: mint_b.to_bytes(),
            amount_a: 1000,
            amount_b: 500,
        });

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(user_token_a.pubkey(), false),
                AccountMeta::new(user_token_b.pubkey(), false),
                AccountMeta::new(vault_token_a.pubkey(), false),
                AccountMeta::new(vault_token_b.pubkey(), false),
                AccountMeta::new_readonly(spl, false),
                AccountMeta::new_readonly(t22, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&ix_data).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&owner.pubkey()),
            &[&owner],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("CreateVault (SPL+T22) logs: {:#?}", result.logs);

        let vault_account = svm.get_account(&vault_pda).unwrap();
        let vault_data = Vault::deserialize(&mut vault_account.data.as_ref()).unwrap();
        assert_eq!(vault_data.reserve_a, 1000);
        assert_eq!(vault_data.reserve_b, 500);
        assert_eq!(vault_data.k, 500_000);
        assert_eq!(vault_data.token_program_a, spl.to_bytes());
        assert_eq!(vault_data.token_program_b, t22.to_bytes());
    }
}
