//! Getting back in the cab in first person (owner 2026-11-05: "only the camera going up, as if the character
//! were climbing into the truck"): from where camera 0 is, to the driver's door on the ground, up its step,
//! through the doorway onto the cab's floor and down onto the seat, where the game's cab camera takes over. No
//! animation of a body: the person is hidden for it (the game's people can't climb, `PERSONAGEM_PROPRIO.md`).

use super::{DOOR_AHEAD, DOOR_OUT, EYE_HEIGHT, WALK_SPEED, heading_of, in_truck};
use crate::telemetry::Truck;

/// Out of the driver's head along the cab's side (metres): on the step, and in the doorway (inside the cab).
const STEP_OUT: f64 = 0.75;
const DOORWAY_OUT: f64 = 0.35;
/// The eyes standing on the cab's floor, over the seated head (metres).
const STANDING_OVER_SEATED: f64 = 0.4;
/// Looking up at the step and the handle, then ahead (radians, up positive).
const LOOK_UP_STEP: f32 = 0.25;
const LOOK_UP_DOOR: f32 = 0.1;
/// How long each part takes (seconds): up the step, through the doorway, onto the seat; walking to the door
/// at a person's pace, at least this.
const UP_THE_STEP: f32 = 0.6;
const THROUGH_THE_DOOR: f32 = 0.6;
const ONTO_THE_SEAT: f32 = 0.7;
const TO_THE_DOOR: f32 = 0.3;

/// A place on the way: where the eyes are (world metres), where they look (heading and pitch, radians, as the
/// walking look keeps them), how long getting there from the last takes, and whether a foot lands there.
#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub at: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub secs: f32,
    pub step: bool,
}

/// The climb under way: its places, and how long it has gone (seconds).
pub struct Climb {
    pub keys: Vec<Key>,
    pub time: f32,
    /// Whether it starts behind the person (third person): the camera into their eyes first, the body hidden there.
    pub approach: bool,
}

/// From behind the person into their eyes (third person, climbing in), quickly: seconds a metre, and at least and
/// at most (owner 2026-11-14: the body went at once and the camera took its time from afar).
const APPROACH_PER_METRE: f32 = 0.12;
const APPROACH_LEAST: f32 = 0.25;
const APPROACH_MOST: f32 = 0.6;

impl Climb {
    /// From `from` (where camera 0 is), looking `yaw`, `pitch`, into the truck's cab.
    pub fn new(from: [f64; 3], yaw: f32, pitch: f32, truck: &Truck) -> Self {
        Climb { keys: path(from, yaw, pitch, truck), time: 0.0, approach: false }
    }

    /// From `from` (camera 0, behind the person in third person) into their `eyes` first, then as `new` from
    /// there: the person seen till the camera is where they look from.
    pub fn from_behind(from: [f64; 3], eyes: [f64; 3], yaw: f32, pitch: f32, truck: &Truck) -> Self {
        let apart = (from[0] - eyes[0]).hypot(from[1] - eyes[1]).hypot(from[2] - eyes[2]) as f32;
        let secs = (apart * APPROACH_PER_METRE).clamp(APPROACH_LEAST, APPROACH_MOST);
        let mut keys = vec![
            Key { at: from, yaw, pitch, secs: 0.0, step: false },
            Key { at: eyes, yaw, pitch, secs, step: false },
        ];
        keys.extend(path(eyes, yaw, pitch, truck).into_iter().skip(1));
        Climb { keys, time: 0.0, approach: true }
    }

    /// Whether the camera reached the person's eyes in the last `dt` seconds (`from_behind`).
    pub fn reached_eyes(&self, dt: f32) -> bool {
        self.approach && self.passed(1, dt)
    }

    /// The place where the door opens: on the ground by it (one later from behind the person).
    pub fn door_key(&self) -> usize {
        if self.approach { 2 } else { 1 }
    }

    /// Out of the truck's cab (owner 2026-11-05: "an animation getting out, not all at once"): from the seat, up
    /// into the doorway, down the step, onto the ground by the door, looking out.
    pub fn out(truck: &Truck) -> Self {
        Climb { keys: path_out(truck), time: 0.0, approach: false }
    }

    /// Where the way ends: the eyes and their look.
    pub fn end(&self) -> Option<([f64; 3], f32, f32)> {
        self.keys.last().map(|key| (key.at, key.yaw, key.pitch))
    }

