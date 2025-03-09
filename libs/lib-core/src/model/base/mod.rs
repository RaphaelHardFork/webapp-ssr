pub mod utils;

use modql::SIden;
use sea_query::types::IntoIden;
use sea_query::{Iden, TableRef};

/// An alias to differenciate String and Uuid as
/// these latter are stored into String in SQlite
pub type UuidStr = String;

// region:		=== Const ===

// const LIST_LIMIT_DEFAULT: i64 = 1000;
// const LIST_LIMIT_MAX: i64 = 5000;

// endregion:	=== Const ===

// region:		=== SeaQuery Idens ===

#[derive(Iden)]
pub enum CommonIden {
    Id,
}

#[derive(Iden)]
pub enum TimestampIden {
    CId,
    CTime,
    MId,
    MTime,
}

// endregion:	=== SeaQuery Idens ===

/// DbBmc (DB backend model controller) trait must be implemented for Bmc's structs
/// It specifies meta information such as table name, timestamps, ...
/// DB backend model controller
pub trait DbBmc {
    const TABLE: &'static str;

    fn table_ref() -> TableRef {
        TableRef::Table(SIden(Self::TABLE).into_iden())
    }
}
