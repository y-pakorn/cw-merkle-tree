use std::{array::TryFromSliceError, error::Error, str::FromStr};

use blake2::{Blake2b512, Digest};
use cosmwasm_std::Uint256;

use crate::{Hasher, HasherError};

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

#[test]
fn hash() -> Result<(), Box<dyn Error>> {
    let result = Blake2.hash_two(&Uint256::from_u128(1), &Uint256::from_u128(1))?;

    assert_eq!(
        result,
        Uint256::from_str(
            "32330994018239876717743352644966702819050069979269935689071830732604725210220"
        )?
    );

    Ok(())
}
