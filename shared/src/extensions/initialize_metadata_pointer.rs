//! InitializeMetadataPointer - Token-2022 Extension 39
//!
//! Initializes the MetadataPointer extension on a mint account, pointing to
//! where the token metadata will be stored.
//!
//! **IMPORTANT**: Must be called BEFORE `InitializeMint2`.
//!
//! ## Discriminator
//!
//! - First byte: 39 (MetadataPointerExtension)
//! - Second byte: 0 (Initialize variant)
//!
//! ## Data Layout (66 bytes)
//!
//! ```text
//! [0]:      39 (extension type)
//! [1]:      0 (initialize)
//! [2..34]:  authority (OptionalNonZeroPubkey, 32 bytes)
//! [34..66]: metadata_address (OptionalNonZeroPubkey, 32 bytes)
//! ```
//!
//! ## Reference
//!
//! - Source: `spl-token-2022/src/extension/metadata_pointer/instruction.rs`
//! - Extension enum: line 22-56
//! - Data struct: line 58-68

use core::slice::from_raw_parts;
use pinocchio::{
    cpi::{invoke_signed, Signer},
    instruction::{InstructionAccount, InstructionView},
    AccountView, Address, ProgramResult,
};

use crate::{write_bytes, UNINIT_BYTE};

pub struct InitializeMetadataPointer<'a> {
    /// The mint account to initialize the extension on.
    pub mint: &'a AccountView,
    /// The address where metadata will be stored (often the mint itself for on-mint metadata).
    pub metadata_address: Option<&'a Address>,
    /// The authority that can update the metadata pointer.
    pub authority: Option<&'a Address>,
}

impl InitializeMetadataPointer<'_> {
    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(&self, signers: &[Signer]) -> ProgramResult {
        // Account metadata
        let account_metas: [InstructionAccount; 1] =
            [InstructionAccount::writable(self.mint.address())];

        // Instruction data layout:
        // -  [0]: extension type discriminator (1 byte, u8) = 39
        // -  [1]: initialize discriminator (1 byte, u8) = 0
        // -  [2..34]: authority (OptionalNonZeroPubkey, 32 bytes)
        // -  [34..66]: metadata_address (OptionalNonZeroPubkey, 32 bytes)
        let mut instruction_data = [UNINIT_BYTE; 66];

        // Set extension type discriminator as u8 at offset [0]
        write_bytes(&mut instruction_data[0..1], &[39]);
        // Set initialize discriminator as u8 at offset [1]
        write_bytes(&mut instruction_data[1..2], &[0]);

        // Set authority as OptionalNonZeroPubkey at offset [2..34]
        if let Some(authority) = self.authority {
            write_bytes(&mut instruction_data[2..34], authority.as_ref());
        } else {
            write_bytes(&mut instruction_data[2..34], &[0u8; 32]);
        }

        // Set metadata_address as OptionalNonZeroPubkey at offset [34..66]
        if let Some(metadata_addr) = self.metadata_address {
            write_bytes(&mut instruction_data[34..66], metadata_addr.as_ref());
        } else {
            write_bytes(&mut instruction_data[34..66], &[0u8; 32]);
        }

        let instruction = InstructionView {
            program_id: &pinocchio_token_2022::ID,
            accounts: &account_metas,
            data: unsafe { from_raw_parts(instruction_data.as_ptr() as _, 66) },
        };

        invoke_signed(&instruction, &[self.mint], signers)
    }
}
