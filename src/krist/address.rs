use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::str;

/// A krist address - v1 and v2 compatible
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "&str", into = "String")]
pub struct Address([u8; Address::LENGTH]);

impl Address {
    /// The required length of krist addresses, in bytes
    pub const LENGTH: usize = 10;

    /// The set of allowed characters for v1 addresses
    pub const V1_CHARS: &'static str = "1234567890abcdef";

    /// The set of allowed characters for v2 addresses
    pub const V2_CHARS: &'static str = "1234567890abcdefghijklmnopqrstuvwxyz";

    /// Get this krist address as a string slice
    pub fn as_str(&self) -> &str {
        // the contents were originally from a utf-8 string, so this should
        // never panic
        str::from_utf8(&self.0).unwrap()
    }

    pub fn as_bytes(&self) -> &[u8; Address::LENGTH] {
        &self.0
    }
}

/// An error caused by an invalid address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Fail)]
pub enum InvalidAddress {
    #[fail(display = "invalid address length: {}", _0)]
    InvalidLength(usize),

    #[fail(display = "illegal character: {} at index {}", _0, _1)]
    IllegalCharacter(char, usize),
}

impl FromStr for Address {
    type Err = InvalidAddress;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // v1 and v2 addresses allow a different set of characters
        let allowed = if s.starts_with('k') {
            Self::V2_CHARS
        } else {
            Self::V1_CHARS
        };

        // search for illegal characters
        if let Some((i, c)) = s.chars().enumerate().find(|&(_, c)| !allowed.contains(c)) {
            return Err(InvalidAddress::IllegalCharacter(c, i));
        }

        // convert to bytes
        let bytes: [u8; Self::LENGTH] = s
            .as_bytes()
            .try_into()
            .map_err(|_| InvalidAddress::InvalidLength(s.len()))?;

        Ok(Self(bytes))
    }
}

impl TryFrom<&str> for Address {
    type Error = InvalidAddress;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
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
            InvalidAddress::InvalidLength(3)
        );
        assert_eq!(
            Address::from_str("abcdefghij").unwrap_err(),
            InvalidAddress::IllegalCharacter('g', 6)
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
