//! Brake fade (owner 2026-11-21: "the brakes heat up with use and, overheated, lose force until they cool down;
//! they cool faster in the rain and at night"; docs/fadiga-freio/PLANO.md): the truck's and the trailer's disc
//! temperatures, and how much of the brake is left.
//!
//! The game's own `truck.brake.temperature` heats and cools, but stops at 200 °C (its `max_brake_temperature`) and
//! reaches it in one hard stop from 120 km/h (owner's log 2026-11-21): no room for a fade that comes on long
//! descents only. So the temperature is modelled here, its shape taken from that log:
//! - heating as the brake times the speed (the energy into the disc): the game's ~4 °C/s per pedal × m/s, here a
//!   third and a bit ([`HEAT`]), more with a load;
//! - cooling toward the air's temperature, exponential: the game's ~0.0095/s (139 °C to 99 °C in 42 s, at any speed)
//!   ([`COOL`]), a little more with speed, more again in the rain and at night.
//!
//! Only the service brake heats: the engine brake and the retarder don't (the right way down a hill).

use std::time::{Duration, Instant};

use westplay_shared::Telemetry;

use crate::weather::{Sky, Weather};

/// Heating, °C per second per (brake 0..1 × speed m/s).
const HEAT: f32 = 1.4;
/// Cooling, per second, of the difference with the air.
const COOL: f32 = 0.0095;
/// Cooling with speed (the air through the wheel; owner 2026-11-21: "by speed: at 90 km/h it cools so much, at 120 a
/// few percent more, at 140 more"): how many times faster than stopped, at each speed (km/h), straight lines between.
/// Stronger than the game's own (none), with [`HEAT`] raised to match: down a hill, slow and braking, is what
/// overheats them.
const WIND: &[(f32, f32)] = &[(0.0, 1.0), (30.0, 1.15), (60.0, 1.35), (90.0, 1.6), (120.0, 1.9), (140.0, 2.1), (180.0, 2.4)];

/// [`WIND`] at a speed (km/h).
fn wind(kmh: f32) -> f32 {
    let kmh = kmh.max(0.0);
    for pair in WIND.windows(2) {
        let ((a, fa), (b, fb)) = (pair[0], pair[1]);
        if kmh <= b {
            return fa + (fb - fa) * (kmh - a) / (b - a);
        }
    }
    WIND[WIND.len() - 1].1
}
/// The air: by day, at night, in the rain (°C).
const AIR_DAY: f32 = 25.0;
const AIR_NIGHT: f32 = 15.0;
const AIR_RAIN: f32 = 18.0;
/// Cooling at night (20:00 to 06:00), and in the rain at its wettest (the wetness 0..100 between none and this).
const NIGHT: f32 = 1.3;
const RAIN: f32 = 2.5;
/// The hottest a disc gets.
const HOTTEST: f32 = 700.0;
/// The least brake ever left (never none: owner's plan, safety of the game).
pub const LEAST: f32 = 0.15;
/// How fast the brake comes back as it cools (per second): no flicker at a threshold.
const RECOVER: f32 = 0.02;
/// A tick longer than this (a pause, a hitch) counts as this.
const LONGEST_TICK: Duration = Duration::from_millis(500);

/// How hard the fade is (the player's choice): its temperatures moved up or down. Light and Hard come with the
/// settings (`Settings::brake_fade_level`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Light,
    #[default]
    Realistic,
    Hard,
}

impl Level {
    pub fn shift(self) -> f32 {
        match self {
            Level::Light => 100.0,
            Level::Realistic => 0.0,
            Level::Hard => -50.0,
        }
    }
}

/// What the brakes are now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fade {
    /// The truck's discs, °C.
    pub truck: f32,
    /// The trailer's, °C (`None`: no trailer coupled).
    pub trailer: Option<f32>,
    /// How much of the brake is left, [`LEAST`]..1.
    pub strength: f32,
}

#[derive(Debug)]
pub struct BrakeFade {
    truck: f32,
    trailer: Option<f32>,
    strength: f32,
    last: Option<Instant>,
}

impl Default for BrakeFade {
    fn default() -> Self {
        Self { truck: AIR_DAY, trailer: None, strength: 1.0, last: None }
    }
}

