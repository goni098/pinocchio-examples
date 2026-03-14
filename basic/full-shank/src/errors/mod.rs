use num_derive::FromPrimitive;
use pinocchio::error::ProgramError;
use thiserror::Error;

/// Errors returned by the PostBoard program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum PostBoardError {
    /// Signer does not match the stored admin key.
    #[error("Caller is not the board admin")]
    NotAdmin,

    /// Board config has already been initialized.
    #[error("Board has already been initialized")]
    BoardAlreadyInitialized,

    /// Board config has not been initialized yet.
    #[error("Board has not been initialized")]
    BoardNotInitialized,

    /// A profile for this authority already exists.
    #[error("Profile already exists for this authority")]
    ProfileAlreadyExists,

    /// No profile found for this authority.
    #[error("Profile not found for this authority")]
    ProfileNotFound,

    /// Signer does not match the post author.
    #[error("Caller is not the post author")]
    NotAuthor,

    /// Post has already been deleted.
    #[error("Post has already been deleted")]
    PostAlreadyDeleted,

    /// User has been suspended and cannot perform this action.
    #[error("User account is suspended")]
    UserSuspended,

    /// Arithmetic overflow while updating a counter.
    #[error("Numerical overflow")]
    NumericalOverflow,
}

impl From<PostBoardError> for ProgramError {
    fn from(e: PostBoardError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
