use std::{fmt::Debug, marker::PhantomData};

use cosmwasm_std::{Order, Storage};
use cw_storage_plus::{Item, Map};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Hasher, MerkleTree, MerkleTreeError};

pub struct SparseMerkleTree<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq> {
    _p: PhantomData<L>,
    pub hashes: Item<'a, Vec<L>>,
    pub leafs: Map<'a, u64, L>,
    pub level: Item<'a, u8>,
    pub zeros: Item<'a, Vec<L>>,
}

impl<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq> SparseMerkleTree<'a, L> {
    pub const fn new(
        hashes_ns: &'a str,
        leafs_ns: &'a str,
        level_ns: &'a str,
        zeros_ns: &'a str,
    ) -> Self {
        Self {
            _p: PhantomData,
            hashes: Item::new(hashes_ns),
            leafs: Map::new(leafs_ns),
            level: Item::new(level_ns),
            zeros: Item::new(zeros_ns),
        }
    }
}

impl<'a, L: Serialize + DeserializeOwned + Clone + Debug + PartialEq, H: Hasher<L>> MerkleTree<L, H>
    for SparseMerkleTree<'a, L>
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
            hashes[i] = hasher.hash_two(&latest, &latest)?;
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
