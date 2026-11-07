//! Measurements for the walk's timing (`docs/personagem/TRANSICOES.md` 7.7, test B): written to the log only,
//! nothing of how the person walks or is shown changes.
//!
//! - **The capsule's speed as a move key goes down or up** (W, S, A, D): every frame for [`BURST`] seconds, the
//!   speed from where the capsule went (as the plugin reads it to walk and turn), the physics' own velocity, and
//!   whether the person counts as moving (the debounced one that switches the animation). So how fast the walking
//!   camera starts and stops, frame by frame, is known before the animations' clocks are tied to it.
//! - **What placing the people costs** each frame (`person::place`: each person moved and its animation updated,
//!   shown or not), every [`EVERY`] seconds: the average and the worst frame, and how many people there are.

use std::time::{Duration, Instant};

use crate::log;

/// How long after a move key goes down or up the speed is recorded (seconds).
const BURST: f32 = 1.5;
/// How often what placing the people costs is written.
const EVERY: Duration = Duration::from_secs(10);
/// The speed the person counts as still under (m/s: `character.rs` `TURNS_ABOVE`), for the summary.
const STILL: f32 = 0.3;

/// One frame of a burst: since the key (s), the frame's length (s), the speed from where it went and the physics'
/// (m/s, `None`: not read), and whether moving.
struct Sample {
    at: f32,
    dt: f32,
    went: f32,
    physics: Option<f32>,
    moving: bool,
}

struct Burst {
    what: &'static str,
    samples: Vec<Sample>,
}

/// How long after a jump leaves the ground its height is recorded (seconds).
const JUMP_BURST: f32 = 1.2;

/// A jump's frames: since it left (s), the walking camera's height over where it left (m: where the body is put
/// from), and the capsule's speed up (m/s, as the physics says).
#[derive(Default)]
struct Jump {
    from: Option<f64>,
    frames: Vec<(f32, f64, f32)>,
}

#[derive(Default)]
pub(super) struct Meter {
    jump: Option<Jump>,
    /// The move keys down last frame (a bit each: W, S, A, D).
    keys: u8,
    burst: Option<Burst>,
    /// Placing the people since the last line: frames, all their time, the worst, the people last placed.
    frames: u32,
    total: Duration,
    worst: Duration,
    people: usize,
    since: Option<Instant>,
}

impl Meter {
    /// A frame on foot: the move keys down now (`keys`), the frame's length (`dt`, s), the speed from where the
    /// capsule went (m/s), the physics' own velocity along the ground (m/s, if read), and whether moving.
    pub(super) fn motion(&mut self, keys: u8, dt: f32, went: f32, physics: Option<f32>, moving: bool) {
        if keys != self.keys {
            let what = match (self.keys, keys) {
                (0, _) => "set off",
                (_, 0) => "let go",
                _ => "keys changed",
            };
            if let Some(burst) = self.burst.take() {
                write(&burst);
            }
            self.burst = Some(Burst { what, samples: Vec::new() });
            self.keys = keys;
        }
        let Some(burst) = self.burst.as_mut() else { return };
        let at = burst.samples.last().map_or(0.0, |last| last.at + dt);
        burst.samples.push(Sample { at, dt, went, physics, moving });
        if at >= BURST
            && let Some(burst) = self.burst.take()
        {
            write(&burst);
        }
    }

    /// The physics' jump left the ground this frame: its height is recorded from now (`height`).
    pub(super) fn jumped(&mut self) {
        self.jump = Some(Jump::default());
    }

    /// A frame of a jump being recorded: the walking camera's height (m, the world's) and the capsule's speed up.
    /// The body is put from that camera: the jump shows only as far as it goes up (the owner's video, 2026-11-06:
    /// the feet never left the ground, `docs/personagem/TRANSICOES.md` 7.10).
    pub(super) fn height(&mut self, dt: f32, y: f64, up: Option<f32>) {
        let Some(jump) = self.jump.as_mut() else { return };
        let from = *jump.from.get_or_insert(y);
        let at = jump.frames.last().map_or(0.0, |last| last.0 + dt);
        jump.frames.push((at, y - from, up.unwrap_or(f32::NAN)));
        if at >= JUMP_BURST {
            let jump = self.jump.take().unwrap_or_default();
            let top = jump.frames.iter().map(|f| f.1).fold(f64::MIN, f64::max);
            let frames: Vec<String> =
                jump.frames.iter().map(|(t, dy, vy)| format!("{t:.3}:{dy:+.3}/{vy:+.2}")).collect();
            log!(
                "measure: jump height (the walking camera, the body's place): highest {top:+.3} m; \
                 s:height/speed up: {}",
                frames.join(" ")
            );
        }
    }

