extern crate chrono;

use super::schema::histories;
use actix_web::{Error, HttpRequest, HttpResponse, Responder};
use futures::future::{ready, Ready};
use serde::{Deserialize, Serialize};

use chrono::prelude::*;
use chrono::DateTime;

#[derive(Queryable, Debug, Serialize, Deserialize)]
pub struct History {
    pub id: i32,
    pub hostname: String,
    pub working_directory: Option<String>,
    pub command: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[table_name = "histories"]
pub struct NewHistory {
    pub hostname: String,
    pub working_directory: String,
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Histories {
    pub elements: Vec<History>,
}

#[derive(Debug, Serialize)]
pub struct DeletedHistoryCount {
    pub count: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
struct SimpleMessage {
    pub message: String,
}

impl Responder for History {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        ready(Ok(HttpResponse::Ok().json(&self)))
    }
}

impl Responder for NewHistory {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let message = SimpleMessage {
            message: String::from("Successfully created"),
        };

        ready(Ok(HttpResponse::Created().json(message)))
    }
}

impl Responder for Histories {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        ready(Ok(HttpResponse::Ok().json(&self)))
    }
}

impl Responder for DeletedHistoryCount {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        ready(Ok(HttpResponse::Ok().json(&self)))
    }
}
