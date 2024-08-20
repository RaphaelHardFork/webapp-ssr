use std::str::FromStr;

use leptos::ServerFnError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub type ServerResult<T> = core::result::Result<T, ServerFnError<ServerError>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ServerError {
    // -- SQL constraints (data already exist in DB)
    TryAgain,

    // -- Data saved
    CannotLogin { code: i64 },

    // -- Leptos server error
    ServerFunction(String),
}

impl FromStr for ServerError {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::ServerFunction(s.to_string()))
    }
}

// region:    --- Error Boilerplate

impl core::fmt::Display for ServerError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for ServerError {}

// endregion: --- Error Boilerplate

pub fn serialize_error_response(error: ServerFnError<ServerError>) -> Value {
    let generic_error = json!({
      "error":{
        "message":"Cannot reach server",
      }
    });

    match error {
        // fake errors from backend business logic
        ServerFnError::WrappedServerError(se) => match se {
            ServerError::CannotLogin { code } => {
                json!({
                  "error":{
                    "message":"Cannot login",
                    "code":code
                  }
                })
            }
            ServerError::TryAgain => {
                json!({
                  "error":{
                    "message":"Try again",
                  }
                })
            }
            ServerError::ServerFunction(_) => generic_error,
        },

        // error from leptos server function
        _ => generic_error,
    }
}
