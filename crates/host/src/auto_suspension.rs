//! The suspension by itself (owner 2026-11-19: "a system adapting the suspension, going up and down by itself as the
//! truck needs, as in real life"), as a real truck's air suspension (ECAS) does, with the game's own commands as the
//! Suspensão app presses them (the plugin holds them for the game), so the game's own limits hold:
//!
//! - the motorway: a while over [`FAST`], lowered a little ([`LOWER_FOR`]: owner 2026-11-19, 1.5 s was so low the
//!   truck bottomed out and was damaged); a while under [`SLOW`], back up as much;
//! - a rough road: the springs jumping while moving, each axle on its own ([`Swing`]: how far from its mean, and its
//!   widest swing, which a lowered truck still shows when its spring only goes one way against its stop), raised a
//!   little; smooth again a while, back down as much.
//!
//! Every move is undone by its opposite, held as long (owner 2026-11-21: "it went back to the game's normal height,
//! not to the very low one I had"): it comes back to the player's own height, never the game's reset.
//! - a load: a trailer coupled standing still, its weight sinking the rear ([`SANK`] more pressed in than before
//!   it), the rear raised in short presses until it's back where it was (as the real one levels the load).
//!
//! - braking long: the brake held hard a while, raised a little from wherever it is ([`BRAKE_RAISE`]), held so while
//!   braking, and down again as much once the brake is let go (owner 2026-11-19: "lowered and braking long, downhill
//!   or not, the truck dips and touches the ground");
//!
//! Only with the engine running (the air suspension moves only then) and the game not paused; the player's own
//! presses in the app leave it alone for [`MANUAL`].

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use westplay_shared::{Telemetry, truck_command_bit};

use crate::settings::Damping;

/// Threshold multiplier by suspension stiffness (calibrated from owner's 2026-11-22 log:
/// good asphalt in Firmest was 0.3–1.6 cm RMS / 0.8–5.1 cm widest; rough asphalt was 2.0–3.2 cm / 7.0–10.5 cm).
fn stiffness_scale(damping: Damping) -> f32 {
    match damping {
        Damping::Soft => 1.35,
        Damping::Normal => 1.0,
        Damping::Firm => 0.95,
        Damping::Firmest => 0.90,
    }
}

/// Over this (m/s, 80 km/h) for [`AFTER`], lowered; under [`SLOW`] (60 km/h) for [`AFTER`], back to normal.
const FAST: f32 = 80.0 / 3.6;
const SLOW: f32 = 60.0 / 3.6;
const AFTER: Duration = Duration::from_secs(5);
/// How long the lowering is held: a little (all the way took about 5 s in the app's Mínima, 1.5 s bottomed out).
const LOWER_FOR: Duration = Duration::from_millis(400);
/// How long the raising on a rough road is held (owner 2026-11-22: 800 ms barely lifted a lowered truck;
/// 2400 ms lifts it substantially clear of rough terrain and lowers back the exact same amount when smooth).
const RAISE_FOR: Duration = Duration::from_millis(2400);
/// The player's own suspension press: left alone this long.
const MANUAL: Duration = Duration::from_secs(60);

/// The springs' jumping: their readings over this long, sampled at most this often; rough over [`ROUGH`] (metres,
/// the root mean square of how far they are from their mean) for [`ROUGH_AFTER`], smooth under [`SMOOTH`] for
/// [`SMOOTH_AFTER`]; only moving over [`MOVING`].
/// Recalibrated (owner 2026-11-22): old 1.2 cm / 6 cm thresholds false-triggered on normal asphalt and 0.9 cm / 20 s
/// prevented returning to normal height once back on paved roads.
const WINDOW: Duration = Duration::from_secs(2);
const SAMPLE: Duration = Duration::from_millis(40);
const ROUGH: f32 = 0.025;
const SMOOTH: f32 = 0.018;
/// An axle's widest swing over [`WINDOW`] (metres): rough over this, smooth under [`SMOOTH_SWING`].
const ROUGH_SWING: f32 = 0.095;
const SMOOTH_SWING: f32 = 0.062;
const ROUGH_AFTER: Duration = Duration::from_secs(3);
const SMOOTH_AFTER: Duration = Duration::from_secs(7);
/// After the app's own press, the springs' readings left out this long (the suspension moving isn't the road).
const QUIET_AFTER: Duration = Duration::from_millis(2500);
const MOVING: f32 = 15.0 / 3.6;

