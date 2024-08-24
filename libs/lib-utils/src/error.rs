use serde::Serialize;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Serialize)]
pub enum Error {
    // -- Envs
    MissingEnv(&'static str),
    WrongEnvFormat(&'static str),

    // -- Files
    CannotCreateDir(String),
    CannotCreateFile(String),
    CannotRemoveFile(String),
    ImpossiblePath(String),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
