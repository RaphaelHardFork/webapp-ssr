mod config;
mod error;

pub mod ctx;
pub mod database;
pub mod model;
pub mod service;

use self::config::config;
pub use error::{Error, Result};
