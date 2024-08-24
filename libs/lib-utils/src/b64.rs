use base64::engine::{general_purpose, Engine};

// region:		=== Base 64 URL safe ===

pub fn b64u_encode(content: impl AsRef<[u8]>) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(content)
}

pub fn b64u_decode(b64u: &str) -> Result<Vec<u8>> {
    general_purpose::URL_SAFE_NO_PAD
        .decode(b64u)
        .map_err(|_| Error::FailToB64uDecode)
}

pub fn b64u_decode_to_string(b64u: &str) -> Result<String> {
    b64u_decode(b64u)
        .ok()
        .and_then(|r| String::from_utf8(r).ok())
        .ok_or(Error::FailToB64uDecode)
}

// endregion:	=== Base 64 URL safe  ===

// region:		=== Base 64 ===

pub fn b64_encode(content: impl AsRef<[u8]>) -> String {
    general_purpose::STANDARD_NO_PAD.encode(content)
}

pub fn b64_decode(b64: &str) -> Result<Vec<u8>> {
    general_purpose::STANDARD_NO_PAD
        .decode(b64)
        .map_err(|_| Error::FailToB64Decode)
}

pub fn b64_decode_to_string(b64: &str) -> Result<String> {
    b64_decode(b64)
        .ok()
        .and_then(|r| String::from_utf8(r).ok())
        .ok_or(Error::FailToB64Decode)
}

// endregion:	=== Base 64 ===

// region:			--- Error
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    FailToB64Decode,
    FailToB64uDecode,
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
// endregion:		--- Error
