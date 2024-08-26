use crate::database;
use derive_more::From;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

pub type Result<T> = core::result::Result<T, Error>;

#[serde_as]
#[derive(Debug, Serialize, From)]
pub enum Error {
    // Base: DB result
    EntityIdNotFound {
        entity: &'static str,
        id: i64,
    },
    EntityIdenNotFound {
        entity: &'static str,
        identifier: String,
    },

    // Base: DB insert
    EmptyField {
        field: &'static str,
    },
    WrongEmailFormat,
    EmailAlreadyValiadted,

    // Modules
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
    Uuid(#[serde_as(as = "DisplayFromStr")] uuid::Error),
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
