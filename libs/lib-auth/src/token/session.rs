use std::str::FromStr;

use hmac::{Hmac, Mac};
use lib_utils::b64::b64_encode;
use sha2::Sha512;
use uuid::Uuid;

use crate::config::auth_config;

use super::{Error, Result};

#[derive(Debug, Clone)]
pub struct SessionToken {
    pub salt: Uuid,
    pub privileged: bool,
    pub signature: String,
}

impl FromStr for SessionToken {
    type Err = Error;

    fn from_str(token_str: &str) -> std::result::Result<Self, Self::Err> {
        let parts: Vec<&str> = token_str.split(':').collect();
        if parts.len() != 3 {
            return Err(Error::InvalidFormat);
        }
        let (salt_str, privileged, signature) = (
            parts[0],
            parts[1].chars().next() == Some('#'),
            parts[2].to_string(),
        );
        let salt = Uuid::parse_str(salt_str).map_err(|_| Error::CannotParseUuid)?;

        Ok(Self {
            salt,
            privileged,
            signature,
        })
    }
}

pub fn generate_session_token(salt: Uuid, privileged: bool) -> Result<String> {
    let config = &auth_config();

    // Sign token_salt
    let sign_b64 = _token_sign_into_b64(salt, &config.SESSION_KEY)?;

    // Format session_token
    let privileged = if privileged { '#' } else { '$' };
    let token = format!("{}:{}:{}", salt, privileged, sign_b64);

    Ok(token)
}

pub fn validate_session_token(salt: Uuid, session_token: SessionToken) -> Result<()> {
    let config = &auth_config();

    // Sign token salt
    let sign_b64 = _token_sign_into_b64(salt, &config.SESSION_KEY)?;

    // Compare sigs
    if sign_b64 != session_token.signature {
        return Err(Error::SignatureNotMatching);
    }

    Ok(())
}

fn _token_sign_into_b64(salt: Uuid, key: &[u8]) -> Result<String> {
    // Create a HMAC-SHA-512 from key
    let mut hmac_sha512 =
        Hmac::<Sha512>::new_from_slice(key).map_err(|_| Error::HmacFailNewFromSlice)?;

    // Add token_salt
    hmac_sha512.update(salt.as_bytes());

    // Finalize
    let hmac_result_bytes = hmac_sha512.finalize().into_bytes();
    let result = b64_encode(hmac_result_bytes);

    Ok(result)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
    type Error = Box<dyn std::error::Error>;
    type Result<T> = core::result::Result<T, Error>; // For tests.

    use super::*;
    use crate::token::Error as TokenError;

    #[test]
    fn test_format_session_token_ok() -> Result<()> {
        // Precomputed values
        let uuid_str = "ed7cb269-05e0-44b1-943c-191e961ca56a";
        let signature = "jowJJpYeHKP+s8MWWoY6i8iefpTNzF7d5zC/kxV9wmmlJokUoaOWmbvRfXZmSoy35gziN6WaXkraqq09atfWfg";

        // -- Exec
        let token_salt = Uuid::parse_str(uuid_str)?;

        let token_str = generate_session_token(token_salt, false)?;
        let fx_token = format!("{}:$:{}", token_salt, signature);

        let priv_token_str = generate_session_token(token_salt, true)?;
        let fx_priv_token = format!("{}:#:{}", token_salt, signature);

        assert_eq!(token_str, fx_token);
        assert_eq!(priv_token_str, fx_priv_token);

        // -- Check

        Ok(())
    }

    #[test]
    fn test_validate_session_token_ok() -> Result<()> {
        // Precomputed values
        let uuid_str = "ed7cb269-05e0-44b1-943c-191e961ca56a";
        let signature = "cQAzDEvidsW1OPZvd5F/pqrxrZhYYDmru3RUy8HmBhMjmP973Yd8+p0zFw2fzwOmPtctSuI07bH7J8J2O216pA";

        // -- Setup & Fixtures
        let token_salt = Uuid::parse_str(uuid_str)?;
        let token_str = generate_session_token(token_salt, false)?;
        let session_token: SessionToken = token_str.parse()?;

        // -- Exec
        validate_session_token(token_salt, session_token)?;

        // -- Check

        Ok(())
    }

    #[test]
    fn test_validate_session_token_err() -> Result<()> {
        // Precomputed values
        let valid_uuid_str = "ed7cb269-05e0-44b1-943c-191e961ca56a";
        let valid_signature = "cQAzDEvidsW1OPZvd5F/pqrxrZhYYDmru3RUy8HmBhMjmP973Yd8+p0zFw2fzwOmPtctSuI07bH7J8J2O216pA";

        let faulty_uuid_str = "a72ab9be-3930-4c72-bb96-207ecb0f00cc";
        let faulty_signature = "rF+5l6e0r9M86pYdA+KMk0TVM7IKkLrlkFk2yDAHUJdQ6ndVBgd6VIkk/8zdEFpXXWgQbqe3Vd2nQuXs3dCV0w";

        // -- Setup & Fixtures
        let valid_token_salt = Uuid::parse_str(valid_uuid_str)?;
        let session_token: SessionToken =
            generate_session_token(valid_token_salt, false)?.parse()?;

        // -- Exec
        let faulty_token_salt = Uuid::parse_str(faulty_uuid_str)?;
        assert!(matches!(
            validate_session_token(faulty_token_salt, session_token),
            Err(TokenError::SignatureNotMatching)
        ));

        // -- Check

        Ok(())
    }
}

// endregion: --- Tests
