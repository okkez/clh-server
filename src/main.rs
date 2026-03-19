use actix_web::middleware::Logger;
use actix_web::{delete, error, get, post, web, HttpResponse};
use actix_web::{App, HttpServer, Responder, Result};
use listenfd::ListenFd;

use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenv::dotenv;

mod actions;
mod models;
mod schema;

use crate::models::*;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[get("/")]
async fn index(pool: web::Data<DbPool>, q: web::Query<SearchQuery>) -> Result<impl Responder> {
    let mut conn = pool.get().expect("cannot get db connection from pool");

    match web::block(move || actions::search(&mut conn, &q)).await {
        Ok(response) => match response {
            Ok((histories, total)) => Ok(HttpResponse::Ok()
                .insert_header(("X-Total-Count", total.to_string()))
                .json(histories)),
            Err(e) => Err(error::ErrorInternalServerError(e)),
        },
        Err(e) => Err(error::ErrorInternalServerError(e)),
    }
}

#[get("/{id}")]
async fn show(pool: web::Data<DbPool>, id: web::Path<i32>) -> Result<impl Responder> {
    let mut conn = pool.get().expect("cannot get db connection from pool");

    match web::block(move || actions::find(&mut conn, *id)).await {
        Ok(response) => match response {
            Ok(r) => Ok(web::Json(r)),
            Err(e) => Err(error::ErrorInternalServerError(e)),
        },
        Err(e) => Err(error::ErrorInternalServerError(e)),
    }
}

#[post("/")]
async fn create(
    pool: web::Data<DbPool>,
    new_history: web::Form<NewHistory>,
) -> Result<impl Responder> {
    let mut conn = pool.get().expect("cannot get db connection from pool");

    let wrapped_response = web::block(move || {
        actions::create_history(
            &mut conn,
            &new_history.hostname,
            &new_history.working_directory,
            &new_history.command,
        )
    })
    .await;

    match wrapped_response {
        Ok(response) => match response {
            Ok(r) => Ok(HttpResponse::Created().json(r)),
            Err(e) => Err(error::ErrorInternalServerError(e)),
        },
        Err(e) => Err(error::ErrorInternalServerError(e)),
    }
}

