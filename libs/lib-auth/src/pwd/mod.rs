//! Responsible for hashing and validating hashes follwing
//! a multi-scheme hashing code design with its own hashing
//! and validation methods.

mod error;
mod scheme;

pub use self::error::{Error, Result};
use self::scheme::{get_scheme, Scheme, DEFAULT_SCHEME};

use lazy_regex::regex_captures;
pub use scheme::SchemeStatus;
use std::str::FromStr;
use uuid::Uuid;

// region:		=== Types ===

/// Sensitives informations do not implement the Debug trait
#[cfg_attr(test, derive(Clone))]
pub struct ContentToHash {
    pub content: String, // clear content
    pub salt: Uuid,
}

// endregion:	=== Types ===

// region:		=== Public functions ===

/// hash pwd with default scheme
pub async fn hash_pwd(to_hash: ContentToHash) -> Result<String> {
    tokio::task::spawn_blocking(move || hash_for_scheme(DEFAULT_SCHEME, to_hash))
        .await
        .map_err(|_| Error::FailSpawnBlockForValidate)?
}

pub async fn validate_pwd(to_hash: ContentToHash, pwd_ref: &str) -> Result<SchemeStatus> {
    let PwdParts {
        scheme_name,
        hashed,
    } = pwd_ref.parse()?;

    let scheme_status = if scheme_name == DEFAULT_SCHEME {
        SchemeStatus::Ok
    } else {
        SchemeStatus::Outdated
    };

    tokio::task::spawn_blocking(move || validate_for_scheme(&scheme_name, to_hash, &hashed))
        .await
        .map_err(|_| Error::FailSpawnBlockForValidate)??;

    Ok(scheme_status)
}

// endregion:	=== Public functions ===

// region:		=== Privates function & types ===

fn hash_for_scheme(scheme_name: &str, to_hash: ContentToHash) -> Result<String> {
    let pwd_hashed = get_scheme(scheme_name)?.hash(to_hash)?;

    Ok(format!("#{scheme_name}#{pwd_hashed}"))
}

fn validate_for_scheme(scheme_name: &str, to_hash: ContentToHash, pwd_ref: &str) -> Result<()> {
    get_scheme(scheme_name)?.validate(to_hash, pwd_ref)?;
    Ok(())
}

struct PwdParts {
    /// Scheme only (e.g., "01")
    scheme_name: String,
    /// Hashed pwd
    hashed: String,
}

impl FromStr for PwdParts {
    type Err = Error;

    fn from_str(pwd_with_scheme: &str) -> Result<Self> {
        regex_captures!(
            r#"^#(\w+)#(.*)"#, // literal regex
            pwd_with_scheme
        )
        .map(|(_, scheme, hashed)| Self {
            scheme_name: scheme.to_string(),
            hashed: hashed.to_string(),
        })
        .ok_or(Error::PwdWithSchemeFailedParse)
    }
}

// endregion:	=== Privates function & types ===

// region:		=== Tests ===

#[cfg(test)]
mod tests {
    pub type Result<T> = core::result::Result<T, Error>;
    pub type Error = Box<dyn std::error::Error>;

    use super::*;

    #[tokio::test]
    async fn test_multi_scheme_ok() -> Result<()> {
        let fx_salt = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
        let fx_to_hash = ContentToHash {
            content: "hello world".to_string(),
            salt: fx_salt,
        };

        let pwd_hashed = hash_for_scheme("01", fx_to_hash.clone())?;
        let pwd_validate = validate_pwd(fx_to_hash, &pwd_hashed).await?;

        assert!(
            matches!(pwd_validate, SchemeStatus::Outdated),
            "status should be Schemestatus::Outdated"
        );

        Ok(())
    }
}

// endregion:	=== Tests ===
