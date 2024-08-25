use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use derive_more::From;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use std::sync::Arc;
use tracing::debug;

use super::middleware::auth::CtxExtError;

pub type Result<T> = core::result::Result<T, Error>;

#[allow(unused)]
#[derive(Debug, From, Serialize)]
pub enum Error {
    // Login
    EmptyLoginPayload,
    UserNotFound {
        identifier: String,
    },
    WrongPwd,
    SessionNotFound {
        token: String,
    },
    CannotSetCookie,
    CannotRemoveCookie,
    EmptyField,

    ServeDir,
    BuildAxumRequest(String),
    GetLeptosConfig(String),

    #[from]
    CtxExt(CtxExtError),
    #[from]
    WebToken(lib_auth::token::Error),
    #[from]
    Model(lib_core::model::Error),
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

// region:        --- Client Error

#[serde_as]
#[derive(Debug, Serialize)]
#[serde(tag = "message", content = "detail")]
#[allow(non_camel_case_types)]
pub enum ClientError {
    SERVICE_ERROR,
}

impl Error {
    pub fn client_status_and_error(&self) -> (StatusCode, ClientError) {
        use Error::*;

        #[allow(unreachable_patterns)]
        match self {
            // fallback
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::SERVICE_ERROR,
            ),
        }
    }
}

// endregion:     --- Client Error
