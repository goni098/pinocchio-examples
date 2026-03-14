use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankType;

/// Emitted when the board is initialized for the first time.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct BoardInitialized {
    /// Public key of the admin who created the board.
    pub admin: [u8; 32],
    /// Minimum lamports required to post.
    pub post_fee: u64,
}

/// Emitted when a new user profile is created.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct ProfileCreated {
    /// Public key of the new profile's authority.
    pub authority: [u8; 32],
    /// Display username chosen at registration.
    pub username: [u8; 32],
}

/// Emitted when a user updates their username.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct UsernameUpdated {
    /// Public key of the profile's authority.
    pub authority: [u8; 32],
    /// The new username.
    pub new_username: [u8; 32],
}

/// Emitted when a new post is published.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct MessagePosted {
    /// Public key of the post author.
    pub author: [u8; 32],
    /// Sequential index of the post within the author's feed.
    pub post_index: u32,
    /// Public key of the parent post, or `None` for a top-level post.
    pub reply_to: Option<[u8; 32]>,
}

/// Emitted when a post is deleted.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct MessageDeleted {
    /// Public key of the post PDA that was deleted.
    pub post: [u8; 32],
    /// Public key of whoever initiated the deletion.
    pub deleted_by: [u8; 32],
}

/// Emitted when a profile is suspended by an admin.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
pub struct ProfileSuspended {
    /// Public key of the suspended profile's authority.
    pub profile_authority: [u8; 32],
}

/// Top-level event enum used for log-based event routing.
#[derive(BorshSerialize, BorshDeserialize, ShankType)]
#[allow(clippy::enum_variant_names)]
pub enum BoardEvent {
    BoardInitialized(BoardInitialized),
    ProfileCreated(ProfileCreated),
    UsernameUpdated(UsernameUpdated),
    MessagePosted(MessagePosted),
    MessageDeleted(MessageDeleted),
    ProfileSuspended(ProfileSuspended),
}
