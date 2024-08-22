use super::{Error, Result, Scheme};
use crate::config::auth_config;
use crate::pwd::ContentToHash;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, Version};
use argon2::{PasswordHasher as _, PasswordVerifier as _};
use std::sync::OnceLock;

pub struct Scheme02;

impl Scheme for Scheme02 {
    fn hash(&self, to_hash: ContentToHash) -> Result<String> {
        let argon2 = get_argon2();

        let salt_b64 = SaltString::encode_b64(to_hash.salt.as_bytes()).map_err(|_| Error::Salt)?;

        // argon2 don't need anymore the salt, because this latter is included in the content
        // so if we change the salt in the DB, the validation still work
        let pwd = argon2
            .hash_password(to_hash.content.as_bytes(), &salt_b64)
            .map_err(|_| Error::Hash)?
            .to_string();

        Ok(pwd)
    }

    fn validate(&self, to_hash: ContentToHash, pwd_ref: &str) -> Result<()> {
        let argon2 = get_argon2();

        let parsed_hash_ref = PasswordHash::new(pwd_ref).map_err(|_| Error::Hash)?;

        argon2
            .verify_password(to_hash.content.as_bytes(), &parsed_hash_ref)
            .map_err(|_| Error::PwdValidate)
    }
}

fn get_argon2() -> &'static Argon2<'static> {
    static INSTANCE: OnceLock<Argon2<'static>> = OnceLock::new();

    INSTANCE.get_or_init(|| {
        let key = &auth_config().PWD_KEY;
        Argon2::new_with_secret(key, Algorithm::Argon2d, Version::V0x13, Params::default())
            // TODO: need to fail early
            .unwrap()
    })
}

// region:    --- Tests
#[cfg(test)]
mod tests {
    pub type Result<T> = core::result::Result<T, Error>;
    pub type Error = Box<dyn std::error::Error>;

    use crate::pwd::ContentToHash;

    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_scheme_02_hash_into_b64u_ok() -> Result<()> {
        let fx_to_hash = ContentToHash {
            content: "hello world".to_string(),
            salt: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?,
        };
        let fx_res = "$argon2d$v=19$m=19456,t=2,p=1$VQ6EAOKbQdSnFkRmVUQAAA$RR8hAzqU8bin8muW8bzfvDqVsB+SQQgjy8X1aXwjvSk";

        let scheme = Scheme02;
        let res = scheme.hash(fx_to_hash)?;

        assert_eq!(res, fx_res);
        Ok(())
    }
}
// endregion:		--- Tests
