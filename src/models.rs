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
/// exactly matches any of them are returned. An empty `pwd` means no filter.
#[derive(Debug, Default)]
pub struct SearchQuery {
    pub pwd: Vec<String>,
    pub hostname: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl SearchQuery {
    /// Builds a query from raw `key=value` pairs, keeping every `pwd` occurrence.
    ///
    /// For the other keys the last occurrence wins. Unknown keys are ignored.
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Result<Self, std::num::ParseIntError> {
        let mut q = SearchQuery::default();
        for (key, value) in pairs {
            match key.as_str() {
                "pwd" => q.pwd.push(value),
                "hostname" => q.hostname = Some(value),
                "limit" => q.limit = Some(value.parse()?),
                "offset" => q.offset = Some(value.parse()?),
                _ => {}
            }
        }
        Ok(q)
    }

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

    fn pairs(raw: &[(&str, &str)]) -> Vec<(String, String)> {
        raw.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_from_pairs_collects_repeated_pwd() {
        let q = SearchQuery::from_pairs(pairs(&[
            ("pwd", "/a"),
            ("hostname", "h"),
            ("pwd", "/b"),
            ("limit", "10"),
            ("offset", "20"),
        ]))
        .unwrap();
        assert_eq!(q.pwd, vec!["/a", "/b"]);
        assert_eq!(q.hostname.as_deref(), Some("h"));
        assert_eq!(q.limit, Some(10));
        assert_eq!(q.offset, Some(20));
    }

    #[test]
    fn test_from_pairs_without_pwd() {
        let q = SearchQuery::from_pairs(pairs(&[("unknown", "x")])).unwrap();
        assert!(q.pwd.is_empty());
        assert!(q.hostname.is_none());
        assert!(q.limit.is_none());
        assert!(q.offset.is_none());
    }

    #[test]
    fn test_from_pairs_rejects_invalid_number() {
        assert!(SearchQuery::from_pairs(pairs(&[("limit", "abc")])).is_err());
        assert!(SearchQuery::from_pairs(pairs(&[("offset", "")])).is_err());
    }
}
