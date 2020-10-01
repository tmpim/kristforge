use serde::{Deserialize, Serialize, Serializer};
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

/// A krist address
#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "&str")]
pub struct Address([u8; Address::LENGTH]);

impl Address {
    pub const LENGTH: usize = 10;
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap()
    }
    pub fn as_bytes(&self) -> &[u8; Address::LENGTH] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid address length: {0}")]
pub struct InvalidAddressLength(usize);

impl FromStr for Address {
    type Err = InvalidAddressLength;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.as_bytes()
            .try_into()
            .map(Self)
            .map_err(|_| InvalidAddressLength(s.len()))
    }
}

impl TryFrom<&str> for Address {
    type Error = InvalidAddressLength;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl Serialize for Address {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.as_str())
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Address> for String {
    fn from(address: Address) -> Self {
        address.to_string()
    }
}

impl Debug for Address {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Address({})", self.as_str())
    }
}

impl PartialEq<str> for Address {
    fn eq(&self, other: &str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl PartialEq<&str> for Address {
    fn eq(&self, other: &&str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl PartialEq<String> for Address {
    fn eq(&self, other: &String) -> bool {
        self.0 == other.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_valid_addresses() {
        assert_eq!(Address::from_str("abcdef1234").unwrap(), "abcdef1234",);
        assert_eq!(Address::from_str("kabcdefghi").unwrap(), "kabcdefghi",);
    }

    #[test]
    fn check_invalid_addresses() {
        assert_eq!(
            Address::from_str("abc").unwrap_err(),
            InvalidAddressLength(3)
        );
    }

    #[test]
    fn test_serialize_deserialize() {
        let address = Address::from_str("kabcdefghi").unwrap();

        assert_eq!(
            address,
            serde_json::from_str::<Address>(&serde_json::to_string(&address).unwrap()).unwrap()
        );
    }
}
