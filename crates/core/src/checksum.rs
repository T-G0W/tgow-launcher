use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidDigest;

impl fmt::Display for InvalidDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected 64 hexadecimal characters")
    }
}

impl std::error::Error for InvalidDigest {}

impl FromStr for Digest {
    type Err = InvalidDigest;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let text = text.strip_prefix("sha256:").unwrap_or(text);
        decode_hex(text)
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .map(Self)
            .ok_or(InvalidDigest)
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({self})")
    }
}

impl Serialize for Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

pub(crate) fn decode_hex(text: &str) -> Option<Vec<u8>> {
    let (pairs, rest) = text.as_bytes().as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    pairs
        .iter()
        .map(|&[high, low]| Some((nibble(high)? << 4) | nibble(low)?))
        .collect()
}

fn nibble(symbol: u8) -> Option<u8> {
    match symbol {
        b'0'..=b'9' => Some(symbol - b'0'),
        b'a'..=b'f' => Some(symbol - b'a' + 10),
        b'A'..=b'F' => Some(symbol - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Digest, decode_hex};

    #[test]
    fn hashes_known_input() {
        assert_eq!(
            Digest::of(b"abc").to_string(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn parses_plain_and_prefixed_hex() {
        let plain: Digest = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
            .parse()
            .unwrap();
        let prefixed: Digest =
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .parse()
                .unwrap();
        assert_eq!(plain, prefixed);
    }

    #[test]
    fn rejects_wrong_length_and_symbols() {
        assert!("abcd".parse::<Digest>().is_err());
        assert!(decode_hex("0g").is_none());
        assert!(decode_hex("abc").is_none());
    }
}
