use super::{Error, Result, Scheme};
use crate::{config::auth_config, pwd::ContentToHash};
use hmac::{Hmac, Mac};
use lib_utils::b64::b64u_encode;
use sha2::Sha512;

pub struct Scheme01;

impl Scheme for Scheme01 {
    fn hash(&self, to_hash: ContentToHash) -> Result<String> {
        let key = &auth_config().PWD_KEY;
        hash(key, to_hash)
    }

    fn validate(&self, to_hash: ContentToHash, raw_pwd_ref: &str) -> Result<()> {
        let raw_pwd_new = self.hash(to_hash)?;
        if raw_pwd_new == raw_pwd_ref {
            Ok(())
        } else {
            Err(Error::PwdValidate)
        }
    }
}

fn hash(key: &[u8], to_hash: ContentToHash) -> Result<String> {
    let ContentToHash { content, salt } = to_hash;

    // create HMAC-SHA512 from key
    let mut hmac_sha512 = Hmac::<Sha512>::new_from_slice(key).map_err(|_| Error::Key)?;

    // Add content
    hmac_sha512.update(content.as_bytes());
    hmac_sha512.update(salt.as_bytes());

    // Finalize and b64u encode
    let hmac_result = hmac_sha512.finalize();
    let hashed = b64u_encode(hmac_result.into_bytes());

    Ok(hashed)
}

// region:    --- Tests
#[cfg(test)]
mod tests {
    pub type Result<T> = core::result::Result<T, Error>;
    pub type Error = Box<dyn std::error::Error>;

    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_scheme_01_hash_into_b64_ok() -> Result<()> {
        let fx_salt = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
        let fx_key = &auth_config().PWD_KEY;
        let fx_to_hash = ContentToHash {
            content: "hello world".to_string(),
            salt: fx_salt,
        };

        // precomputed
        let fx_res = "KOhcTh3wE6QkANgadKtRy_rxYZrW3x-H3BaPDPtIT8eEaRNhHyIWSIDQTaNWSZga-CyU9MaqW0I183AyiggxQQ".to_string();

        let res = hash(fx_key, fx_to_hash)?;

        assert_eq!(res, fx_res);

        Ok(())
    }
}
// endregion:		--- Tests