    /// Whether the way reached its place `key` (counted from its start) in the last `dt` seconds of `go`.
    pub fn passed(&self, key: usize, dt: f32) -> bool {
        let end: f32 = self.keys.iter().take(key + 1).map(|k| k.secs).sum();
        self.time - dt < end && end <= self.time
    }

    /// Goes on by `dt` seconds: where the eyes are and look (none once on the seat), and whether a foot landed.
    pub fn go(&mut self, dt: f32) -> (Option<([f64; 3], f32, f32)>, bool) {
        let before = self.time;
        self.time += dt;
        let mut end = 0.0;
        let mut landed = false;
        for key in &self.keys {
            end += key.secs;
            landed |= key.step && before < end && end <= self.time;
        }
        (view(&self.keys, self.time), landed)
    }
}

/// The cab's places on the way in and out (world metres): the ground by the door, the step, the doorway, the
/// seat; the truck's own way, and toward the cab from outside its door (headings).
struct Places {
    ground: [f64; 3],
    step: [f64; 3],
    doorway: [f64; 3],
    seat: [f64; 3],
    ahead: f32,
    toward_cab: f32,
}

fn places(truck: &Truck) -> Places {
    let [hx, hy, hz] = truck.head.map(f64::from);
    let side = if hx > 0.0 { 1.0 } else { -1.0 };
    let turn = f64::from(truck.heading) * std::f64::consts::TAU;
    let standing = hy + STANDING_OVER_SEATED;
    Places {
        ground: in_truck(truck, [hx + side * f64::from(DOOR_OUT), EYE_HEIGHT, hz - f64::from(DOOR_AHEAD)]),
        step: in_truck(truck, [hx + side * STEP_OUT, (EYE_HEIGHT + standing) / 2.0, hz - f64::from(DOOR_AHEAD)]),
        doorway: in_truck(truck, [hx + side * DOORWAY_OUT, standing, hz - f64::from(DOOR_AHEAD) / 2.0]),
        seat: in_truck(truck, [hx, hy, hz]),
        ahead: heading_of(-turn.sin(), -turn.cos()),
        toward_cab: heading_of(-side * turn.cos(), side * turn.sin()),
    }
}

/// Getting out, looking down at the step and the ground, then ahead (radians, down negative); how long each part
/// takes (seconds): up off the seat into the doorway, down the step, onto the ground.
const LOOK_DOWN_STEP: f32 = -0.45;
const LOOK_DOWN_GROUND: f32 = -0.2;
const OFF_THE_SEAT: f32 = 0.6;
const DOWN_THE_STEP: f32 = 0.55;
const ONTO_THE_GROUND: f32 = 0.6;

