use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    error::ProgramError,
    sysvars::{
        rent::{
            Rent, DEFAULT_EXEMPTION_THRESHOLD, DEFAULT_LAMPORTS_PER_BYTE,
            DEFAULT_LAMPORTS_PER_BYTE_YEAR,
        },
        Sysvar,
    },
    AccountView, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use shank::ShankType;
// use spl_token_2022_interface::extension::ExtensionType;

#[derive(BorshDeserialize, BorshSerialize, ShankType)]
pub struct CreateMintArgs {
    /// Token decimals
    pub decimals: u8,
    /// Token name
    pub name: [u8; 32],
    /// Token symbol
    pub symbol: [u8; 8],
    /// Token URI
    pub uri: [u8; 64],
}

pub fn create_mint(
    _program_id: &pinocchio::Address,
    accounts: &[AccountView],
    args: CreateMintArgs,
) -> ProgramResult {
    let [payer, mint, token_program, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate payer is signer
    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let is_token_2022 = token_program.address().eq(&pinocchio_token_2022::ID);
    let is_spl_token = token_program.address().eq(&pinocchio_token::ID);

    // Validate token program
    if !is_token_2022 && !is_spl_token {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Validate system program
    if system_program.address().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Check if mint already exists
    if mint.lamports().ne(&0) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let (lamports_required, space) = if is_spl_token {
        let mint_size = pinocchio_token::state::Mint::LEN;
        let lamports = Rent::get()?.minimum_balance_unchecked(pinocchio_token::state::Mint::LEN);

        (lamports, mint_size)
    } else {
        let mint_size = 234;

        let metadata_len = 4 + 117;

        let larmports = Rent::get()?.minimum_balance_unchecked(mint_size);

        (larmports, mint_size)
    };

    // Create the mint account
    CreateAccount {
        from: payer,
        to: mint,
        lamports: lamports_required,
        space: space as u64,
        owner: token_program.address(),
    }
    .invoke()?;

    if is_token_2022 {
        // Initialize metadata pointer extension
        // This must be done before InitializeMint
        shared::extensions::InitializeMetadataPointer {
            authority: None,
            metadata_address: Some(mint.address()),
            mint,
        }
        .invoke()?;
    }

    // Initialize the mint
    if is_token_2022 {
        pinocchio_token_2022::instructions::InitializeMint2 {
            mint,
            decimals: args.decimals,
            mint_authority: payer.address(),
            freeze_authority: None,
            token_program: token_program.address(),
        }
        .invoke()?;
    } else {
        pinocchio_token::instructions::InitializeMint2 {
            mint,
            decimals: args.decimals,
            mint_authority: payer.address(),
            freeze_authority: None,
        }
        .invoke()?;
    }

    if is_token_2022 {
        // Initialize token metadata
        // Convert fixed-size arrays to strings
        let name = shared::helpers::from_fixed_bytes(&args.name)?;
        let symbol = shared::helpers::from_fixed_bytes(&args.symbol)?;
        let uri = shared::helpers::from_fixed_bytes(&args.uri)?;

        shared::extensions::InitializeTokenMetadata {
            metadata: mint,
            update_authority: payer,
            mint,
            mint_authority: payer,
            name,
            symbol,
            uri,
        }
        .invoke()?;
    }

    if is_token_2022 {
        pinocchio_token_2022::instructions::SetAuthority {
            account: mint,
            authority: payer,
            authority_type: pinocchio_token_2022::instructions::AuthorityType::MintTokens,
            new_authority: None,
            token_program: &pinocchio_token_2022::ID,
        }
        .invoke()?;
    } else {
        pinocchio_token::instructions::SetAuthority {
            account: mint,
            authority: payer,
            authority_type: pinocchio_token::instructions::AuthorityType::MintTokens,
            new_authority: None,
        }
        .invoke()?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    extern crate std;

    use litesvm::LiteSVM;
    use pinocchio::Address;
    use shared::helpers::to_fixed_bytes;
    use solana_sdk::{
        instruction::Instruction, message::AccountMeta, native_token::LAMPORTS_PER_SOL,
        program_pack::Pack, signature::Keypair, signer::Signer, transaction::Transaction,
    };
    use spl_token_2022_interface::extension::StateWithExtensions;
    use spl_token_metadata_interface::state::TokenMetadata;

    use crate::{instructions::CreateMintArgs, BasicMintInstruction};

    #[test]
    fn create_mint_token() {
        let mut svm = LiteSVM::new();

        let payer = Keypair::new();

        let program_id = Address::new_from_array(crate::ID);

        svm.add_program_from_file(program_id, "../../target/deploy/basic_mint.so")
            .unwrap();

        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let mint = Keypair::new();

        let name: [u8; 32] = to_fixed_bytes("Goni098");
        let symbol: [u8; 8] = to_fixed_bytes("Goni");
        let uri: [u8; 64] = to_fixed_bytes("https://github.com/goni098");

        let ix_data = BasicMintInstruction::CreateMint(CreateMintArgs {
            decimals: 8,
            name,
            symbol,
            uri,
        });

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(mint.pubkey(), true),
                AccountMeta::new_readonly(spl_token_interface::ID, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&ix_data).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer, &mint],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();

        std::println!("Program executed successfully!");
        std::println!("Transaction logs: {:#?}", result.logs);

        let mint = svm.get_account(&mint.pubkey()).unwrap();

        let mint_data = spl_token_interface::state::Mint::unpack(&mint.data).unwrap();

        assert_eq!(mint_data.decimals, 8);
        assert!(mint_data.mint_authority.is_none());
        assert!(mint_data.freeze_authority.is_none());
    }

    #[test]
    fn calculate_mint_space() {
        use spl_token_2022_interface::extension::ExtensionType;
        use spl_token_2022_interface::state::Mint;
        use spl_token_metadata_interface::state::TokenMetadata;
        use spl_type_length_value::variable_len_pack::VariableLenPack;

        let mint = Keypair::new();
        let payer = Keypair::new();

        let mint_space =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
                .unwrap();

        let token_metadata = TokenMetadata {
            update_authority: Some(payer.pubkey()).try_into().unwrap(),
            mint: mint.pubkey(),
            name: std::string::String::from("Goni098"),
            symbol: std::string::String::from("Goni"),
            uri: std::string::String::from("https://github.com/goni098"),
            additional_metadata: [].to_vec(),
        };

        let metadata_space = token_metadata.get_packed_len().unwrap();

        std::println!("mint space {}", mint_space);
        std::println!("token_metadata space {}", metadata_space);
    }

    #[test]
    fn create_mint_token_2022_with_metadata() {
        use spl_token_2022_interface::{
            extension::{metadata_pointer::MetadataPointer, BaseStateWithExtensions},
            state::Mint,
        };

        let mut svm = LiteSVM::new();

        let payer = Keypair::new();
        let mint = Keypair::new();

        let program_id = Address::new_from_array(crate::ID);

        svm.add_program_from_file(program_id, "../../target/deploy/basic_mint.so")
            .unwrap();

        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let name: [u8; 32] = to_fixed_bytes("Goni098");
        let symbol: [u8; 8] = to_fixed_bytes("Goni");
        let uri: [u8; 64] = to_fixed_bytes("https://github.com/goni098");

        let ix_data = BasicMintInstruction::CreateMint(CreateMintArgs {
            decimals: 8,
            name,
            symbol,
            uri,
        });

        let ix = Instruction {
            program_id,
            accounts: [
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(mint.pubkey(), true),
                AccountMeta::new_readonly(spl_token_2022_interface::ID, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ]
            .to_vec(),
            data: borsh::to_vec(&ix_data).unwrap(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer, &mint],
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx).unwrap();

        std::println!("Transaction logs:\n{:#?}", result.logs);

        // -------------------------
        // ASSERTIONS
        // -------------------------

        let mint_account = svm.get_account(&mint.pubkey()).unwrap();

        let mint_state = StateWithExtensions::<Mint>::unpack(&mint_account.data).unwrap();

        assert_eq!(mint_state.base.decimals, 8);
        assert!(mint_state.base.mint_authority.is_none());
        assert!(mint_state.base.freeze_authority.is_none());

        // --- MetadataPointer extension ---

        let enabled_extentions = mint_state.get_extension_types().unwrap();
        std::dbg!(enabled_extentions);

        let metadata_pointer = mint_state.get_extension::<MetadataPointer>().unwrap();
        let token_metadata = mint_state
            .get_variable_len_extension::<TokenMetadata>()
            .unwrap();

        std::dbg!(metadata_pointer);

        assert_eq!(token_metadata.name, "Goni098");
        assert_eq!(token_metadata.symbol, "Goni");
        assert_eq!(token_metadata.uri, "https://github.com/goni098");
    }
}
