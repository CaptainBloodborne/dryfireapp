// src/application/ballistics_use_cases.rs

use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::ballistics::{
            AdjustmentUnit, BallisticInput, BallisticProfile, Environment,
            TrajectoryPoint,
        },
        errors::{DomainError, DomainResult},
        repositories::armory::PageQuery,
    },
};

#[derive(Debug)]
pub struct ComputeTrajectoryInput {
    pub input: BallisticInput,
    pub env: Environment,
    pub unit: AdjustmentUnit,
    pub step_m: f64,
    pub max_range_m: f64,
}

pub struct ComputeTrajectoryUseCase<'a> { pub state: &'a AppState }
impl<'a> ComputeTrajectoryUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, i: ComputeTrajectoryInput)
        -> DomainResult<Vec<TrajectoryPoint>>
    {
        // Pure CPU — could be moved to spawn_blocking if it gets heavy.
        self.state.ballistic_calculator
            .compute(&i.input, &i.env, i.unit, i.step_m, i.max_range_m)
    }
}

#[derive(Debug)]
pub struct SaveBallisticProfileInput {
    pub owner_id: Uuid,
    pub name: String,
    pub gun_id: Option<Uuid>,
    pub lot_id: Option<Uuid>,
    pub input: BallisticInput,
}

pub struct SaveBallisticProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> SaveBallisticProfileUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, i: SaveBallisticProfileInput) -> DomainResult<Uuid> {
        let profile = BallisticProfile {
            id: Uuid::new_v4(), owner_id: i.owner_id, name: i.name,
            gun_id: i.gun_id, lot_id: i.lot_id, input: i.input,
        };
        self.state.ballistic_profile_repo.create(&profile).await?;
        Ok(profile.id)
    }
}

pub struct ListBallisticProfilesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListBallisticProfilesUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<BallisticProfile>, i64)>
    {
        self.state.ballistic_profile_repo.list(owner_id, page).await
    }
}

pub struct GetBallisticProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> GetBallisticProfileUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, id: Uuid, owner_id: Uuid) -> DomainResult<BallisticProfile> {
        self.state.ballistic_profile_repo.find_by_id(id, owner_id).await?
            .ok_or(DomainError::BallisticProfileNotFound)
    }
}