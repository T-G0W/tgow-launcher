use std::fmt;

use ed25519_compact::{PublicKey, Signature};

use crate::checksum::decode_hex;

pub const RELEASE_KEYS: [&str; 0] = [];
pub const LEGACY_KEYS: [&str; 1] =
    ["ebed6bdcda2575c635ac22688814112731846d307adce100f1e585cc97be876f"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidKey,
    MalformedSignature,
    NoKeys,
    Mismatch,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidKey => "invalid public key",
            Self::MalformedSignature => "malformed signature",
            Self::NoKeys => "no trusted key for this release",
            Self::Mismatch => "signature does not match",
        })
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct TrustedKeys {
    keys: Vec<PublicKey>,
}

impl TrustedKeys {
    pub fn from_hex(keys: &[&str]) -> Result<Self, Error> {
        keys.iter()
            .map(|key| {
                decode_hex(key)
                    .and_then(|bytes| PublicKey::from_slice(&bytes).ok())
                    .ok_or(Error::InvalidKey)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|keys| Self { keys })
    }

    #[must_use]
    pub fn release() -> Self {
        Self::from_hex(&RELEASE_KEYS).unwrap_or(Self { keys: Vec::new() })
    }

    #[must_use]
    pub fn legacy() -> Self {
        Self::from_hex(&LEGACY_KEYS).unwrap_or(Self { keys: Vec::new() })
    }

    pub fn verify(&self, message: &[u8], signature: &str) -> Result<(), Error> {
        if self.keys.is_empty() {
            return Err(Error::NoKeys);
        }
        let signature = decode_hex(signature.trim())
            .and_then(|bytes| Signature::from_slice(&bytes).ok())
            .ok_or(Error::MalformedSignature)?;
        if self
            .keys
            .iter()
            .any(|key| key.verify(message, &signature).is_ok())
        {
            Ok(())
        } else {
            Err(Error::Mismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use ed25519_compact::{KeyPair, Seed};

    use super::{Error, TrustedKeys};

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
    }

    fn signer(seed: u8) -> (KeyPair, String) {
        let pair = KeyPair::from_seed(Seed::new([seed; 32]));
        let public = hex(pair.pk.as_ref());
        (pair, public)
    }

    #[test]
    fn accepts_a_signature_from_any_trusted_key() {
        let (old, old_public) = signer(1);
        let (_, new_public) = signer(2);
        let keys = TrustedKeys::from_hex(&[&new_public, &old_public]).unwrap();
        let message = br#"{"version":"1.0.0"}"#;
        let signature = hex(old.sk.sign(message, None).as_ref());
        assert_eq!(keys.verify(message, &signature), Ok(()));
    }

    #[test]
    fn rejects_changed_messages_and_foreign_keys() {
        let (pair, public) = signer(3);
        let (stranger, _) = signer(4);
        let keys = TrustedKeys::from_hex(&[&public]).unwrap();
        let signature = hex(pair.sk.sign(b"original", None).as_ref());
        assert_eq!(keys.verify(b"changed", &signature), Err(Error::Mismatch));
        let foreign = hex(stranger.sk.sign(b"original", None).as_ref());
        assert_eq!(keys.verify(b"original", &foreign), Err(Error::Mismatch));
    }

    #[test]
    fn reports_malformed_input() {
        let (_, public) = signer(5);
        let keys = TrustedKeys::from_hex(&[&public]).unwrap();
        assert_eq!(keys.verify(b"x", "zz"), Err(Error::MalformedSignature));
        assert_eq!(
            TrustedKeys::from_hex(&["00"]).err(),
            Some(Error::InvalidKey)
        );
        assert_eq!(
            TrustedKeys::release().verify(b"x", "00"),
            Err(Error::NoKeys)
        );
    }

    #[test]
    fn verifies_the_real_ferrovox_release() {
        let manifest = include_bytes!("../tests/data/ferrovox-1.0.2/latest.json");
        let signature = include_str!("../tests/data/ferrovox-1.0.2/latest.json.sig");
        assert_eq!(TrustedKeys::legacy().verify(manifest, signature), Ok(()));
        let mut tampered = manifest.to_vec();
        tampered[20] ^= 1;
        assert_eq!(
            TrustedKeys::legacy().verify(&tampered, signature),
            Err(Error::Mismatch)
        );
    }
}
