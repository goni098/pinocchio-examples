use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::Address;
use shank::ShankAccount;

use crate::{
    types::{PostStatus, UserStatus},
    ID,
};

// ─── BoardConfig ───────────────────────────────────────────────────────────────

/// Global board configuration stored in a singleton PDA.
#[derive(BorshDeserialize, BorshSerialize, ShankAccount)]
pub struct BoardConfig {
    /// Public key of the board administrator.
    pub admin: [u8; 32],
    /// Minimum lamports a user must hold to post (spam prevention).
    pub post_fee: u64,
    /// Total number of posts ever created on this board.
    pub post_count: u64,
    /// When `true`, new posts are disabled board-wide.
    pub is_paused: bool,
    /// PDA bump seed.
    pub bump: u8,
}

impl BoardConfig {
    pub const SPACE: usize = 32  // admin
        + 8  // post_fee
        + 8  // post_count
        + 1  // is_paused
        + 1; // bump

    pub const SEED: &[u8; 5] = b"board";

    pub fn derive() -> (Address, u8) {
        Address::find_program_address(&[Self::SEED], &ID.into())
    }
}

// ─── UserProfile ───────────────────────────────────────────────────────────────

/// Per-user profile stored in a PDA seeded by the user's public key.
#[derive(BorshDeserialize, BorshSerialize, ShankAccount)]
pub struct UserProfile {
    /// The wallet that owns and controls this profile.
    pub authority: [u8; 32],
    /// Display username (UTF-8, zero-padded to 32 bytes).
    pub username: [u8; 32],
    /// Total number of posts this user has created.
    pub post_count: u32,
    /// Current status of this profile.
    pub status: UserStatus,
    /// PDA bump seed.
    pub bump: u8,
}

impl UserProfile {
    pub const SPACE: usize = 32  // authority
        + 32 // username
        + 4  // post_count
        + 1  // status enum discriminant
        + 1; // bump

    pub const SEED: &[u8; 7] = b"profile";

    pub fn derive(authority: &Address) -> (Address, u8) {
        Address::find_program_address(&[Self::SEED, authority.as_ref()], &ID.into())
    }
}

// ─── Post ──────────────────────────────────────────────────────────────────────

/// A single post stored in a PDA seeded by [author, post_index].
#[derive(BorshDeserialize, BorshSerialize, ShankAccount)]
pub struct Post {
    /// Public key of the post's author.
    pub author: [u8; 32],
    /// Message content (UTF-8, zero-padded to 128 bytes).
    pub content: [u8; 128],
    /// Public key of the parent post, present when this is a reply.
    pub reply_to: Option<[u8; 32]>,
    /// Current visibility status of this post.
    pub status: PostStatus,
    /// Sequential index assigned at post creation time.
    pub post_index: u32,
    /// PDA bump seed.
    pub bump: u8,
}

impl Post {
    pub const SPACE: usize = 32   // author
        + 128  // content
        + 1    // reply_to Option discriminant
        + 32   // reply_to data (always reserved)
        + 1    // status enum discriminant
        + 4    // post_index
        + 1;   // bump

    pub const SEED: &[u8; 4] = b"post";

    pub fn derive(author: &Address, post_index: u32) -> (Address, u8) {
        Address::find_program_address(
            &[Self::SEED, author.as_ref(), &post_index.to_le_bytes()],
            &ID.into(),
        )
    }
}
