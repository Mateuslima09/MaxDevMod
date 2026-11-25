//! Phase 1: Real-time Truck & Trailer Dirt/Mud Simulation Engine (pure CPU telemetry, zero GPU impact).
//!
//! Tracks independent surface-driven dirt accumulation (`dust`, `mud`, `grime`, `wetness`) for the player's
//! tractor and up to 4 coupled trailers using official SCS Telemetry SDK channels:
//! - `SCS_TELEMETRY_CONFIG_substances` -> maps each numeric substance index to `SurfaceKind`
//! - `truck.wheel.substance[i]` & `trailer.wheel.substance[i]` -> exact road surface under each wheel
//! - `truck.speed`, `paused`, `trailer.connected`, and rain/wetness state.
//!
//! Includes rate-limited diagnostic logging and unit tests so Phase 1 can be verified in-game
//! before Phase 2 adds the D3D11 deferred 3D projection pass.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use crate::log;

/// Maximum coupled trailers tracked independently (primary trailer 0 + 3 extra trailers).
pub const MAX_TRAILERS: usize = 4;

/// Classified road surface under a wheel from SCS `substances` config names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurfaceKind {
    /// Clean paved road (asphalt, concrete, tarmac, bridge, metal, wood).
    Paved = 0,
    /// Dry loose surface (dirt, gravel, sand, grass, terrain, shoulder, offroad).
    Dusty = 1,
    /// Heavy wet/muddy surface (mud, clay, swamp, water, puddle, snow, slush).
    Muddy = 2,
}

impl SurfaceKind {
    /// Classifies an SCS substance name (e.g. `"asphalt"`, `"dirt"`, `"gravel"`, `"mud"`, `"grass"`).
    pub fn from_name(name: &str) -> Self {
        let lower = name.to_ascii_lowercase();
        if ["mud", "clay", "swamp", "water", "puddle", "snow", "slush", "wet_dirt"]
            .iter()
            .any(|k| lower.contains(k))
        {
            Self::Muddy
        } else if ["dirt", "gravel", "sand", "soil", "earth", "grass", "terrain", "field", "offroad", "shoulder", "forest"]
            .iter()
            .any(|k| lower.contains(k))
        {
            Self::Dusty
        } else {
            Self::Paved
        }
    }
}

/// Accumulation intensity preset (configurable from the phone in Phase 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtIntensity {
    Off,
    Light,
    Realistic,
    Intense,
}

impl DirtIntensity {
    fn multiplier(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Light => 0.45,
            Self::Realistic => 1.0,
            Self::Intense => 2.2,
        }
    }
}

/// Independent dirt layers for a single vehicle unit (tractor or one trailer).
#[derive(Debug, Clone, Copy, Default)]
pub struct UnitDirt {
    /// Dry tan/ochre dust (0.0..=1.0).
    pub dust: f32,
    /// Heavy dark-brown mud on lower body & wheel arches (0.0..=1.0).
    pub mud: f32,
    /// Dark grey wet-asphalt road spray / soot (0.0..=1.0).
    pub grime: f32,
    /// Current moisture level of the dirt (0.0 = dry matte, 1.0 = wet glossy dark).
    pub wetness: f32,
}

