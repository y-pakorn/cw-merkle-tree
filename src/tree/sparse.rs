use std::{fmt::Debug, marker::PhantomData};

use cosmwasm_std::{Order, Storage};
use cw_storage_plus::{Item, Map};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Hasher, MerkleTree, MerkleTreeError};

pub struct SparseMerkleTree<
    'a,
    L: Serialize + DeserializeOwned + Clone + Debug + PartialEq,
    H: Hasher<L>,
> {
    _l: PhantomData<L>,
    _h: PhantomData<H>,
    pub hashes: Item<'a, Vec<L>>,
    pub leafs: Map<'a, u64, L>,
    pub level: Item<'a, u8>,
    pub zeros: Item<'a, Vec<L>>,
}

impl<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq, H: Hasher<L>>
    SparseMerkleTree<'a, L, H>
{
    pub const fn new(
        hashes_ns: &'a str,
        leafs_ns: &'a str,
        level_ns: &'a str,
        zeros_ns: &'a str,
    ) -> Self {
        Self {
            _l: PhantomData,
            _h: PhantomData,
            hashes: Item::new(hashes_ns),
            leafs: Map::new(leafs_ns),
            level: Item::new(level_ns),
            zeros: Item::new(zeros_ns),
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
        self.level.save(storage, &level)?;

        let mut hashes = vec![default_leaf];

        for i in 1..level as usize {
            let latest = &hashes[i - 1];
            hashes.push(hasher.hash_two(&latest, &latest)?);
        }

        self.hashes.save(storage, &hashes)?;
        self.zeros.save(storage, &hashes)?;

        Ok(())
    }

    fn is_valid_root(&self, storage: &dyn Storage, root: &L) -> Result<bool, MerkleTreeError> {
        Ok(self.hashes.load(storage)?.last().unwrap() == root)
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

        let zeros = self.zeros.load(storage)?;
        let mut hashes = self.hashes.load(storage)?;
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

            cur_hash = hasher.hash_two(&left, &right)?;
            cur_idx /= 2;
        }

        self.hashes.save(storage, &hashes)?;

        Ok((index, hashes.into_iter().next_back().unwrap()))
    }

    fn get_latest_root(&self, storage: &dyn Storage) -> Result<L, MerkleTreeError> {
        Ok(self.hashes.load(storage)?.into_iter().next_back().unwrap())
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
                "20440131195474697977177675138122460070080428738123630012135291638286263683716"
            )?
        );
        assert_eq!(new_root, TREE.get_latest_root(&storage)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        let (index, new_root) = TREE.insert(&mut storage, leaf, &Blake2)?;

        assert_eq!(index, 1);
        assert_eq!(
            new_root,
            Uint256::from_str(
                "17583439540779748128045581041758430207126949480967999715753965799367859941748"
            )?
        );
        assert_eq!(new_root, TREE.get_latest_root(&storage)?);
        assert!(TREE.is_valid_root(&storage, &new_root)?);

        Ok(())
    }
}
