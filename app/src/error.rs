use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, From)]
pub enum Error {
    ServerError { code: i64 },
    TryLater,
    Unauthorized,

    // -- Server
    ServerFunctionError(leptos::ServerFnError),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
