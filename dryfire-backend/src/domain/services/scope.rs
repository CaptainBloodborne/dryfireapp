//! Scope adjustment calculations.
//!
//! Two operations:
//!
//! 1. **Click-calculator**: given a measured POI offset (in cm at a
//!    known distance), how many clicks to dial?
//! 2. **Re-zero**: given a profile and a current zero distance, how
//!    many clicks to move to a new zero distance? Done by running the
//!    ballistic solver at both distances and converting the diff.
//!
//! Two MOA conventions exist:
//! - **True MOA**: 1/60 of a degree - 1.047″ at 100 yd.
//! - **Shooter's MOA / IPHY**: arbitrarily 1″ at 100 yd.

use serde::{Deserialize, Serialize};

use crate::domain::services::ballistics::{Trajectory, TrajectoryRequest, solve};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AdjustmentUnit {
    /// True MOA — 1/60°, 1.047″ at 100 yd.
    Moa,
    /// Shooter's MOA — exactly 1″ at 100 yd. Specify only when your
    /// optic actually behaves this way.
    Iphy,
    /// Mil-radian: 0.001 radians, 1 cm at 10 m, exact.
    Mil,
}

impl AdjustmentUnit {
    /// Linear size of one unit at the given distance, in metres.
    pub fn unit_size_m(&self, distance_m: f64) -> f64 {
        match self {
            AdjustmentUnit::Moa => distance_m * (1.0_f64 / 60.0).to_radians().tan(),
            AdjustmentUnit::Iphy => {
                // Exactly 1 inch per 100 yards by definition.
                // 1 inch = 0.0254 m, 100 yd = 91.44 m.
                distance_m * (0.0254 / 91.44)
            }
            AdjustmentUnit::Mil => distance_m * 1e-3,
        }
    }
}

/// Click-value as a fraction of an adjustment unit, e.g. 1/4 MOA = 0.25,
/// 0.1 MIL = 0.1.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClickValue {
    pub fraction_of_unit: f64,
}

impl ClickValue {
    /// Linear movement per click at `distance_m`, in cm.
    pub fn linear_cm_per_click(&self, unit: AdjustmentUnit, distance_m: f64) -> f64 {
        unit.unit_size_m(distance_m) * self.fraction_of_unit * 100.0
    }
}