/// Standing still (m/s); the rear more pressed in than this (metres) after coupling: sunk under the load; back
/// within [`LEVEL`]; a press of [`LEVEL_PRESS`] each [`LEVEL_EVERY`], at most [`LEVEL_PRESSES`]; measured once the
/// coupling has settled ([`SETTLE`]).
const STILL: f32 = 0.3;
const SANK: f32 = 0.03;
const LEVEL: f32 = 0.01;
const LEVEL_PRESS: Duration = Duration::from_millis(300);
const LEVEL_EVERY: Duration = Duration::from_millis(1500);
const LEVEL_PRESSES: u32 = 8;
const SETTLE: Duration = Duration::from_secs(3);

/// Braking long: the brake over [`BRAKE_HARD`] (0..1) moving over [`BRAKE_MOVING`] for [`BRAKE_AFTER`]; raised for
/// [`BRAKE_RAISE`]; under [`BRAKE_LET_GO`] for [`BRAKE_RELEASED`], down as much.
const BRAKE_HARD: f32 = 0.3;
const BRAKE_LET_GO: f32 = 0.1;
const BRAKE_MOVING: f32 = 20.0 / 3.6;
const BRAKE_AFTER: Duration = Duration::from_secs(2);
const BRAKE_RELEASED: Duration = Duration::from_secs(1);
const BRAKE_RAISE: Duration = Duration::from_millis(400);

/// A line in the log this often while it's on and moving (for tuning [`ROUGH`] from the owner's drives).
const LOG_EVERY: Duration = Duration::from_secs(5);

/// How the springs jump over [`WINDOW`] (metres): their root mean square about their mean, and their widest swing.
#[derive(Clone, Copy, Debug)]
struct Swing {
    rms: f32,
    widest: f32,
}

/// A spring's reading at or under this is the plugin's "not read" (-1).
const UNREAD: f32 = -0.9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Normal,
    Low,
    High,
}

/// Levelling the rear under a load: the rear's reading before the trailer, when it was coupled, and the presses.
#[derive(Clone, Copy, Debug)]
struct Levelling {
    before: f32,
    coupled_at: Instant,
    presses: u32,
    next_at: Instant,
}

pub struct AutoSuspension {
    level: Level,
    fast_since: Option<Instant>,
    slow_since: Option<Instant>,
    rough_since: Option<Instant>,
    smooth_since: Option<Instant>,
    /// The springs' readings over [`WINDOW`]: the front's and the rear's, each.
    samples: VecDeque<(Instant, [f32; 2])>,
    /// The rear's reading standing still without a trailer (the one before a load), and whether one was coupled.
    rear_unloaded: Option<f32>,
    coupled: bool,
    levelling: Option<Levelling>,
    /// What's held now (command bits) and until when.
    pressing: Option<(u64, Instant)>,
    manual_until: Option<Instant>,
    logged_at: Option<Instant>,
    /// The readings left out until (`QUIET_AFTER` the app's own press).
    quiet_until: Option<Instant>,
    /// Braking hard since, let go since, and whether it's raised for it now.
    braking_since: Option<Instant>,
    let_go_since: Option<Instant>,
    brake_raised: bool,
}

impl Default for AutoSuspension {
    fn default() -> Self {
        Self {
            level: Level::Normal,
            fast_since: None,
            slow_since: None,
            rough_since: None,
            smooth_since: None,
            samples: VecDeque::new(),
            rear_unloaded: None,
            coupled: false,
            levelling: None,
            pressing: None,
            manual_until: None,
            logged_at: None,
            quiet_until: None,
            braking_since: None,
            let_go_since: None,
            brake_raised: false,
        }
    }
}

