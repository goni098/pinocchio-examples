//! InitializeTokenMetadata - Token Metadata Interface
//!
//! Initializes on-mint token metadata with name, symbol, and URI.
//!
//! **IMPORTANT**: Must be called AFTER `InitializeMint2` and AFTER `InitializeMetadataPointer`.
//!
//! ## Discriminator (SHA256-based)
//!
//! The discriminator is calculated as:
//! ```text
//! SHA256("spl_token_metadata_interface:initialize_account")[0..8]
//! = 0xd2e11ea258b84d8d
//! ```
//!
//! ## Data Layout (variable size)
//!
//! ```text
//! [0..8]:   discriminator (8 bytes) = 0xd2e11ea258b84d8d
//! [8..]:    Borsh-serialized metadata {name, symbol, uri}
//! ```
//!
//! Borsh string encoding:
//! ```text
//! [0..4]:   length as u32 little-endian
//! [4..]:    UTF-8 bytes
//! ```
//!
//! ## Accounts
//!
//! ```text
//! 0. [writable] metadata (the mint account for on-mint metadata)
//! 1. [] update_authority
//! 2. [] mint
//! 3. [signer] mint_authority
//! ```
//!
//! ## Reference
//!
//! - Source: `spl-token-metadata-interface/src/instruction.rs`
//! - Instruction struct: line 18-30
//! - Discriminator: line 22
//! - Test: line 342-356

use core::slice::from_raw_parts;
use pinocchio::{
    cpi::{invoke_signed, Signer},
    instruction::{InstructionAccount, InstructionView},
    AccountView, ProgramResult,
};

use crate::{helpers::serialize_borsh_string, write_bytes, UNINIT_BYTE};

/// The SHA256-based discriminator for InitializeTokenMetadata.
///
/// Calculated as: SHA256("spl_token_metadata_interface:initialize_account")[0..8]
const DISCRIMINATOR: [u8; 8] = [0xd2, 0xe1, 0x1e, 0xa2, 0x58, 0xb8, 0x4d, 0x8d];

pub struct InitializeTokenMetadata<'a> {
    /// The metadata account (same as mint for on-mint metadata).
    pub metadata: &'a AccountView,
    /// The authority that can update the metadata.
    pub update_authority: &'a AccountView,
    /// The mint account.
    pub mint: &'a AccountView,
    /// The mint authority (must be signer).
    pub mint_authority: &'a AccountView,
    /// Token name.
    pub name: &'a str,
    /// Token symbol.
    pub symbol: &'a str,
    /// Token URI (JSON metadata).
    pub uri: &'a str,
}

impl InitializeTokenMetadata<'_> {
    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        self.invoke_signed(&[])
    }

    #[allow(unexpected_cfgs)]
    pub fn invoke_signed(&self, signers: &[Signer]) -> ProgramResult {
        // Account metadata
        let account_metas: [InstructionAccount; 4] = [
            InstructionAccount::writable(self.metadata.address()),
            InstructionAccount::readonly(self.update_authority.address()),
            InstructionAccount::readonly(self.mint.address()),
            InstructionAccount::readonly_signer(self.mint_authority.address()),
        ];

        // Calculate total size needed:
        // - 8 bytes for discriminator
        // - 4 + name.len() for name
        // - 4 + symbol.len() for symbol
        // - 4 + uri.len() for uri
        let name_size = 4 + self.name.len();
        let symbol_size = 4 + self.symbol.len();
        let uri_size = 4 + self.uri.len();
        let total_size = 8 + name_size + symbol_size + uri_size;

        // We use a fixed buffer to maintain zero-allocation.
        // 384 bytes is ~10% of stack and sufficient for:
        // - Discriminator: 8 bytes
        // - name: ~100 bytes max
        // - symbol: ~20 bytes max
        // - uri: ~200 bytes max
        // Total: ~376 bytes
        //
        // For larger metadata, use UpdateField to add additional fields after initialization.
        const MAX_BUFFER_SIZE: usize = 384;
        let mut instruction_data = [UNINIT_BYTE; MAX_BUFFER_SIZE];

        if total_size > MAX_BUFFER_SIZE {
            // Return error if metadata is too large
            // Use UpdateField instruction for additional fields
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }

        // Write discriminator (8 bytes)
        write_bytes(&mut instruction_data[0..8], &DISCRIMINATOR);

        // Write Borsh-serialized strings
        let mut offset = 8;

        // Serialize name
        let name_bytes_len = {
            let temp_slice = unsafe {
                core::slice::from_raw_parts_mut(
                    instruction_data[offset..].as_mut_ptr() as *mut u8,
                    name_size,
                )
            };
            serialize_borsh_string(temp_slice, self.name)
        };
        offset += name_bytes_len;

        // Serialize symbol
        let symbol_bytes_len = {
            let temp_slice = unsafe {
                core::slice::from_raw_parts_mut(
                    instruction_data[offset..].as_mut_ptr() as *mut u8,
                    symbol_size,
                )
            };
            serialize_borsh_string(temp_slice, self.symbol)
        };
        offset += symbol_bytes_len;

        // Serialize uri
        let uri_bytes_len = {
            let temp_slice = unsafe {
                core::slice::from_raw_parts_mut(
                    instruction_data[offset..].as_mut_ptr() as *mut u8,
                    uri_size,
                )
            };
            serialize_borsh_string(temp_slice, self.uri)
        };
        offset += uri_bytes_len;

        let instruction = InstructionView {
            program_id: &pinocchio_token_2022::ID,
            accounts: &account_metas,
            data: unsafe { from_raw_parts(instruction_data.as_ptr() as _, offset) },
        };

        invoke_signed(
            &instruction,
            &[
                self.metadata,
                self.update_authority,
                self.mint,
                self.mint_authority,
            ],
            signers,
        )
    }
}