    /// Placing the people took `took`, `people` of them.
    pub(super) fn placed(&mut self, took: Duration, people: usize) {
        let now = Instant::now();
        let since = *self.since.get_or_insert(now);
        self.frames += 1;
        self.total += took;
        self.worst = self.worst.max(took);
        self.people = people;
        if now.duration_since(since) >= EVERY {
            log!(
                "measure: placing the people ({}): {} frames, {:.0} µs a frame on average, the worst {:.0} µs",
                self.people,
                self.frames,
                self.total.as_secs_f64() * 1e6 / f64::from(self.frames.max(1)),
                self.worst.as_secs_f64() * 1e6
            );
            *self = Meter {
                keys: self.keys,
                burst: self.burst.take(),
                jump: self.jump.take(),
                since: Some(now),
                ..Meter::default()
            };
        }
    }
}

/// A burst in the log: when the speed went under or over [`STILL`] and when the person stopped or started
/// moving, then every frame (`seconds: from where it went / the physics' m/s, M moving`).
fn write(burst: &Burst) {
    let first = |test: &dyn Fn(&Sample) -> bool| {
        burst.samples.iter().find(|s| test(s)).map_or("never".into(), |s| format!("{:.2} s", s.at))
    };
    let summary = if burst.what == "let go" {
        format!(
            "under {STILL} m/s at {}, the physics' at {}, not moving at {}",
            first(&|s| s.went < STILL),
            first(&|s| s.physics.is_some_and(|v| v < STILL)),
            first(&|s| !s.moving)
        )
    } else {
        let top = burst.samples.iter().map(|s| s.went).fold(0.0, f32::max);
        format!(
            "over {STILL} m/s at {}, 90% of {top:.2} m/s at {}, moving at {}",
            first(&|s| s.went > STILL),
            first(&|s| s.went >= 0.9 * top),
            first(&|s| s.moving)
        )
    };
    let frames: Vec<String> = burst
        .samples
        .iter()
        .map(|s| {
            let physics = s.physics.map_or("-".into(), |v| format!("{v:.2}"));
            format!("{:.3}+{:.3}:{:.2}/{physics}{}", s.at, s.dt, s.went, if s.moving { "M" } else { "" })
        })
        .collect();
    log!("measure: {} ({summary}): {}", burst.what, frames.join(" "));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_burst_starts_with_the_keys_and_ends_after_its_time() {
        let mut meter = Meter::default();
        meter.motion(0, 0.016, 0.0, Some(0.0), false);
        assert!(meter.burst.is_none());
        meter.motion(1, 0.016, 0.0, Some(0.0), false);
        assert_eq!(meter.burst.as_ref().map(|b| b.what), Some("set off"));
        for _ in 0..200 {
            meter.motion(1, 0.016, 1.6, Some(1.6), true);
        }
        // Done after BURST seconds, written and let go.
        assert!(meter.burst.is_none());
        meter.motion(0, 0.016, 1.0, Some(1.0), true);
        assert_eq!(meter.burst.as_ref().map(|b| b.what), Some("let go"));
        meter.motion(4, 0.016, 1.0, Some(1.0), true);
        assert_eq!(meter.burst.as_ref().map(|b| b.what), Some("set off"));
    }

    #[test]
    fn placing_is_summed_and_kept_between_lines() {
        let mut meter = Meter::default();
        meter.placed(Duration::from_micros(30), 10);
        meter.placed(Duration::from_micros(90), 10);
        assert_eq!(meter.frames, 2);
        assert_eq!(meter.worst, Duration::from_micros(90));
        assert_eq!(meter.people, 10);
    }
}
