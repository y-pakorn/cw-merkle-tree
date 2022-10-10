<h1 align="center">CW Merkle Tree</h1>
<p align="center">
    <a href="https://crates.io/crates/cw-merkle-tree">
        <img alt="Crates.io" src="https://img.shields.io/crates/v/cw-merkle-tree">
    </a>
    <a href="https://docs.rs/cw-merkle-tree">
        <img alt="docs.rs" src="https://img.shields.io/docsrs/cw-merkle-tree">
    </a>
    <img alt="Crates.io" src="https://img.shields.io/crates/l/cw-merkle-tree">
</p>

Sparse merkle tree with variants implementation for CosmWasm smart contract

## Variants

### Sparse Merkle Tree

Normal sparse merkle tree with customizable tree level and default leaf.

### Sparse Merkle Tree With History

Like sparse merkle tree but able to check valid root hash with all previous root hashes.

### Sparse Merkle Tree With History Bounded

Like sparse merkle tree but able to check valid root hash with previous root hashes upto specified history level.

## Example Usage

### Hasher

Implement the hasher trait for desired hasher struct.

```rust
#[derive(Clone, Copy, Debug)]
pub struct Blake2;

impl Hasher<Uint256> for Blake2 {
    fn hash_two(&self, left: &Uint256, right: &Uint256) -> Result<Uint256, HasherError> {
        let mut hasher = Blake2b512::new();
        hasher.update(left.to_be_bytes());
        hasher.update(right.to_be_bytes());
        Ok(Uint256::from_be_bytes(
            hasher.finalize()[0..32]
                .try_into()
                .map_err(|e: TryFromSliceError| HasherError::Custom(e.to_string()))?,
        ))
    }
}
```

### Merkle Tree

First, instantiate the merkle tree by using `new` constructor function and specify leaf and hasher type.

```rust
const TREE: SparseMerkleTree<Uint256, Blake2> =
    SparseMerkleTree::new("hashes", "leafs", "level", "zeros");
```

Then initialize the tree by invoking the `init` function, preferably in `instantiate` entry point.
  
```rust
  pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let hashed_zero = Blake2.hash_two(&Uint256::zero(), &Uint256::zero())?;
    TREE.init(deps.storage, 20, hashed_zero, &Blake2)?;
    
    // ...
}
```

Next, insert a leaf into next available index of the tree by invoking `insert` function, `insert` will return inserted index and the new root.

```rust
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
```
