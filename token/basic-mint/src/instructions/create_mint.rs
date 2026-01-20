use core::slice::from_raw_parts;

use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    cpi::{invoke, Seed, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    sysvars::{rent::Rent, Sysvar},
    AccountView, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token_2022::instructions::InitializeMint2;
use shank::ShankType;
use solana_address::Address;

use crate::ID;

/// Size constants for Token-2022 mint with metadata extension
const MINT_SIZE: usize = 82;
const ACCOUNT_TYPE_SIZE: usize = 1;
const EXTENSION_TYPE_SIZE: usize = 2;
const LENGTH_SIZE: usize = 2;
const METADATA_POINTER_SIZE: usize = 64; // authority (32) + metadata_address (32)

/// Token-2022 instruction discriminators
const METADATA_POINTER_INIT_DISCRIMINATOR: [u8; 2] = [39, 0]; // Extension instruction 39, sub-instruction 0
const TOKEN_METADATA_INIT_DISCRIMINATOR: [u8; 8] = [210, 225, 30, 162, 88, 184, 77, 141]; // spl_token_metadata_interface::Initialize

/// Seed prefix for mint PDA
pub const MINT_SEED_PREFIX: &[u8; 4] = b"mint";

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

/// Calculate the minimum space needed for a mint with metadata pointer and token metadata
fn calculate_mint_space(name_len: usize, symbol_len: usize, uri_len: usize) -> usize {
    // Base mint size
    let mut size = MINT_SIZE;

    // Account type byte (required for extensions)
    size += ACCOUNT_TYPE_SIZE;

    // Metadata pointer extension (type + length + value)
    size += EXTENSION_TYPE_SIZE + LENGTH_SIZE + METADATA_POINTER_SIZE;

    // Token metadata extension (type + length + value)
    // TokenMetadata struct: update_authority (32) + mint (32) + name (4 + len) + symbol (4 + len) + uri (4 + len) + additional_metadata (4)
    let metadata_value_size = 32 + 32 + 4 + name_len + 4 + symbol_len + 4 + uri_len + 4;
    size += EXTENSION_TYPE_SIZE + LENGTH_SIZE + metadata_value_size;

    size
}

/// Get the actual length of a fixed-size byte array (until first zero or end)
fn get_string_len(bytes: &[u8]) -> usize {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

/// Derive the mint PDA address
pub fn derive_mint_pda(program_id: &Address) -> (Address, u8) {
    Address::find_program_address(&[MINT_SEED_PREFIX], program_id)
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

    // Validate token program is Token-2022
    if token_program.address().ne(&pinocchio_token_2022::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Validate system program
    if system_program.address().ne(&pinocchio_system::ID) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Derive mint PDA
    let (pda, bump) = derive_mint_pda(&Address::new_from_array(ID));

    if mint.address().as_array().ne(pda.as_array()) {
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if mint already exists
    if mint.lamports().ne(&0) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Calculate string lengths
    let name_len = get_string_len(&args.name);
    let symbol_len = get_string_len(&args.symbol);
    let uri_len = get_string_len(&args.uri);

    // Calculate space needed
    let account_span = calculate_mint_space(name_len, symbol_len, uri_len);
    let lamports_required = Rent::get()?.minimum_balance_unchecked(account_span);

    // Create signer seeds
    let bump_bytes = &[bump];
    let seeds = [
        Seed::from(MINT_SEED_PREFIX.as_slice()),
        Seed::from(bump_bytes),
    ];
    let signers = Signer::from(&seeds);

    // Create the mint account
    CreateAccount {
        from: payer,
        to: mint,
        lamports: lamports_required,
        space: account_span as u64,
        owner: token_program.address(),
    }
    .invoke_signed(&[signers])?;

    // Initialize metadata pointer extension
    // This must be done before InitializeMint
    initialize_metadata_pointer(
        mint,
        payer.address(),
        mint.address(),
        token_program.address(),
    )?;

    // Initialize the mint
    InitializeMint2 {
        mint,
        decimals: args.decimals,
        mint_authority: payer.address(),
        freeze_authority: Some(payer.address()),
        token_program: token_program.address(),
    }
    .invoke()?;

    // Initialize token metadata
    initialize_token_metadata(
        mint,
        payer.address(),
        mint.address(),
        payer,
        &args.name[..name_len],
        &args.symbol[..symbol_len],
        &args.uri[..uri_len],
        token_program.address(),
    )?;

    Ok(())
}

/// Initialize the metadata pointer extension on the mint
fn initialize_metadata_pointer(
    mint: &AccountView,
    authority: &pinocchio::Address,
    metadata_address: &pinocchio::Address,
    token_program: &pinocchio::Address,
) -> ProgramResult {
    // Instruction data layout:
    // - [0]: Token instruction discriminator (39 = MetadataPointerExtension)
    // - [1]: Sub-instruction (0 = Initialize)
    // - [2..34]: authority (32 bytes)
    // - [34..66]: metadata_address (32 bytes)
    let mut instruction_data = [0u8; 66];
    instruction_data[0] = METADATA_POINTER_INIT_DISCRIMINATOR[0];
    instruction_data[1] = METADATA_POINTER_INIT_DISCRIMINATOR[1];
    instruction_data[2..34].copy_from_slice(authority.as_array());
    instruction_data[34..66].copy_from_slice(metadata_address.as_array());

    let accounts = [InstructionAccount::writable(mint.address())];

    let instruction = InstructionView {
        program_id: token_program,
        accounts: &accounts,
        data: &instruction_data,
    };

    invoke(&instruction, &[mint])
}

/// Initialize token metadata on the mint
fn initialize_token_metadata(
    mint: &AccountView,
    update_authority: &pinocchio::Address,
    mint_address: &pinocchio::Address,
    mint_authority: &AccountView,
    name: &[u8],
    symbol: &[u8],
    uri: &[u8],
    token_program: &pinocchio::Address,
) -> ProgramResult {
    // Calculate instruction data size
    // - 8 bytes: discriminator
    // - 4 bytes: name length (u32)
    // - name_len bytes: name
    // - 4 bytes: symbol length (u32)
    // - symbol_len bytes: symbol
    // - 4 bytes: uri length (u32)
    // - uri_len bytes: uri
    let data_len = 8 + 4 + name.len() + 4 + symbol.len() + 4 + uri.len();

    // Build instruction data
    let mut instruction_data = [0u8; 256]; // Max size buffer
    let mut offset = 0;

    // Discriminator
    instruction_data[offset..offset + 8].copy_from_slice(&TOKEN_METADATA_INIT_DISCRIMINATOR);
    offset += 8;

    // Name (borsh string: u32 length + bytes)
    instruction_data[offset..offset + 4].copy_from_slice(&(name.len() as u32).to_le_bytes());
    offset += 4;
    instruction_data[offset..offset + name.len()].copy_from_slice(name);
    offset += name.len();

    // Symbol (borsh string: u32 length + bytes)
    instruction_data[offset..offset + 4].copy_from_slice(&(symbol.len() as u32).to_le_bytes());
    offset += 4;
    instruction_data[offset..offset + symbol.len()].copy_from_slice(symbol);
    offset += symbol.len();

    // URI (borsh string: u32 length + bytes)
    instruction_data[offset..offset + 4].copy_from_slice(&(uri.len() as u32).to_le_bytes());
    offset += 4;
    instruction_data[offset..offset + uri.len()].copy_from_slice(uri);

    let accounts = [
        InstructionAccount::writable(mint.address()),
        InstructionAccount::readonly(update_authority),
        InstructionAccount::readonly(mint_address),
        InstructionAccount::readonly_signer(mint_authority.address()),
    ];

    let instruction = InstructionView {
        program_id: token_program,
        accounts: &accounts,
        data: unsafe { from_raw_parts(instruction_data.as_ptr(), data_len) },
    };

    invoke(&instruction, &[mint, mint_authority])
}
