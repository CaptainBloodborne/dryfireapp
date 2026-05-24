// src/application/license_use_cases.rs

use chrono::{Duration, NaiveDate, Utc};
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::license::{License, LicenseKind},
        errors::{DomainError, DomainResult},
        repositories::armory::PageQuery,
    },
};

#[derive(Debug)]
pub struct CreateLicenseInput {
    pub owner_id: Uuid,
    pub kind: String,
    pub issuing_org: String,
    pub issued_at: NaiveDate,
    pub expires_at: Option<NaiveDate>,
    pub document_url: Option<String>,
    pub instructions: Option<String>,
    pub linked_gun_ids: Vec<Uuid>,
}

pub struct CreateLicenseUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateLicenseUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self, input))]
    pub async fn execute(&self, input: CreateLicenseInput) -> DomainResult<Uuid> {
        let kind: LicenseKind = input.kind.parse()?;
        let lic = License::issue(
            input.owner_id, kind, input.issuing_org, input.issued_at,
            input.expires_at, input.document_url, input.instructions,
        )?;
        self.state.license_repo.create(&lic).await?;
        for gun_id in input.linked_gun_ids {
            // best-effort link; ignore link errors here
            let _ = self.state.license_repo.link_gun(lic.id(), gun_id).await;
        }
        Ok(lic.id())
    }
}

pub struct GetLicenseUseCase<'a> { pub state: &'a AppState }
impl<'a> GetLicenseUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, id: Uuid, owner_id: Uuid) -> DomainResult<License> {
        self.state.license_repo.find_by_id(id, owner_id).await?
            .ok_or(DomainError::LicenseNotFound)
    }
}

pub struct ListLicensesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListLicensesUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<License>, i64)>
    {
        self.state.license_repo.list(owner_id, page).await
    }
}

pub struct DeleteLicenseUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteLicenseUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()> {
        self.state.license_repo.delete(id, owner_id).await
    }
}

pub struct DeadlinesUseCase<'a> { pub state: &'a AppState }
impl<'a> DeadlinesUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(
        &self, owner_id: Uuid, from: NaiveDate, to: NaiveDate,
    ) -> DomainResult<Vec<License>> {
        self.state.license_repo.deadlines_in(owner_id, from, to).await
    }
}

/// Background job — invoked from a Tokio interval task in main.
pub struct ProcessExpiryNotificationsUseCase<'a> { pub state: &'a AppState }
impl<'a> ProcessExpiryNotificationsUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    pub async fn execute(&self) -> DomainResult<usize> {
        let mut total_sent = 0;
        for days in [90, 60, 45, 30, 14] {
            let candidates = self.state.license_repo
                .expiring_in_days_globally(days).await?;
            for lic in candidates {
                if !self.state.license_repo.mark_notified(lic.id(), days).await? {
                    continue;       // already notified
                }
                if let Some(user) = self.state.user_repo.find_by_id(lic.owner_id()).await? {
                    let body = format!(
                        "License {:?} ({}) expires in {} days at {}",
                        lic.kind(), lic.issuing_org(), days, lic.expires_at(),
                    );
                    if let Err(e) = self.state.mailer
                        .send_notification(user.email(), "License expiring", &body).await
                    {
                        tracing::warn!(error = ?e, "expiry mail failed");
                    }
                    total_sent += 1;
                }
            }
        }
        Ok(total_sent)
    }
}