use crate::domain::{
    entities::ballistics::{
        AdjustmentUnit, BallisticInput, Environment, TrajectoryPoint,
    },
    errors::DomainResult,
};

pub trait BallisticCalculator: Send + Sync {
    fn compute(
        &self,
        input: &BallisticInput,
        env: &Environment,
        unit: AdjustmentUnit,
        step_m: f64,
        max_range_m: f64,
    ) -> DomainResult<Vec<TrajectoryPoint>>;
}