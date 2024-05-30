use std::collections::HashMap;

use actix_web::middleware::Logger;
use actix_web::{delete, error, get, post, web};
use actix_web::{App, HttpResponse, HttpServer, Responder, Result};
use listenfd::ListenFd;

use diesel;
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
async fn index(
    pool: web::Data<DbPool>,
    q: web::Query<HashMap<String, String>>,
) -> Result<impl Responder> {
    let mut conn = pool.get().expect("cannot get db connection from pool");

    match web::block(move || actions::search(&mut conn, &q)).await {
        Ok(response) => match response {
            Ok(h) => Ok(web::Json(h)),
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
            Ok(r) => Ok(web::Json(r)),
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
            .data(pool.clone())
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
