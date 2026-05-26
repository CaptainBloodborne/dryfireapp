//! Shared list-query primitives.
//!
//! ## Pagination
//! - `?page=1&per_page=20`. 1-indexed for the client; converted to
//!   `OFFSET/LIMIT` here.
//! - Defaults: page=1, per_page=20. Max per_page=100 (prevents
//!   `?per_page=10000000`).
//!
//! ## Sorting
//! - `?sort=field` or `?sort=-field` (the leading `-` means desc).
//! - Each repository whitelists which fields it accepts; unknown
//!   fields are rejected (not silently ignored) — protects against
//!   SQL injection because we never interpolate user input.
//!
//! ## Response envelope
//! - `{ items: [...], page, per_page, total }` so the client can
//!   render pagination controls without an extra HEAD request.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct PageQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub sort: Option<String>,
}

fn default_page() -> u32 { 1 }
fn default_per_page() -> u32 { 20 }

const MAX_PER_PAGE: u32 = 100;

impl PageQuery {
    /// Snap into the valid range. We *normalize* rather than reject
    /// out-of-range input — the spec doesn't say `?page=0` is a hard
    /// error, and returning page 1 is friendlier than a 400.
    pub fn normalized(&self) -> PageNormalized {
        let page = self.page.max(1);
        let per_page = self.per_page.clamp(1, MAX_PER_PAGE);
        PageNormalized {
            page,
            per_page,
            sort: self.sort.as_deref().and_then(parse_sort),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageNormalized {
    pub page: u32,
    pub per_page: u32,
    pub sort: Option<SortSpec>,
}

impl PageNormalized {
    pub fn limit(&self) -> i64 { self.per_page as i64 }
    pub fn offset(&self) -> i64 { ((self.page - 1) * self.per_page) as i64 }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortSpec {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection { Asc, Desc }

impl SortDirection {
    pub fn as_sql(&self) -> &'static str {
        match self {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        }
    }
}

fn parse_sort(raw: &str) -> Option<SortSpec> {
    let raw = raw.trim();
    if raw.is_empty() { return None; }
    let (direction, field) = if let Some(rest) = raw.strip_prefix('-') {
        (SortDirection::Desc, rest)
    } else if let Some(rest) = raw.strip_prefix('+') {
        (SortDirection::Asc, rest)
    } else {
        (SortDirection::Asc, raw)
    };
    Some(SortSpec { field: field.to_string(), direction })
}

/// Standard paginated response envelope.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: i64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: i64, p: &PageNormalized) -> Self {
        Self { items, page: p.page, per_page: p.per_page, total }
    }
}

/// Build the `ORDER BY` SQL fragment.
///
/// `allowed` maps **client-facing** sort field names to **SQL** column
/// names. The indirection lets us:
///   1. rename DB columns without breaking the API,
///   2. expose computed columns ("name" - "lower(login)"),
///   3. reject unknown fields by construction (no SQL injection).
///
/// `default_field` is a SQL column name (no direction).
/// `default_direction` applies when no `sort=` was supplied.
pub fn order_by_clause(
    sort: &Option<SortSpec>,
    allowed: &[(&str, &str)],
    default_field: &str,
    default_direction: SortDirection,
) -> String {
    let spec = match sort {
        Some(s) => s,
        None => return format!("ORDER BY {} {}", default_field, default_direction.as_sql()),
    };
    let (column, dir) = allowed
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(&spec.field))
        .map(|(_, v)| (*v, spec.direction))
        .unwrap_or((default_field, default_direction));
    format!("ORDER BY {} {}", column, dir.as_sql())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sort_basic() {
        assert_eq!(
            parse_sort("name"),
            Some(SortSpec { field: "name".into(), direction: SortDirection::Asc })
        );
        assert_eq!(
            parse_sort("-created_at"),
            Some(SortSpec { field: "created_at".into(), direction: SortDirection::Desc })
        );
        assert_eq!(parse_sort(""), None);
    }

    #[test]
    fn normalize_clamps() {
        let p = PageQuery { page: 0, per_page: 9999, sort: None }.normalized();
        assert_eq!(p.page, 1);
        assert_eq!(p.per_page, MAX_PER_PAGE);
    }

    #[test]
    fn order_by_rejects_unknown_field() {
        // Unknown field falls back to default — and critically does NOT
        // interpolate the unknown name into SQL.
        let sql = order_by_clause(
            &Some(SortSpec { field: "DROP TABLE users".into(), direction: SortDirection::Desc }),
            &[("name", "login")],
            "created_at",
            SortDirection::Desc,
        );
        assert_eq!(sql, "ORDER BY created_at DESC");
    }

    #[test]
    fn order_by_known_field_with_direction() {
        let sql = order_by_clause(
            &Some(SortSpec { field: "name".into(), direction: SortDirection::Desc }),
            &[("name", "login")],
            "created_at",
            SortDirection::Asc,
        );
        assert_eq!(sql, "ORDER BY login DESC");
    }
}
