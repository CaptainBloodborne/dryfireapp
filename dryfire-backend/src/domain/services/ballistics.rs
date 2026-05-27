//! External point-mass ballistics solver.
//!
//! Model:
//! - **Point-mass projectile**: drag, gravity, wind. No spin drift, no
//!   Coriolis (negligible under ~1500 m for civilian shooting).
//! - **G1 drag function** (standard small-arms shape factor). The
//!   `BC` (ballistic coefficient) input is the G1 BC.
//! - **Atmospheric correction**: standard-atmosphere baseline at 15°C,
//!   1013.25 hPa, 0% humidity. Air density goes into the ODE directly,
//!   so any density change automatically affects drag.
//! - **Wind**: constant magnitude + direction over the trajectory.
//!   Direction follows the clock convention: 12 o'clock is downrange,
//!   3 o'clock is from the right.
//! - **Integrator**: classic RK4, fixed dt = 0.001 s. Stops when the
//!   bullet crosses the requested max distance or after ≤ 10 s of TOF
//!   (a fail-safe).
//!
//! Output rows are interpolated at user-supplied step distances.
//!
//! Units throughout the solver: **metric SI**.
//! - distance: m
//! - velocity: m/s
//! - weight (bullet): kg
//! - energy: J
//! - drop/drift: m (converted to cm and MOA/MIL in the response layer)
//!
//! ## On the drag formula
//!
//! The G1 ballistic coefficient encodes the bullet's drag deceleration
//! *relative to the standard G1 reference projectile* (a 1-lb, 1-inch
//! diameter shape). The deceleration of a bullet with BC `b` is:
//!
//!     a_drag = (0.5 · ρ · Cd(M) · A_ref / m_ref) / b · v · |v|
//!
//! where Cd(M) comes from the G1 drag table. The bullet's own area /
//! mass do **not** appear here — BC already encodes them, relative to
//! the reference. This is the source of a common implementation bug
//! (dividing by both the actual A/m and by BC, applying drag twice).

use serde::{Deserialize, Serialize};

// ---- physical constants ----------------------------------------- //

/// Gravitational acceleration, m/s².
const G: f64 = 9.80665;

/// G1 reference projectile: 1 lb mass.
const G1_REF_MASS_KG: f64 = 0.45359237;

/// G1 reference projectile: 1 inch diameter cross-section area, m².
const G1_REF_AREA_M2: f64 = std::f64::consts::PI * (0.0254 / 2.0) * (0.0254 / 2.0);

/// Pre-computed reference A/m factor used in the drag equation.
/// Roughly 1.118 × 10⁻³ m²/kg.
const G1_REF_FACTOR: f64 = G1_REF_AREA_M2 / G1_REF_MASS_KG;

// ---- inputs ----------------------------------------------------- //

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bullet {
    /// Diameter in millimetres. Informational — not used by the G1
    /// drag computation (BC absorbs it) but stored on profiles for
    /// display and future G7 / multi-BC support.
    pub caliber_mm: f64,
    /// Bullet mass in **grains** (1 grain = 64.79891 mg). Accepted in
    /// grains because every cartridge spec ships them; converted to
    /// kilograms internally and used only for energy calculation.
    pub weight_grain: f64,
    /// Muzzle velocity in m/s.
    pub muzzle_velocity_mps: f64,
    /// G1 ballistic coefficient.
    pub bc_g1: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sight {
    /// Height of optic axis above bore axis, mm.
    pub height_mm: f64,
    /// Zero distance in metres (the distance at which point-of-aim
    /// equals point-of-impact after the rifle is sighted in).
    pub zero_distance_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atmosphere {
    pub temperature_c: f64,
    /// Either pressure (hPa) or altitude (m). Use whichever is known;
    /// see [`Atmosphere::density`] for the precedence.
    pub pressure_hpa: Option<f64>,
    pub altitude_m: Option<f64>,
    /// Relative humidity, 0.0 to 1.0.
    pub humidity: f64,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            temperature_c: 15.0,
            pressure_hpa: Some(1013.25),
            altitude_m: None,
            humidity: 0.0,
        }
    }
}

