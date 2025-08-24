use diesel::dsl::*;
use diesel::prelude::*;
use std::collections::HashMap;

use crate::models;

pub fn find(
    conn: &mut PgConnection,
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
    conn: &mut PgConnection,
    q: &HashMap<String, String>,
) -> Result<Vec<models::History>, diesel::result::Error> {
    use crate::schema::histories::dsl::*;

    let results = match q.get("pwd") {
        Some(pwd) => histories
            .filter(working_directory.eq(&pwd))
            .order(updated_at.desc())
            .load::<models::History>(conn)?,
        None => histories
            .order(updated_at.desc())
            .limit(10000)
            .load::<models::History>(conn)?,
    };

    Ok(results)
}

pub fn create_history(
    conn: &mut PgConnection,
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
        .do_update()
        .set(updated_at.eq(now))
        .execute(conn)?;

    Ok(new_history)
}

pub fn delete_history(
    conn: &mut PgConnection,
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


#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{Connection, connection::{AnsiTransactionManager, TransactionManager}};
    use dotenv::dotenv;

    fn setup() -> PgConnection {
        dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        PgConnection::establish(&database_url)
            .expect("Error connecting to the database")
    }

    #[test]
    fn test_create_and_search_history() {
        let mut conn = setup();
        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let h = "test-host";
            let w = "/test/dir";
            let c = "test command";

            // Create a history
            let new_history = create_history(conn, h, w, c)?;
            assert_eq!(new_history.hostname, h);
            assert_eq!(new_history.working_directory, w);
            assert_eq!(new_history.command, c);

            // Search for the created history
            let mut query = HashMap::new();
            query.insert("pwd".to_string(), w.to_string());
            let results = search(conn, &query)?;

            // Verify the result
            assert_eq!(results.len(), 1);
            let found_history = &results[0];
            assert_eq!(found_history.hostname, h);
            assert_eq!(found_history.working_directory, Some(w.to_string()));
            assert_eq!(found_history.command, c);

            Ok(())
        });
    }

    #[test]
    fn test_delete_history() {
        let mut conn = setup();
        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let h = "delete-host";
            let w = "/delete/dir";
            let c = "delete command";

            // Create a history
            create_history(conn, h, w, c)?;

            // Find it
            let mut query = HashMap::new();
            query.insert("pwd".to_string(), w.to_string());
            let results = search(conn, &query)?;
            assert_eq!(results.len(), 1);
            let history_to_delete = &results[0];

            // Delete it
            let delete_result = delete_history(conn, history_to_delete.id)?;
            assert_eq!(delete_result.count, 1);

            // Verify it's gone
            let find_result = find(conn, history_to_delete.id)?;
            assert!(find_result.is_none());

            Ok(())
        });
    }

    #[test]
    fn test_upsert_logic() {
        let mut conn = setup();
        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let h = "upsert-host";
            let w = "/upsert/dir";
            let c = "upsert command";

            // Create it once
            create_history(conn, h, w, c)?;
            diesel::sql_query("COMMIT;").execute(conn)?; // to update updated_at properly
            let results1 = search(conn, &HashMap::new())?;
            let initial_update_time = results1.iter().find(|history| history.command == c).unwrap().updated_at;

            // Create it again to trigger update
            std::thread::sleep(std::time::Duration::from_secs(1));
            create_history(conn, h, w, c)?;

            // Search and verify
            let mut query = HashMap::new();
            query.insert("pwd".to_string(), w.to_string());
            let results2 = search(conn, &query)?;
            assert_eq!(results2.len(), 1, "Should not create a new record");
            let updated_history = &results2[0];
            println!("{:?}\n{:?}", results1, results2);
            assert!(updated_history.updated_at > initial_update_time, "updated_at should be different");

            Ok(())
        });
    }
}
