use std::fmt::Debug;

use cosmwasm_std::Storage;
use serde::{de::DeserializeOwned, Serialize};

use crate::{HasherError, MerkleTreeError};

pub trait Hasher<T>: Clone + Debug {
    fn hash_two(&self, left: &T, right: &T) -> Result<T, HasherError>;
}

pub trait MerkleTree<L: Serialize + DeserializeOwned + Clone + Debug + PartialEq, H: Hasher<L>> {
    fn init(
        &self,
        storage: &mut dyn Storage,
        level: u8,
        default_leaf: L,
        hasher: &H,
    ) -> Result<(), MerkleTreeError>;

    fn is_valid_root(&self, storage: &dyn Storage, root: &L) -> Result<bool, MerkleTreeError>;

    fn insert(
        &self,
        storage: &mut dyn Storage,
        leaf: L,
        hasher: &H,
    ) -> Result<(u64, L), MerkleTreeError>;

    fn get_latest_root(&self, storage: &dyn Storage) -> Result<L, MerkleTreeError>;
}