impl UnitDirt {
    /// Combined visual dirt level in `0.0..=1.0` for HUD/phone percentage display.
    pub fn total(&self) -> f32 {
        (self.dust * 0.65 + self.mud * 1.0 + self.grime * 0.75).clamp(0.0, 1.0)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Advances dirt simulation for one unit over `dt` seconds.
    pub fn step(&mut self, dt: f32, speed_mps: f32, rain: f32, surface: SurfaceKind, mult: f32) {
        if dt <= 0.0 || !dt.is_finite() || mult <= 0.0 {
            return;
        }
        let dt = dt.min(0.5);
        let speed = speed_mps.abs();
        let rain = rain.clamp(0.0, 1.0);

        // Wetness tracks rain quickly and dries gradually in dry weather / wind.
        let target_wet = if surface == SurfaceKind::Muddy {
            rain.max(0.85)
        } else {
            rain
        };
        let wet_rate = if target_wet > self.wetness { 0.08 } else { 0.012 + (speed / 30.0) * 0.02 };
        self.wetness = (self.wetness + (target_wet - self.wetness) * (wet_rate * dt).min(1.0)).clamp(0.0, 1.0);

        // Moving factor: 0 while stopped (< ~1.5 km/h), ramps smoothly up to highway speed (~80 km/h).
        if speed < 0.4 {
            // Heavy rain while parked gently rinses loose surface dust.
            if rain > 0.35 && self.dust > 0.0 {
                self.dust = (self.dust - 0.004 * rain * dt).max(0.0);
            }
            return;
        }
        let speed_factor = (speed / 22.2).clamp(0.08, 1.4);
        let k = dt * speed_factor * mult;

        match surface {
            SurfaceKind::Paved => {
                if rain > 0.15 {
                    // Wet asphalt road spray builds dark grime and slightly rinses dry dust.
                    self.grime = (self.grime + 0.0022 * rain * k).clamp(0.0, 1.0);
                    self.mud = (self.mud + 0.0004 * rain * k).clamp(0.0, 1.0);
                    self.dust = (self.dust - 0.0015 * rain * k).max(0.0);
                } else {
                    // Dry paved highway: very subtle dust & exhaust grime over long trips.
                    self.dust = (self.dust + 0.00018 * k).clamp(0.0, 1.0);
                    self.grime = (self.grime + 0.00012 * k).clamp(0.0, 1.0);
                }
            }
            SurfaceKind::Dusty => {
                if rain > 0.15 {
                    // Wet dirt/gravel turns into mud rapidly and converts existing dust into mud.
                    let converted = (self.dust * 0.06 * rain * dt).min(self.dust);
                    self.dust = (self.dust - converted).max(0.0);
                    self.mud = (self.mud + converted + 0.014 * (0.4 + rain) * k).clamp(0.0, 1.0);
                    self.grime = (self.grime + 0.002 * rain * k).clamp(0.0, 1.0);
                } else {
                    // Dry dirt/gravel road kicks up heavy dust quickly.
                    self.dust = (self.dust + 0.011 * k).clamp(0.0, 1.0);
                    self.mud = (self.mud + 0.0012 * k).clamp(0.0, 1.0);
                }
            }
            SurfaceKind::Muddy => {
                // Direct mud / swamp / puddle: fastest mud accumulation.
                self.mud = (self.mud + 0.022 * k).clamp(0.0, 1.0);
                self.grime = (self.grime + 0.004 * k).clamp(0.0, 1.0);
            }
        }
    }

    /// Gradually washes the unit over `dt` seconds (for realistic gas-station washing).
    pub fn wash_step(&mut self, dt: f32) -> bool {
        let rate = (dt.clamp(0.0, 0.5)) * 0.35;
        self.dust = (self.dust - rate).max(0.0);
        self.mud = (self.mud - rate * 0.85).max(0.0);
        self.grime = (self.grime - rate).max(0.0);
        self.wetness = (self.wetness + rate * 1.5).clamp(0.0, 1.0);
        self.total() <= 0.005
    }
}

struct State {
    substances: HashMap<u32, SurfaceKind>,
    truck_wheels: [u32; crate::reversing::MOST_WHEELS],
    trailer_wheels: [u32; crate::reversing::MOST_WHEELS],
    truck: UnitDirt,
    trailers: [UnitDirt; MAX_TRAILERS],
    intensity: DirtIntensity,
    last_frame: Option<Instant>,
    last_surface: SurfaceKind,
    last_logged_bucket: u8,
}

impl Default for State {
    fn default() -> Self {
        Self {
            substances: HashMap::new(),
            truck_wheels: [0; crate::reversing::MOST_WHEELS],
            trailer_wheels: [0; crate::reversing::MOST_WHEELS],
            truck: UnitDirt::default(),
            trailers: [UnitDirt::default(); MAX_TRAILERS],
            intensity: DirtIntensity::Realistic,
            last_frame: None,
            last_surface: SurfaceKind::Paved,
            last_logged_bucket: 0,
        }
    }
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));

/// Records the SDK's `substances` table (`index -> substance name`) from `SCS_TELEMETRY_EVENT_CONFIGURATION`.
pub fn set_substances(entries: &[(u32, String)]) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.substances.clear();
    let mut dusty = 0usize;
    let mut muddy = 0usize;
    for (idx, name) in entries {
        let kind = SurfaceKind::from_name(name);
        if kind == SurfaceKind::Dusty {
            dusty += 1;
        } else if kind == SurfaceKind::Muddy {
            muddy += 1;
        }
        s.substances.insert(*idx, kind);
    }
    log!(
        "dirt: {} substances mapped ({} dusty, {} muddy)",
        s.substances.len(),
        dusty,
        muddy
    );
}

/// Updates a single wheel's live substance index from `truck.wheel.substance[i]` or `trailer.wheel.substance[i]`.
pub fn set_wheel_substance(is_trailer: bool, wheel: usize, substance_index: u32) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let arr = if is_trailer { &mut s.trailer_wheels } else { &mut s.truck_wheels };
    if let Some(slot) = arr.get_mut(wheel) {
        *slot = substance_index;
    }
}

/// Resets per-truck transient state when the player changes trucks.
pub fn truck_changed() {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.last_frame = None;
    s.last_logged_bucket = 0;
}

/// Immediately washes the truck and all trailers (for instant phone wash).
#[allow(dead_code)]
pub fn wash_all() {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.truck.clear();
    for t in &mut s.trailers {
        t.clear();
    }
    s.last_logged_bucket = 0;
    log!("dirt: truck and trailers washed");
}

/// Returns true if any vehicle unit has enough dirt to warrant GPU decal rendering.
pub fn has_visible_dirt() -> bool {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.intensity != DirtIntensity::Off
        && (s.truck.total() > 0.015 || s.trailers.iter().any(|t| t.total() > 0.015))
}