fn bits(names: &[&str]) -> u64 {
    names.iter().filter_map(|name| truck_command_bit(name)).fold(0, |all, bit| all | bit)
}

/// Since when `now` has held (`None`: it doesn't).
fn since(was: Option<Instant>, holds: bool, now: Instant) -> Option<Instant> {
    if holds { was.or(Some(now)) } else { None }
}

fn held_for(since: Option<Instant>, long: Duration, now: Instant) -> bool {
    since.is_some_and(|since| now.saturating_duration_since(since) >= long)
}

impl AutoSuspension {
    /// The player pressed one of the suspension's commands in the app: theirs for a while.
    pub fn manual(&mut self, now: Instant) {
        self.manual_until = Some(now + MANUAL);
        self.pressing = None;
        self.levelling = None;
        // Where it is now isn't known: taken as normal, so going fast lowers it again.
        self.level = Level::Normal;
    }

    /// How rough the road is now: the axle that jumps most's ([`Swing`]).
    fn roughness(&self) -> Option<Swing> {
        if self.samples.len() < 10 {
            return None;
        }
        let axle = |at: usize| {
            let values: Vec<f32> = self.samples.iter().map(|(_, v)| v[at]).collect();
            let n = values.len() as f32;
            let mean = values.iter().sum::<f32>() / n;
            let rms = (values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n).sqrt();
            let widest = values.iter().copied().fold(f32::MIN, f32::max) - values.iter().copied().fold(f32::MAX, f32::min);
            Swing { rms, widest }
        };
        let (front, rear) = (axle(0), axle(1));
        Some(Swing { rms: front.rms.max(rear.rms), widest: front.widest.max(rear.widest) })
    }

    fn press(&mut self, names: &[&str], hold: Duration, now: Instant, why: std::fmt::Arguments) -> u64 {
        log!("suspension by itself: {why}");
        let press = bits(names);
        self.pressing = Some((press, now + hold));
        // The suspension moving now isn't the road: its readings start again after it.
        self.samples.clear();
        self.rough_since = None;
        self.smooth_since = None;
        self.quiet_until = Some(now + hold + QUIET_AFTER);
        press
    }

