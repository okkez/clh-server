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

/// Query parameters for `GET /`
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub pwd: Option<String>,
    pub hostname: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl SearchQuery {
    /// Returns the effective limit, capped at 10,000.
    pub fn effective_limit(&self) -> i64 {
        self.limit.unwrap_or(1000).clamp(1, 10_000)
    }

    /// Returns the effective offset, floored at 0.
    pub fn effective_offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
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