/// Returns all 5 vehicle units' dirt states (`[0]` = truck, `[1..=4]` = trailers `0..=3`).
pub fn all_units() -> [UnitDirt; 5] {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    [s.truck, s.trailers[0], s.trailers[1], s.trailers[2], s.trailers[3]]
}

/// Returns a read-only snapshot of `(truck_dirt, trailer_0_dirt)`.
#[allow(dead_code)]
pub fn snapshot() -> (UnitDirt, UnitDirt) {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    (s.truck, s.trailers[0])
}

/// Advances Phase 1 dirt simulation on `SCS_TELEMETRY_EVENT_FRAME_END`.
pub fn on_frame_end(speed_mps: f32, paused: bool, trailer_connected: bool) {
    let now = Instant::now();
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let prev = s.last_frame.replace(now);
    if paused {
        return;
    }
    let Some(prev) = prev else { return };
    let dt = now.duration_since(prev).as_secs_f32();
    let mult = s.intensity.multiplier();
    if mult <= 0.0 {
        return;
    }

    // Worst (dirtiest) surface currently under any active wheel of truck / trailer.
    let truck_surface = s
        .truck_wheels
        .iter()
        .filter_map(|id| s.substances.get(id).copied())
        .max()
        .unwrap_or(SurfaceKind::Paved);

    let trailer_surface = if trailer_connected {
        s.trailer_wheels
            .iter()
            .filter_map(|id| s.substances.get(id).copied())
            .max()
            .unwrap_or(truck_surface)
    } else {
        SurfaceKind::Paved
    };

    // Rain intensity will be piped from host weather in Phase 4; Phase 1 validates surface & speed cleanly.
    let rain = 0.0;

    s.truck.step(dt, speed_mps, rain, truck_surface, mult);
    if trailer_connected {
        for t in &mut s.trailers {
            t.step(dt, speed_mps, rain, trailer_surface, mult);
        }
    }

    if truck_surface != s.last_surface {
        s.last_surface = truck_surface;
        log!("dirt: road surface -> {:?} (truck {:.0}%)", truck_surface, s.truck.total() * 100.0);
    }

    let bucket = ((s.truck.total() * 10.0).floor() as u8).min(10);
    if bucket > s.last_logged_bucket {
        s.last_logged_bucket = bucket;
        log!(
            "dirt: truck {:.0}% (dust={:.2} mud={:.2} grime={:.2}) trailer={:.0}%",
            s.truck.total() * 100.0,
            s.truck.dust,
            s.truck.mud,
            s.truck.grime,
            s.trailers[0].total() * 100.0
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_scs_substances_accurately() {
        assert_eq!(SurfaceKind::from_name("asphalt"), SurfaceKind::Paved);
        assert_eq!(SurfaceKind::from_name("road_concrete"), SurfaceKind::Paved);
        assert_eq!(SurfaceKind::from_name("dirt"), SurfaceKind::Dusty);
        assert_eq!(SurfaceKind::from_name("gravel_road"), SurfaceKind::Dusty);
        assert_eq!(SurfaceKind::from_name("grass"), SurfaceKind::Dusty);
        assert_eq!(SurfaceKind::from_name("mud_deep"), SurfaceKind::Muddy);
        assert_eq!(SurfaceKind::from_name("water_puddle"), SurfaceKind::Muddy);
    }

    #[test]
    fn dirt_accumulates_faster_on_dusty_and_muddy_roads_and_stays_clamped() {
        let mut paved = UnitDirt::default();
        let mut dusty = UnitDirt::default();
        let mut muddy = UnitDirt::default();

        for _ in 0..60 {
            paved.step(0.5, 20.0, 0.0, SurfaceKind::Paved, 1.0);
            dusty.step(0.5, 20.0, 0.0, SurfaceKind::Dusty, 1.0);
            muddy.step(0.5, 20.0, 0.8, SurfaceKind::Muddy, 1.0);
        }

        assert!(paved.total() < 0.05, "paved should stay nearly clean: {}", paved.total());
        assert!(dusty.dust > 0.2, "dusty road should build dust: {}", dusty.dust);
        assert!(muddy.mud > dusty.mud, "muddy road should build more mud than dry dirt");
        assert!(muddy.total() <= 1.0 && muddy.total().is_finite());
    }

    #[test]
    fn stationary_truck_does_not_accumulate_dirt_and_wash_cleans_gradually() {
        let mut unit = UnitDirt { dust: 0.6, mud: 0.8, grime: 0.4, wetness: 0.2 };
        let before = unit.total();
        unit.step(1.0, 0.0, 0.0, SurfaceKind::Muddy, 1.0);
        assert!((unit.total() - before).abs() < 1e-6);

        let mut clean = false;
        for _ in 0..20 {
            if unit.wash_step(0.25) {
                clean = true;
                break;
            }
        }
        assert!(clean, "gradual washing should finish cleanly");
        assert!(unit.total() <= 0.005);
    }
}
