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
///
/// `pwd` may be repeated (`?pwd=/a&pwd=/b`); histories whose working directory
/// exactly matches any of them are returned. Without any `pwd` key no directory
/// filter is applied; `pwd=` (empty value) matches an empty working directory.
///
/// Deserialize with `serde_html_form`, not `serde_urlencoded` (actix's
/// `web::Query`): only the former collects repeated keys into a `Vec`. Other
/// keys must appear at most once; repeating them is a deserialize error.
#[derive(Debug, Default, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub pwd: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(query: &str) -> Result<SearchQuery, String> {
        serde_html_form::from_str(query).map_err(|e| e.to_string())
    }

    #[test]
    fn test_query_collects_repeated_pwd_even_when_interleaved() {
        let q = parse("pwd=%2Fa&hostname=h&pwd=%2Fb").unwrap();
        assert_eq!(q.pwd, vec!["/a", "/b"]);
    }

    #[test]
    fn test_query_keeps_empty_pwd_as_filter_value() {
        let q = parse("pwd=").unwrap();
        assert_eq!(q.pwd, vec![""]);
    }

    #[test]
    fn test_query_parses_single_valued_keys() {
        let q = parse("hostname=h&limit=10&offset=20").unwrap();
        assert!(q.pwd.is_empty());
        assert_eq!(q.hostname.as_deref(), Some("h"));
        assert_eq!(q.limit, Some(10));
        assert_eq!(q.offset, Some(20));
    }

    #[test]
    fn test_query_ignores_unknown_keys() {
        let q = parse("unknown=x").unwrap();
        assert!(q.pwd.is_empty());
        assert!(q.hostname.is_none());
    }

    #[test]
    fn test_query_rejects_repeated_single_valued_keys() {
        assert!(parse("hostname=a&hostname=b").is_err());
        assert!(parse("limit=1&limit=2").is_err());
    }

    #[test]
    fn test_query_rejects_invalid_number() {
        assert!(parse("limit=abc").is_err());
        assert!(parse("offset=abc").is_err());
    }
}
