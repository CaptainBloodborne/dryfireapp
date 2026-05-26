//! Laws aggregate — versioned legal documents with FTS-friendly shape.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::errors::ValidationError;

// ---------------- LawTag (closed vocabulary) ---------------- //

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LawTag {
    Transport,
    Carry,
    Storage,
    Hunting,
    SelfDefense,
    Sport,
    License,
    Inheritance,
    Inspection,
    Penalty,
    Other,
}

impl LawTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            LawTag::Transport    => "transport",
            LawTag::Carry        => "carry",
            LawTag::Storage      => "storage",
            LawTag::Hunting      => "hunting",
            LawTag::SelfDefense  => "self_defense",
            LawTag::Sport        => "sport",
            LawTag::License      => "license",
            LawTag::Inheritance  => "inheritance",
            LawTag::Inspection   => "inspection",
            LawTag::Penalty      => "penalty",
            LawTag::Other        => "other",
        }
    }
}

impl std::str::FromStr for LawTag {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "transport"    => Ok(LawTag::Transport),
            "carry"        => Ok(LawTag::Carry),
            "storage"      => Ok(LawTag::Storage),
            "hunting"      => Ok(LawTag::Hunting),
            "self_defense" => Ok(LawTag::SelfDefense),
            "sport"        => Ok(LawTag::Sport),
            "license"      => Ok(LawTag::License),
            "inheritance"  => Ok(LawTag::Inheritance),
            "inspection"   => Ok(LawTag::Inspection),
            "penalty"      => Ok(LawTag::Penalty),
            "other"        => Ok(LawTag::Other),
            other          => Err(ValidationError::Custom(
                format!("unknown law tag `{other}`"))),
        }
    }
}

// ---------------- LawCategory ---------------- //

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawCategory {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LawCategory {
    pub fn new(
        code: String,
        name: String,
        parent_id: Option<Uuid>,
        sort_order: i32,
    ) -> Result<Self, ValidationError> {
        if code.trim().is_empty() || name.trim().is_empty() {
            return Err(ValidationError::Custom(
                "code and name are required".into()));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            code: code.trim().into(),
            name: name.trim().into(),
            parent_id,
            sort_order,
            created_at: now,
            updated_at: now,
        })
    }
}

// ---------------- Law ---------------- //

#[derive(Debug, Clone, Serialize)]
pub struct Law {
    pub id: Uuid,
    pub law_key: String,
    pub title: String,
    pub summary: Option<String>,
    pub body: String,
    pub region: String,
    pub category_id: Option<Uuid>,
    pub tags: Vec<LawTag>,
    pub current_version: i32,
    pub effective_at: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Law {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        law_key: String,
        title: String,
        summary: Option<String>,
        body: String,
        region: String,
        category_id: Option<Uuid>,
        tags: Vec<LawTag>,
        effective_at: NaiveDate,
    ) -> Result<Self, ValidationError> {
        if law_key.trim().is_empty() {
            return Err(ValidationError::Custom("law_key is required".into()));
        }
        if title.trim().is_empty() {
            return Err(ValidationError::Custom("title is required".into()));
        }
        if body.trim().is_empty() {
            return Err(ValidationError::Custom("body is required".into()));
        }
        if region.trim().is_empty() {
            return Err(ValidationError::Custom("region is required".into()));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            law_key: law_key.trim().into(),
            title: title.trim().into(),
            summary,
            body,
            region: region.trim().into(),
            category_id,
            tags,
            current_version: 1,
            effective_at,
            created_at: now,
            updated_at: now,
        })
    }
}

// ---------------- LawVersion (historical snapshot) ---------------- //

#[derive(Debug, Clone, Serialize)]
pub struct LawVersion {
    pub id: Uuid,
    pub law_id: Uuid,
    pub law_key: String,
    pub version: i32,
    pub title: String,
    pub summary: Option<String>,
    pub body: String,
    pub tags: Vec<LawTag>,
    pub category_id: Option<Uuid>,
    pub effective_at: NaiveDate,
    pub snapshot_at: DateTime<Utc>,
}

// ---------------- search-result decoration ---------------- //

/// Returned by the search endpoint. `rank` is PostgreSQL's `ts_rank_cd`
/// score (higher is more relevant); `snippet` is `ts_headline` over
/// the body, with `<mark>` around matched terms.
#[derive(Debug, Clone, Serialize)]
pub struct LawSearchHit {
    pub law: Law,
    pub rank: f32,
    pub snippet: String,
}

// ---------------- tests ---------------- //

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn law_tag_roundtrip() {
        for t in [
            LawTag::Transport, LawTag::Carry, LawTag::Storage,
            LawTag::Hunting, LawTag::SelfDefense, LawTag::Sport,
            LawTag::License, LawTag::Inheritance, LawTag::Inspection,
            LawTag::Penalty, LawTag::Other,
        ] {
            let s = t.as_str();
            let back: LawTag = s.parse().unwrap();
            assert_eq!(t, back);
        }
        assert!("magic".parse::<LawTag>().is_err());
    }

    #[test]
    fn law_new_validates_required() {
        let e = Law::new(
            "".into(), "Title".into(), None, "Body".into(),
            "RU".into(), None, vec![],
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }

    #[test]
    fn law_new_starts_at_version_1() {
        let l = Law::new(
            "fz-150".into(), "Об оружии".into(), None, "...".into(),
            "RU".into(), None, vec![LawTag::Storage, LawTag::Carry],
            NaiveDate::from_ymd_opt(1996, 12, 13).unwrap(),
        ).unwrap();
        assert_eq!(l.current_version, 1);
        assert_eq!(l.tags.len(), 2);
    }

    #[test]
    fn category_validates() {
        assert!(LawCategory::new("storage".into(), "Хранение".into(), None, 1).is_ok());
        assert!(LawCategory::new("".into(), "Name".into(), None, 0).is_err());
    }
}
