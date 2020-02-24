#[macro_use]
extern crate diesel;
extern crate dotenv;

use actix_web::{App, Error, HttpResponse, HttpServer, Responder};
use actix_web::{get, post, web};
use actix_web::middleware::Logger;
use listenfd::ListenFd;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::pg::PgConnection;
use dotenv::dotenv;

mod actions;
mod models;
mod schema;

use self::models::*;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[get("/")]
async fn index(pool: web::Data<DbPool>) -> impl Responder {
    let conn = pool.get().expect("cannot get db connection from pool");
    let chunk = "dummy";

    let results = web::block(move || actions::search(&conn, &chunk))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::InternalServerError().finish()
        });

    match results {
        Ok(s) => match s {
            Some(h) => web::Json(h),
            None => web::Json(vec![]),
        },
        Err(e) => web::Json(vec![]),
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
async fn create(pool: web::Data<DbPool>, new_history: web::Json<NewHistory>) -> impl Responder {
    let conn = pool.get().expect("cannot get db connection from pool");

    web::block(move || actions::create_history(&conn, &new_history.hostname, &new_history.working_directory, &new_history.command))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::InternalServerError().finish()
        })
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder().build(manager).expect("Failed to create pool");

    let mut listenfd = ListenFd::from_env();
    let mut server =
        HttpServer::new(move || {
            App::new()
                .data(pool.clone())
                .wrap(Logger::default())
                .service(index)
                .service(show)
                .service(create)
        });

        server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
            server.listen(l)?
        } else {
            server.bind("127.0.0.1:8088")?
        };
    server.run().await
}