// Click calculator

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClicksRequest {
    /// Observed POI relative to point of aim. Positive vertical = high,
    /// positive horizontal = right. The "click" computation returns
    /// what to dial to bring POI to POA, i.e. the *opposite* sign.
    pub poi_offset_v_cm: f64,
    pub poi_offset_h_cm: f64,
    pub distance_m: f64,
    pub unit: AdjustmentUnit,
    pub click_value: ClickValue,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClicksResponse {
    pub clicks_v: i32, // positive = dial up
    pub clicks_h: i32, // positive = dial right
    pub residual_v_cm: f64,
    pub residual_h_cm: f64,
}

pub fn compute_clicks(req: &ClicksRequest) -> ClicksResponse {
    let cm_per_click = req.click_value.linear_cm_per_click(req.unit, req.distance_m);
    // To bring POI to POA, dial *opposite* of POI offset.
    let v_clicks_exact = -req.poi_offset_v_cm / cm_per_click;
    let h_clicks_exact = -req.poi_offset_h_cm / cm_per_click;
    let v_clicks = v_clicks_exact.round() as i32;
    let h_clicks = h_clicks_exact.round() as i32;
    ClicksResponse {
        clicks_v: v_clicks,
        clicks_h: h_clicks,
        residual_v_cm: req.poi_offset_v_cm + (v_clicks as f64) * cm_per_click,
        residual_h_cm: req.poi_offset_h_cm + (h_clicks as f64) * cm_per_click,
    }
}

// Re-zero between distances

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReZeroRequest {
    /// Source ballistic state (used as-is — the solver will be invoked
    /// twice, with `sight.zero_distance_m` set to current/desired).
    pub base_request: TrajectoryRequest,
    pub current_zero_m: f64,
    pub desired_zero_m: f64,
    pub unit: AdjustmentUnit,
    pub click_value: ClickValue,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReZeroResponse {
    pub clicks_v: i32,
    pub elevation_change_cm: f64,
}

/// Compute the vertical-click delta to move the zero from
/// `current_zero_m` to `desired_zero_m`. (Horizontal change is 0
/// barring wind, which we ignore for this purpose.)
pub fn compute_rezero(req: &ReZeroRequest) -> ReZeroResponse {
    let mut cur = req.base_request.clone();
    cur.sight.zero_distance_m = req.current_zero_m;
    cur.steps_m = vec![req.desired_zero_m];
    let cur_traj: Trajectory = solve(&cur);
    let cur_drop = cur_traj.points.first().map(|p| p.drop_cm).unwrap_or(0.0);

    // After re-zeroing at `desired_zero_m`, drop at that distance is 0
    // by definition. So the elevation change is `cur_drop` cm (we need
    // to dial *down* by that much, hence the sign).
    let cm_per_click = req.click_value.linear_cm_per_click(req.unit, req.desired_zero_m);
    let clicks_v = (-cur_drop / cm_per_click).round() as i32;
    ReZeroResponse { clicks_v, elevation_change_cm: -cur_drop }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_moa_at_100m_is_about_07cm() {
        let click = ClickValue { fraction_of_unit: 0.25 };
        let cm = click.linear_cm_per_click(AdjustmentUnit::Moa, 100.0);
        // 1 true MOA at 100 m ≈ 2.908 cm - 1/4 ≈ 0.727 cm
        assert!((cm - 0.727).abs() < 0.01, "got {} cm", cm);
    }

    #[test]
    fn tenth_mil_at_100m_is_1cm() {
        let click = ClickValue { fraction_of_unit: 0.1 };
        let cm = click.linear_cm_per_click(AdjustmentUnit::Mil, 100.0);
        // 1 mil at 100 m = 10 cm - 0.1 mil = 1 cm
        assert!((cm - 1.0).abs() < 1e-9, "got {} cm", cm);
    }

    #[test]
    fn iphy_differs_from_moa_at_long_range() {
        let cm_moa = AdjustmentUnit::Moa.unit_size_m(1000.0) * 100.0;
        let cm_iphy = AdjustmentUnit::Iphy.unit_size_m(1000.0) * 100.0;
        // True MOA is 1.047 * shooter MOA. At 1000 m the difference is
        // ~1.4 cm — well above any click value, so absolutely matters.
        assert!(cm_moa > cm_iphy);
        assert!((cm_moa / cm_iphy - 1.047197).abs() < 1e-3);
    }

    #[test]
    fn poi_high_means_dial_down() {
        let req = ClicksRequest {
            poi_offset_v_cm: 3.0,   // hit 3 cm high
            poi_offset_h_cm: 0.0,
            distance_m: 100.0,
            unit: AdjustmentUnit::Mil,
            click_value: ClickValue { fraction_of_unit: 0.1 }, // 1 cm/click
        };
        let resp = compute_clicks(&req);
        // 3 cm high - dial 3 clicks down - -3.
        assert_eq!(resp.clicks_v, -3);
        assert!(resp.residual_v_cm.abs() < 1e-9);
    }

    #[test]
    fn round_to_nearest_click() {
        let req = ClicksRequest {
            poi_offset_v_cm: 0.4,   // very close to one 0.1-mil click
            poi_offset_h_cm: 0.0,
            distance_m: 100.0,
            unit: AdjustmentUnit::Mil,
            click_value: ClickValue { fraction_of_unit: 0.1 },
        };
        let resp = compute_clicks(&req);
        assert_eq!(resp.clicks_v, 0);   // 0.4 cm rounds to 0 clicks
        assert!((resp.residual_v_cm - 0.4).abs() < 1e-9);
    }
}
