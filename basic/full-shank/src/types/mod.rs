use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;

// ─── Status Enums ──────────────────────────────────────────────────────────────

/// Lifecycle status of a user profile.
#[derive(BorshDeserialize, BorshSerialize, ShankType, Clone, PartialEq, Debug)]
pub enum UserStatus {
    /// Profile is active and can post.
    Active,
    /// Profile has been suspended by an admin.
    Suspended,
}

/// Lifecycle status of a post.
#[derive(BorshDeserialize, BorshSerialize, ShankType, Clone, PartialEq, Debug)]
pub enum PostStatus {
    /// Post is publicly visible.
    Published,
    /// Post has been deleted; account is closed.
    Deleted,
}

// ─── Instruction Argument Structs ──────────────────────────────────────────────

/// Arguments for the `InitBoard` instruction.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct InitBoardArgs {
    /// Minimum lamports required to post (spam prevention fee).
    pub post_fee: u64,
}

/// Arguments for the `CreateProfile` instruction.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct CreateProfileArgs {
    /// Display username encoded as UTF-8 bytes (max 32 bytes, zero-padded).
    pub username: [u8; 32],
}

/// Arguments for the `PostMessage` instruction.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct PostMessageArgs {
    /// Message body encoded as UTF-8 bytes (max 128 bytes, zero-padded).
    pub content: [u8; 128],
    /// Public key of the post being replied to, or `None` for a top-level post.
    pub reply_to: Option<[u8; 32]>,
}

/// Arguments for the `UpdateUsername` instruction.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct UpdateUsernameArgs {
    /// New display username (max 32 bytes, zero-padded).
    pub username: [u8; 32],
}
