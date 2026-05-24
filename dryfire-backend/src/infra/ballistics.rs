// src/infra/ballistics.rs

use crate::domain::{
    entities::ballistics::{
        AdjustmentUnit, BallisticInput, Environment, TrajectoryPoint,
    },
    errors::{DomainError, DomainResult},
    services::ballistics::BallisticCalculator,
};

pub struct SimpleG1Calculator;

const GRAIN_TO_KG: f64 = 6.479_891e-5;
const STD_AIR_DENSITY: f64 = 1.225;    // kg/m³ at 15°C, 1013.25 hPa
const G: f64 = 9.80665;

impl SimpleG1Calculator {
    /// Density-of-air correction from environment.
    fn air_density(env: &Environment) -> f64 {
        let t_c = env.temperature_c.unwrap_or(15.0);
        let p_hpa = env.pressure_hpa.unwrap_or(1013.25);
        let p_pa = p_hpa * 100.0;
        let t_k = t_c + 273.15;
        // Ideal-gas: ρ = p / (R_specific_dry_air · T)
        const R: f64 = 287.058;
        p_pa / (R * t_k)
    }
}

impl BallisticCalculator for SimpleG1Calculator {
    fn compute(
        &self,
        input: &BallisticInput,
        env: &Environment,
        unit: AdjustmentUnit,
        step_m: f64,
        max_range_m: f64,
    ) -> DomainResult<Vec<TrajectoryPoint>> {
        if input.muzzle_velocity_mps <= 0.0 {
            return Err(DomainError::BallisticInput("muzzle_velocity must be > 0".into()));
        }
        if input.ballistic_coefficient <= 0.0 {
            return Err(DomainError::BallisticInput("bc must be > 0".into()));
        }
        if step_m <= 0.0 || max_range_m <= 0.0 || max_range_m < step_m {
            return Err(DomainError::BallisticInput("bad distance step/range".into()));
        }

        let m = input.bullet_weight_grains * GRAIN_TO_KG;
        let bc = input.ballistic_coefficient;
        let rho = Self::air_density(env);
        let rho_ratio = rho / STD_AIR_DENSITY;

        let wind_mps = env.wind_speed_mps.unwrap_or(0.0);
        let wind_dir = env.wind_direction_deg.unwrap_or(90.0).to_radians();
        let wind_x = wind_mps * wind_dir.sin();   // crosswind, positive = right

        // initial conditions, gun at origin; line-of-sight along +x.
        // sight is "sight_height_mm" above bore, so to be zeroed at
        // zero_distance_m the barrel must tilt up by `theta`.
        // We find theta with a quick bisection on the no-air case.
        let sight_h = input.sight_height_mm * 1e-3;
        let theta = solve_zero_angle(
            input.muzzle_velocity_mps, sight_h, input.zero_distance_m,
        );

        let dt = 1e-3_f64;
        let mut vx = input.muzzle_velocity_mps * theta.cos();
        let mut vy = input.muzzle_velocity_mps * theta.sin();
        let mut x = 0.0_f64;
        let mut y = -sight_h;        // LOS sits sight_h above bore at origin
        let mut z = 0.0_f64;          // crosswind drift
        let mut t = 0.0_f64;

        let mut next_sample_at = step_m;
        let mut out = Vec::with_capacity((max_range_m / step_m) as usize + 1);

        while x <= max_range_m + step_m {
            let v_rel_x = vx - 0.0;          // headwind ignored for simplicity
            let v_rel_z = 0.0 - wind_x;
            let v_rel = (v_rel_x * v_rel_x + vy * vy + v_rel_z * v_rel_z).sqrt();

            // G1-ish drag: F = (rho_ratio / bc) · k · v²   — simplified
            // We treat (rho_ratio / bc) as a drag coefficient scaler.
            let cd = 0.5 * rho_ratio / bc;
            let ax = -cd * v_rel * v_rel_x / m * 1e-4;
            let ay = -G - cd * v_rel * vy / m * 1e-4;
            let az = -cd * v_rel * v_rel_z / m * 1e-4;

            vx += ax * dt;
            vy += ay * dt;
            let vz_step = az * dt;

            x += vx * dt;
            y += vy * dt;
            z += (vz_step) * dt;
            t += dt;

            if x >= next_sample_at {
                let drop_cm = y * 100.0;
                let drift_cm = z * 100.0;
                let v = (vx * vx + vy * vy).sqrt();
                let energy_j = 0.5 * m * v * v;

                // unit-aware corrections
                let scale = unit.metres_at_one_metre() * x; // m per "1 unit" at range x
                let elevation_units = if scale > 0.0 { -y / scale } else { 0.0 };
                let windage_units   = if scale > 0.0 { -z / scale } else { 0.0 };

                out.push(TrajectoryPoint {
                    distance_m: next_sample_at,
                    drop_cm, drift_cm,
                    velocity_mps: v,
                    energy_j, time_s: t,
                    elevation_units, windage_units,
                });
                next_sample_at += step_m;
            }
            if vy < -200.0 || t > 30.0 { break; }
        }
        Ok(out)
    }
}

fn solve_zero_angle(v0: f64, sight_h: f64, zero_m: f64) -> f64 {
    // vacuum closed-form: y(zero_m) = -sight_h + zero_m*tan(θ) - g*zero_m²/(2 v0² cos²θ)
    // Solve y = 0 for small θ ⇒ θ ≈ (g·zero_m)/(2 v0²) + sight_h/zero_m
    (G * zero_m) / (2.0 * v0 * v0) + sight_h / zero_m
}