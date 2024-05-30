use actix_web::{body::BoxBody, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use chrono::prelude::*;
use chrono::DateTime;

use diesel::{Insertable, Queryable};

#[derive(Queryable, Debug, Serialize, Deserialize)]
pub struct History {
    pub id: i32,
    pub hostname: String,
    pub working_directory: Option<String>,
    pub command: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::histories)]
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
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(&self)
    }
}

impl Responder for NewHistory {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let message = SimpleMessage {
            message: String::from("Successfully created"),
        };

        HttpResponse::Created().json(message)
    }
}

impl Responder for Histories {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(&self)
    }
}

impl Responder for DeletedHistoryCount {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(&self)
    }
}
