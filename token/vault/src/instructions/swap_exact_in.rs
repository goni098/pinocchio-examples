use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, Address, ProgramResult,
};
use shank::ShankType;
use shared::event::emit;

use shared::token::transfer_tokens;

use crate::{
    accounts::Vault,
    events::{SwapExactInExecuted, VaultEvent},
};

#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct SwapExactInArgs {
    /// Exact amount of input tokens to swap.
    pub amount_in: u64,
    /// Minimum output tokens accepted (slippage protection).
    pub min_amount_out: u64,
}

/// Swap an exact amount of tokens in for an AMM-calculated amount out.
///
/// Direction is determined by which vault token account is passed as
/// `vault_token_in`: if it matches `vault.token_account_a`, the swap goes
/// A → B; otherwise B → A.
///
/// Token A and token B may belong to different token programs (SPL Token vs
/// Token-2022). The caller must pass the correct program for each side;
/// both are validated against what is stored in the vault.
///
/// ### Accounts:
///   0. `[SIGNER]`  user — signer and authority of user token accounts
///   1. `[WRITE]`   vault — the vault PDA
///   2. `[WRITE]`   user_token_in  — user's source token account
///   3. `[WRITE]`   vault_token_in — vault's matching source token account
///   4. `[WRITE]`   user_token_out — user's destination token account
///   5. `[WRITE]`   vault_token_out — vault's matching destination token account
///   6. `[]`        token_program_in  — SPL Token or Token-2022 for the input token
///   7. `[]`        token_program_out — SPL Token or Token-2022 for the output token
pub fn swap_exact_in(
    _program_id: &Address,
    accounts: &mut [AccountView],
    args: SwapExactInArgs,
) -> ProgramResult {
    let [user, vault, user_token_in, vault_token_in, user_token_out, vault_token_out, token_program_in, token_program_out] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !user.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if vault.lamports().eq(&0) {
        return Err(ProgramError::UninitializedAccount);
    }

    let vault_data = Vault::try_from_slice(&vault.try_borrow()?)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Determine swap direction by matching vault_token_in against stored addresses.
    let is_a_to_b = vault_token_in
        .address()
        .as_array()
        .eq(&vault_data.token_account_a);
    let is_b_to_a = vault_token_in
        .address()
        .as_array()
        .eq(&vault_data.token_account_b);

    if !is_a_to_b && !is_b_to_a {
        return Err(ProgramError::InvalidArgument);
    }

    // Verify vault_token_out matches the opposite side.
    let expected_out = if is_a_to_b {
        &vault_data.token_account_b
    } else {
        &vault_data.token_account_a
    };
    if vault_token_out.address().as_array().ne(expected_out) {
        return Err(ProgramError::InvalidArgument);
    }

    // Validate token programs against vault-stored programs.
    let (expected_prog_in, expected_prog_out) = if is_a_to_b {
        (&vault_data.token_program_a, &vault_data.token_program_b)
    } else {
        (&vault_data.token_program_b, &vault_data.token_program_a)
    };
    if token_program_in.address().as_array().ne(expected_prog_in) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if token_program_out.address().as_array().ne(expected_prog_out) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (reserve_in, reserve_out) = if is_a_to_b {
        (vault_data.reserve_a, vault_data.reserve_b)
    } else {
        (vault_data.reserve_b, vault_data.reserve_a)
    };

    // Constant product: amount_out = reserve_out - k / (reserve_in + amount_in)
    let new_reserve_in = (reserve_in as u128)
        .checked_add(args.amount_in as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let new_reserve_out = vault_data
        .k
        .checked_div(new_reserve_in)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let amount_out_u128 = (reserve_out as u128)
        .checked_sub(new_reserve_out)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let amount_out =
        u64::try_from(amount_out_u128).map_err(|_| ProgramError::ArithmeticOverflow)?;

    if amount_out < args.min_amount_out {
        return Err(ProgramError::Custom(1)); // SlippageExceeded
    }

    // Transfer amount_in from user to vault (user signs).
    transfer_tokens(
        user_token_in,
        vault_token_in,
        user,
        args.amount_in,
        token_program_in.address(),
        &[],
    )?;

    // Transfer amount_out from vault to user (vault PDA signs).
    let bump_bytes = &[vault_data.bump];
    let seeds = [
        Seed::from(Vault::SEED_PREFIX),
        Seed::from(&vault_data.owner),
        Seed::from(&vault_data.vault_id),
        Seed::from(bump_bytes),
    ];
    let vault_signer = Signer::from(&seeds);

    transfer_tokens(
        vault_token_out,
        user_token_out,
        vault,
        amount_out,
        token_program_out.address(),
        &[vault_signer],
    )?;

    // Update reserves (k stays unchanged; integer rounding is expected).
    let (new_reserve_a, new_reserve_b) = if is_a_to_b {
        (
            u64::try_from(new_reserve_in).map_err(|_| ProgramError::ArithmeticOverflow)?,
            u64::try_from(new_reserve_out).map_err(|_| ProgramError::ArithmeticOverflow)?,
        )
    } else {
        (
            u64::try_from(new_reserve_out).map_err(|_| ProgramError::ArithmeticOverflow)?,
            u64::try_from(new_reserve_in).map_err(|_| ProgramError::ArithmeticOverflow)?,
        )
    };

    let mut updated = vault_data;
    updated.reserve_a = new_reserve_a;
    updated.reserve_b = new_reserve_b;

    updated
        .serialize(&mut vault.try_borrow_mut()?.as_mut())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    emit(&VaultEvent::SwapExactInExecuted(SwapExactInExecuted {
        vault: vault.address().to_bytes(),
        amount_in: args.amount_in,
        amount_out,
        reserve_a: new_reserve_a,
        reserve_b: new_reserve_b,
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

    use crate::{accounts::Vault, VaultInstruction, ID};

    fn spl_token_account(mint: [u8; 32], owner: [u8; 32], amount: u64) -> [u8; 165] {
        let mut data = [0u8; 165];
        data[0..32].copy_from_slice(&mint);
        data[32..64].copy_from_slice(&owner);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1; // Initialized
        data
    }

    fn spl_token_id() -> Address {
        pinocchio_token::ID
    }

    #[test]
    fn swap_exact_in() {
        let mut svm = LiteSVM::new();

        let user = Keypair::new();
        let owner_bytes = user.pubkey().to_bytes();
        let vault_id_bytes = Keypair::new().pubkey().to_bytes();
        let mint_a = Keypair::new().pubkey();
        let mint_b = Keypair::new().pubkey();

        let program_id = Address::new_from_array(ID);
        svm.add_program_from_file(program_id, "../../target/deploy/vault.so")
            .unwrap();
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (vault_pda, vault_bump) = Vault::derive(&user.pubkey(), &vault_id_bytes);

        let vault_token_a = Keypair::new();
        let vault_token_b = Keypair::new();
        let user_token_in = Keypair::new();
        let user_token_out = Keypair::new();

        let spl_token = spl_token_id();

        // Pre-set vault state: reserve_a = 1000, reserve_b = 500, k = 500_000.
        let vault_state = Vault {
            owner: owner_bytes,
            vault_id: vault_id_bytes,
            token_mint_a: mint_a.to_bytes(),
            token_mint_b: mint_b.to_bytes(),
            token_account_a: vault_token_a.pubkey().to_bytes(),
            token_account_b: vault_token_b.pubkey().to_bytes(),
            token_program_a: spl_token.to_bytes(),
            token_program_b: spl_token.to_bytes(),
            reserve_a: 1000,
            reserve_b: 500,
            k: 500_000,
            bump: vault_bump,
        };

        svm.set_account(
            vault_pda,
            Account {
                data: borsh::to_vec(&vault_state).unwrap(),
                lamports: LAMPORTS_PER_SOL,
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // Vault token accounts (owned by vault PDA).
        svm.set_account(
            vault_token_a.pubkey(),
            Account {
                data: spl_token_account(mint_a.to_bytes(), vault_pda.to_bytes(), 1000).to_vec(),
                lamports: LAMPORTS_PER_SOL,
                owner: spl_token,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
        svm.set_account(
            vault_token_b.pubkey(),
            Account {
                data: spl_token_account(mint_b.to_bytes(), vault_pda.to_bytes(), 500).to_vec(),
                lamports: LAMPORTS_PER_SOL,
                owner: spl_token,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // User token accounts: 100 token_a (to swap in), 0 token_b (receive).
        svm.set_account(
            user_token_in.pubkey(),
            Account {
                data: spl_token_account(mint_a.to_bytes(), owner_bytes, 100).to_vec(),
                lamports: LAMPORTS_PER_SOL,
                owner: spl_token,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
        svm.set_account(
            user_token_out.pubkey(),
            Account {
                data: spl_token_account(mint_b.to_bytes(), owner_bytes, 0).to_vec(),
                lamports: LAMPORTS_PER_SOL,
                owner: spl_token,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

        // amount_out = 500 - 500_000 / (1000 + 100) = 500 - 454 = 46
        let ix_data = VaultInstruction::SwapExactIn(super::SwapExactInArgs {
            amount_in: 100,
            min_amount_out: 40,
        });

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(user.pubkey(), true),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(user_token_in.pubkey(), false),
                AccountMeta::new(vault_token_a.pubkey(), false),
                AccountMeta::new(user_token_out.pubkey(), false),
                AccountMeta::new(vault_token_b.pubkey(), false),
                AccountMeta::new_readonly(spl_token, false), // token_program_in (A = SPL)
                AccountMeta::new_readonly(spl_token, false), // token_program_out (B = SPL)
            ]
            .to_vec(),
            data: borsh::to_vec(&ix_data).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&user.pubkey()),
            &[&user],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();
        std::println!("SwapExactIn logs: {:#?}", result.logs);

        // Verify vault reserves updated.
        let vault_account = svm.get_account(&vault_pda).unwrap();
        let updated = Vault::deserialize(&mut vault_account.data.as_ref()).unwrap();
        // new_reserve_a = 1000 + 100 = 1100
        // new_reserve_b = 500_000 / 1100 = 454
        assert_eq!(updated.reserve_a, 1100);
        assert_eq!(updated.reserve_b, 454);

        // Verify token balances.
        let uto = svm.get_account(&user_token_out.pubkey()).unwrap();
        let uto_amount = u64::from_le_bytes(uto.data[64..72].try_into().unwrap());
        assert_eq!(uto_amount, 46); // 500 - 454

        let uti = svm.get_account(&user_token_in.pubkey()).unwrap();
        let uti_amount = u64::from_le_bytes(uti.data[64..72].try_into().unwrap());
        assert_eq!(uti_amount, 0); // 100 - 100
    }
}
