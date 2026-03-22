use diesel::dsl::*;
use diesel::prelude::*;

use crate::models;

type HistoriesQuery<'a> = crate::schema::histories::BoxedQuery<'a, diesel::pg::Pg>;

fn with_filters<'a>(query: HistoriesQuery<'a>, q: &'a models::SearchQuery) -> HistoriesQuery<'a> {
    use crate::schema::histories::dsl::*;
    let mut query = query;
    if let Some(ref pwd) = q.pwd {
        query = query.filter(working_directory.eq(pwd));
    }
    if let Some(ref host) = q.hostname {
        query = query.filter(hostname.eq(host));
    }
    query
}

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
    q: &models::SearchQuery,
) -> Result<(Vec<models::History>, i64), diesel::result::Error> {
    use crate::schema::histories::dsl::*;

    let total: i64 = with_filters(histories.into_boxed(), q)
        .count()
        .get_result(conn)?;

    let results = with_filters(histories.into_boxed(), q)
        .order((updated_at.desc(), id.desc()))
        .limit(q.effective_limit())
        .offset(q.effective_offset())
        .load::<models::History>(conn)?;

    Ok((results, total))
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
    use diesel::Connection;
    use dotenv::dotenv;

    fn setup() -> PgConnection {
        dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        PgConnection::establish(&database_url).expect("Error connecting to the database")
    }

    fn make_query(
        pwd: Option<&str>,
        hostname: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> models::SearchQuery {
        models::SearchQuery {
            pwd: pwd.map(str::to_string),
            hostname: hostname.map(str::to_string),
            limit,
            offset,
        }
    }

    #[test]
    fn test_create_and_search_history() {
        let mut conn = setup();
        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let h = "test-host";
            let w = "/test/dir";
            let c = "test command";

            let new_history = create_history(conn, h, w, c)?;
            assert_eq!(new_history.hostname, h);
            assert_eq!(new_history.working_directory, w);
            assert_eq!(new_history.command, c);

            let q = make_query(Some(w), None, None, None);
            let (results, total) = search(conn, &q)?;

            assert_eq!(results.len(), 1);
            assert_eq!(total, 1);
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

            create_history(conn, h, w, c)?;

            let q = make_query(Some(w), None, None, None);
            let (results, _) = search(conn, &q)?;
            assert_eq!(results.len(), 1);
            let history_to_delete = &results[0];

            let delete_result = delete_history(conn, history_to_delete.id)?;
            assert_eq!(delete_result.count, 1);

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

            create_history(conn, h, w, c)?;
            diesel::sql_query("COMMIT;").execute(conn)?;
            let q_all = make_query(None, None, Some(10000), None);
            let (results1, _) = search(conn, &q_all)?;
            let initial_update_time = results1
                .iter()
                .find(|history| history.command == c)
                .unwrap()
                .updated_at;

            std::thread::sleep(std::time::Duration::from_secs(1));
            create_history(conn, h, w, c)?;

            let q = make_query(Some(w), None, None, None);
            let (results2, total) = search(conn, &q)?;
            assert_eq!(results2.len(), 1, "Should not create a new record");
            assert_eq!(total, 1);
            let updated_history = &results2[0];
            assert!(
                updated_history.updated_at > initial_update_time,
                "updated_at should be different"
            );

            Ok(())
        });
    }

    #[test]
    fn test_search_pagination() {
        let mut conn = setup();
        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let w = "/pagination/dir";
            create_history(conn, "host-a", w, "cmd-alpha")?;
            create_history(conn, "host-b", w, "cmd-beta")?;
            create_history(conn, "host-c", w, "cmd-gamma")?;

            // First page: 2 items
            let q1 = make_query(Some(w), None, Some(2), Some(0));
            let (page1, total) = search(conn, &q1)?;
            assert_eq!(page1.len(), 2);
            assert_eq!(total, 3);

            // Second page: 1 item
            let q2 = make_query(Some(w), None, Some(2), Some(2));
            let (page2, total2) = search(conn, &q2)?;
            assert_eq!(page2.len(), 1);
            assert_eq!(total2, 3);

            // No overlap between pages
            let ids1: Vec<i32> = page1.iter().map(|h| h.id).collect();
            let ids2: Vec<i32> = page2.iter().map(|h| h.id).collect();
            assert!(ids1.iter().all(|id| !ids2.contains(id)));

            Ok(())
        });
    }

    #[test]
    fn test_search_hostname_filter() {
        let mut conn = setup();
        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let w = "/hostname/dir";
            create_history(conn, "target-host", w, "cmd-for-target")?;
            create_history(conn, "other-host", w, "cmd-for-other")?;

            let q = make_query(None, Some("target-host"), None, None);
            let (results, total) = search(conn, &q)?;

            assert_eq!(total, 1);
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].hostname, "target-host");

            Ok(())
        });
    }

    #[test]
    fn test_search_returns_total_count() {
        let mut conn = setup();
        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let w = "/count/dir";
            for i in 0..5 {
                create_history(conn, "count-host", w, &format!("cmd-{i}"))?;
            }

            // limit=2 but total should reflect all 5
            let q = make_query(Some(w), None, Some(2), Some(0));
            let (results, total) = search(conn, &q)?;

            assert_eq!(results.len(), 2);
            assert_eq!(total, 5);

            Ok(())
        });
    }

    #[test]
    fn test_search_combined_filters() {
        let mut conn = setup();
        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let w = "/combo/dir";
            create_history(conn, "combo-host", w, "combo-cmd")?;
            create_history(conn, "combo-host", "/other/dir", "other-cmd")?;
            create_history(conn, "other-host", w, "yet-other-cmd")?;

            let q = make_query(Some(w), Some("combo-host"), None, None);
            let (results, total) = search(conn, &q)?;

            assert_eq!(total, 1);
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].command, "combo-cmd");

            Ok(())
        });
    }

    #[test]
    fn test_search_limit_cap() {
        let q = models::SearchQuery {
            pwd: None,
            hostname: None,
            limit: Some(99_999),
            offset: None,
        };
        assert_eq!(q.effective_limit(), 10_000);

        let q_zero = models::SearchQuery {
            pwd: None,
            hostname: None,
            limit: Some(0),
            offset: None,
        };
        assert_eq!(q_zero.effective_limit(), 1);
    }
}
