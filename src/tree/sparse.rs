use std::{fmt::Debug, marker::PhantomData};

use cosmwasm_std::{Order, Storage};
use cw_storage_plus::{Item, Map};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Hasher, MerkleTree, MerkleTreeError};

/// Normal sparse merkle tree with customizable tree level and default leaf.
pub struct SparseMerkleTree<
    'a,
    L: Serialize + DeserializeOwned + Clone + Debug + PartialEq,
    H: Hasher<L>,
> {
    _l: PhantomData<L>,
    _h: PhantomData<H>,
    pub hashes: Item<'a, (Vec<L>, Vec<L>)>,
    pub leafs: Map<'a, u64, L>,
    pub level: Item<'a, u8>,
    pub root: Item<'a, L>,
}

impl<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq, H: Hasher<L>>
    SparseMerkleTree<'a, L, H>
{
    pub const fn new(
        hashes_ns: &'a str,
        leafs_ns: &'a str,
        level_ns: &'a str,
        root_ns: &'a str,
    ) -> Self {
        Self {
            _l: PhantomData,
            _h: PhantomData,
            hashes: Item::new(hashes_ns),
            leafs: Map::new(leafs_ns),
            level: Item::new(level_ns),
            root: Item::new(root_ns),
        }
    }
}

impl<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq, H: Hasher<L>> MerkleTree<L, H>
    for SparseMerkleTree<'a, L, H>
{
    fn init(
        &self,
        storage: &mut dyn Storage,
        level: u8,
        default_leaf: L,
        hasher: &H,
    ) -> Result<(), MerkleTreeError> {
        self.level
            .may_load(storage)?
            .is_none()
            .then_some(())
            .ok_or(MerkleTreeError::AlreadyInit)?;

        self.level.save(storage, &level)?;

        let mut hashes = vec![default_leaf];

        for i in 1..level as usize {
            let latest = &hashes[i - 1];
            hashes.push(hasher.hash_two(latest, latest)?);
        }

        self.hashes.save(storage, &(hashes.clone(), hashes))?;

        Ok(())
    }

    fn is_valid_root(&self, storage: &dyn Storage, root: &L) -> Result<bool, MerkleTreeError> {
        Ok(self.root.may_load(storage)?.as_ref() == Some(root))
    }

    fn insert(
        &self,
        storage: &mut dyn Storage,
        leaf: L,
        hasher: &H,
    ) -> Result<(u64, L), MerkleTreeError> {
        let level = self.level.load(storage)?;
        let index = {
            self.leafs
                .keys(storage, None, None, Order::Descending)
                .next()
                .transpose()?
                .map(|e| e + 1)
                .unwrap_or_default()
        };

        (index < 2u64.pow(level as u32))
            .then_some(())
            .ok_or(MerkleTreeError::ExceedMaxLeaf)?;

        self.leafs.save(storage, index, &leaf)?;

        let (mut hashes, zeros) = self.hashes.load(storage)?;
        let mut cur_hash = leaf;
        let mut cur_idx = index;

        for i in 0..level as usize {
            let (left, right) = match cur_idx % 2 == 0 {
                true => {
                    hashes[i] = cur_hash.clone();
                    (&cur_hash, &zeros[i])
                }
                false => (&hashes[i], &cur_hash),
            };

            cur_hash = hasher.hash_two(left, right)?;
            cur_idx /= 2;
        }

        self.hashes.save(storage, &(hashes, zeros))?;
        self.root.save(storage, &cur_hash)?;

        Ok((index, cur_hash))
    }

    fn get_latest_root(&self, storage: &dyn Storage) -> Result<L, MerkleTreeError> {
        Ok(self
            .root
            .may_load(storage)?
            .unwrap_or(self.hashes.load(storage)?.1.last().unwrap().clone()))
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, str::FromStr};

    use cosmwasm_std::{testing::MockStorage, Uint256};

    use crate::{test_utils::Blake2, Hasher, MerkleTree};

    use super::SparseMerkleTree;

    const TREE: SparseMerkleTree<Uint256, Blake2> =
        SparseMerkleTree::new("hashes", "leafs", "level", "zeros");

    #[test]
    fn init() -> Result<(), Box<dyn Error>> {
        let mut storage = MockStorage::new();

        TREE.init(
            &mut storage,
            20,
            Blake2.hash_two(&Uint256::zero(), &Uint256::zero())?,
            &Blake2,
        )?;

        assert_eq!(
            TREE.get_latest_root(&storage)?,
            Uint256::from_str(
                "9249403463272353962338525770558810268347485650856754165003644360089862036530"
            )?
        );

        Ok(())
    }

    #[test]
    fn insert() -> Result<(), Box<dyn Error>> {
        let mut storage = MockStorage::new();

        TREE.init(
            &mut storage,
            20,
            Blake2.hash_two(&Uint256::zero(), &Uint256::zero())?,
            &Blake2,
        )?;

        let leaf = Blake2.hash_two(&Uint256::one(), &Uint256::one())?;

        let (index, new_root) = TREE.insert(&mut storage, leaf, &Blake2)?;

        assert_eq!(index, 0);
        assert_eq!(
            new_root,
            Uint256::from_str(
                "65270348628983318905821145914244198139930176154042934882987463098115489862117"
            )?
        );
        assert_eq!(new_root, TREE.get_latest_root(&storage)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        let (index, new_root) = TREE.insert(&mut storage, leaf, &Blake2)?;

        assert_eq!(index, 1);
        assert_eq!(
            new_root,
            Uint256::from_str(
                "31390868241958093005646829964058364480768696680064791450319134920411060649604"
            )?
        );
        assert_eq!(new_root, TREE.get_latest_root(&storage)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        Ok(())
    }

    #[test]
    fn root_history() -> Result<(), Box<dyn Error>> {
        let mut storage = MockStorage::new();

        TREE.init(
            &mut storage,
            20,
            Blake2.hash_two(&Uint256::zero(), &Uint256::zero())?,
            &Blake2,
        )?;

        let leaf = Blake2.hash_two(&Uint256::from_u128(5), &Uint256::from_u128(5))?;

        let (_, old_root) = TREE.insert(&mut storage, leaf, &Blake2)?;
        let (_, new_root) = TREE.insert(&mut storage, leaf, &Blake2)?;

        assert!(!TREE.is_valid_root(&storage, &old_root)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        Ok(())
    }
}
