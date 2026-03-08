use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token_2022::instructions::metadata_pointer::Initialize as InitializeMetadataPointer;
use shank::ShankType;

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

    // For Token-2022 with embedded metadata:
    // - Allocate exactly `try_calculate_account_len(&[MetadataPointer])` = 234 bytes so that
    //   both MetadataPointerInstruction::Initialize and InitializeMint2 (which does a strict
    //   size == try_calculate_account_len check) succeed.
    // - Pre-fund with lamports for the *full* final size, because InitializeTokenMetadata
    //   internally reallocates the account via `AccountInfo::realloc` and assumes the
    //   account already holds enough SOL for the new rent-exempt balance.
    let (space, lamport_space) = if is_spl_token {
        let s = pinocchio_token::state::Mint::LEN;
        (s, s)
    } else {
        use core::mem::size_of;
        use pinocchio::Address;

        // Token-2022 pads the base Mint up to TokenAccount::BASE_LEN, then adds 1 byte for the
        // account type tag before extensions begin.
        const EXTENSIONS_OFFSET: usize = pinocchio_token_2022::state::TokenAccount::BASE_LEN + 1
            - pinocchio_token_2022::state::Mint::BASE_LEN;

        // MetadataPointer extension TLV: 2 (type u16) + 2 (len u16) + 32 (authority) + 32 (metadata_address)
        const METADATA_POINTER_SIZE: usize = size_of::<u16>() * 2 + size_of::<Address>() * 2;

        // Account space required before InitializeTokenMetadata reallocs.
        let account_space =
            pinocchio_token_2022::state::Mint::BASE_LEN + EXTENSIONS_OFFSET + METADATA_POINTER_SIZE;

        // TokenMetadata TLV header: 8 (ArrayDiscriminator) + 4 (PodU32 length field)
        const METADATA_TLV_HEADER: usize = 8 + 4;

        // TokenMetadata base payload: update_authority (32) + mint (32) + 4×(u32 string-length prefix)
        const METADATA_BASE_PAYLOAD: usize = size_of::<Address>() * 2 + size_of::<u32>() * 4;

        // Compute actual string lengths (null-terminated in fixed arrays)
        let name_len = args
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(args.name.len());
        let symbol_len = args
            .symbol
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(args.symbol.len());
        let uri_len = args
            .uri
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(args.uri.len());

        let full_space = account_space
            + METADATA_TLV_HEADER
            + METADATA_BASE_PAYLOAD
            + name_len
            + symbol_len
            + uri_len;

        (account_space, full_space)
    };

    let rent = Rent::get()?;

    CreateAccount {
        from: payer,
        to: mint,
        lamports: rent.minimum_balance_unchecked(lamport_space),
        space: space as u64,
        owner: token_program.address(),
    }
    .invoke()?;

    if is_token_2022 {
        // Initialize metadata pointer extension
        // This must be done before InitializeMint
        InitializeMetadataPointer {
            authority: None,
            metadata_address: Some(mint.address()),
            mint,
            token_program: token_program.address(),
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
        // Discriminator = sha256("spl_token_metadata_interface:initialize_account")[..8]
        const DISCRIMINATOR: [u8; 8] = [210, 225, 30, 162, 88, 184, 77, 141];

        let name = shared::helpers::from_fixed_bytes(&args.name)?;
        let symbol = shared::helpers::from_fixed_bytes(&args.symbol)?;
        let uri = shared::helpers::from_fixed_bytes(&args.uri)?;

        let name_b = name.as_bytes();
        let symbol_b = symbol.as_bytes();
        let uri_b = uri.as_bytes();

        // Max ix_data size: 8 + (4+32) + (4+8) + (4+64) = 124
        let mut ix_data = [0u8; 128];
        let mut pos = 0;

        ix_data[pos..pos + 8].copy_from_slice(&DISCRIMINATOR);
        pos += 8;

        ix_data[pos..pos + 4].copy_from_slice(&(name_b.len() as u32).to_le_bytes());
        pos += 4;
        ix_data[pos..pos + name_b.len()].copy_from_slice(name_b);
        pos += name_b.len();

        ix_data[pos..pos + 4].copy_from_slice(&(symbol_b.len() as u32).to_le_bytes());
        pos += 4;
        ix_data[pos..pos + symbol_b.len()].copy_from_slice(symbol_b);
        pos += symbol_b.len();

        ix_data[pos..pos + 4].copy_from_slice(&(uri_b.len() as u32).to_le_bytes());
        pos += 4;
        ix_data[pos..pos + uri_b.len()].copy_from_slice(uri_b);
        pos += uri_b.len();

        {
            use pinocchio::cpi::{invoke_signed_unchecked, CpiAccount};
            use pinocchio::instruction::{InstructionAccount, InstructionView};

            let instruction_accounts = [
                InstructionAccount::writable(mint.address()),
                InstructionAccount::readonly(payer.address()),
                InstructionAccount::readonly(mint.address()),
                InstructionAccount::readonly_signer(payer.address()),
            ];

            let instruction = InstructionView {
                program_id: &pinocchio_token_2022::ID,
                accounts: &instruction_accounts,
                data: &ix_data[..pos],
            };

            // `invoke` requires account_views.len() == instruction.accounts.len()
            // and no duplicate borrows. TokenMetadata::initialize_account has 4
            // accounts where mint appears at [0] and [2], payer at [1] and [3].
            // We use invoke_signed_unchecked with CpiAccount (raw-pointer-based)
            // so the same AccountView can appear in multiple slots.
            //
            // SAFETY: mint and payer are not mutably borrowed elsewhere.
            // Token-2022 reads mint at [2] after writing to it at [0], so the
            // aliased access is benign within the sub-program's sequential logic.
            let cpi_accounts = [
                CpiAccount::from(mint as &AccountView),
                CpiAccount::from(payer as &AccountView),
                CpiAccount::from(mint as &AccountView),
                CpiAccount::from(payer as &AccountView),
            ];

            unsafe { invoke_signed_unchecked(&instruction, &cpi_accounts, &[]) }
        }
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
            multisig_signers: &[],
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