/// The brake left at a disc temperature: all of it to 250 °C, down to the least ([`LEAST`], no brake to speak of)
/// at 400 °C and over (owner 2026-11-21: "at 400 °C the truck has no brakes any more": with the least only at 550,
/// the weakened brake never heated the discs that far, and the sounds of the brake gone never came).
pub fn strength_at(celsius: f32, level: Level) -> f32 {
    let t = celsius - level.shift();
    let past = ((t - 250.0) / 150.0).clamp(0.0, 1.0);
    (1.0 - (1.0 - LEAST) * past).max(LEAST)
}

/// Whether braking grinds (owner 2026-11-21): by the hottest disc, from 320 °C on the realistic level; with a trailer
/// whose discs are much cooler (or hotter) than the truck's, later, as the cooler ones still hold a good part of the
/// brake: 30 % of the difference later, 380 °C at most. The dashboard's warning goes by the brake left, both together.
pub fn grinds(fade: &Fade, level: Level) -> bool {
    let shift = level.shift();
    let (hottest, gap) = match fade.trailer {
        Some(trailer) => (fade.truck.max(trailer), (fade.truck - trailer).abs()),
        None => (fade.truck, 0.0),
    };
    let from = (320.0 + 0.3 * gap).min(380.0);
    hottest - shift >= from
}

/// Whether it's night in the game (20:00 to 06:00), from its minutes (and a Convoy's offset).
fn night(t: &Telemetry) -> bool {
    let hour = (i64::from(t.game_time) + i64::from(t.multiplayer_time_offset)).rem_euclid(1440) / 60;
    !(6..20).contains(&hour)
}

