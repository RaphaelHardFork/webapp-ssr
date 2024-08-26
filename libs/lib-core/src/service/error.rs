use derive_more::From;
use serde::Serialize;

use crate::model;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From, Serialize)]
pub enum Error {
    EmptyLoginPayload,

    #[from]
    Model(model::Error),
    #[from]
    Pwd(lib_auth::pwd::Error),
    #[from]
    Token(lib_auth::token::Error),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
