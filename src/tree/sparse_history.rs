use std::fmt::Debug;

use cosmwasm_std::Empty;
use cw_storage_plus::{Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Hasher, MerkleTree};

use super::SparseMerkleTree;

/// Like [SparseMerkleTree] but able to check valid root hash with all previous root hashes.
pub struct SparseMerkleTreeWithHistory<
    'a,
    L: Serialize + DeserializeOwned + Clone + Debug + PartialEq + PrimaryKey<'a>,
    H: Hasher<L>,
> {
    pub tree: SparseMerkleTree<'a, L, H>,
    pub root_history: Map<'a, L, Empty>,
}

impl<
        'a,
        L: Serialize + DeserializeOwned + Clone + Debug + PartialEq + PrimaryKey<'a>,
        H: Hasher<L>,
    > SparseMerkleTreeWithHistory<'a, L, H>
{
    pub const fn new(
        hashes_ns: &'a str,
        leafs_ns: &'a str,
        level_ns: &'a str,
        root_ns: &'a str,
        root_history_ns: &'a str,
    ) -> Self {
        Self {
            tree: SparseMerkleTree::new(hashes_ns, leafs_ns, level_ns, root_ns),
            root_history: Map::new(root_history_ns),
        }
    }
}

impl<
        'a,
        L: Serialize + DeserializeOwned + Clone + Debug + PartialEq + PrimaryKey<'a>,
        H: Hasher<L>,
    > MerkleTree<L, H> for SparseMerkleTreeWithHistory<'a, L, H>
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
        Ok(self.root_history.has(storage, root.clone()))
    }

    fn insert(
        &self,
        storage: &mut dyn cosmwasm_std::Storage,
        leaf: L,
        hasher: &H,
    ) -> Result<(u64, L), crate::MerkleTreeError> {
        let (index, latest_root) = self.tree.insert(storage, leaf, hasher)?;

        self.root_history
            .save(storage, latest_root.clone(), &Empty {})?;

        Ok((index, latest_root))
    }

    fn get_latest_root(
        &self,
        storage: &dyn cosmwasm_std::Storage,
    ) -> Result<L, crate::MerkleTreeError> {
        self.tree.get_latest_root(storage)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use cosmwasm_std::{testing::MockStorage, Uint256};

    use crate::{test_utils::Blake2, Hasher, MerkleTree};

    use super::SparseMerkleTreeWithHistory;

    const TREE: SparseMerkleTreeWithHistory<Vec<u8>, Blake2> =
        SparseMerkleTreeWithHistory::new("hashes", "leafs", "level", "zeros", "root_history");
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
                144, 77, 181, 73, 235, 223, 13, 204, 30, 18, 199, 252, 182, 160, 89, 248, 240, 219,
                173, 150, 189, 114, 165, 70, 40, 159, 110, 9, 165, 185, 17, 229
            ]
        );
        assert_eq!(new_root, TREE.get_latest_root(&storage)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        let (index, new_root) = TREE.insert(&mut storage, leaf, &Blake2)?;

        assert_eq!(index, 1);
        assert_eq!(
            new_root,
            [
                69, 102, 154, 15, 149, 187, 157, 26, 123, 248, 50, 67, 177, 207, 6, 143, 94, 80,
                242, 17, 127, 26, 94, 197, 222, 220, 255, 245, 136, 20, 62, 132
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

        let (_, old_root) = TREE.insert(&mut storage, leaf.clone(), &Blake2)?;
        let (_, new_root) = TREE.insert(&mut storage, leaf, &Blake2)?;

        assert!(TREE.is_valid_root(&storage, &old_root)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        Ok(())
    }
}
