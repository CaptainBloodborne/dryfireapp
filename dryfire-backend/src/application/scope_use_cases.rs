//! Scope adjustment use cases.

use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::scope::ScopeProfile,
        errors::{DomainError, DomainResult},
        services::scope::{
            ClicksRequest, ClicksResponse, ReZeroRequest, ReZeroResponse,
            compute_clicks, compute_rezero,
        },
    },
};

pub struct ComputeClicksUseCase;
impl ComputeClicksUseCase {
    pub fn execute(req: &ClicksRequest) -> DomainResult<ClicksResponse> {
        if req.distance_m <= 0.0 {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "distance_m must be positive".into())));
        }
        if req.click_value.fraction_of_unit <= 0.0 {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "click_value must be positive".into())));
        }
        Ok(compute_clicks(req))
    }
}

pub struct ReZeroUseCase;
impl ReZeroUseCase {
    pub fn execute(req: &ReZeroRequest) -> DomainResult<ReZeroResponse> {
        if req.current_zero_m <= 0.0 || req.desired_zero_m <= 0.0 {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "zero distances must be positive".into())));
        }
        Ok(compute_rezero(req))
    }
}

pub struct CreateScopeProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateScopeProfileUseCase<'a> {
    pub async fn execute(&self, p: ScopeProfile) -> DomainResult<ScopeProfile> {
        self.state.scope_profile_repo.create(&p).await?;
        Ok(p)
    }
}

pub struct ListScopeProfilesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListScopeProfilesUseCase<'a> {
    pub async fn execute(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<ScopeProfile>, i64)> {
        self.state.scope_profile_repo.list_for_user(user_id, limit, offset).await
    }
}

pub struct GetScopeProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> GetScopeProfileUseCase<'a> {
    pub async fn execute(&self, user_id: Uuid, id: Uuid) -> DomainResult<ScopeProfile> {
        self.state.scope_profile_repo.find(user_id, id).await?
            .ok_or(DomainError::ScopeProfileNotFound)
    }
}

pub struct UpdateScopeProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateScopeProfileUseCase<'a> {
    pub async fn execute(&self, p: ScopeProfile) -> DomainResult<ScopeProfile> {
        self.state.scope_profile_repo.update(&p).await?;
        Ok(p)
    }
}

pub struct DeleteScopeProfileUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteScopeProfileUseCase<'a> {
    pub async fn execute(&self, user_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.scope_profile_repo.delete(user_id, id).await
    }
}
