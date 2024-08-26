use crate::web;
use derive_more::From;
use lib_core::model;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    // -- Modules
    #[from]
    Web(web::Error),
    #[from]
    LibWeb(lib_web::Error),

    // -- Externals
    #[from]
    Model(model::Error),
    #[from]
    Core(lib_core::Error),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
