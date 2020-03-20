use diesel::prelude::*;
use std::collections::HashMap;

use crate::models;

pub fn find(
    conn: &PgConnection,
    history_id: i32,
) -> Result<Option<models::History>, diesel::result::Error> {
    use crate::schema::histories::dsl::*;

    let history = histories
        .filter(id.eq(history_id))
        .first::<models::History>(conn)
        .optional()?;

    Ok(history)
}

pub fn search(
    conn: &PgConnection,
    q: &HashMap<String, String>,
) -> Result<Vec<models::History>, diesel::result::Error> {
    use crate::schema::histories::dsl::*;

    let results = match q.get("pwd") {
        Some(pwd) => histories
            .filter(working_directory.eq(&pwd))
            .order(created_at.desc())
            .load::<models::History>(conn)?,
        None => histories
            .order(created_at.desc())
            .load::<models::History>(conn)?,
    };

    Ok(results)
}

pub fn create_history(
    conn: &PgConnection,
    h: &str,
    w: &str,
    c: &str,
) -> Result<models::NewHistory, diesel::result::Error> {
    use crate::schema::histories::dsl::*;

    let new_history = models::NewHistory {
        hostname: h.to_string(),
        working_directory: w.to_string(),
        command: c.to_string(),
    };

    diesel::insert_into(histories)
        .values(&new_history)
        .on_conflict((hostname, working_directory, command))
        .do_nothing()
        .execute(conn)?;

    Ok(new_history)
}

pub fn delete_history(
    conn: &PgConnection,
    history_id: i32,
) -> Result<models::DeletedHistoryCount, diesel::result::Error> {
    use crate::schema::histories::dsl::*;

    let deleted_count = diesel::delete(histories.filter(id.eq(history_id))).execute(conn)?;
    let deleted_history_count = models::DeletedHistoryCount {
        count: deleted_count,
        message: String::from("Successfully deleted"),
    };

    Ok(deleted_history_count)
}
