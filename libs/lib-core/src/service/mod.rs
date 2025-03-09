mod error;

pub mod login;
pub mod register;

pub use error::{Error, Result};

// Flatten service fns
pub use login::*;
pub use register::*;