impl Atmosphere {
    /// Air density at the current conditions, kg/m³.
    ///
    /// If pressure is given we use the ideal-gas relation with a
    /// humidity correction (saturated vapour subtracts from the
    /// effective dry-air partial pressure).
    /// Otherwise we approximate from altitude via the international
    /// standard atmosphere lapse rate.
    pub fn density(&self) -> f64 {
        let t_kelvin = self.temperature_c + 273.15;

        let pressure_pa = match (self.pressure_hpa, self.altitude_m) {
            (Some(p), _) => p * 100.0,
            (None, Some(h)) => {
                // ISA below 11 km: P = P0 · (1 - L·h / T0)^(g·M / R·L)
                let p0 = 101325.0;
                p0 * (1.0 - 0.0000225577 * h).powf(5.2559)
            }
            (None, None) => 101325.0,
        };

        // Saturation vapour pressure (Tetens), Pa.
        let sat_pa = 610.78
            * (17.27 * self.temperature_c / (self.temperature_c + 237.3)).exp();
        let vapour_pa = self.humidity.clamp(0.0, 1.0) * sat_pa;
        let dry_pa = pressure_pa - vapour_pa;

        // ρ = (p_dry / R_d · T) + (p_vapour / R_v · T)
        const R_DRY: f64 = 287.058;   // J/(kg·K)
        const R_VAPOUR: f64 = 461.495;
        dry_pa / (R_DRY * t_kelvin) + vapour_pa / (R_VAPOUR * t_kelvin)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wind {
    /// Speed in m/s.
    pub speed_mps: f64,
    /// Clock-position the wind is coming **from**: 12 = head-on,
    /// 3 = from shooter's right, 6 = tail, 9 = from left.
    pub from_clock: f64,
}

impl Wind {
    /// Returns (downrange component, crosswind component) in m/s.
    /// Downrange is positive for a headwind (slows the bullet).
    pub fn components(&self) -> (f64, f64) {
        let angle = (self.from_clock / 12.0) * std::f64::consts::TAU;
        let downrange = self.speed_mps * angle.cos();
        let cross = self.speed_mps * angle.sin();
        (downrange, cross)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryRequest {
    pub bullet: Bullet,
    pub sight: Sight,
    pub atmosphere: Atmosphere,
    pub wind: Wind,
    /// Distances at which to report. Must be positive and sorted; the
    /// solver linearly interpolates between integration steps.
    pub steps_m: Vec<f64>,
}

// ---- output ----------------------------------------------------- //

#[derive(Debug, Clone, Serialize)]
pub struct TrajectoryPoint {
    pub distance_m: f64,
    pub time_of_flight_s: f64,
    pub velocity_mps: f64,
    pub energy_j: f64,
    /// Vertical drop **from line of sight**, cm. Positive = below LOS
    /// (typical case past zero distance).
    pub drop_cm: f64,
    pub drift_cm: f64,
    pub elevation_moa: f64,
    pub windage_moa: f64,
    pub elevation_mil: f64,
    pub windage_mil: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Trajectory {
    pub points: Vec<TrajectoryPoint>,
}

// ---- solver ----------------------------------------------------- //

/// G1 drag curve — interpolation table of (Mach, Cd).
/// Standard published values; truncated to the speeds we care about.
const G1: &[(f64, f64)] = &[
    (0.00, 0.2629), (0.50, 0.2492), (0.70, 0.2462),
    (0.85, 0.2547), (0.90, 0.2728), (0.95, 0.3357),
    (1.00, 0.3950), (1.05, 0.4318), (1.10, 0.4423),
    (1.20, 0.4392), (1.40, 0.4220), (1.60, 0.4002),
    (1.80, 0.3792), (2.00, 0.3580), (2.50, 0.3210),
    (3.00, 0.2965), (4.00, 0.2640),
];

fn g1_cd(mach: f64) -> f64 {
    if mach <= G1[0].0 { return G1[0].1; }
    if mach >= G1[G1.len() - 1].0 { return G1[G1.len() - 1].1; }
    let i = G1.windows(2).position(|w| w[1].0 >= mach).unwrap();
    let (m0, c0) = G1[i];
    let (m1, c1) = G1[i + 1];
    let t = (mach - m0) / (m1 - m0);
    c0 + t * (c1 - c0)
}

fn speed_of_sound(temp_c: f64) -> f64 {
    20.05 * (temp_c + 273.15).sqrt()
}

/// Compute a trajectory.
///
/// Returns interpolated [`TrajectoryPoint`]s at each requested distance
/// (skipping any beyond the trajectory's reach, e.g. if velocity dies
/// before then).
pub fn solve(req: &TrajectoryRequest) -> Trajectory {
    // ---- unit conversions ---------------------------------------- //
    // Bullet mass is used for energy only; the drag computation uses
    // the G1 reference projectile's A/m, scaled by 1/BC.
    let mass_kg = req.bullet.weight_grain * 6.479_891e-5;
    let v0 = req.bullet.muzzle_velocity_mps;
    let bc = req.bullet.bc_g1.max(0.01);

    let rho = req.atmosphere.density();
    let sound = speed_of_sound(req.atmosphere.temperature_c);

    let (wind_dr, wind_cr) = req.wind.components();

    // ---- determine launch angle so we cross LOS at zero distance --- //
    // The optic sits sight.height_mm above the bore. The bullet starts
    // at y = -sight_height, flies a parabola, and must reach y = 0 at
    // x = zero_distance.
    let sight_height = req.sight.height_mm * 1e-3;
    let launch_angle = find_launch_angle(
        v0, sight_height, req.sight.zero_distance_m,
        bc, rho, sound,
    );

    // ---- state --------------------------------------------------- //
    // Position: x = downrange (m), y = vertical (m), z = lateral (m).
    // Velocity is decomposed from v0 so that √(vx² + vy²) = v0 at
    // the muzzle (preserves total speed, unlike taking vx = v0 and
    // adding a small vy component).
    let mut x = 0.0_f64;
    let mut y = -sight_height;
    let mut z = 0.0_f64;
    let mut vx = v0 * launch_angle.cos();
    let mut vy = v0 * launch_angle.sin();
    let mut vz = 0.0_f64;
    let mut t = 0.0_f64;

    // ---- record breakpoints ------------------------------------- //
    let mut traj_raw: Vec<TrajectoryStep> = Vec::with_capacity(8000);
    traj_raw.push(TrajectoryStep { t, x, y, z, v: v0 });

    let dt = 0.001;
    let max_distance = req.steps_m.last().copied().unwrap_or(1000.0);
    let safety_max_x = max_distance + 50.0;
    let safety_max_t = 10.0;

    while x < safety_max_x && t < safety_max_t {
        // RK4
        let k1 = derivs(vx, vy, vz, bc, rho, sound, wind_dr, wind_cr);
        let k2 = derivs(
            vx + 0.5 * dt * k1.ax, vy + 0.5 * dt * k1.ay, vz + 0.5 * dt * k1.az,
            bc, rho, sound, wind_dr, wind_cr,
        );
        let k3 = derivs(
            vx + 0.5 * dt * k2.ax, vy + 0.5 * dt * k2.ay, vz + 0.5 * dt * k2.az,
            bc, rho, sound, wind_dr, wind_cr,
        );
        let k4 = derivs(
            vx + dt * k3.ax, vy + dt * k3.ay, vz + dt * k3.az,
            bc, rho, sound, wind_dr, wind_cr,
        );

        let ax = (k1.ax + 2.0 * k2.ax + 2.0 * k3.ax + k4.ax) / 6.0;
        let ay = (k1.ay + 2.0 * k2.ay + 2.0 * k3.ay + k4.ay) / 6.0;
        let az = (k1.az + 2.0 * k2.az + 2.0 * k3.az + k4.az) / 6.0;

        // position uses the average velocity over dt (trapezoid)
        x += (vx + 0.5 * ax * dt) * dt;
        y += (vy + 0.5 * ay * dt) * dt;
        z += (vz + 0.5 * az * dt) * dt;
        vx += ax * dt;
        vy += ay * dt;
        vz += az * dt;
        t += dt;

        let v = (vx * vx + vy * vy + vz * vz).sqrt();
        traj_raw.push(TrajectoryStep { t, x, y, z, v });

        if v < 30.0 { break; } // bullet is essentially spent
    }

    // ---- interpolate at requested distances --------------------- //
    let points: Vec<TrajectoryPoint> = req.steps_m.iter()
        .filter_map(|&d| interpolate_at(&traj_raw, d, mass_kg))
        .collect();

    Trajectory { points }
}

#[derive(Clone, Copy)]
struct TrajectoryStep {
    t: f64,
    x: f64, y: f64, z: f64,
    v: f64,
}

struct Accel { ax: f64, ay: f64, az: f64 }

/// Acceleration on the bullet from drag + gravity + wind.
///
/// Standard G1 form: drag deceleration is the G1-reference projectile's
/// drag (Cd · ρ · A_ref / m_ref) divided by BC. The actual bullet's
/// caliber and mass do not enter the drag equation — they're encoded
/// inside the BC.
fn derivs(
    vx: f64, vy: f64, vz: f64,
    bc: f64, rho: f64, sound: f64,
    wind_dr: f64, wind_cr: f64,
) -> Accel {
    // Velocity relative to air.
    let vrx = vx - wind_dr;
    let vrz = vz - wind_cr;
    let vry = vy;
    let vrel = (vrx * vrx + vry * vry + vrz * vrz).sqrt();
    let mach = vrel / sound;
    let cd = g1_cd(mach);

    let drag_factor = 0.5 * rho * cd * G1_REF_FACTOR / bc;
    Accel {
        ax: -drag_factor * vrel * vrx,
        ay: -drag_factor * vrel * vry - G,
        az: -drag_factor * vrel * vrz,
    }
}

/// Linearly interpolate the trajectory state at downrange distance `d`.
fn interpolate_at(traj: &[TrajectoryStep], d: f64, mass: f64) -> Option<TrajectoryPoint> {
    if d <= 0.0 || traj.last()?.x < d {
        return None;
    }
    let i = traj.windows(2).position(|w| w[1].x >= d)?;
    let a = traj[i];
    let b = traj[i + 1];
    let f = (d - a.x) / (b.x - a.x);
    let y = a.y + f * (b.y - a.y);
    let z = a.z + f * (b.z - a.z);
    let v = a.v + f * (b.v - a.v);
    let t = a.t + f * (b.t - a.t);

    // y is the bullet's height relative to bore axis at the muzzle.
    // We accounted for sight height by starting the bullet at y = -h,
    // so a positive drop_m means the bullet is below the line of sight.
    let drop_m = -y;
    let drift_m = z;

    let drop_cm = drop_m * 100.0;
    let drift_cm = drift_m * 100.0;

    // 1 MOA at distance d (m) covers d · tan(1/60°) m ≈ d · 2.908882e-4 m.
    let one_moa_m = d * (1.0_f64 / 60.0).to_radians().tan();
    // 1 mil = 0.001 rad → d / 1000 metres.
    let one_mil_m = d * 1e-3;

    let elevation_moa = drop_m / one_moa_m;
    let windage_moa = drift_m / one_moa_m;
    let elevation_mil = drop_m / one_mil_m;
    let windage_mil = drift_m / one_mil_m;

    Some(TrajectoryPoint {
        distance_m: d,
        time_of_flight_s: t,
        velocity_mps: v,
        energy_j: 0.5 * mass * v * v,
        drop_cm,
        drift_cm,
        elevation_moa,
        windage_moa,
        elevation_mil,
        windage_mil,
    })
}

/// Find the launch angle (in radians) so the bullet crosses the line
/// of sight at the zero distance. Bisection search; converges to
/// micro-radian precision in 40 iterations.
fn find_launch_angle(
    v0: f64, sight_height: f64, zero_d: f64,
    bc: f64, rho: f64, sound: f64,
) -> f64 {
    let mut low: f64 = 0.0;
    let mut high: f64 = 0.05; // ~3°, enough for any civilian shot

    for _ in 0..40 {
        let mid = 0.5 * (low + high);
        let vx0 = v0 * mid.cos();
        let vy0 = v0 * mid.sin();
        let drop = simulate_drop_at(vx0, vy0, sight_height, zero_d, bc, rho, sound);
        // drop > 0 means bullet was above LOS at d → angle too high
        if drop > 0.0 { high = mid; } else { low = mid; }
    }
    0.5 * (low + high)
}

fn simulate_drop_at(
    vx0: f64, vy0: f64, sight_height: f64, target_x: f64,
    bc: f64, rho: f64, sound: f64,
) -> f64 {
    let mut x = 0.0_f64;
    let mut y = -sight_height;
    let mut vx = vx0;
    let mut vy = vy0;
    let dt = 0.002;
    let mut t = 0.0_f64;
    while x < target_x && t < 5.0 {
        let a = derivs(vx, vy, 0.0, bc, rho, sound, 0.0, 0.0);
        x += (vx + 0.5 * a.ax * dt) * dt;
        y += (vy + 0.5 * a.ay * dt) * dt;
        vx += a.ax * dt;
        vy += a.ay * dt;
        t += dt;
    }
    y
}

// ---- CSV serialiser --------------------------------------------- //

pub fn trajectory_to_csv(traj: &Trajectory) -> String {
    let mut out = String::with_capacity(traj.points.len() * 80);
    out.push_str("distance_m,time_s,velocity_mps,energy_j,drop_cm,drift_cm,elev_moa,wind_moa,elev_mil,wind_mil\n");
    for p in &traj.points {
        use std::fmt::Write;
        let _ = writeln!(
            out,
            "{:.1},{:.4},{:.2},{:.1},{:.2},{:.2},{:.3},{:.3},{:.3},{:.3}",
            p.distance_m, p.time_of_flight_s, p.velocity_mps, p.energy_j,
            p.drop_cm, p.drift_cm,
            p.elevation_moa, p.windage_moa, p.elevation_mil, p.windage_mil,
        );
    }
    out
}

// ---- tests ------------------------------------------------------ //

#[cfg(test)]
mod tests {
    use super::*;

    fn req_308_at_zero() -> TrajectoryRequest {
        // .308 Win, 175 gr SMK, 800 m/s muzzle, BC 0.505 G1, 5 cm scope,
        // 100 m zero, standard atmosphere, no wind.
        TrajectoryRequest {
            bullet: Bullet {
                caliber_mm: 7.82,
                weight_grain: 175.0,
                muzzle_velocity_mps: 800.0,
                bc_g1: 0.505,
            },
            sight: Sight { height_mm: 50.0, zero_distance_m: 100.0 },
            atmosphere: Atmosphere::default(),
            wind: Wind { speed_mps: 0.0, from_clock: 12.0 },
            steps_m: vec![0.0, 100.0, 200.0, 500.0],
        }
    }

    #[test]
    fn zero_is_on_target_at_100m() {
        let traj = solve(&req_308_at_zero());
        let p100 = traj.points.iter().find(|p| (p.distance_m - 100.0).abs() < 0.1).unwrap();
        // Solver converged: drop at zero distance should be within
        // ±1 cm of 0 (sub-MOA-bracket precision is more than adequate).
        assert!(p100.drop_cm.abs() < 1.0, "drop at 100 m = {:.2} cm", p100.drop_cm);
    }

    #[test]
    fn bullet_drops_past_zero() {
        let traj = solve(&req_308_at_zero());
        let p500 = traj.points.iter().find(|p| (p.distance_m - 500.0).abs() < 0.1).unwrap();
        // A 175 gr .308 at 800 m/s, 100 m zero, drops ~130-160 cm at
        // 500 m. We give wide tolerance because this is point-mass G1
        // not a Pejsa/Litz fit.
        assert!(p500.drop_cm > 80.0 && p500.drop_cm < 220.0,
                "drop at 500 m = {:.1} cm", p500.drop_cm);
        // Velocity should drop noticeably.
        assert!(p500.velocity_mps < 700.0, "v = {:.0} m/s", p500.velocity_mps);
    }

    #[test]
    fn higher_air_density_means_more_drop() {
        let mut hot = req_308_at_zero();
        hot.atmosphere.temperature_c = 35.0;

        let mut cold = req_308_at_zero();
        cold.atmosphere.temperature_c = -20.0;

        let drop_hot = solve(&hot).points.iter()
            .find(|p| (p.distance_m - 500.0).abs() < 0.1).unwrap().drop_cm;
        let drop_cold = solve(&cold).points.iter()
            .find(|p| (p.distance_m - 500.0).abs() < 0.1).unwrap().drop_cm;

        // Colder air → denser → more drag → more drop.
        assert!(drop_cold > drop_hot, "cold drop {} should exceed hot drop {}",
                drop_cold, drop_hot);
    }

    #[test]
    fn wind_pushes_bullet() {
        let mut windy = req_308_at_zero();
        windy.wind = Wind { speed_mps: 5.0, from_clock: 3.0 };
        let t = solve(&windy);
        let p500 = t.points.iter()
            .find(|p| (p.distance_m - 500.0).abs() < 0.1).unwrap();
        // Wind from 3 o'clock pushes left → drift should be negative.
        // (z increases to shooter's left because wind cross component
        // pushes from right.) Sign convention here just needs to be
        // consistent.
        assert!(p500.drift_cm.abs() > 5.0, "drift = {:.2} cm", p500.drift_cm);
    }

    #[test]
    fn csv_starts_with_header() {
        let traj = solve(&req_308_at_zero());
        let csv = trajectory_to_csv(&traj);
        assert!(csv.starts_with("distance_m,time_s,"));
        assert!(csv.lines().count() == traj.points.len() + 1);
    }

    #[test]
    fn moa_and_mil_consistent() {
        let traj = solve(&req_308_at_zero());
        let p500 = traj.points.iter()
            .find(|p| (p.distance_m - 500.0).abs() < 0.1).unwrap();
        // 1 MOA ≈ 0.2909 mil. drop_moa * 0.2909 should ≈ drop_mil.
        let ratio = p500.elevation_mil / p500.elevation_moa;
        assert!((ratio - 0.2909).abs() < 0.001, "ratio {:.4}", ratio);
    }
}