use serde::{Deserialize, Serialize};
use actix_web::{Error, HttpRequest, HttpResponse, Responder};
use futures::future::{ready, Ready};
use super::schema::histories;

use chrono::{DateTime, Local};

#[derive(Queryable, Debug, Serialize, Deserialize)]
pub struct History {
    pub id: i32,
    pub hostname: String,
    pub working_directory: Option<String>,
    pub command: String,
    // pub created_at: DateTime<Local>,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[table_name="histories"]
pub struct NewHistory {
    pub hostname: String,
    pub working_directory: String,
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Histories {
    pub elements: Vec<History>,
}

impl Responder for History {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();

        ready(Ok(HttpResponse::Ok()
                 .content_type("application/json")
                 .body(body)))
    }
}

impl Responder for NewHistory {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        ready(Ok(HttpResponse::Created()
                 .content_type("application/json")
                 .body("{\"message\": \"Successfully created\"}")))
    }
}

impl Responder for Histories {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();

        ready(Ok(HttpResponse::Ok()
                 .content_type("application/json")
                 .body(body)))
    }
}