/// The places out: from the seat (looking ahead), into the doorway, down the step, onto the ground by the door
/// (where the walk starts), turned away from the cab.
fn path_out(truck: &Truck) -> Vec<Key> {
    let Places { ground, step, doorway, seat, ahead, toward_cab } = places(truck);
    let out = (toward_cab + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
    let key = |at, yaw, pitch, secs, step| Key { at, yaw, pitch, secs, step };
    vec![
        key(seat, ahead, 0.0, 0.0, false),
        key(doorway, out, LOOK_DOWN_STEP, OFF_THE_SEAT, false),
        key(step, out, LOOK_DOWN_STEP, DOWN_THE_STEP, true),
        key(ground, out, LOOK_DOWN_GROUND, ONTO_THE_GROUND, true),
    ]
}

/// The places: from `from`, to the door on the ground, up the step, through the doorway, onto the seat.
fn path(from: [f64; 3], yaw: f32, pitch: f32, truck: &Truck) -> Vec<Key> {
    let Places { ground, step, doorway, seat, ahead, toward_cab } = places(truck);
    let walk = (from[0] - ground[0]).hypot(from[2] - ground[2]).hypot(from[1] - ground[1]) as f32 / WALK_SPEED;
    let key = |at, yaw, pitch, secs, step| Key { at, yaw, pitch, secs, step };
    vec![
        key(from, yaw, pitch, 0.0, false),
        key(ground, toward_cab, LOOK_UP_DOOR, walk.max(TO_THE_DOOR), false),
        key(step, toward_cab, LOOK_UP_STEP, UP_THE_STEP, true),
        key(doorway, toward_cab, LOOK_UP_DOOR, THROUGH_THE_DOOR, true),
        key(seat, ahead, 0.0, ONTO_THE_SEAT, false),
    ]
}

/// Where the eyes are and look `time` seconds into the climb (none past its end): from place to place, eased
/// in and out (a person sets off and slows down at each).
fn view(keys: &[Key], time: f32) -> Option<([f64; 3], f32, f32)> {
    let mut start = 0.0;
    for pair in keys.windows(2) {
        let [from, to] = [pair[0], pair[1]];
        if time < start + to.secs {
            let u = ((time - start) / to.secs).clamp(0.0, 1.0);
            let s = u * u * (3.0 - 2.0 * u);
            let at = std::array::from_fn(|i| from.at[i] + (to.at[i] - from.at[i]) * f64::from(s));
            let off =
                (to.yaw - from.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            let yaw = (from.yaw + off * s).rem_euclid(std::f32::consts::TAU);
            return Some((at, yaw, from.pitch + (to.pitch - from.pitch) * s));
        }
        start += to.secs;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn truck() -> Truck {
        Truck {
            position: [100.0, 10.0, 200.0],
            heading: 0.0,
            speed: 0.0,
            pitch: 0.0,
            roll: 0.0,
            parking_brake: true,
            multiplayer: false,
            head: [-0.6, 2.5, -1.8],
        }
    }

    #[test]
    fn the_climb_goes_from_the_camera_up_the_step_onto_the_seat() {
        let from = [97.0, 11.8, 199.0];
        let mut climb = Climb::new(from, 1.0, 0.0, &truck());
        let (first, _) = climb.go(0.0);
        assert_eq!(first.unwrap().0, from);
        // Up all the way, a foot landing twice (the step, the cab's floor), then the seat: the driver's head.
        let mut landed = 0;
        let mut last = None;
        let mut highest: f64 = 0.0;
        while let (Some(view), foot) = climb.go(0.05) {
            landed += usize::from(foot);
            highest = highest.max(view.0[1]);
            last = Some(view);
        }
        assert_eq!(landed, 2);
        let (at, yaw, pitch) = last.unwrap();
        assert!((at[0] - 99.4).abs() < 0.05 && (at[1] - 12.5).abs() < 0.05 && (at[2] - 198.2).abs() < 0.05, "{at:?}");
        assert!((yaw.min(std::f32::consts::TAU - yaw)) < 0.1 && pitch.abs() < 0.05);
        // Standing in the doorway over the seated head.
        assert!((highest - 12.9).abs() < 0.01);
        assert!(climb.go(0.05).0.is_none());
    }

    #[test]
    fn the_door_is_looked_at_from_outside_it() {
        // Facing north, the left-hand door to the west: looking east, into the cab (heading 3π/2).
        let keys = path([97.0, 11.8, 199.0], 0.0, 0.0, &truck());
        assert!((keys[1].yaw - 3.0 * std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!((keys[1].at[0] - 98.4).abs() < 1e-6 && (keys[1].at[1] - 11.8).abs() < 1e-6);
    }

    #[test]
    fn getting_out_goes_from_the_seat_down_the_step_onto_the_ground_where_the_walk_starts() {
        let truck = truck();
        let mut out = Climb::out(&truck);
        let (first, _) = out.go(0.0);
        // From the driver's head, to the ground by the door (the walk's start: `walk_start`'s place).
        assert!((first.unwrap().0[0] - 99.4).abs() < 1e-6);
        let (end, yaw, pitch) = out.end().unwrap();
        assert_eq!(end, places(&truck).ground);
        // Turned away from the cab, looking a little down.
        let off = (yaw - places(&truck).toward_cab).rem_euclid(std::f32::consts::TAU);
        assert!((off - std::f32::consts::PI).abs() < 1e-4 && pitch < 0.0);
        // Down all the way in under 2 s, a foot landing twice (the step, the ground), never under the ground.
        // (The ground's landing comes with the way's end: `climb_frame` plays it before it looks at the end.)
        let (mut landed, mut lowest, mut secs) = (0, f64::MAX, 0.0);
        loop {
            let (view, foot) = out.go(0.05);
            landed += usize::from(foot);
            let Some(view) = view else { break };
            lowest = lowest.min(view.0[1]);
            secs += 0.05;
        }
        assert_eq!(landed, 2);
        assert!(lowest >= end[1] - 1e-6 && secs < 2.0, "{lowest} {secs}");
    }
}
