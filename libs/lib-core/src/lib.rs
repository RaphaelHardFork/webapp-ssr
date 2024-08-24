mod config;
mod error;

pub mod ctx;
pub mod database;
pub mod model;

use self::config::config;
pub use error::{Error, Result};
