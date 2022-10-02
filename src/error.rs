use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MerkleTreeError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Hasher: {0}")]
    Hasher(#[from] HasherError),

    #[error("Total leaf exceed maximum leaf")]
    ExceedMaxLeaf,
}

#[derive(Debug, Error)]
pub enum HasherError {
    #[error(transparent)]
    Error(#[from] Box<dyn std::error::Error>),

    #[error("{0}")]
    Custom(String),
}

impl HasherError {
    pub fn custom(description: impl ToString) -> Self {
        Self::Custom(description.to_string())
    }
}
