use crate::domain::{
    entities::scope::{ZeroRequest, ZeroResponse},
    errors::DomainResult,
};

pub trait ScopeAdjuster: Send + Sync {
    fn compute_zero(&self, req: &ZeroRequest) -> DomainResult<ZeroResponse>;
}