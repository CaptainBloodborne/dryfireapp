//! DB row types.
//!
//! Conversion failure (e.g. an unknown enum value coming from a hand-
//! edited DB row) is mapped to `DomainError::Infra`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::domain::{
    entities::user::{Language, Region, User, UserStatus},
    errors::{DomainError, DomainResult},
};

#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub login: String,
    pub firstname: String,
    pub surname: String,
    pub email: String,
    pub date_of_birth: NaiveDate,
    pub region: String,
    pub language: String,
    pub status: String,
    pub last_visit_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserRow {
    pub fn into_domain(self) -> DomainResult<User> {
        let region = Region::new(self.region)
            .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad region in DB: {e}")))?;
        let language: Language = self
            .language
            .parse()
            .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad language in DB: {e}")))?;
        let status: UserStatus = self
            .status
            .parse()
            .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad status in DB: {e}")))?;

        Ok(User::rehydrate(
            self.id,
            self.login,
            self.firstname,
            self.surname,
            self.email,
            self.date_of_birth,
            region,
            language,
            status,
            self.last_visit_at,
            self.created_at,
            self.updated_at,
        ))
    }
}