#[delete("/{id}")]
async fn delete(pool: web::Data<DbPool>, id: web::Path<i32>) -> Result<impl Responder> {
    let mut conn = pool.get().expect("cannot get db connection from pool");

    match web::block(move || actions::delete_history(&mut conn, *id)).await {
        Ok(response) => match response {
            Ok(r) => Ok(web::Json(r)),
            Err(e) => Err(error::ErrorInternalServerError(e)),
        },
        Err(e) => Err(error::ErrorInternalServerError(e)),
    }
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool");

    let mut conn = pool.get().expect("cannot get db connection from pool");
    conn.run_pending_migrations(MIGRATIONS).unwrap();

    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(Logger::default())
            .service(index)
            .service(show)
            .service(create)
            .service(delete)
    });

    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind("0.0.0.0:8088")?
    };
    server.run().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test};
    use diesel::prelude::*;
    use diesel_migrations::MigrationHarness;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup_pool() -> DbPool {
        dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = r2d2::Pool::builder()
            .max_size(4)
            .build(manager)
            .expect("Failed to create pool");

        let mut conn = pool.get().expect("cannot get db connection from pool");
        conn.run_pending_migrations(MIGRATIONS).unwrap();
        drop(conn);

        pool
    }

    fn test_history(label: &str) -> NewHistory {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();

        NewHistory {
            hostname: format!("handler-test-{label}-{unique}"),
            working_directory: format!("pwd-{label}-{unique}"),
            command: format!("command-{label}-{unique}"),
        }
    }

    fn cleanup_history(pool: &DbPool, history: &NewHistory) -> Result<(), diesel::result::Error> {
        use crate::schema::histories::dsl::*;

        let mut conn = pool.get().expect("cannot get db connection from pool");
        diesel::delete(
            histories
                .filter(hostname.eq(&history.hostname))
                .filter(working_directory.eq(&history.working_directory))
                .filter(command.eq(&history.command)),
        )
        .execute(&mut conn)?;

        Ok(())
    }

    struct TestHistoryGuard {
        pool: DbPool,
        history: NewHistory,
    }

    impl TestHistoryGuard {
        fn new(pool: &DbPool, label: &str) -> Self {
            let history = test_history(label);
            cleanup_history(pool, &history).expect("failed to cleanup test history before test");

            Self {
                pool: pool.clone(),
                history,
            }
        }

        fn history(&self) -> &NewHistory {
            &self.history
        }
    }

    impl Drop for TestHistoryGuard {
        fn drop(&mut self) {
            if let Err(error) = cleanup_history(&self.pool, &self.history) {
                if std::thread::panicking() {
                    eprintln!("failed to cleanup test history during panic: {error}");
                } else {
                    panic!("failed to cleanup test history: {error}");
                }
            }
        }
    }

    fn seed_history(pool: &DbPool, history: &NewHistory) -> History {
        let mut conn = pool.get().expect("cannot get db connection from pool");
        actions::create_history(
            &mut conn,
            &history.hostname,
            &history.working_directory,
            &history.command,
        )
        .expect("failed to seed history");

        let query = SearchQuery {
            pwd: Some(history.working_directory.clone()),
            hostname: None,
            limit: None,
            offset: None,
        };

        let (results, _) =
            actions::search(&mut conn, &query).expect("failed to load seeded history");
        results
            .into_iter()
            .find(|candidate| {
                candidate.hostname == history.hostname && candidate.command == history.command
            })
            .expect("seeded history should exist")
    }

    #[actix_rt::test]
    async fn test_index_filters_histories_by_pwd() {
        let pool = setup_pool();
        let matching = TestHistoryGuard::new(&pool, "index-match");
        let other = TestHistoryGuard::new(&pool, "index-other");

        seed_history(&pool, matching.history());
        seed_history(&pool, other.history());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .service(index)
                .service(show)
                .service(create)
                .service(delete),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/?pwd={}", matching.history().working_directory))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().contains_key("x-total-count"));

        let body: Vec<History> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].hostname, matching.history().hostname);
        assert_eq!(
            body[0].working_directory,
            Some(matching.history().working_directory.clone())
        );
        assert_eq!(body[0].command, matching.history().command);
    }

    #[actix_rt::test]
    async fn test_index_returns_histories_without_query() {
        let pool = setup_pool();
        let history = TestHistoryGuard::new(&pool, "index-all");

        seed_history(&pool, history.history());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .service(index)
                .service(show)
                .service(create)
                .service(delete),
        )
        .await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().contains_key("x-total-count"));

        let body: Vec<History> = test::read_body_json(resp).await;
        assert!(body.iter().any(|candidate| {
            candidate.hostname == history.history().hostname
                && candidate.working_directory == Some(history.history().working_directory.clone())
                && candidate.command == history.history().command
        }));
    }

    #[actix_rt::test]
    async fn test_show_returns_history_by_id() {
        let pool = setup_pool();
        let history = TestHistoryGuard::new(&pool, "show-existing");

        let seeded = seed_history(&pool, history.history());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .service(index)
                .service(show)
                .service(create)
                .service(delete),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/{}", seeded.id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: Option<History> = test::read_body_json(resp).await;
        let returned = body.expect("expected an existing history");
        assert_eq!(returned.id, seeded.id);
        assert_eq!(returned.hostname, history.history().hostname);
        assert_eq!(
            returned.working_directory,
            Some(history.history().working_directory.clone())
        );
        assert_eq!(returned.command, history.history().command);
    }

    #[actix_rt::test]
    async fn test_show_returns_null_for_missing_history() {
        let pool = setup_pool();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(index)
                .service(show)
                .service(create)
                .service(delete),
        )
        .await;

        let req = test::TestRequest::get().uri("/2147483647").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: Option<History> = test::read_body_json(resp).await;
        assert!(body.is_none());
    }

    #[actix_rt::test]
    async fn test_create_persists_history_and_returns_created() {
        let pool = setup_pool();
        let history = TestHistoryGuard::new(&pool, "create");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .service(index)
                .service(show)
                .service(create)
                .service(delete),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_form(history.history())
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: NewHistory = test::read_body_json(resp).await;
        assert_eq!(body.hostname, history.history().hostname);
        assert_eq!(body.working_directory, history.history().working_directory);
        assert_eq!(body.command, history.history().command);

        let mut conn = pool.get().expect("cannot get db connection from pool");
        let query = SearchQuery {
            pwd: Some(body.working_directory.clone()),
            hostname: None,
            limit: None,
            offset: None,
        };
        let (results, _) =
            actions::search(&mut conn, &query).expect("failed to search created history");

        assert!(results.iter().any(|candidate| {
            candidate.hostname == body.hostname && candidate.command == body.command
        }));
    }

    #[actix_rt::test]
    async fn test_delete_removes_history() {
        let pool = setup_pool();
        let history = TestHistoryGuard::new(&pool, "delete");

        let seeded = seed_history(&pool, history.history());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .service(index)
                .service(show)
                .service(create)
                .service(delete),
        )
        .await;

        let req = test::TestRequest::delete()
            .uri(&format!("/{}", seeded.id))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["count"], 1);
        assert_eq!(body["message"], "Successfully deleted");

        let mut conn = pool.get().expect("cannot get db connection from pool");
        let found = actions::find(&mut conn, seeded.id).expect("failed to look up deleted history");
        assert!(found.is_none());
    }

    #[actix_rt::test]
    async fn test_index_filters_by_hostname() {
        let pool = setup_pool();
        let target = TestHistoryGuard::new(&pool, "hostname-target");
        let other = TestHistoryGuard::new(&pool, "hostname-other");

        seed_history(&pool, target.history());
        seed_history(&pool, other.history());

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .service(index)
                .service(show)
                .service(create)
                .service(delete),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/?hostname={}", target.history().hostname))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let total: i64 = resp
            .headers()
            .get("x-total-count")
            .expect("X-Total-Count header should be present")
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(total, 1);

        let body: Vec<History> = test::read_body_json(resp).await;
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].hostname, target.history().hostname);
    }

    #[actix_rt::test]
    async fn test_index_pagination() {
        let pool = setup_pool();

        // Seed 3 records with the same unique pwd so they're isolated
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pwd = format!("pwd-pagination-{unique}");
        let hostname = format!("host-pagination-{unique}");

        {
            let mut conn = pool.get().expect("cannot get db connection from pool");
            for i in 0..3 {
                actions::create_history(&mut conn, &hostname, &pwd, &format!("cmd-{i}"))
                    .expect("failed to seed pagination history");
            }
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .service(index)
                .service(show)
                .service(create)
                .service(delete),
        )
        .await;

        // Page 1: limit=2, offset=0
        let req = test::TestRequest::get()
            .uri(&format!("/?pwd={pwd}&limit=2&offset=0"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let total: i64 = resp
            .headers()
            .get("x-total-count")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(total, 3);
        let page1: Vec<History> = test::read_body_json(resp).await;
        assert_eq!(page1.len(), 2);

        // Page 2: limit=2, offset=2
        let req = test::TestRequest::get()
            .uri(&format!("/?pwd={pwd}&limit=2&offset=2"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let page2: Vec<History> = test::read_body_json(resp).await;
        assert_eq!(page2.len(), 1);

        // No id overlap between pages
        let ids1: Vec<i32> = page1.iter().map(|h| h.id).collect();
        let ids2: Vec<i32> = page2.iter().map(|h| h.id).collect();
        assert!(ids1.iter().all(|id| !ids2.contains(id)));

        // Cleanup seeded records
        let mut conn = pool.get().expect("cannot get db connection from pool");
        use crate::schema::histories::dsl;
        diesel::delete(dsl::histories.filter(dsl::hostname.eq(&hostname)))
            .execute(&mut conn)
            .expect("failed to cleanup pagination test data");
    }
}
