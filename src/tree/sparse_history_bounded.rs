use std::fmt::Debug;

use cosmwasm_std::{Empty, Order, Storage};
use cw_storage_plus::{Bound, Item, Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Hasher, MerkleTree, MerkleTreeError};

use super::SparseMerkleTree;

pub struct SparseMerkleTreeWithHistoryBounded<
    'a,
    L: Serialize + DeserializeOwned + Clone + Debug + PartialEq + PrimaryKey<'a>,
    H: Hasher<L>,
    const HISTORY_LEVEL: u32,
> {
    pub history_index: Item<'a, u32>,
    pub root_history: Map<'a, L, Empty>,
    pub root_index: Map<'a, u32, L>,
    pub tree: SparseMerkleTree<'a, L, H>,
}

impl<
        'a,
        L: Serialize + DeserializeOwned + Clone + Debug + PartialEq + PrimaryKey<'a>,
        H: Hasher<L>,
        const HISTORY_LEVEL: u32,
    > SparseMerkleTreeWithHistoryBounded<'a, L, H, HISTORY_LEVEL>
{
    pub const fn new(
        hashes_ns: &'a str,
        leafs_ns: &'a str,
        level_ns: &'a str,
        zeros_ns: &'a str,
        root_history_ns: &'a str,
        root_index_ns: &'a str,
        history_index_ns: &'a str,
    ) -> Self {
        Self {
            history_index: Item::new(history_index_ns),
            root_history: Map::new(root_history_ns),
            root_index: Map::new(root_index_ns),
            tree: SparseMerkleTree::new(hashes_ns, leafs_ns, level_ns, zeros_ns),
        }
    }

    /// Remove storage unused and out of range stored root.
    /// The removed root might not be the most recent.
    pub fn update_history_level(&self, storage: &mut dyn Storage) -> Result<(), MerkleTreeError> {
        let updated_idx = self.history_index.may_load(storage)?.unwrap_or_default() % HISTORY_LEVEL;
        self.history_index.save(storage, &updated_idx)?;

        let mut root_range = self
            .root_index
            .range(
                storage,
                Some(Bound::inclusive(HISTORY_LEVEL)),
                None,
                Order::Ascending,
            )
            .collect::<Vec<_>>()
            .into_iter();
        while let Some(Ok((idx, root))) = root_range.next() {
            self.root_index.remove(storage, idx);
            self.root_history.remove(storage, root);
        }

        Ok(())
    }
}