impl BrakeFade {
    /// One step: `cargo_kg` the job's cargo (0 without), `weather` as last read (`None`: not known).
    pub fn tick(
        &mut self,
        level: Level,
        telemetry: Option<&Telemetry>,
        cargo_kg: f32,
        weather: Option<&Weather>,
        now: Instant,
    ) -> Fade {
        let dt = self.last.map_or(0.0, |last| now.duration_since(last).min(LONGEST_TICK).as_secs_f32());
        self.last = Some(now);
        if let Some(t) = telemetry.filter(|t| !t.paused) {
            let wet = weather.map_or(0.0, |w| match (w.now, w.wetness) {
                (_, Some(wetness)) => f32::from(wetness.min(100)) / 100.0,
                (Sky::Rain, None) => 1.0,
                _ => 0.0,
            });
            let dark = night(t);
            let air = if wet > 0.3 {
                AIR_RAIN
            } else if dark {
                AIR_NIGHT
            } else {
                AIR_DAY
            };
            let speed = t.speed.abs();
            let cool = COOL * wind(speed * 3.6) * (1.0 + (RAIN - 1.0) * wet) * if dark { NIGHT } else { 1.0 };
            let energy = HEAT * t.brake.clamp(0.0, 1.0) * speed;
            let load = cargo_kg.max(0.0) / 1000.0;
            let step = |disc: f32, share: f32| {
                (disc + (energy * share - cool * (disc - air)) * dt).clamp(air.min(disc), HOTTEST)
            };
            if t.trailer_connected {
                // The trailer's brakes take the load's share (owner 2026-11-21: "heavy is one thing and empty another"):
                // empty, the trailer's brakes do less than the truck's (load-sensing, EBS), so they heat 40 % less and
                // the truck works a little more; loaded, they do more and more the heavier (about the truck's at 8 t,
                // 1.6 times at 20 t).
                let truck_share = if load < 0.5 { 1.1 } else { 1.0 + load / 80.0 };
                self.truck = step(self.truck, truck_share);
                self.trailer = Some(step(self.trailer.unwrap_or(air), 0.6 + load / 20.0));
            } else {
                self.truck = step(self.truck, 1.0);
                self.trailer = None;
            }
        }
        // How much of the braking each does (owner 2026-11-21): empty, the trailer's brakes hold back (30 %, the
        // truck 70 %); loaded, more and more, 60 % from 15 t on.
        let share = 0.3 + 0.3 * (cargo_kg.max(0.0) / 15_000.0).min(1.0);
        let target = match self.trailer {
            Some(trailer) => (1.0 - share) * strength_at(self.truck, level) + share * strength_at(trailer, level),
            None => strength_at(self.truck, level),
        };
        // Down at once, back slowly.
        self.strength = if target < self.strength { target } else { (self.strength + RECOVER * dt).min(target) };
        self.strength = self.strength.clamp(LEAST, 1.0);
        Fade { truck: self.truck, trailer: self.trailer, strength: self.strength }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driving(kmh: f32, brake: f32) -> Telemetry {
        Telemetry { speed: kmh / 3.6, brake, game_time: 12 * 60, engine_enabled: true, ..Default::default() }
    }

    /// Runs `secs` seconds of `t`, a tick every 100 ms, from `start`; the last fade and the next instant.
    fn run(
        fade: &mut BrakeFade,
        t: &Telemetry,
        weather: Option<&Weather>,
        start: Instant,
        secs: u32,
    ) -> (Fade, Instant) {
        let mut now = start;
        let mut last = fade.tick(Level::Realistic, Some(t), 0.0, weather, now);
        for _ in 0..secs * 10 {
            now += Duration::from_millis(100);
            last = fade.tick(Level::Realistic, Some(t), 0.0, weather, now);
        }
        (last, now)
    }

    #[test]
    fn the_curve_keeps_all_the_brake_cool_and_never_none() {
        assert_eq!(strength_at(200.0, Level::Realistic), 1.0);
        assert_eq!(strength_at(400.0, Level::Realistic), LEAST);
        assert!((strength_at(325.0, Level::Realistic) - (1.0 + LEAST) / 2.0).abs() < 1e-4);
        assert_eq!(strength_at(2000.0, Level::Realistic), LEAST);
        assert_eq!(strength_at(320.0, Level::Light), 1.0);
        assert!(strength_at(250.0, Level::Hard) < 1.0);
    }

    #[test]
    fn braking_heats_and_not_braking_does_not() {
        let start = Instant::now();
        let mut fade = BrakeFade::default();
        let (cruising, at) = run(&mut fade, &driving(80.0, 0.0), None, start, 30);
        assert!(cruising.truck <= AIR_DAY + 0.01);
        let (braking, _) = run(&mut fade, &driving(60.0, 0.2), None, at, 30);
        assert!(braking.truck > 100.0, "{braking:?}");
    }

    #[test]
    fn a_long_descent_fades_and_a_rest_brings_it_back() {
        let start = Instant::now();
        let mut fade = BrakeFade::default();
        // Down a hill at 65 km/h on the brake alone, 4 minutes.
        let (down, at) = run(&mut fade, &driving(65.0, 0.25), None, start, 240);
        assert!(down.truck > 350.0 && down.strength < 0.7, "{down:?}");
        assert!(down.strength >= LEAST);
        // Stopped 3 minutes.
        let (rested, _) = run(&mut fade, &driving(0.0, 0.0), None, at, 180);
        assert!(rested.truck < 250.0 && rested.strength == 1.0, "{rested:?}");
    }

    #[test]
    fn the_brake_comes_back_slowly() {
        let start = Instant::now();
        let mut fade = BrakeFade { truck: 420.0, ..Default::default() };
        let first = fade.tick(Level::Realistic, Some(&driving(0.0, 0.0)), 0.0, None, start);
        fade.truck = 100.0;
        let next = fade.tick(Level::Realistic, Some(&driving(0.0, 0.0)), 0.0, None, start + Duration::from_secs(1));
        assert!(next.strength > first.strength && next.strength < first.strength + 0.03, "{first:?} {next:?}");
    }

    #[test]
    fn the_wind_by_speed() {
        assert_eq!(wind(0.0), 1.0);
        assert!((wind(90.0) - 1.6).abs() < 1e-4 && (wind(105.0) - 1.75).abs() < 1e-4);
        assert_eq!(wind(300.0), 2.4);
    }

    #[test]
    fn speed_cools_faster() {
        let start = Instant::now();
        let cooled = |kmh: f32| {
            let mut fade = BrakeFade { truck: 300.0, ..Default::default() };
            run(&mut fade, &driving(kmh, 0.0), None, start, 60).0.truck
        };
        assert!(cooled(100.0) < cooled(20.0) - 40.0, "{} {}", cooled(100.0), cooled(20.0));
    }

    #[test]
    fn rain_and_night_cool_faster() {
        let start = Instant::now();
        let cooled = |t: &Telemetry, weather: Option<&Weather>| {
            let mut fade = BrakeFade { truck: 300.0, ..Default::default() };
            run(&mut fade, t, weather, start, 60).0.truck
        };
        let day = cooled(&driving(30.0, 0.0), None);
        let rain = Weather { now: Sky::Rain, coming: Sky::Rain, change_in: None, wetness: Some(100) };
        let wet = cooled(&driving(30.0, 0.0), Some(&rain));
        let dark = cooled(&Telemetry { game_time: 23 * 60, ..driving(30.0, 0.0) }, None);
        assert!(wet < day - 20.0, "{wet} {day}");
        assert!(dark < day - 5.0, "{dark} {day}");
    }

    #[test]
    fn a_trailer_has_its_own_disc_and_a_load_heats_it_more() {
        let start = Instant::now();
        let mut fade = BrakeFade::default();
        let coupled = Telemetry { trailer_connected: true, ..driving(60.0, 0.2) };
        let mut now = start;
        let mut last = fade.tick(Level::Realistic, Some(&coupled), 20_000.0, None, now);
        for _ in 0..300 {
            now += Duration::from_millis(100);
            last = fade.tick(Level::Realistic, Some(&coupled), 20_000.0, None, now);
        }
        assert!(last.trailer.is_some_and(|trailer| trailer > last.truck), "{last:?}");
        let alone = fade.tick(Level::Realistic, Some(&driving(60.0, 0.0)), 0.0, None, now + Duration::from_millis(100));
        assert_eq!(alone.trailer, None);
    }

    #[test]
    fn an_empty_trailer_heats_less_than_the_truck() {
        let start = Instant::now();
        let mut fade = BrakeFade::default();
        let coupled = Telemetry { trailer_connected: true, ..driving(60.0, 0.2) };
        let mut now = start;
        let mut last = fade.tick(Level::Realistic, Some(&coupled), 0.0, None, now);
        for _ in 0..300 {
            now += Duration::from_millis(100);
            last = fade.tick(Level::Realistic, Some(&coupled), 0.0, None, now);
        }
        assert!(last.trailer.is_some_and(|trailer| trailer < last.truck - 20.0), "{last:?}");
    }

    #[test]
    fn it_grinds_by_the_hottest_disc_later_with_a_cool_one_beside_it() {
        let fade = |truck: f32, trailer: Option<f32>| Fade { truck, trailer, strength: 1.0 };
        let level = Level::Realistic;
        assert!(!grinds(&fade(310.0, None), level));
        assert!(grinds(&fade(325.0, None), level));
        // The owner's picture: the truck at 435 °C, the trailer at 250 °C.
        assert!(grinds(&fade(435.0, Some(250.0)), level));
        assert!(!grinds(&fade(340.0, Some(150.0)), level));
        assert!(grinds(&fade(330.0, Some(320.0)), level));
        // The trailer hotter works the same way round.
        assert!(grinds(&fade(250.0, Some(435.0)), level));
        assert!(!grinds(&fade(325.0, None), Level::Light));
    }

    #[test]
    fn an_empty_trailer_brakes_less_of_the_whole() {
        let start = Instant::now();
        let coupled = Telemetry { trailer_connected: true, ..driving(0.0, 0.0) };
        let left = |cargo_kg: f32| {
            let mut fade = BrakeFade { truck: 400.0, trailer: Some(100.0), ..Default::default() };
            fade.tick(Level::Realistic, Some(&coupled), cargo_kg, None, start).strength
        };
        // The truck's brake gone, the trailer's whole: empty, little left; loaded, most of it.
        assert!((left(0.0) - (0.7 * LEAST + 0.3)).abs() < 0.01, "{}", left(0.0));
        assert!((left(20_000.0) - (0.4 * LEAST + 0.6)).abs() < 0.01, "{}", left(20_000.0));
    }

    #[test]
    fn paused_nothing_moves() {
        let start = Instant::now();
        let mut fade = BrakeFade { truck: 300.0, ..Default::default() };
        let paused = Telemetry { paused: true, ..driving(0.0, 0.0) };
        let (still, _) = run(&mut fade, &paused, None, start, 10);
        assert_eq!(still.truck, 300.0);
    }
}
