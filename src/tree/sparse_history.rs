use std::fmt::Debug;

use cosmwasm_std::Empty;
use cw_storage_plus::{Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Hasher, MerkleTree};

use super::SparseMerkleTree;

pub struct SparseMerkleTreeWithHistory<
    'a,
    L: Serialize + DeserializeOwned + Clone + Debug + PartialEq,
> where
    for<'r> &'r L: PrimaryKey<'r>,
{
    pub tree: SparseMerkleTree<'a, L>,
    pub root_history: Map<'a, &'a L, Empty>,
}

impl<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq, H: Hasher<L>> MerkleTree<L, H>
    for SparseMerkleTreeWithHistory<'a, L>
where
    for<'r> &'r L: PrimaryKey<'r>,
{
    fn init(
        &mut self,
        storage: &mut dyn cosmwasm_std::Storage,
        level: u8,
        default_leaf: L,
        hasher: &H,
    ) -> Result<(), crate::MerkleTreeError> {
        self.tree.init(storage, level, default_leaf, hasher)
    }

    fn is_valid_root(
        &self,
        storage: &dyn cosmwasm_std::Storage,
        root: &L,
    ) -> Result<bool, crate::MerkleTreeError> {
        Ok(self.root_history.has(storage, root))
    }

    fn insert(
        &mut self,
        storage: &mut dyn cosmwasm_std::Storage,
        leaf: L,
        hasher: &H,
    ) -> Result<(u64, L), crate::MerkleTreeError> {
        let (index, latest_root) = self.tree.insert(storage, leaf, hasher)?;

        self.root_history.save(storage, &latest_root, &Empty {})?;

        Ok((index, latest_root))
    }

    fn get_latest_root(
        &self,
        storage: &dyn cosmwasm_std::Storage,
    ) -> Result<L, crate::MerkleTreeError> {
        MerkleTree::<_, H>::get_latest_root(&self.tree, storage)
    }
}
