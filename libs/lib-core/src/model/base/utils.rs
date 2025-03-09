use super::TimestampIden;
use lib_utils::time::{format_time, now_utc};

use modql::field::{Field, Fields};
use sea_query::IntoIden;

// region:		=== Timestamps ===
pub fn add_timestamps_for_create(fields: &mut Fields, user_id: i64) {
    let now = now_utc();

    fields.push(Field::new(TimestampIden::CId.into_iden(), user_id.into()));
    fields.push(Field::new(
        TimestampIden::CTime.into_iden(),
        format_time(now).into(),
    ));
    fields.push(Field::new(TimestampIden::MId.into_iden(), user_id.into()));
    fields.push(Field::new(
        TimestampIden::MTime.into_iden(),
        format_time(now).into(),
    ));
}

pub fn add_timestamps_for_update(fields: &mut Fields, user_id: i64) {
    let now = now_utc();
    fields.push(Field::new(TimestampIden::MId.into_iden(), user_id.into()));
    fields.push(Field::new(
        TimestampIden::MTime.into_iden(),
        format_time(now).into(),
    ));
}

// endregion:	=== Timestamps ===