    /// The commands to hold now (bits, with the phones'), `on` as the player set it, using Normal stiffness scale.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn tick(&mut self, on: bool, telemetry: Option<&Telemetry>, now: Instant) -> u64 {
        self.tick_with_damping(on, Damping::Normal, ROUGH_AFTER, SMOOTH_AFTER, telemetry, now)
    }

    /// The commands to hold now (bits, with the phones'), scaling rough/smooth road thresholds to match `damping`
    /// and using `rough_after` / `smooth_after` as the player-configured delays.
    pub fn tick_with_damping(
        &mut self,
        on: bool,
        damping: Damping,
        rough_after: Duration,
        smooth_after: Duration,
        telemetry: Option<&Telemetry>,
        now: Instant,
    ) -> u64 {
        if let Some((held, until)) = self.pressing {
            if now < until {
                return held;
            }
            self.pressing = None;
        }
        let Some(t) = telemetry.filter(|t| on && t.engine_enabled && !t.paused) else {
            self.fast_since = None;
            self.slow_since = None;
            self.rough_since = None;
            self.smooth_since = None;
            self.samples.clear();
            self.levelling = None;
            return 0;
        };
        let speed = t.speed.abs();
        let [front, rear, _] = t.suspension;
        let (front, rear) = ((front > UNREAD).then_some(front), (rear > UNREAD).then_some(rear));

        // The springs' readings, for the road's roughness.
        let quiet = self.quiet_until.is_some_and(|until| now < until);
        if let (Some(front), Some(rear)) = (front, rear)
            && !quiet
            && self.samples.back().is_none_or(|(at, _)| now.saturating_duration_since(*at) >= SAMPLE)
        {
            self.samples.push_back((now, [front, rear]));
        }
        while self.samples.front().is_some_and(|(at, _)| now.saturating_duration_since(*at) > WINDOW) {
            self.samples.pop_front();
        }
        let roughness = self.roughness().filter(|_| speed > MOVING);
        if speed > MOVING && self.logged_at.is_none_or(|at| now.saturating_duration_since(at) >= LOG_EVERY) {
            self.logged_at = Some(now);
            log!(
                "suspension by itself: {:.0} km/h, road {}, {:?}, {damping:?}",
                speed * 3.6,
                roughness.map_or("not read".to_owned(), |r| format!("{:.1} cm (widest {:.1} cm)", r.rms * 100.0, r.widest * 100.0)),
                self.level
            );
        }

        // A load: the rear's reading standing still without a trailer, and its levelling once one is coupled.
        let still = speed < STILL;
        if t.trailer_connected && !self.coupled && still {
            if let (Some(before), Some(_)) = (self.rear_unloaded, rear) {
                self.levelling = Some(Levelling { before, coupled_at: now, presses: 0, next_at: now + SETTLE });
            }
        }
        self.coupled = t.trailer_connected;
        if !t.trailer_connected && still && self.level == Level::Normal {
            self.rear_unloaded = rear;
        }
        if !still {
            self.levelling = None;
        }

        if self.manual_until.is_some_and(|until| now < until) {
            return 0;
        }

        if let Some(mut levelling) = self.levelling
            && now >= levelling.next_at
            && let Some(rear) = rear
        {
            let sunk = rear - levelling.before;
            if levelling.presses == 0 && sunk < SANK {
                self.levelling = None;
            } else if sunk <= LEVEL || levelling.presses >= LEVEL_PRESSES {
                log!(
                    "suspension by itself: the load levelled ({:.1} cm left, {} presses, {:.0} s)",
                    sunk * 100.0,
                    levelling.presses,
                    now.saturating_duration_since(levelling.coupled_at).as_secs_f32()
                );
                self.levelling = None;
            } else {
                levelling.presses += 1;
                levelling.next_at = now + LEVEL_EVERY;
                self.levelling = Some(levelling);
                return self.press(
                    &["rearsuspup"],
                    LEVEL_PRESS,
                    now,
                    format_args!("the rear {:.1} cm down under the load: raised", sunk * 100.0),
                );
            }
        }

        // Braking long: up a little from where it is, held while braking, down as much once let go. Nothing else
        // meanwhile (it would undo it).
        self.braking_since = since(self.braking_since, t.brake > BRAKE_HARD && speed > BRAKE_MOVING, now);
        self.let_go_since = since(self.let_go_since, t.brake < BRAKE_LET_GO, now);
        if !self.brake_raised && held_for(self.braking_since, BRAKE_AFTER, now) {
            self.brake_raised = true;
            let kmh = speed * 3.6;
            return self.press(
                &["frontsuspup", "rearsuspup"],
                BRAKE_RAISE,
                now,
                format_args!("raised a little, braking long at {kmh:.0} km/h"),
            );
        }
        if self.brake_raised {
            if held_for(self.let_go_since, BRAKE_RELEASED, now) {
                self.brake_raised = false;
                return self.press(
                    &["frontsuspdwn", "rearsuspdwn"],
                    BRAKE_RAISE,
                    now,
                    format_args!("the brake let go: down as much again"),
                );
            }
            return 0;
        }

        self.fast_since = since(self.fast_since, speed > FAST, now);
        self.slow_since = since(self.slow_since, speed < SLOW, now);
        // Only what was read moving (and not braking hard) counts as a rough road; stopping or braking clears
        // `rough_since` so it can never false-trigger "rough road at 0 km/h" (owner 2026-11-22).
        // Thresholds scale with the chosen suspension stiffness (Soft moves more, Firm/Firmest move less).
        let scale = stiffness_scale(damping);
        if let Some(r) = roughness {
            let is_rough = t.brake <= BRAKE_HARD && (r.rms > ROUGH * scale || r.widest > ROUGH_SWING * scale);
            self.rough_since = since(self.rough_since, is_rough, now);
            self.smooth_since =
                since(self.smooth_since, r.rms < SMOOTH * scale && r.widest < SMOOTH_SWING * scale, now);
        } else {
            self.rough_since = None;
        }
        let rough = held_for(self.rough_since, rough_after, now);
        let kmh = speed * 3.6;

        match self.level {
            // A rough road: up a little (from lowered, as far again as it went down).
            Level::Normal | Level::Low if rough => {
                let hold = if self.level == Level::Low { LOWER_FOR + RAISE_FOR } else { RAISE_FOR };
                self.level = Level::High;
                self.press(&["frontsuspup", "rearsuspup"], hold, now, format_args!("raised on a rough road at {kmh:.0} km/h"))
            }
            // Down again only after a while smooth (never while it stays rough, whatever the speed).
            Level::High if held_for(self.smooth_since, smooth_after, now) => {
                self.level = Level::Normal;
                self.press(
                    &["frontsuspdwn", "rearsuspdwn"],
                    RAISE_FOR,
                    now,
                    format_args!("down as much again (smooth again) at {kmh:.0} km/h"),
                )
            }
            Level::Normal if held_for(self.fast_since, AFTER, now) => {
                self.level = Level::Low;
                self.press(&["frontsuspdwn", "rearsuspdwn"], LOWER_FOR, now, format_args!("lowered a little at {kmh:.0} km/h"))
            }
            Level::Low if held_for(self.slow_since, AFTER, now) => {
                self.level = Level::Normal;
                self.press(&["frontsuspup", "rearsuspup"], LOWER_FOR, now, format_args!("up as much again at {kmh:.0} km/h"))
            }
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driving(kmh: f32) -> Telemetry {
        Telemetry {
            speed: kmh / 3.6,
            engine_enabled: true,
            electric_enabled: true,
            suspension: [0.1, 0.1, -1.0],
            ..Default::default()
        }
    }

    #[test]
    fn lowers_a_little_on_the_motorway_and_comes_back_in_town() {
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        let down = bits(&["frontsuspdwn", "rearsuspdwn"]);
        assert_eq!(auto.tick(true, Some(&driving(90.0)), start), 0);
        assert_eq!(auto.tick(true, Some(&driving(90.0)), start + AFTER), down);
        assert_eq!(auto.tick(true, Some(&driving(90.0)), start + AFTER + LOWER_FOR), 0);
        assert_eq!(auto.tick(true, Some(&driving(95.0)), start + AFTER * 3), 0);
        let slow = start + AFTER * 4;
        assert_eq!(auto.tick(true, Some(&driving(40.0)), slow), 0);
        assert_eq!(auto.tick(true, Some(&driving(40.0)), slow + AFTER), bits(&["frontsuspup", "rearsuspup"]));
    }

    #[test]
    fn raises_on_a_rough_road() {
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        let mut held = 0;
        for i in 0..200u32 {
            let mut t = driving(40.0);
            let bump = if i % 2 == 0 { 0.03 } else { -0.03 };
            t.suspension = [0.1 + bump, 0.1 + bump, -1.0];
            held |= auto.tick(true, Some(&t), start + SAMPLE * i);
        }
        assert_ne!(held & bits(&["frontsuspup"]), 0);
        assert_eq!(auto.level, Level::High);
    }

    #[test]
    fn stays_up_while_the_road_is_rough_and_slow() {
        // The owner's dirt road (2026-11-19): up, then down 0.6 s later, over and over. Now up it stays.
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        let rough = |i: u32| {
            let mut t = driving(30.0);
            let bump = if i % 2 == 0 { 0.03 } else { -0.03 };
            t.suspension = [0.1 + bump, 0.1 + bump, -1.0];
            t
        };
        let mut downs = 0;
        for i in 0..1500u32 {
            if auto.tick(true, Some(&rough(i)), start + SAMPLE * i) & bits(&["frontsuspdwn"]) != 0 {
                downs += 1;
            }
        }
        assert_eq!(auto.level, Level::High);
        assert_eq!(downs, 0, "never down while rough");
        // Smooth for a long while: down once.
        let later = start + SAMPLE * 1500;
        // Presses counted as they start (each is held a few ticks).
        let (mut back, mut was) = (0, false);
        for i in 0..800u32 {
            let now = auto.tick(true, Some(&driving(30.0)), later + SAMPLE * i) & bits(&["frontsuspdwn"]) != 0;
            back += u32::from(now && !was);
            was = now;
        }
        assert_eq!(back, 1);
    }

    #[test]
    fn a_lowered_truck_on_a_bad_road_rises_too() {
        // Lowered, the spring against its stop: the axle reads near its stop most of the time and jumps up now and
        // then (a small mean swing, a wide one).
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        let mut held = 0;
        for i in 0..200u32 {
            let mut t = driving(40.0);
            let hit = if i % 10 == 0 { -0.10 } else { 0.0 };
            t.suspension = [0.2 + hit, 0.2, -1.0];
            held |= auto.tick(true, Some(&t), start + SAMPLE * i);
        }
        assert_ne!(held & bits(&["frontsuspup"]), 0);
    }

    #[test]
    fn never_raises_for_rough_road_when_stopped() {
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        // Brief rough samples while moving (not long enough to trigger), then stop.
        for i in 0..30u32 {
            let mut t = driving(30.0);
            let bump = if i % 2 == 0 { 0.04 } else { -0.04 };
            t.suspension = [0.1 + bump, 0.1 + bump, -1.0];
            auto.tick(true, Some(&t), start + SAMPLE * i);
        }
        let stopped = start + SAMPLE * 30;
        let held = auto.tick(true, Some(&driving(0.0)), stopped + ROUGH_AFTER + Duration::from_secs(1));
        assert_eq!(held, 0);
        assert_eq!(auto.level, Level::Normal);
    }

    #[test]
    fn never_the_games_reset() {
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        let mut held = 0;
        for i in 0..3000u32 {
            let kmh = if i < 1000 { 90.0 } else { 40.0 };
            held |= auto.tick(true, Some(&driving(kmh)), start + SAMPLE * i);
        }
        assert_eq!(held & bits(&["suspreset"]), 0);
    }

    #[test]
    fn up_a_little_braking_long_and_back_as_much() {
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        let braking = Telemetry { brake: 0.6, ..driving(70.0) };
        let up = bits(&["frontsuspup", "rearsuspup"]);
        assert_eq!(auto.tick(true, Some(&braking), start), 0);
        assert_eq!(auto.tick(true, Some(&braking), start + BRAKE_AFTER), up);
        // Held while braking: nothing more.
        let later = start + BRAKE_AFTER + Duration::from_secs(5);
        assert_eq!(auto.tick(true, Some(&braking), later), 0);
        let free = driving(50.0);
        assert_eq!(auto.tick(true, Some(&free), later), 0);
        assert_eq!(auto.tick(true, Some(&free), later + BRAKE_RELEASED), bits(&["frontsuspdwn", "rearsuspdwn"]));
    }

    #[test]
    fn levels_the_rear_under_a_load() {
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        let mut parked = driving(0.0);
        auto.tick(true, Some(&parked), start);
        parked.trailer_connected = true;
        parked.suspension = [0.1, 0.15, 0.1];
        auto.tick(true, Some(&parked), start + Duration::from_secs(1));
        assert_eq!(auto.tick(true, Some(&parked), start + Duration::from_secs(5)), bits(&["rearsuspup"]));
    }

    #[test]
    fn not_with_it_off_the_engine_off_or_the_player_pressing() {
        let start = Instant::now();
        let mut auto = AutoSuspension::default();
        auto.tick(false, Some(&driving(90.0)), start);
        assert_eq!(auto.tick(false, Some(&driving(90.0)), start + AFTER), 0);
        let off = Telemetry { engine_enabled: false, ..driving(90.0) };
        auto.tick(true, Some(&off), start);
        assert_eq!(auto.tick(true, Some(&off), start + AFTER), 0);
        auto.manual(start);
        auto.tick(true, Some(&driving(90.0)), start);
        assert_eq!(auto.tick(true, Some(&driving(90.0)), start + AFTER), 0);
    }
}
