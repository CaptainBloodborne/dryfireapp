use crate::domain::{
    entities::scope::{ZeroRequest, ZeroResponse},
    errors::{DomainError, DomainResult},
    services::scope::ScopeAdjuster,
};

pub struct ArithmeticScopeAdjuster;

impl ScopeAdjuster for ArithmeticScopeAdjuster {
    fn compute_zero(&self, req: &ZeroRequest) -> DomainResult<ZeroResponse> {
        if req.distance_m <= 0.0 {
            return Err(DomainError::BallisticInput("distance_m must be > 0".into()));
        }
        if req.click_value <= 0.0 {
            return Err(DomainError::BallisticInput("click_value must be > 0".into()));
        }

        // 1 unit (MOA/MIL) covers `unit_at_range` cm at the request's distance.
        let unit_cm_at_range = req.unit.metres_at_one_metre() * req.distance_m * 100.0;
        let one_click_cm = unit_cm_at_range * req.click_value;

        // To bring POI to zero we move it by the *opposite* of the offset.
        let elevation_units_raw = -req.vertical_cm / unit_cm_at_range;
        let windage_units_raw   = -req.horizontal_cm / unit_cm_at_range;

        let elev_clicks = (-req.vertical_cm / one_click_cm).round() as i32;
        let wind_clicks = (-req.horizontal_cm / one_click_cm).round() as i32;

        Ok(ZeroResponse {
            elevation_clicks: elev_clicks,
            windage_clicks: wind_clicks,
            elevation_units: elevation_units_raw,
            windage_units: windage_units_raw,
        })
    }
}