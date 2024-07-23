use crate::model;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use derive_more::From;
use std::sync::Arc;
use tracing::debug;

pub type Result<T> = core::result::Result<T, Error>;

#[allow(unused)]
#[derive(Debug, From)]
pub enum Error {
    ServeDir,

    #[from]
    Model(model::Error),

    #[from]
    AxumHttp(axum::http::Error),

    #[from]
    LeptosConfig(leptos::leptos_config::errors::LeptosConfigError),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate

// region:    --- Axum IntoResponse

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        debug!("{:<12} - web::Error {self:?}", "INTO_RES");

        // Create a placeholder Axum reponse.
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

        // Insert the Error into the reponse.
        response.extensions_mut().insert(Arc::new(self));

        response
    }
}

// endregion: --- Axum IntoResponse
