use super::Address;
use hex::FromHexError;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "&str", into = "String")]
pub struct ShortHash(pub [u8; ShortHash::LENGTH]);

impl ShortHash {
    pub const LENGTH: usize = 6;

    pub fn bytes(self) -> [u8; ShortHash::LENGTH] {
        self.0
    }

    pub fn into_hex(self) -> String {
        hex::encode(self.0)
    }
}

impl FromStr for ShortHash {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut hash = [0u8; Self::LENGTH];
        hex::decode_to_slice(s, &mut hash)?;
        Ok(Self(hash))
    }
}

impl TryFrom<&str> for ShortHash {
    type Error = FromHexError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl Display for ShortHash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.into_hex())
    }
}

impl From<ShortHash> for String {
    fn from(hash: ShortHash) -> Self {
        hash.into_hex()
    }
}

impl Debug for ShortHash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "ShortHash({})", self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "&str", into = "String")]
pub struct Hash([u8; Hash::LENGTH]);

impl Hash {
    pub const LENGTH: usize = 32;

    pub fn bytes(self) -> [u8; Hash::LENGTH] {
        self.0
    }

    pub fn into_hex(self) -> String {
        hex::encode(self.0)
    }
}

impl FromStr for Hash {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut hash = [0u8; Self::LENGTH];
        hex::decode_to_slice(s, &mut hash)?;
        Ok(Self(hash))
    }
}

impl TryFrom<&str> for Hash {
    type Error = FromHexError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.into_hex())
    }
}

impl From<Hash> for String {
    fn from(hash: Hash) -> Self {
        hash.into_hex()
    }
}

impl Debug for Hash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Hash({})", self)
    }
}

/// A mined block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Block {
    pub height: u64,
    pub value: u32,
    pub hash: Hash,
    pub short_hash: ShortHash,
    pub address: Address,
}
