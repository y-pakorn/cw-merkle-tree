use std::fmt::Debug;

use cosmwasm_std::Empty;
use cw_storage_plus::{Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Hasher, MerkleTree};

use super::SparseMerkleTree;

pub struct SparseMerkleTreeWithHistory<
    'a,
    L: Serialize + DeserializeOwned + Clone + Debug + PartialEq,
    H: Hasher<L>,
> where
    for<'r> &'r L: PrimaryKey<'r>,
{
    pub tree: SparseMerkleTree<'a, L, H>,
    pub root_history: Map<'a, &'a L, Empty>,
}

impl<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq, H: Hasher<L>>
    SparseMerkleTreeWithHistory<'a, L, H>
where
    for<'r> &'r L: PrimaryKey<'r>,
{
    pub const fn new(
        hashes_ns: &'a str,
        leafs_ns: &'a str,
        level_ns: &'a str,
        zeros_ns: &'a str,
        root_history_ns: &'a str,
    ) -> Self {
        Self {
            tree: SparseMerkleTree::new(hashes_ns, leafs_ns, level_ns, zeros_ns),
            root_history: Map::new(root_history_ns),
        }
    }
}

impl<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq, H: Hasher<L>> MerkleTree<L, H>
    for SparseMerkleTreeWithHistory<'a, L, H>
where
    for<'r> &'r L: PrimaryKey<'r>,
{
    fn init(
        &self,
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
        &self,
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
        self.tree.get_latest_root(storage)
    }
}
