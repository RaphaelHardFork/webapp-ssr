use axum::http::StatusCode;
use derive_more::From;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::database;

pub type Result<T> = core::result::Result<T, Error>;

#[serde_as]
#[derive(Debug, Serialize, From)]
pub enum Error {
    // Controller
    EmptyField {
        field: &'static str,
    },
    WrongEmailFormat,

    EntityNotFound {
        entity: &'static str,
        id: i64,
    },
    WrongUuidFormat {
        uuid: String,
    },
    UuidParsingFail(String),
    IdentifierNotFound {
        identifier: String,
    },

    // Session
    NoAuthToken,

    // Database
    #[from]
    SQLiteConnection(database::Error),

    // Libs
    #[from]
    Utils(lib_utils::Error),
    #[from]
    Pwd(lib_auth::pwd::Error),
    #[from]
    Token(lib_auth::token::Error),

    // Externals
    #[from]
    SeaQuery(#[serde_as(as = "DisplayFromStr")] sea_query::error::Error),
    #[from]
    Sqlx(#[serde_as(as = "DisplayFromStr")] sqlx::Error),
}

// region:      --- Error Boilerplate

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

// endregion:   --- Error Boilerplate
