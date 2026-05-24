// src/application/scope_use_cases.rs

use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::{
            ballistics::AdjustmentUnit,
            scope::{ScopeProfile, ZeroRequest, ZeroResponse},
        },
        errors::{DomainError, DomainResult},
        repositories::armory::PageQuery,
    },
};

pub struct ComputeZeroUseCase<'a> { pub state: &'a AppState }
impl<'a> ComputeZeroUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, req: ZeroRequest) -> DomainResult<ZeroResponse> {
        self.state.scope_adjuster.compute_zero(&req)
    }
}

#[derive(Debug)]
pub struct SaveScopeProfileInput {
    pub owner_id: Uuid,
    pub gun_id: Option<Uuid>,
    pub name: String,
    pub unit: AdjustmentUnit,
    pub click_value: f64,
    pub elevation_max_clicks: Option<i32>,
    pub windage_max_clicks: Option<i32>,
    pub mount_height_mm: f64,
}

pub struct SaveScopeProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> SaveScopeProfileUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, i: SaveScopeProfileInput) -> DomainResult<Uuid> {
        let p = ScopeProfile {
            id: Uuid::new_v4(),
            owner_id: i.owner_id, gun_id: i.gun_id,
            name: i.name, unit: i.unit, click_value: i.click_value,
            elevation_max_clicks: i.elevation_max_clicks,
            windage_max_clicks: i.windage_max_clicks,
            mount_height_mm: i.mount_height_mm,
        };
        self.state.scope_profile_repo.create(&p).await?;
        Ok(p.id)
    }
}

pub struct ListScopeProfilesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListScopeProfilesUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<ScopeProfile>, i64)>
    {
        self.state.scope_profile_repo.list(owner_id, page).await
    }
}

pub struct GetScopeProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> GetScopeProfileUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, id: Uuid, owner_id: Uuid) -> DomainResult<ScopeProfile> {
        self.state.scope_profile_repo.find_by_id(id, owner_id).await?
            .ok_or(DomainError::ScopeProfileNotFound)
    }
}