//! Use cases for the ballistics domain.
//!
//! The actual math lives in `domain::services::ballistics`; this layer
//! just orchestrates: validate input, load profile (if requested),
//! invoke solver, persist if asked. Pure-math functions are easy to
//! test in isolation, so most error paths here are persistence-related.

use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::ballistics::BallisticProfile,
        errors::{DomainError, DomainResult},
        services::ballistics::{Trajectory, TrajectoryRequest, solve},
    },
};

// compute ad-hoc trajectory

pub struct ComputeTrajectoryUseCase;

impl ComputeTrajectoryUseCase {
    pub fn execute(req: &TrajectoryRequest) -> DomainResult<Trajectory> {
        if req.bullet.muzzle_velocity_mps <= 0.0 {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "muzzle_velocity_mps must be positive".into())));
        }
        if req.bullet.bc_g1 <= 0.0 {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "bc_g1 must be positive".into())));
        }
        if req.sight.zero_distance_m <= 0.0 {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "zero_distance_m must be positive".into())));
        }
        if req.steps_m.is_empty() {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "steps_m must contain at least one distance".into())));
        }
        Ok(solve(req))
    }
}

// CRUD on profiles

pub struct CreateBallisticProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateBallisticProfileUseCase<'a> {
    pub async fn execute(&self, p: BallisticProfile) -> DomainResult<BallisticProfile> {
        self.state.ballistic_profile_repo.create(&p).await?;
        Ok(p)
    }
}

pub struct GetBallisticProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> GetBallisticProfileUseCase<'a> {
    pub async fn execute(&self, user_id: Uuid, id: Uuid) -> DomainResult<BallisticProfile> {
        self.state.ballistic_profile_repo.find(user_id, id).await?
            .ok_or(DomainError::BallisticProfileNotFound)
    }
}

pub struct ListBallisticProfilesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListBallisticProfilesUseCase<'a> {
    pub async fn execute(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<BallisticProfile>, i64)> {
        self.state.ballistic_profile_repo.list_for_user(user_id, limit, offset).await
    }
}

pub struct UpdateBallisticProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateBallisticProfileUseCase<'a> {
    pub async fn execute(&self, p: BallisticProfile) -> DomainResult<BallisticProfile> {
        self.state.ballistic_profile_repo.update(&p).await?;
        Ok(p)
    }
}

pub struct DeleteBallisticProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteBallisticProfileUseCase<'a> {
    pub async fn execute(&self, user_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.ballistic_profile_repo.delete(user_id, id).await
    }
}
