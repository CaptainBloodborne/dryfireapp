use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::errors::ValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LawTag {
    Storage, Carry, Hunting, Sport, SelfDefense, Transport, General,
}
impl LawTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Storage=>"storage",Self::Carry=>"carry",Self::Hunting=>"hunting",
            Self::Sport=>"sport",Self::SelfDefense=>"self_defense",
            Self::Transport=>"transport",Self::General=>"general",
        }
    }
}
impl FromStr for LawTag {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "storage"=>Self::Storage,"carry"=>Self::Carry,"hunting"=>Self::Hunting,
            "sport"=>Self::Sport,"self_defense"=>Self::SelfDefense,
            "transport"=>Self::Transport,"general"=>Self::General,
            o => return Err(ValidationError::Custom(format!("law_tag `{o}`"))),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Law {
    pub id: Uuid,
    pub region: String,
    pub slug: String,
    pub title: String,
    pub body: String,
    pub version: i32,
    pub tags: Vec<LawTag>,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}