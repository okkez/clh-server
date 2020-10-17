#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
extern crate dotenv;

use std::collections::HashMap;

use actix_web::middleware::Logger;
use actix_web::{delete, get, post, web};
use actix_web::{App, HttpResponse, HttpServer, Responder};
use listenfd::ListenFd;

use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use diesel_migrations::embed_migrations;
use dotenv::dotenv;

mod actions;
mod models;
mod schema;

use self::models::*;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[get("/")]
async fn index(pool: web::Data<DbPool>, q: web::Query<HashMap<String, String>>) -> impl Responder {
    let conn = pool.get().expect("cannot get db connection from pool");

    let results = web::block(move || actions::search(&conn, &q))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::InternalServerError().finish()
        });

    match results {
        Ok(h) => web::Json(h),
        Err(_e) => web::Json(vec![]),
    }
}

#[get("/{id}")]
async fn show(pool: web::Data<DbPool>, id: web::Path<i32>) -> impl Responder {
    let conn = pool.get().expect("cannot get db connection from pool");

    web::block(move || actions::find(&conn, *id))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::InternalServerError().finish()
        })
}

#[post("/")]
async fn create(pool: web::Data<DbPool>, new_history: web::Form<NewHistory>) -> impl Responder {
    let conn = pool.get().expect("cannot get db connection from pool");

    web::block(move || {
        actions::create_history(
            &conn,
            &new_history.hostname,
            &new_history.working_directory,
            &new_history.command,
        )
    })
    .await
    .map_err(|e| {
        eprintln!("{}", e);
        HttpResponse::InternalServerError().finish()
    })
}

#[delete("/{id}")]
async fn delete(pool: web::Data<DbPool>, id: web::Path<i32>) -> impl Responder {
    let conn = pool.get().expect("cannot get db connection from pool");

    web::block(move || actions::delete_history(&conn, *id))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::InternalServerError().finish()
        })
}

embed_migrations!("./migrations");

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool");

    let conn = pool.get().expect("cannot get db connection from pool");
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout()).expect("failed to run migrations");

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