impl<
        'a,
        L: Serialize + DeserializeOwned + Clone + Debug + PartialEq + PrimaryKey<'a>,
        H: Hasher<L>,
        const HISTORY_LEVEL: u32,
    > MerkleTree<L, H> for SparseMerkleTreeWithHistoryBounded<'a, L, H, HISTORY_LEVEL>
{
    fn init(
        &self,
        storage: &mut dyn Storage,
        level: u8,
        default_leaf: L,
        hasher: &H,
    ) -> Result<(), MerkleTreeError> {
        self.tree.init(storage, level, default_leaf, hasher)
    }

    fn is_valid_root(&self, storage: &dyn Storage, root: &L) -> Result<bool, MerkleTreeError> {
        Ok(self.root_history.has(storage, root.clone()))
    }

    fn insert(
        &self,
        storage: &mut dyn Storage,
        leaf: L,
        hasher: &H,
    ) -> Result<(u64, L), MerkleTreeError> {
        let (index, latest_root) = self.tree.insert(storage, leaf, hasher)?;
        let cur_idx = self.history_index.may_load(storage)?.unwrap_or_default();
        let next_idx = (cur_idx + 1) % HISTORY_LEVEL;

        // Remove old root
        if let Some(root) = self.root_index.may_load(storage, next_idx)? {
            self.root_history.remove(storage, root);
        }

        // Insert new root
        self.root_history
            .save(storage, latest_root.clone(), &Empty {})?;
        self.root_index.save(storage, next_idx, &latest_root)?;

        // Update current index
        self.history_index.save(storage, &next_idx)?;

        Ok((index, latest_root))
    }

    fn get_latest_root(&self, storage: &dyn Storage) -> Result<L, MerkleTreeError> {
        self.tree.get_latest_root(storage)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use cosmwasm_std::{testing::MockStorage, Uint256};

    use crate::{test_utils::Blake2, Hasher, MerkleTree};

    use super::SparseMerkleTreeWithHistoryBounded;

    const TREE: SparseMerkleTreeWithHistoryBounded<Vec<u8>, Blake2, 5> =
        SparseMerkleTreeWithHistoryBounded::new(
            "hashes",
            "leafs",
            "level",
            "zeros",
            "root_history",
            "root_index",
            "history_index",
        );
    const ZERO: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    #[test]
    fn init() -> Result<(), Box<dyn Error>> {
        let mut storage = MockStorage::new();
        let zero_vec = ZERO.to_vec();

        TREE.init(
            &mut storage,
            20,
            Blake2.hash_two(&zero_vec, &zero_vec)?,
            &Blake2,
        )?;

        assert_eq!(
            TREE.get_latest_root(&storage)?,
            [
                20, 114, 250, 18, 41, 94, 49, 107, 184, 78, 231, 47, 187, 225, 122, 14, 76, 178,
                156, 226, 121, 99, 103, 48, 22, 79, 157, 174, 92, 246, 92, 50
            ]
        );

        Ok(())
    }

    #[test]
    fn insert() -> Result<(), Box<dyn Error>> {
        let mut storage = MockStorage::new();
        let zero_vec = ZERO.to_vec();
        let one_vec = Uint256::one().to_be_bytes().to_vec();

        TREE.init(
            &mut storage,
            20,
            Blake2.hash_two(&zero_vec, &zero_vec)?,
            &Blake2,
        )?;

        let leaf = Blake2.hash_two(&one_vec, &one_vec)?;

        let (index, new_root) = TREE.insert(&mut storage, leaf.clone(), &Blake2)?;

        assert_eq!(index, 0);
        assert_eq!(
            new_root,
            [
                45, 48, 180, 75, 130, 217, 36, 211, 56, 209, 169, 100, 90, 90, 130, 183, 22, 180,
                158, 1, 50, 4, 40, 127, 94, 211, 229, 143, 202, 226, 138, 132
            ]
        );
        assert_eq!(new_root, TREE.get_latest_root(&storage)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        let (index, new_root) = TREE.insert(&mut storage, leaf, &Blake2)?;

        assert_eq!(index, 1);
        assert_eq!(
            new_root,
            [
                38, 223, 223, 196, 242, 242, 23, 6, 14, 235, 4, 249, 197, 168, 160, 77, 102, 150,
                4, 52, 233, 58, 198, 244, 107, 32, 147, 134, 58, 154, 177, 116
            ]
        );
        assert_eq!(new_root, TREE.get_latest_root(&storage)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        Ok(())
    }

    #[test]
    fn root_history() -> Result<(), Box<dyn Error>> {
        let mut storage = MockStorage::new();
        let zero_vec = ZERO.to_vec();
        let one_vec = Uint256::one().to_be_bytes().to_vec();

        TREE.init(
            &mut storage,
            20,
            Blake2.hash_two(&zero_vec, &zero_vec)?,
            &Blake2,
        )?;

        let leaf = Blake2.hash_two(&one_vec, &one_vec)?;

        let (_, very_old_root) = TREE.insert(&mut storage, leaf.clone(), &Blake2)?;
        let (_, old_root) = TREE.insert(&mut storage, leaf.clone(), &Blake2)?;
        let _ = TREE.insert(&mut storage, leaf.clone(), &Blake2)?;
        let _ = TREE.insert(&mut storage, leaf.clone(), &Blake2)?;
        let _ = TREE.insert(&mut storage, leaf.clone(), &Blake2)?;
        let (_, new_root) = TREE.insert(&mut storage, leaf, &Blake2)?;

        assert!(!TREE.is_valid_root(&storage, &very_old_root)?);
        assert!(TREE.is_valid_root(&storage, &old_root)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        Ok(())
    }
}
