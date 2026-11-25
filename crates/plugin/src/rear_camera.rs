//! Our own reversing camera, made from one of the game's mirrors (owner 2026-11-14: "our own camera, behind the
//! truck and behind the trailer"; docs/camera-de-re/RETROVISOR.md, step 3c).
//!
//! The game draws each mirror into a target of its own (the two main ones 256 x 512, R11G11B10), and gives each
//! draw, in the vertex shader's b0 (a ring buffer, `VSSetConstantBuffers1` with an offset), a block with the object
//! seen from the mirror: rows 0-3 its modelview (camera-relative), rows 4-7 and 8-11 that times two projections.
//! While the reversing camera is on the screen, as each draw of the frame's second main mirror binds its block, the
//! block is opened (`Map` NO_OVERWRITE: the game writes it the same way) and rewritten as seen from a camera moved
//! by [`MOVE`]: the modelview turned and moved ([`camera`]), the projections found from the block itself
//! (P = PMV · MV⁻¹) and applied again. The game culls the objects for its own mirror, so what that one doesn't see
//! may be missing.
//!
//! First try (2026-11-14 12:06): the camera moved 12 m the way the mirror looks. The picture moved as it should
//! (60 fps), but the mirror looks down a little, so the camera went under the ground (grey, owner's pictures).
//! Second (12:30): it moves [`FORWARD`] metres level, the way the mirror looks seen from above. The world's
//! "level" comes from the mirror's own turn (`TURN`), read from a block of an object at the camera (the sky's:
//! translation nil).
//! Third (12:40): the camera is placed and turned ([`plan`]): at the middle of the truck's back (or the trailer's,
//! coupled), looking straight back along it and [`PITCH`] down, from the SDK's places and headings (back is
//! (sin, 0, cos) of the heading). The mirror's side is the side it looks out to.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

use windows::Win32::Graphics::Direct3D11::{
    D3D11_MAP_WRITE_NO_OVERWRITE, D3D11_MAPPED_SUBRESOURCE, ID3D11Buffer, ID3D11DeviceContext,
};
use windows::core::Interface;

use crate::log;

/// How far the camera moves, level, the way the mirror looks (metres).
const FORWARD: f32 = 14.0;
/// How far down the camera looks (degrees).
const PITCH: f32 = 22.0;
/// How wide our camera sees across the screen (degrees; owner 2026-11-14: "open it up, like a wider field of
/// view"), and the screen's picture's shape (wide over tall) for how far it sees up and down.
const WIDE: f32 = 120.0;
const SCREEN_SHAPE: f32 = 1.28;
/// The page's picture's shape as last told (`mirror::shape`; f32 bits), else SCREEN_SHAPE.
static SHAPE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn shape() -> f32 {
    match SHAPE.load(Ordering::Relaxed) {
        0 => SCREEN_SHAPE,
        bits => f32::from_bits(bits).clamp(0.5, 4.0),
    }
}
/// The camera's height over the ground (the truck's origin), behind the truck and behind a trailer (metres).
/// At the bumpers, about the tyres' tops (owner 2026-11-14: at 1.8 and 3.2 m it went into the trailer's deck; the
/// truck's at 1.05 still did a little).
const HEIGHT_TRUCK: f32 = 0.85;
const HEIGHT_TRAILER: f32 = 1.1;
/// The back of a truck past its fifth wheel, and of a trailer past its origin (by its rear axles) (metres).
/// (Without the SDK's layout, `reversing::VEHICLES`: with it, each one's rear end, worked out from its last axle.)
const TRUCK_PAST_HOOK: f32 = 1.4;
const TRAILER_PAST_ORIGIN: f32 = 2.0;
/// How far past the rear end the camera is (metres; owner 2026-11-14: 2 m past the trailer's origin was under it,
/// between its axles: its rear is 5.5 m past).
const PAST_REAR: f32 = 0.0;
/// 0 since 2026-11-14 14:50: the camera at the parking sensors' own place (the rear end they measure from, owner:
/// "the sensor is right whatever the trailer"); the metre in made up for a guessed mirror place, measured since.
/// How far ahead of a trailer's rear end (as worked out from its last axle) the camera is, metres: that end comes
/// out about a metre too far back (owner 2026-11-14 13:45: the camera nearly at the fuel pump behind the bumper).
const TRAILER_IN: f32 = 0.0;
/// Behind a trailer, the camera goes at least this far past the rear end worked out from its last axle (metres): the
/// rays measure the game's collision box, which on some trailers ends a metre short of the body (owner 2026-11-20, a
/// random trailer: measured 0.99 m ahead of the axles' guess, the camera inside the trailer).
const TRAILER_CAMERA_PAST_GUESS: f32 = 0.3;
/// A mirror's place from the driver's head, in the truck's own space: out to its side, up, forward (metres).
const MIRROR_OUT: f32 = 1.45;
const MIRROR_UP: f32 = 0.15;
const MIRROR_FORWARD: f32 = 0.3;


/// Whether the camera is being moved (the reversing camera on the screen, the mirrors drawn).
static ON: AtomicBool = AtomicBool::new(false);
/// Whether our camera is on (the screen shows it alone: `mirror::ours_place`).
pub fn active() -> bool {
    ON.load(Ordering::Relaxed)
}

/// The frame our camera came on at, and how many frames on before it's ready to be seen (its view settled from the
/// frames' votes): the page covers the screen until then (owner 2026-11-19: "going into reverse the screen shows the
/// map, then the plain mirror, then the camera").
static ON_AT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const READY_AFTER: u64 = 20;

/// Whether our camera has been on long enough to be shown.
pub fn ready() -> bool {
    active()
        && PRESENTS.load(Ordering::Relaxed).saturating_sub(ON_AT.load(Ordering::Relaxed)) >= READY_AFTER
        // F2's presses done and our mirror held (the log, 2026-11-19: on at 63.97 s, F2 again at 64.24, our mirror
        // switched five times till 64.97).
        && crate::virtual_toggle::done_for_reversing()
        && crate::mirror::ours_steady()
}

/// Where our picture is in what the screen is composed from (u, v, top left to bottom right): all of our mirror's
/// own target (`mirror` copies it: every truck has one; the mirrors put together only some).
pub const PLACE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
/// How bright the picture's mean is shown (before toning, 1 - e^(-x)): the mirror's picture is exposed to it.
pub const EXPOSURE: f32 = 0.35;
/// How the screen turns our picture back: its x and y swapped, then flipped along each (set from the owner's
/// pictures).
/// The second flips across: the picture came out as seen looking back (the fuel pump on the trailer's right showed
/// on the left, owner 2026-11-14), so the guide lines (drawn as a reversing camera shows, its right on the right)
/// curved the wrong way with the truck alone. Flipped, it's shown as the mirrors and a car's camera show it.
/// From the mirror's own target (2026-11-14 13:55; before, the mirrors put together, which show it the other way up):
/// swapped only, the flip across now in the target's being upside down.
pub const FLIP: [bool; 2] = [false, false];

static LOGGED: AtomicBool = AtomicBool::new(false);
/// Whether the next plan is written to the log (every 2 s while on: what it was made from).
static LOG_PLAN: AtomicBool = AtomicBool::new(false);

/// Our mirror's turn (world to view: its rows the view's axes in the world, y up). Not the sky's: the sky is a
/// model turned on its own (z up; 2026-11-14 the camera looked the right way only with the truck facing north).
/// The one most of the mirror's objects have: the ground's and the roads' (no turn of their own), counted in a
/// frame (`VOTES`) for the next.
static TURN: Mutex<Option<[[f32; 3]; 3]>> = Mutex::new(None);

/// This frame's objects' turns, counted: rounded first two rows → how many, and the turn.
static VOTES: LazyLock<Mutex<std::collections::HashMap<[i32; 6], (u32, [[f32; 3]; 3])>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// A block's object's turn counted, if it is a true turn (unit rows at right angles, not mirrored).
/// The truck's own axes in the view's world (rows: its right, up and back), as the SDK said at the last present.
/// Tilted as the truck is (its pitch and roll: on a slope the level axes were 0.08 off and nothing was measured,
/// owner 2026-11-14), each of the four ways the SDK's pitch and roll may turn (the one the parts match is used).
static TRUCK_AXES: Mutex<Option<[[[f32; 3]; 3]; 4]>> = Mutex::new(None);
/// Where the truck's own parts (their origin the truck's) are seen from our mirror this frame: rounded → count, place.
static CHASSIS_VOTES: LazyLock<Mutex<std::collections::HashMap<[i32; 3], (u32, [f32; 3])>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
/// The view's turn taken (looking back) since the last log, and the most voted one last refused (looking elsewhere).
static TURNS_TAKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static TURN_REFUSED: Mutex<Option<[[f32; 3]; 3]>> = Mutex::new(None);
/// The blocks near the mirror looked at as the truck's parts, and how far the closest one's turn was from the
/// truck's (since the last log: why the mirror's place wasn't measured, owner 2026-11-14).
static CHASSIS_NEAR: Mutex<(u64, f32)> = Mutex::new((0, f32::MAX));
/// What was measured of each mirror, known by where it looks (level, unit): its side, and up and back. Not by its
/// texture: the game hands its mirrors' pictures round a few textures, one holding the left mirror's view now and the
/// right's a moment later as the head turns (owner 2026-11-14: four textures for two mirrors, the measurements mixed,
/// the camera put 2.7 m off, among the wheels).
static MEASURED: Mutex<Vec<([f32; 2], Option<f32>, Option<[f32; 2]>)>> = Mutex::new(Vec::new());
/// Where the mirror the measurements now are of looks.
static LOOK_OF: Mutex<Option<[f32; 2]>> = Mutex::new(None);
/// Two looks the same mirror's (within about 5 degrees): the two main mirrors look 15 or more apart.
const SAME_LOOK: f32 = 0.996;

/// Where a view's turn looks, level and unit.
fn flat_look(turn: &[[f32; 3]; 3]) -> [f32; 2] {
    let look = [-turn[2][0], -turn[2][2]];
    let length = (look[0] * look[0] + look[1] * look[1]).sqrt().max(1e-6);
    [look[0] / length, look[1] / length]
}

/// The mirror's turn as this frame's own blocks vote it (looking back), if enough voted.
/// Only a turn the truck's own parts seen in the mirror agree with (each part's turn, undone by the truck's, is the
/// mirror's): the town's houses, lined up with the street, passed for the mirror with the truck lined up the same way
/// (owner 2026-11-14: turning hard in reverse, the camera flashed ahead); a truck's mirror always sees the truck.
fn voted_turn(behind: [f32; 3]) -> Option<[[f32; 3]; 3]> {
    let truck_turns = TRUCK_TURNS.lock().unwrap_or_else(|e| e.into_inner());
    let supported = |turn: &[[f32; 3]; 3]| -> u32 {
        truck_turns
            .values()
            .filter(|(_, seen)| (0..3).all(|r| (0..3).all(|c| (seen[r][c] - turn[r][c]).abs() < 0.04)))
            .map(|(count, _)| *count)
            .sum()
    };
    VOTES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .filter(|(count, turn)| *count >= 4 && looks_back(turn, behind) && supported(turn) >= 3)
        .max_by_key(|(count, _)| *count)
        .map(|(_, turn)| *turn)
}

/// Each block's turn undone by the truck's (the mirror's turn, if the block is the truck's), counted (`voted_turn`).
static TRUCK_TURNS: LazyLock<Mutex<std::collections::HashMap<[i32; 6], (u32, [[f32; 3]; 3])>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Whether our mirror's picture just drawn was of the mirror our camera was moved for: its own blocks vote the turn
/// it was moved by. Not (the game gave the texture the other mirror's view), and it isn't shown: it'd be our camera
/// moved from the wrong mirror's place.
pub fn view_matches() -> bool {
    let Some(used) = *TURN.lock().unwrap_or_else(|e| e.into_inner()) else { return false };
    let behind = back(crate::telemetry::truck().heading, (1.0, 1.0));
    voted_turn(behind).is_some_and(|now| {
        let (a, b) = (flat_look(&used), flat_look(&now));
        a[0] * b[0] + a[1] * b[1] > SAME_LOOK
    })
}
/// Which side our mirror's view was of, as its blocks voted last (-1 the truck's left, 1 its right, 0 not known), and
/// the frames in a row it was of the other side than the driver's (owner 2026-11-18: the camera always from the left
/// one, the driver's side, so the right one can be a virtual mirror of the player's: `hide_virtual`).
static OURS_SIDE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
static WRONG_SIDE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Frames our mirror may hold the other side's view before another is taken for ours (`mirror`): 4 frames (~0.1 s)
/// so the very first reverse switches to the driver's mirror almost at once (owner 2026-11-22: 3 s black screen).
const WRONG_SIDE_FRAMES: u64 = 4;
/// Blocks read a frame of each main mirror, only to know which side its view is of (which virtual mirror to hide,
/// all the time); the most voted turn needs a handful.
const SIDE_VOTES: u32 = 64;

/// The side the driver sits on (-1 the truck's left, 1 its right): the head's place across the cab.
pub fn driver_side() -> i32 {
    if crate::telemetry::truck().head[0] > 0.0 { 1 } else { -1 }
}

/// Which side our mirror's view is of now (-1 left, 1 right, 0 not known yet).
pub fn ours_side() -> i32 {
    OURS_SIDE.load(Ordering::Relaxed)
}

/// Whether our mirror has held the other side's view a while (another mirror is to be ours: `mirror`); asked, it's
/// counted afresh.
pub fn ours_wrong_side() -> bool {
    if WRONG_SIDE.load(Ordering::Relaxed) >= WRONG_SIDE_FRAMES {
        WRONG_SIDE.store(0, Ordering::Relaxed);
        return true;
    }
    false
}

/// The side a mirror's view looks out to (-1 the truck's left, 1 its right), by its turn (as `plan` places it).
fn side_of(turn: &[[f32; 3]; 3], behind: [f32; 3]) -> i32 {
    let look = flat_look(turn);
    let right = [behind[2], 0.0, -behind[0]];
    if look[0] * right[0] + look[1] * right[2] >= 0.0 { 1 } else { -1 }
}

/// How far out to its side the mirror is from the truck's middle (metres), measured (`chassis`), smoothed.
static MIRROR_SIDE: Mutex<Option<f32>> = Mutex::new(None);

/// The player's own moves of the camera (owner 2026-11-20: "can I adjust the reversing camera myself?"), each truck's
/// and trailer's (`nudge_key`): metres further back and higher. Kept in the game's WestPlay folder.
static NUDGES: Mutex<Option<Nudges>> = Mutex::new(None);
/// The one in use now (f32 bits: back, up, tilt, turn, wide, side, roll), for `plan`.
static NUDGE_NOW: [std::sync::atomic::AtomicU32; 7] = [const { std::sync::atomic::AtomicU32::new(0) }; 7];
/// One press: 5 cm fine step (owner 2026-11-20), 50 cm fast step for long trailers (owner 2026-11-24: up to 25 m
/// back for bitrem / rodotrem), and ±6 m for height and sideways offset.
const NUDGE_STEP: f32 = 0.05;
const NUDGE_FAR_STEP: f32 = 0.50;
const NUDGE_BACK_MOST: f32 = 25.0;
const NUDGE_MOST: f32 = 6.0;
/// The camera turned by the player (owner 2026-11-20: "if it looks too far down, turn it up; and to the sides; and a
/// wider view"; owner 2026-11-24: lateral tilt / roll): degrees a press and the most either way.
const TILT_STEP: f32 = 1.0;
const TILT_MOST: f32 = 30.0;
const TURN_STEP: f32 = 2.0;
const TURN_MOST: f32 = 45.0;
const ROLL_STEP: f32 = 1.0;
const ROLL_MOST: f32 = 45.0;
const WIDE_STEP: f32 = 5.0;
const WIDE_LEAST: f32 = -60.0;
const WIDE_MOST: f32 = 40.0;
/// What is kept for a truck or trailer: back, up (metres), tilt, turn, wide (degrees), side (metres), roll (degrees).
type Nudge = [f32; 7];

#[derive(Default)]
struct Nudges {
    /// The host's count of presses last seen (`None`: none yet: the ones before the plugin came aren't taken).
    seen: Option<u32>,
    by_key: std::collections::BTreeMap<String, Nudge>,
}

fn nudges_file() -> std::path::PathBuf {
    crate::log::game_dir().join("camera-de-re.txt")
}

/// Each line `key back up tilt turn wide side roll` (older 2- and 5-number lines still read cleanly).
fn read_nudges(text: &str) -> std::collections::BTreeMap<String, Nudge> {
    text.lines()
        .filter_map(|line| {
            let mut words: Vec<&str> = line.trim().split(' ').collect();
            let mut numbers = Vec::new();
            while numbers.len() < 7 && words.len() > 1 {
                let Some(number) = words.last().and_then(|w| w.parse::<f32>().ok()).filter(|n| n.is_finite()) else {
                    break;
                };
                numbers.insert(0, number);
                words.pop();
            }
            let key = words.join(" ");
            if key.trim().is_empty() || numbers.len() < 2 {
                return None;
            }
            let mut kept = [0.0; 7];
            kept[..numbers.len()].copy_from_slice(&numbers);
            Some((key.trim().to_owned(), kept))
        })
        .collect()
}

fn write_nudges(by_key: &std::collections::BTreeMap<String, Nudge>) -> String {
    by_key
        .iter()
        .map(|(key, [back, up, tilt, turn, wide, side, roll])| {
            format!("{key} {back} {up} {tilt} {turn} {wide} {side} {roll}\n")
        })
        .collect()
}

/// Which truck or trailer the camera is on: a trailer by its layout (its kingpin, axles and width, to the
/// centimetre: the SDK names none), the truck alone by its model.
fn nudge_key(t: &westplay_shared::Telemetry, truck_id: &str) -> String {
    if t.trailer_connected && t.trailer_half_width > 0.0 {
        format!("trailer:{:.2}:{:.2}:{:.2}", t.trailer_hook_z, t.trailer_axle_z, t.trailer_half_width)
    } else {
        format!("truck:{truck_id}")
    }
}

/// A press on the camera's screen: the move kept for this truck or trailer, after it.
fn nudge(now: Nudge, press: u32) -> Nudge {
    use westplay_shared::{
        CAMERA_NUDGE_BACK, CAMERA_NUDGE_BACK_FAR, CAMERA_NUDGE_DOWN, CAMERA_NUDGE_FORWARD, CAMERA_NUDGE_FORWARD_FAR,
        CAMERA_NUDGE_LEFT, CAMERA_NUDGE_NARROWER, CAMERA_NUDGE_RESET, CAMERA_NUDGE_RIGHT, CAMERA_NUDGE_ROLL_LEFT,
        CAMERA_NUDGE_ROLL_RIGHT, CAMERA_NUDGE_TILT_DOWN, CAMERA_NUDGE_TILT_UP, CAMERA_NUDGE_TURN_LEFT,
        CAMERA_NUDGE_TURN_RIGHT, CAMERA_NUDGE_UP, CAMERA_NUDGE_WIDER,
    };
    let [back, up, tilt, turn, wide, side, roll] = now;
    let clamp = |v: f32, most: f32| (v.clamp(-most, most) * 100.0).round() / 100.0;
    let degrees = |v: f32, least: f32, most: f32| (v.clamp(least, most) * 10.0).round() / 10.0;
    match press {
        CAMERA_NUDGE_BACK => [clamp(back + NUDGE_STEP, NUDGE_BACK_MOST), up, tilt, turn, wide, side, roll],
        CAMERA_NUDGE_FORWARD => [clamp(back - NUDGE_STEP, NUDGE_BACK_MOST), up, tilt, turn, wide, side, roll],
        CAMERA_NUDGE_BACK_FAR => [clamp(back + NUDGE_FAR_STEP, NUDGE_BACK_MOST), up, tilt, turn, wide, side, roll],
        CAMERA_NUDGE_FORWARD_FAR => [clamp(back - NUDGE_FAR_STEP, NUDGE_BACK_MOST), up, tilt, turn, wide, side, roll],
        CAMERA_NUDGE_UP => [back, clamp(up + NUDGE_STEP, NUDGE_MOST), tilt, turn, wide, side, roll],
        CAMERA_NUDGE_DOWN => [back, clamp(up - NUDGE_STEP, NUDGE_MOST), tilt, turn, wide, side, roll],
        CAMERA_NUDGE_LEFT => [back, up, tilt, turn, wide, clamp(side - NUDGE_STEP, NUDGE_MOST), roll],
        CAMERA_NUDGE_RIGHT => [back, up, tilt, turn, wide, clamp(side + NUDGE_STEP, NUDGE_MOST), roll],
        CAMERA_NUDGE_TILT_UP => [back, up, degrees(tilt + TILT_STEP, -TILT_MOST, TILT_MOST), turn, wide, side, roll],
        CAMERA_NUDGE_TILT_DOWN => [back, up, degrees(tilt - TILT_STEP, -TILT_MOST, TILT_MOST), turn, wide, side, roll],
        CAMERA_NUDGE_TURN_RIGHT => [back, up, tilt, degrees(turn + TURN_STEP, -TURN_MOST, TURN_MOST), wide, side, roll],
        CAMERA_NUDGE_TURN_LEFT => [back, up, tilt, degrees(turn - TURN_STEP, -TURN_MOST, TURN_MOST), wide, side, roll],
        CAMERA_NUDGE_ROLL_RIGHT => [back, up, tilt, turn, wide, side, degrees(roll + ROLL_STEP, -ROLL_MOST, ROLL_MOST)],
        CAMERA_NUDGE_ROLL_LEFT => [back, up, tilt, turn, wide, side, degrees(roll - ROLL_STEP, -ROLL_MOST, ROLL_MOST)],
        CAMERA_NUDGE_WIDER => [back, up, tilt, turn, degrees(wide + WIDE_STEP, WIDE_LEAST, WIDE_MOST), side, roll],
        CAMERA_NUDGE_NARROWER => [back, up, tilt, turn, degrees(wide - WIDE_STEP, WIDE_LEAST, WIDE_MOST), side, roll],
        CAMERA_NUDGE_RESET => [0.0; 7],
        _ => now,
    }
}

/// Once a frame, as the telemetry's published: the player's presses on the camera's screen taken (the host's
/// `camera_nudge`), and the move for the truck or trailer the camera's on now, for `plan` and the page.
pub fn nudged(t: &westplay_shared::Telemetry) -> Nudge {
    let Some(bridge) = crate::bridge::get() else { return [0.0; 7] };
    let truck = bridge.plugin.truck.read().unwrap_or_default();
    let key = nudge_key(t, truck.id.as_str());
    let pressed = bridge.host.camera_nudge.load(Ordering::Acquire);
    let mut guard = NUDGES.lock().unwrap_or_else(|e| e.into_inner());
    let nudges = guard.get_or_insert_with(|| Nudges {
        seen: None,
        by_key: std::fs::read_to_string(nudges_file()).map(|text| read_nudges(&text)).unwrap_or_default(),
    });
    let now = nudges.by_key.get(&key).copied().unwrap_or([0.0; 7]);
    let now = match nudges.seen.replace(pressed) {
        Some(seen) if seen != pressed && pressed != 0 => {
            let moved = nudge(now, pressed & 0xff);
            if moved == [0.0; 7] {
                nudges.by_key.remove(&key);
            } else {
                nudges.by_key.insert(key.clone(), moved);
            }
            log!("retrovisor: the camera moved by the player for {key}: {moved:?} (back, up, tilt, turn, wide, side, roll)");
            if let Err(e) = std::fs::write(nudges_file(), write_nudges(&nudges.by_key)) {
                log!("retrovisor: the camera's moves not kept ({e})");
            }
            moved
        }
        _ => now,
    };
    for (slot, value) in NUDGE_NOW.iter().zip(now) {
        slot.store(value.to_bits(), Ordering::Relaxed);
    }
    now
}

fn nudge_now() -> Nudge {
    NUDGE_NOW.each_ref().map(|slot| f32::from_bits(slot.load(Ordering::Relaxed)))
}

/// How wide our camera sees now (degrees): [`WIDE`] and the player's change.
fn wide() -> f32 {
    (WIDE + nudge_now()[4]).clamp(WIDE + WIDE_LEAST, WIDE + WIDE_MOST)
}
/// How high (over the truck's origin) and how far back the mirror is (metres), measured as its side, smoothed: the
/// camera is placed from the mirror, so a mirror lower than guessed put it lower (owner 2026-11-14, a Scania: at a
/// forklift's wheel).
static MIRROR_UP_BACK: Mutex<Option<[f32; 2]>> = Mutex::new(None);

/// A block of the truck's own (turned as the truck is: the mirror's turn times the truck's axes, near): where it is
/// seen from, counted. The truck's parts share its origin, so the most counted is the truck's origin from the mirror.
/// Whether it is one (it's left out of our camera's picture).
fn chassis(numbers: &[f32; 48]) -> bool {
    let (Some(mirror), Some(axes)) = (
        *TURN.lock().unwrap_or_else(|e| e.into_inner()),
        *TRUCK_AXES.lock().unwrap_or_else(|e| e.into_inner()),
    ) else {
        return false;
    };
    let m = rows(numbers, 0);
    let t = [m[0][3], m[1][3], m[2][3]];
    if m[3] != [0.0, 0.0, 0.0, 1.0] || t.iter().map(|x| x * x).sum::<f32>() > 144.0 {
        return false;
    }
    // The truck's turn seen from the mirror: mirror (world to view) times its axes as columns; the closest way.
    let (way, off) = axes
        .iter()
        .map(|axes| {
            let mut off = 0f32;
            for r in 0..3 {
                for c in 0..3 {
                    let expected: f32 = (0..3).map(|k| mirror[r][k] * axes[c][k]).sum();
                    off = off.max((m[r][c] - expected).abs());
                }
            }
            off
        })
        .enumerate()
        .fold((0, f32::MAX), |best, (way, off)| if off < best.1 { (way, off) } else { best });
    {
        let mut near = CHASSIS_NEAR.lock().unwrap_or_else(|e| e.into_inner());
        near.0 += 1;
        near.1 = near.1.min(off);
    }
    if off >= 0.04 {
        return false;
    }
    let key = t.map(|x| (x * 20.0).round() as i32);
    CHASSIS_VOTES.lock().unwrap_or_else(|e| e.into_inner()).entry(key).or_insert((0, t)).0 += 1;
    WAYS.lock().unwrap_or_else(|e| e.into_inner())[way] += 1;
    // Left out only within the truck's bounds (from its origin as seen last frame): a wall, a fence, a parked
    // trailer or truck lined up with it is turned as it is too, and went missing from the picture (owner 2026-11-14:
    // "things behind the camera disappear"). Its parts' own origins differ (a mod Scania's, by a metre or two), so
    // not the origin itself: where the block is, in the truck's own axes.
    let Some(origin) = *ORIGIN_SEEN.lock().unwrap_or_else(|e| e.into_inner()) else { return true };
    let d: [f32; 3] = std::array::from_fn(|i| t[i] - origin[i]);
    let world: [f32; 3] = std::array::from_fn(|i| (0..3).map(|k| mirror[k][i] * d[k]).sum());
    let [right, up, behind] = axes[way];
    let along = |axis: [f32; 3]| (0..3).map(|i| world[i] * axis[i]).sum::<f32>();
    let rear = f32::from_bits(TRUCK_REAR.load(Ordering::Relaxed));
    let rear = if rear > 0.5 { rear } else { 9.0 };
    along(right).abs() < 1.7 && (-1.5..4.6).contains(&along(up)) && (-3.5..rear + 0.5).contains(&along(behind))
}

/// The truck's origin as seen from the mirror, last measured (`chassis`'s bounds), and its rear end behind it
/// (metres, as f32 bits; 0: not known).
static ORIGIN_SEEN: Mutex<Option<[f32; 3]>> = Mutex::new(None);
static TRUCK_REAR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// How many of the truck's parts matched each of the four ways its pitch and roll may turn (`TRUCK_AXES`) this
/// frame, and the way most matched lately: our camera is tilted as the truck is with it.
static WAYS: Mutex<[u32; 4]> = Mutex::new([0; 4]);
static WAY: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// The frames each way clearly won.
static WAY_TOTALS: Mutex<[u64; 4]> = Mutex::new([0; 4]);
const WAY_SIGNS: [(f32, f32); 4] = [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)];

/// Whether the truck's own parts are left out of our camera's picture: a camera on the truck's back sees none of
/// it, but ours, moved from a mirror, had a mod Scania's chassis across its top (owner 2026-11-14).
const HIDE_TRUCK: bool = true;
/// Whether the block bound last, for the next draw, is left out (the truck's, or one that couldn't be moved).
static TRUCK_NEXT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A new pass (the game's targets set anew): the block bound last is the last pass's, no longer left out (a light
/// pass's screen-wide draw, with no block of its own, isn't).
pub fn new_pass() {
    TRUCK_NEXT.store(false, Ordering::Relaxed);
}

/// Whether the game's draw now is one of the truck's own parts in our camera's picture: left out.
pub fn skip_draw(kind: &str, count: u32) -> bool {
    let skip = HIDE_TRUCK && TRUCK_NEXT.load(Ordering::Relaxed) && ON.load(Ordering::Relaxed) && crate::mirror::drawing_ours();
    note(|| format!("{kind} {count}{}", if skip { " (left out: the truck's)" } else { "" }));
    skip
}

/// Turn votes (rounded first two rows → how many, the turn).
type Votes = std::collections::HashMap<[i32; 6], (u32, [[f32; 3]; 3])>;

/// A block's object's turn counted into `votes`, and the mirror's turn if it's one of the truck's parts into
/// `truck_turns` (as `vote` does for our camera), for one texture's side (`side_tally`).
fn tally(numbers: &[f32; 48], votes: &mut Votes, truck_turns: &mut Votes) {
    let m = rows(numbers, 0);
    if m[3] != [0.0, 0.0, 0.0, 1.0] {
        return;
    }
    let turn: [[f32; 3]; 3] = std::array::from_fn(|r| [m[r][0], m[r][1], m[r][2]]);
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let unit = turn.iter().all(|r| (dot(*r, *r) - 1.0).abs() < 2e-3);
    let square = dot(turn[0], turn[1]).abs() < 2e-3 && dot(turn[1], turn[2]).abs() < 2e-3;
    let identity = (turn[0][0] - 1.0).abs() < 1e-4 && (turn[1][1] - 1.0).abs() < 1e-4;
    if !unit || !square || identity || cross(turn[0], turn[1]).iter().zip(turn[2]).map(|(a, b)| a * b).sum::<f32>() < 0.9 {
        return;
    }
    let key: [i32; 6] = std::array::from_fn(|i| (turn[i / 3][i % 3] * 500.0).round() as i32);
    votes.entry(key).or_insert((0, turn)).0 += 1;
    if let Some(all) = *TRUCK_AXES.lock().unwrap_or_else(|e| e.into_inner()) {
        let mut keys: Vec<[i32; 6]> = Vec::with_capacity(4);
        for axes in all {
            let mirror: [[f32; 3]; 3] =
                std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| turn[r][k] * axes[k][c]).sum()));
            let key: [i32; 6] = std::array::from_fn(|i| (mirror[i / 3][i % 3] * 100.0).round() as i32);
            if !keys.contains(&key) {
                keys.push(key);
                truck_turns.entry(key).or_insert((0, mirror)).0 += 1;
            }
        }
    }
}

/// The turn `votes` vote (looking back, the truck's parts agreeing), as [`voted_turn`] for our camera.
fn best_turn(votes: &Votes, truck_turns: &Votes, behind: [f32; 3]) -> Option<[[f32; 3]; 3]> {
    let supported = |turn: &[[f32; 3]; 3]| -> u32 {
        truck_turns
            .values()
            .filter(|(_, seen)| (0..3).all(|r| (0..3).all(|c| (seen[r][c] - turn[r][c]).abs() < 0.04)))
            .map(|(count, _)| *count)
            .sum()
    };
    votes
        .values()
        .filter(|(count, turn)| *count >= 4 && looks_back(turn, behind) && supported(turn) >= 3)
        .max_by_key(|(count, _)| *count)
        .map(|(_, turn)| *turn)
}

/// This frame's votes of each main mirror's texture (its address → votes, the truck's parts' votes, blocks read),
/// and each one's side as last voted (-1 left, 1 right) with the frame (owner 2026-11-18: one texture's side was
/// known and the other's taken as the other side; the right virtual mirror was hidden, the left shown).
#[derive(Default)]
struct TextureVotes {
    votes: Votes,
    truck_turns: Votes,
    read: u32,
}
static TEXTURE_VOTES: LazyLock<Mutex<std::collections::HashMap<usize, TextureVotes>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
static SIDES: LazyLock<Mutex<std::collections::HashMap<usize, (i32, u64)>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
/// Frames a texture's side holds after it was last voted.
const SIDE_FOR: u64 = 30;
static PRESENTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Which side a main mirror's texture holds the view of now (-1 left, 1 right, 0 not known lately): this frame's own
/// votes if it was drawn (owner 2026-11-18: the game swapped the views between its textures from one frame to the
/// next, and sides a frame late blinked the mirrors), else as last voted.
pub fn side_of_texture(address: usize) -> i32 {
    if let Some(side) = sides_now().get(&address) {
        return *side;
    }
    let now = PRESENTS.load(Ordering::Relaxed);
    let sides = SIDES.lock().unwrap_or_else(|e| e.into_inner());
    let Some(&(side, at)) = sides.get(&address).filter(|(_, at)| now.saturating_sub(*at) <= SIDE_FOR) else { return 0 };
    // Another texture told the same side since: the game handed the views round (ETS2 1.62, owner 2026-11-21: with
    // the right one chosen, both showed, each texture kept as "right" from a frame of its own). Not known then: the
    // side's told from our mirror's (hide_virtual).
    if sides.iter().any(|(other, (other_side, other_at))| *other != address && *other_side == side && *other_at > at) {
        return 0;
    }
    side
}

/// Blocks a texture's votes need this frame before its side is told from them.
const SIDE_READ_ENOUGH: u32 = 8;

/// This frame's sides of the main mirrors drawn so far, from their own votes, worked out once a frame (at the first
/// ask after they were drawn: the virtual mirrors are put on the screen after). Two told the same side, neither's
/// taken (one's wrong).
fn sides_now() -> std::collections::HashMap<usize, i32> {
    static NOW: LazyLock<Mutex<(u64, usize, std::collections::HashMap<usize, i32>)>> =
        LazyLock::new(|| Mutex::new((u64::MAX, 0, std::collections::HashMap::new())));
    let frame = PRESENTS.load(Ordering::Relaxed);
    let votes = TEXTURE_VOTES.lock().unwrap_or_else(|e| e.into_inner());
    let mut now = NOW.lock().unwrap_or_else(|e| e.into_inner());
    // Worked out again only if more textures were voted since (a mirror drawn after the last ask).
    if now.0 == frame && now.1 == votes.len() {
        return now.2.clone();
    }
    let behind = back(crate::telemetry::truck().heading, (1.0, 1.0));
    let mut sides: std::collections::HashMap<usize, i32> = votes
        .iter()
        .filter(|(texture, these)| **texture >= 0x1000 && these.read >= SIDE_READ_ENOUGH)
        .filter_map(|(texture, these)| {
            best_turn(&these.votes, &these.truck_turns, behind).map(|turn| (*texture, side_of(&turn, behind)))
        })
        .collect();
    let lefts = sides.values().filter(|side| **side < 0).count();
    let rights = sides.values().filter(|side| **side > 0).count();
    if lefts > 1 || rights > 1 {
        sides.retain(|_, side| (*side < 0 && lefts <= 1) || (*side > 0 && rights <= 1));
    }
    *now = (frame, votes.len(), sides.clone());
    sides
}

/// Moves votes collected during a high-quality mirror's G-buffer pass (`from`, a small group tag) onto its final
/// R11G11B10 texture (`to`), so `side_of_texture(to)` knows which side it is (owner 2026-11-22: ATS with high mirror
/// quality drew all 3D geometry in the `f10x4` G-buffer pass before the final target was bound).
pub fn alias_texture_votes(from: usize, to: usize) {
    if from == 0 || to == 0 || from == to {
        return;
    }
    let mut all = TEXTURE_VOTES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(votes) = all.remove(&from) {
        all.insert(to, votes);
    }
}

/// One of a main mirror's blocks counted for its texture's side (at most [`SIDE_VOTES`] a frame each).
fn side_tally(texture: usize, numbers: &[f32; 48]) {
    if texture == 0 {
        return;
    }
    let mut all = TEXTURE_VOTES.lock().unwrap_or_else(|e| e.into_inner());
    if all.len() > 8 && !all.contains_key(&texture) {
        return;
    }
    let these = all.entry(texture).or_default();
    if these.read >= SIDE_VOTES {
        return;
    }
    these.read += 1;
    let TextureVotes { votes, truck_turns, .. } = these;
    tally(numbers, votes, truck_turns);
}

fn vote(numbers: &[f32; 48]) {
    let m = rows(numbers, 0);
    if m[3] != [0.0, 0.0, 0.0, 1.0] {
        return;
    }
    let turn: [[f32; 3]; 3] = std::array::from_fn(|r| [m[r][0], m[r][1], m[r][2]]);
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let unit = turn.iter().all(|r| (dot(*r, *r) - 1.0).abs() < 2e-3);
    let square = dot(turn[0], turn[1]).abs() < 2e-3 && dot(turn[1], turn[2]).abs() < 2e-3;
    let identity = (turn[0][0] - 1.0).abs() < 1e-4 && (turn[1][1] - 1.0).abs() < 1e-4;
    if !unit || !square || identity || cross(turn[0], turn[1]).iter().zip(turn[2]).map(|(a, b)| a * b).sum::<f32>() < 0.9 {
        return;
    }
    let key: [i32; 6] = std::array::from_fn(|i| (turn[i / 3][i % 3] * 500.0).round() as i32);
    VOTES.lock().unwrap_or_else(|e| e.into_inner()).entry(key).or_insert((0, turn)).0 += 1;
    // The mirror's turn if this is one of the truck's parts: its turn times the truck's axes.
    // Each of the four ways the SDK's pitch and roll may turn (the way isn't known before a mirror's turn is: on a
    // slope, with one way only, nothing agreed and nothing was ever taken, owner 2026-11-14, a DAF XD); each key once.
    if let Some(all) = *TRUCK_AXES.lock().unwrap_or_else(|e| e.into_inner()) {
        let mut keys: Vec<[i32; 6]> = Vec::with_capacity(4);
        let mut truck_turns = TRUCK_TURNS.lock().unwrap_or_else(|e| e.into_inner());
        for axes in all {
            let mirror: [[f32; 3]; 3] =
                std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| turn[r][k] * axes[k][c]).sum()));
            let key: [i32; 6] = std::array::from_fn(|i| (mirror[i / 3][i % 3] * 100.0).round() as i32);
            if !keys.contains(&key) {
                keys.push(key);
                truck_turns.entry(key).or_insert((0, mirror)).0 += 1;
            }
        }
    }
}

/// The blocks already rewritten this frame (buffer, first constant): one bound for two draws is moved once.
/// Each with whether it's one of the truck's parts.
static DONE: LazyLock<Mutex<std::collections::HashMap<(usize, u32), bool>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
/// How many blocks were rewritten, skipped and failed, since the last log.
static COUNTS: Mutex<[u64; 3]> = Mutex::new([0; 3]);

/// At each present: on while the page shows the mirror's picture.
pub fn on_present(frame: u64) {
    let on = crate::mirror::showing() && crate::mirror::seen();
    probe_present(on);
    if let Some(shape) = crate::mirror::shape() {
        SHAPE.store(shape.to_bits(), Ordering::Relaxed);
    }
    if on != ON.swap(on, Ordering::Relaxed) {
        ON_AT.store(frame, Ordering::Relaxed);
        log!("retrovisor: our camera {}", if on { "on (the second main mirror moved)" } else { "off" });
    }
    DONE.lock().unwrap_or_else(|e| e.into_inner()).clear();
    // The mirror's turn for the next frame: the one most of this frame's objects had (the ground's, the roads').
    // Only a turn that looks back from the truck, level (a mirror's): in a town the houses, walls and fences lined up
    // with the street outvoted the ground, and the camera looked 130 degrees off, ahead (owner 2026-11-14).
    let behind_now = back(crate::telemetry::truck().heading, (1.0, 1.0));
    PRESENTS.store(frame, Ordering::Relaxed);
    // Each main mirror's texture's side, from its own blocks this frame, kept for the frames it isn't drawn in.
    {
        let now = sides_now();
        TEXTURE_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        let mut sides = SIDES.lock().unwrap_or_else(|e| e.into_inner());
        for (texture, side) in now {
            sides.insert(texture, (side, frame));
        }
        sides.retain(|_, (_, at)| frame.saturating_sub(*at) <= 600);
    }
    let voted = voted_turn(behind_now);
    let mut ours_side = side_of_texture(crate::mirror::ours_texture());
    if ours_side == 0
        && let Some(turn) = voted
    {
        ours_side = side_of(&turn, behind_now);
    }
    if ours_side != 0 && OURS_SIDE.swap(ours_side, Ordering::Relaxed) != ours_side {
        log!(
            "retrovisor: our mirror's view is of the {} one (the driver's: {})",
            if ours_side < 0 { "left" } else { "right" },
            if driver_side() < 0 { "left" } else { "right" }
        );
    }
    // Held a while by the other side's view, another texture is taken for ours (`mirror`).
    if ours_side != 0 && ours_side != driver_side() {
        WRONG_SIDE.fetch_add(1, Ordering::Relaxed);
    } else if ours_side != 0 {
        WRONG_SIDE.store(0, Ordering::Relaxed);
    }
    // Our camera's turn: only the driver's side mirror's (owner 2026-11-18); the other one's view in our texture (the
    // game hands the views round) isn't our camera's.
    if let Some(turn) = voted
        && side_of(&turn, behind_now) == driver_side()
    {
        TURNS_TAKEN.fetch_add(1, Ordering::Relaxed);
        *TURN.lock().unwrap_or_else(|e| e.into_inner()) = Some(turn);
    }
    TRUCK_TURNS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    let mut votes = VOTES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((count, turn)) = votes.values().max_by_key(|(count, _)| *count).copied()
        && count >= 4
        && TURN.lock().unwrap_or_else(|e| e.into_inner()).is_none_or(|taken| taken != turn)
    {
        *TURN_REFUSED.lock().unwrap_or_else(|e| e.into_inner()) = Some(turn);
    }
    votes.clear();
    drop(votes);
    // Another mirror's view now: its own measurements (kept), and none made this frame (its parts were looked for
    // with the other's turn).
    let mut switched = false;
    if let Some(turn) = *TURN.lock().unwrap_or_else(|e| e.into_inner()) {
        // Where it looks as seen from the truck (across, back), not in the world: turning hard in reverse, the truck
        // and its mirror turned past 5 degrees and it was taken for another mirror, its measurements swapped, the
        // camera jumping (owner 2026-11-14).
        let world = flat_look(&turn);
        let behind = back(crate::telemetry::truck().heading, (1.0, 1.0));
        let look = [world[0] * behind[2] - world[1] * behind[0], world[0] * behind[0] + world[1] * behind[2]];
        let mut of = LOOK_OF.lock().unwrap_or_else(|e| e.into_inner());
        if of.is_none_or(|old| old[0] * look[0] + old[1] * look[1] < SAME_LOOK) {
            switched = true;
            let mut measured = MEASURED.lock().unwrap_or_else(|e| e.into_inner());
            let mut side = MIRROR_SIDE.lock().unwrap_or_else(|e| e.into_inner());
            let mut up_back = MIRROR_UP_BACK.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(old) = *of {
                measured.retain(|(seen, _, _)| seen[0] * old[0] + seen[1] * old[1] < SAME_LOOK);
                measured.push((old, *side, *up_back));
            }
            (*side, *up_back) = measured
                .iter()
                .find(|(seen, _, _)| seen[0] * look[0] + seen[1] * look[1] >= SAME_LOOK)
                .map_or((None, None), |(_, side, up_back)| (*side, *up_back));
            *of = Some(look);
        }
    }
    // The mirror's side from the truck's middle: the truck's origin as seen from it (its parts' most counted place).
    let mut chassis = CHASSIS_VOTES.lock().unwrap_or_else(|e| e.into_inner());
    let best = chassis.values().max_by_key(|(count, _)| *count).copied();
    chassis.clear();
    drop(chassis);
    if let Some((count, seen)) = best
        && count >= 3
    {
        *ORIGIN_SEEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(seen);
    }
    if let Some(rear) = crate::bridge::get().and_then(|bridge| bridge.plugin.telemetry.read()).map(|t| t.rear_z) {
        TRUCK_REAR.store(rear.to_bits(), Ordering::Relaxed);
    }
    let truck = crate::telemetry::truck();
    let behind = back(truck.heading, (1.0, 1.0));
    let right = [behind[2], 0.0, -behind[0]];
    *TRUCK_AXES.lock().unwrap_or_else(|e| e.into_inner()) =
        Some(WAY_SIGNS.map(|(p, r)| tilted_axes(truck.heading, truck.pitch * p, truck.roll * r)));
    {
        // The way the truck's parts matched (only a tilted truck tells them apart: level, all four match alike).
        let mut ways = WAYS.lock().unwrap_or_else(|e| e.into_inner());
        // The SDK's way never changes: counted over time, from frames with a clear winner only, and the most counted
        // kept. Taken frame by frame, a nearly level MAN's four ways nearly tied and the winner swapped, the camera
        // hopping up and down (owner 2026-11-14).
        let mut sorted = *ways;
        sorted.sort_unstable();
        let (first, second) = (sorted[3], sorted[2]);
        if first >= 5 && first >= second * 2 {
            let best = ways.iter().position(|c| *c == first).unwrap_or(0);
            let mut totals = WAY_TOTALS.lock().unwrap_or_else(|e| e.into_inner());
            totals[best] += 1;
            let kept = (0..4).max_by_key(|i| totals[*i]).unwrap_or(0);
            WAY.store(kept, Ordering::Relaxed);
        }
        *ways = [0; 4];
    }
    if let (Some((count, seen)), Some(mirror)) = (best, *TURN.lock().unwrap_or_else(|e| e.into_inner()))
        && count >= 3
        && !switched
    {
        // The mirror from the truck's origin, in the world: minus the origin seen from it, turned back.
        let from_origin: [f32; 3] = std::array::from_fn(|i| -(0..3).map(|k| mirror[k][i] * seen[k]).sum::<f32>());
        let side: f32 = (0..3).map(|i| from_origin[i] * right[i]).sum();
        let up = from_origin[1];
        let back: f32 = (0..3).map(|i| from_origin[i] * behind[i]).sum();
        // As a truck's mirror is: half a metre to two and a half out from its middle, over the cab's floor, near the
        // driver's head lengthwise; all three, or it's another part (one passing the side only put the camera under
        // the truck, owner 2026-11-14).
        if (0.5..=2.5).contains(&side.abs()) && (1.0..=3.6).contains(&up) && (back - truck.head[2]).abs() < 2.0 {
            let mut smoothed = MIRROR_SIDE.lock().unwrap_or_else(|e| e.into_inner());
            *smoothed = Some(smoothed.map_or(side, |old| old * 0.5 + side * 0.5));
            drop(smoothed);
            let mut smoothed = MIRROR_UP_BACK.lock().unwrap_or_else(|e| e.into_inner());
            *smoothed = Some(smoothed.map_or([up, back], |[u, b]| [u * 0.5 + up * 0.5, b * 0.5 + back * 0.5]));
        }
    }
    if frame.is_multiple_of(120) && on {
        LOG_PLAN.store(true, Ordering::Relaxed);
    }
    if frame.is_multiple_of(600) && on {
        let mut counts = COUNTS.lock().unwrap_or_else(|e| e.into_inner());
        log!("retrovisor: our camera, the last 600 frames: {} blocks moved, {} left, {} failed", counts[0], counts[1], counts[2]);
        *counts = [0; 3];
        let refused = TURN_REFUSED.lock().unwrap_or_else(|e| e.into_inner()).take();
        log!(
            "retrovisor: the mirror's turn taken {} times; the most voted one refused (not looking back, or not the truck's parts'): {refused:?}",
            TURNS_TAKEN.swap(0, Ordering::Relaxed)
        );
        let mut near = CHASSIS_NEAR.lock().unwrap_or_else(|e| e.into_inner());
        log!("retrovisor: the truck's parts looked for: {} blocks near the mirror, the closest turn off by {:.3} (0.04 at most)", near.0, near.1);
        *near = (0, f32::MAX);
    }
}


/// A main mirror's block read (not changed), for its texture's side (`SIDE_VOTES` a frame at most each): every main
/// mirror, ours too while our camera isn't moving it.
///
/// # Safety
/// As [`on_vs_constant_buffers1`].
unsafe fn side_vote(
    context: *mut c_void,
    start: u32,
    count: u32,
    buffers: *const *mut c_void,
    firsts: *const u32,
    counts: *const u32,
) {
    if start != 0 || count == 0 || buffers.is_null() || firsts.is_null() {
        return;
    }
    let texture = crate::mirror::main_pass();
    if texture == 0
        || TEXTURE_VOTES.lock().unwrap_or_else(|e| e.into_inner()).get(&texture).is_some_and(|v| v.read >= SIDE_VOTES)
    {
        return;
    }
    // SAFETY: the game's first buffer and its range (the caller's contract).
    let (raw, first, constants) = unsafe { (*buffers, *firsts, if counts.is_null() { 0 } else { *counts }) };
    if raw.is_null() || constants < 12 {
        return;
    }
    // SAFETY: the game's live context and buffer; the block is within the bound range (12 constants of it), only read.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        let Some(buffer) = ID3D11Buffer::from_raw_borrowed(&raw) else { return };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        if context.Map(buffer, 0, D3D11_MAP_WRITE_NO_OVERWRITE, 0, Some(&mut mapped)).is_ok() {
            let block = (mapped.pData as *const f32).add(first as usize * 4);
            let mut numbers = [0f32; 48];
            std::ptr::copy_nonoverlapping(block, numbers.as_mut_ptr(), 48);
            context.Unmap(buffer, 0);
            side_tally(texture, &numbers);
        }
    }
}

/// `VSSetConstantBuffers1`: a block bound in b0 for our mirror's draw, rewritten from our camera.
///
/// # Safety
/// The game's live context and arguments: `count` buffers, and as many firsts and counts if not null.
pub unsafe fn on_vs_constant_buffers1(
    context: *mut c_void,
    start: u32,
    count: u32,
    buffers: *const *mut c_void,
    firsts: *const u32,
    counts: *const u32,
) {
    if !crate::mirror::drawing_ours() || !ON.load(Ordering::Relaxed) {
        // Not our camera's draw: a few of a main mirror's blocks read only, for the side its view is of.
        // SAFETY: as `side_vote` says (the caller's contract).
        unsafe { side_vote(context, start, count, buffers, firsts, counts) };
        return;
    }
    if start != 0 || count == 0 || buffers.is_null() || firsts.is_null() {
        note(|| format!("VSSetConstantBuffers1 b{start} x{count}{}", if firsts.is_null() { " (no offsets)" } else { "" }));
        return;
    }
    // SAFETY: the game's first buffer and its range (the caller's contract).
    let (raw, first, constants) =
        unsafe { (*buffers, *firsts, if counts.is_null() { 0 } else { *counts }) };
    if raw.is_null() || constants < 12 {
        return;
    }
    TRUCK_NEXT.store(false, Ordering::Relaxed);
    if let Some(truck) = DONE.lock().unwrap_or_else(|e| e.into_inner()).get(&(raw as usize, first)).copied() {
        TRUCK_NEXT.store(truck, Ordering::Relaxed);
        note(|| format!("b0 {raw:?}+{first} again{}", if truck { " (truck)" } else { "" }));
        return;
    }
    // SAFETY: the game's live context and buffer; the block is within the bound range (12 constants of it).
    let outcome = unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        let Some(buffer) = ID3D11Buffer::from_raw_borrowed(&raw) else { return };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        match context.Map(buffer, 0, D3D11_MAP_WRITE_NO_OVERWRITE, 0, Some(&mut mapped)) {
            Ok(()) => {
                let block = (mapped.pData as *mut f32).add(first as usize * 4);
                let mut numbers = [0f32; 48];
                std::ptr::copy_nonoverlapping(block, numbers.as_mut_ptr(), 48);
                vote(&numbers);
                side_tally(crate::mirror::main_pass(), &numbers);
                let truck = chassis(&numbers);
                let seen = rows(&numbers, 0);
                let turn = *TURN.lock().unwrap_or_else(|e| e.into_inner());
                let moved = turn.is_some_and(|turn| camera(&mut numbers, &plan(&turn)));
                if moved {
                    std::ptr::copy_nonoverlapping(numbers.as_ptr(), block, 48);
                }
                // Left out too: a block with its place a row lower (its rows 1 to 3 a turn and where), that couldn't be
                // moved, drawn from the mirror's place across our picture (owner 2026-11-14: a mod Scania's body, a dark
                // band across the top). Only those: the mirrors' high quality lights them with screen-wide draws whose
                // blocks hold the picture's size and such (left out, the camera was black, owner 2026-11-14).
                let dot = |a: [f32; 4], b: [f32; 4]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
                let lower = [seen[1], seen[2], seen[3]];
                let placed_lower = lower.iter().all(|row| (dot(*row, *row) - 1.0).abs() < 0.01)
                    && dot(lower[0], lower[1]).abs() < 0.01
                    && dot(lower[1], lower[2]).abs() < 0.01
                    && dot(lower[0], lower[2]).abs() < 0.01;
                let leave_out = truck || (!moved && placed_lower);
                TRUCK_NEXT.store(leave_out, Ordering::Relaxed);
                DONE.lock().unwrap_or_else(|e| e.into_inner()).insert((raw as usize, first), leave_out);
                note(|| {
                    format!(
                        "b0 {raw:?}+{first} {}{} rows {:?}",
                        if moved { "moved" } else { "LEFT" },
                        if leave_out { " (left out)" } else { "" },
                        seen
                    )
                });
                context.Unmap(buffer, 0);
                if moved { 0 } else { 1 }
            }
            Err(e) => {
                if !LOGGED.swap(true, Ordering::Relaxed) {
                    log!("retrovisor: our camera can't open the game's block: {e}");
                }
                2
            }
        }
    };
    COUNTS.lock().unwrap_or_else(|e| e.into_inner())[outcome] += 1;
}

type M4 = [[f32; 4]; 4];

fn rows(numbers: &[f32; 48], first: usize) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| numbers[(first + r) * 4 + c]))
}

fn product(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| a[r][k] * b[k][c]).sum()))
}

/// An affine 4 x 4's inverse (its last row 0 0 0 1), if it has one.
fn affine_inverse(m: &M4) -> Option<M4> {
    let [a, b, c] = [m[0][0], m[0][1], m[0][2]];
    let [d, e, f] = [m[1][0], m[1][1], m[1][2]];
    let [g, h, i] = [m[2][0], m[2][1], m[2][2]];
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-9 {
        return None;
    }
    let r = [
        [(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det],
        [(f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det],
        [(d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det],
    ];
    let t = [m[0][3], m[1][3], m[2][3]];
    let mut out = [[0.0; 4]; 4];
    for row in 0..3 {
        for col in 0..3 {
            out[row][col] = r[row][col];
        }
        out[row][3] = -(r[row][0] * t[0] + r[row][1] * t[1] + r[row][2] * t[2]);
    }
    out[3][3] = 1.0;
    Some(out)
}


/// How every object moves in view space when the camera moves FORWARD, level, the way it looks: the camera looks
/// along view -z, that is the world's -row 2, flattened (y, up, taken out).
fn shift(turn: &[[f32; 3]; 3]) -> [f32; 3] {
    let ahead = [-turn[2][0], 0.0, -turn[2][2]];
    let length = (ahead[0] * ahead[0] + ahead[2] * ahead[2]).sqrt().max(1e-3);
    let step = [ahead[0] / length * FORWARD, 0.0, ahead[2] / length * FORWARD];
    // An object at world offset w from the camera is at R w in view space; the camera moved by step: R (w - step).
    std::array::from_fn(|r| -(turn[r][0] * step[0] + turn[r][1] * step[1] + turn[r][2] * step[2]))
}

/// How the view changes for our camera: every object turned by `turn` and then moved by `shift` (view space).
#[derive(Clone, Copy, Debug)]
struct Move {
    turn: [[f32; 3]; 3],
    shift: [f32; 3],
}

const IDENTITY: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

fn mul3(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

fn apply(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
}

fn transpose(m: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| m[c][r]))
}

#[cfg(test)]
fn det(m: &[[f32; 3]; 3]) -> f32 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn normal(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    v.map(|x| x / length)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Whether a view's turn (world to view) is a mirror's on this truck: it looks (minus its third row) within about 45
/// degrees of the truck's back, and its up is the world's (either way: the game's mirrors are drawn upside down).
fn looks_back(turn: &[[f32; 3]; 3], behind: [f32; 3]) -> bool {
    let look = [-turn[2][0], -turn[2][2]];
    let length = (look[0] * look[0] + look[1] * look[1]).sqrt();
    length > 0.5 && (look[0] * behind[0] + look[1] * behind[2]) / length > 0.7 && turn[1][1].abs() > 0.8
}

/// The truck's right, up and back in the world, turned by its heading, pitch and roll (turns): heading about y,
/// then pitch about x, then roll about z.
fn tilted_axes(heading: f32, pitch: f32, roll: f32) -> [[f32; 3]; 3] {
    let (h, p, r) = (heading * std::f32::consts::TAU, pitch * std::f32::consts::TAU, roll * std::f32::consts::TAU);
    let turn_y = |v: [f32; 3]| [v[0] * h.cos() + v[2] * h.sin(), v[1], -v[0] * h.sin() + v[2] * h.cos()];
    let turn_x = |v: [f32; 3]| [v[0], v[1] * p.cos() - v[2] * p.sin(), v[1] * p.sin() + v[2] * p.cos()];
    let turn_z = |v: [f32; 3]| [v[0] * r.cos() - v[1] * r.sin(), v[0] * r.sin() + v[1] * r.cos(), v[2]];
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|v| turn_y(turn_x(turn_z(v))))
}

/// A heading (turns) as "back" in the view's world, by the way (`way`: the signs of its sine and cosine).
fn back(heading: f32, way: (f32, f32)) -> [f32; 3] {
    let angle = heading * std::f32::consts::TAU;
    [way.0 * angle.sin(), 0.0, way.1 * angle.cos()]
}

/// Our camera's move for the mirror whose turn is `mirror` (world to view), from the SDK's truck and trailer; the
/// level move of [`shift`] without them.
fn plan(mirror: &[[f32; 3]; 3]) -> Move {
    let truck = crate::telemetry::truck();
    if truck.position == [0.0; 3] {
        return Move { turn: IDENTITY, shift: shift(mirror) };
    }
    let up = [0.0, 1.0, 0.0];
    let looks = normal([-mirror[2][0], 0.0, -mirror[2][2]]);
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    // The SDK's heading here: back is (sin, 0, cos). Found from the owner's 8 captures of 2026-11-14 (each mirror
    // looked 0.77 to 1.0 that way); chosen per frame, another way won once by chance (a mirror looking outward).
    let way = (1.0, 1.0);
    // The truck's axes in the world: right, up, back (its own +x, +y, +z).
    let axes = |heading: f32| {
        let b = back(heading, way);
        let forward = b.map(|x| -x);
        ([-forward[2], 0.0, forward[0]], b)
    };
    let (right, behind) = axes(truck.heading);
    let world = |right: [f32; 3], behind: [f32; 3], v: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|i| right[i] * v[0] + up[i] * v[1] + behind[i] * v[2])
    };
    // The truck tilted as it is (its pitch and roll, the way its parts matched): a camera on its back goes up a ramp
    // with it (owner 2026-11-14: on a ramp it stayed level, looking at the ground). The mirror's place stays in the
    // level axes it was measured in.
    let (pitch_sign, roll_sign) = WAY_SIGNS[WAY.load(Ordering::Relaxed)];
    let [t_right, t_up, t_behind] = tilted_axes(truck.heading, truck.pitch * pitch_sign, truck.roll * roll_sign);
    let tilted = |v: [f32; 3]| -> [f32; 3] { std::array::from_fn(|i| t_right[i] * v[0] + t_up[i] * v[1] + t_behind[i] * v[2]) };
    // The mirror: on the side it looks out to.
    let side = if dot(looks, right) >= 0.0 { 1.0 } else { -1.0 };
    let measured = *MIRROR_SIDE.lock().unwrap_or_else(|e| e.into_inner());
    let out = measured.unwrap_or(side * MIRROR_OUT);
    let up_back = *MIRROR_UP_BACK.lock().unwrap_or_else(|e| e.into_inner());
    let [mirror_up, mirror_back] = up_back.unwrap_or([truck.head[1] + MIRROR_UP, truck.head[2] - MIRROR_FORWARD]);
    let mirror_at = world(right, behind, [out, mirror_up, mirror_back]);
    let layout = {
        let vehicles = crate::reversing::VEHICLES.lock().unwrap_or_else(|e| e.into_inner());
        // The rear ends as published to the page (the parking sensors'): the trailer's measured by the plugin's
        // rays through the game's physics, load and all (ray.rs), not the guess from its last axle (owner
        // 2026-11-14: "the sensor is right whatever the trailer; the camera goes into things first").
        let published = crate::bridge::get().and_then(|bridge| bridge.plugin.telemetry.read());
        let (truck_rear, trailer_rear) = published
            .map(|t| (t.rear_z, t.trailer_rear_z))
            .filter(|(truck, _)| *truck > 0.5)
            .unwrap_or((vehicles.truck.rear_z, vehicles.trailer.rear_z));
        (truck_rear, trailer_rear, vehicles.truck.ground_y, vehicles.trailer.ground_y, vehicles.trailer.rear_z)
    };
    // Where our camera goes, from the truck's origin, and which way is back there.
    let [n_back, n_up, tilt, turn, _, n_side, roll] = nudge_now();
    let trailer = crate::telemetry::trailer();
    let apart: [f32; 3] = std::array::from_fn(|i| (trailer.position[i] - truck.position[i]) as f32);
    let apart_sq = apart[0] * apart[0] + apart[1] * apart[1] + apart[2] * apart[2];
    // Guard against stale/uninitialized trailer telemetry (e.g. after a live truck/trailer swap where
    // `trailer.position` can be kilometres away or at origin): a coupled trailer is always within 30 m.
    let trailer_near = trailer.connected && trailer.placed && apart_sq.is_finite() && apart_sq <= 30.0 * 30.0;
    let (target, looking_back, camera_up, camera_right) = if trailer_near {
        // Tilted as the trailer is (its own pitch and roll, turned the way the truck's parts matched), owner
        // 2026-11-14: "the trailer's too, to be 100%".
        let [r_right, r_up, r_behind] = tilted_axes(trailer.heading, trailer.pitch * pitch_sign, trailer.roll * roll_sign);
        let end = trailer_camera_end(layout.1, layout.4);
        let at = [n_side, layout.3 + HEIGHT_TRAILER + n_up, end + PAST_REAR - TRAILER_IN + n_back];
        let rear: [f32; 3] = std::array::from_fn(|i| r_right[i] * at[0] + r_up[i] * at[1] + r_behind[i] * at[2]);
        (std::array::from_fn(|i| apart[i] + rear[i]), r_behind, r_up, r_right)
    } else {
        let hook = crate::telemetry::truck_hook().map_or(1.6, |h| h[2]);
        let end = if layout.0 > 0.5 { layout.0 } else { hook + TRUCK_PAST_HOOK };
        (tilted([n_side, layout.2 + HEIGHT_TRUCK + n_up, end + PAST_REAR + n_back]), t_behind, t_up, t_right)
    };
    // Turned by the player toward the truck's right (the screen's right) or left, about the camera's up.
    let looking_back = {
        let (sin, cos) = turn.to_radians().sin_cos();
        normal(std::array::from_fn(|i| looking_back[i] * cos + camera_right[i] * sin))
    };
    let step: [f32; 3] = std::array::from_fn(|i| target[i] - mirror_at[i]);
    // Our camera's turn: looking back and PITCH down (rows: its right, up and -look in the world).
    let (sin, cos) = (PITCH - tilt).to_radians().sin_cos();
    let look = normal(std::array::from_fn(|i| looking_back[i] * cos - camera_up[i] * sin));
    // Lying on its side in the mirror's picture (rolled a quarter turn): the screen's wide picture along the
    // mirror's tall one, every pixel of it used; the screen turns it back (`mirror::ours_place`).
    let upright = rolled_as(mirror, look.map(|x| -x), camera_up);
    let rolled = if roll.abs() > 1e-4 {
        let (rs, rc) = roll.to_radians().sin_cos();
        [
            std::array::from_fn(|i| upright[0][i] * rc + upright[1][i] * rs),
            std::array::from_fn(|i| -upright[0][i] * rs + upright[1][i] * rc),
            upright[2],
        ]
    } else {
        upright
    };
    let ours = [rolled[1], rolled[0].map(|v| -v), rolled[2]];
    let turn = mul3(&ours, &transpose(mirror));
    let moved = apply(&ours, step);
    if LOG_PLAN.swap(false, Ordering::Relaxed) {
        log!(
            "retrovisor: our camera plan: heading {:.4} (trailer {} {:.4}), mirror side measured {measured:?}, up and back {up_back:?}, rears {layout:?}, mirror rows {mirror:?}, looks {looks:?}, side {side},              back {behind:?}, right {right:?}, mirror at {mirror_at:?}, target {target:?}, step {step:?}, our look {look:?}, ours {ours:?}",
            truck.heading,
            trailer.connected,
            trailer.heading,
        );
    }
    Move { turn, shift: moved.map(|v| -v) }
}

/// How far behind a trailer's origin the camera goes (metres): at the rear end measured (`measured`, the rays'), but
/// never ahead of the one worked out from its last axle (`guessed`), a little past it (the collision box the rays
/// hit may end short of the body); with neither, `TRAILER_PAST_ORIGIN`.
fn trailer_camera_end(measured: f32, guessed: f32) -> f32 {
    let guessed = (guessed > 0.5).then_some(guessed + TRAILER_CAMERA_PAST_GUESS);
    let measured = (measured > 0.5).then_some(measured);
    match (measured, guessed) {
        (Some(measured), Some(guessed)) => measured.max(guessed),
        (Some(end), None) | (None, Some(end)) => end,
        (None, None) => TRAILER_PAST_ORIGIN,
    }
}

/// The level way to look along view axis `z` (its right and up: up the world's y as near as can be).
fn upright(z: [f32; 3], up: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let along = up[0] * z[0] + up[1] * z[1] + up[2] * z[2];
    let y = normal(std::array::from_fn(|i| up[i] - along * z[i]));
    (cross(y, z), y)
}

/// A turn with view axis `z`, rolled about it as `mirror` is about its own (the game draws a mirror rolled, and turns
/// the picture back when it shows it: 2026-11-14, our first upright camera came out upside down). Its determinant
/// is the mirror's too.
/// The mirror's roll about `up` (the truck's: the mirror rolls with it), given to a view looking along `z`.
fn rolled_as(mirror: &[[f32; 3]; 3], z: [f32; 3], up: [f32; 3]) -> [[f32; 3]; 3] {
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (x0, y0) = upright(mirror[2], up);
    let (x, y) = upright(z, up);
    let mix = |row: [f32; 3]| -> [f32; 3] {
        // Its roll in quarter turns only (lying, upside down: how the game draws its mirrors), not each mirror's own
        // slight tilt: the left's and the right's differ, and the camera tilted a little whenever the game swapped them
        // (owner 2026-11-14, a DAF).
        let (a, b) = (dot(row, x0).round(), dot(row, y0).round());
        std::array::from_fn(|i| a * x[i] + b * y[i])
    };
    [mix(mirror[0]), mix(mirror[1]), z]
}

/// One draw's block (rows 0-11) as seen from our camera (`how`: every object turned, then moved, in view space);
/// `false` if it isn't an object's (a picture drawn flat).
fn camera(numbers: &mut [f32; 48], how: &Move) -> bool {
    let mut modelview = rows(numbers, 0);
    if modelview[3] != [0.0, 0.0, 0.0, 1.0] {
        return false;
    }
    let identity = (0..3).all(|r| (0..4).all(|c| (modelview[r][c] - if r == c { 1.0 } else { 0.0 }).abs() < 1e-6));
    if identity {
        return false;
    }
    let Some(inverse) = affine_inverse(&modelview) else { return false };
    let projections = [product(&rows(numbers, 4), &inverse), product(&rows(numbers, 8), &inverse)];
    let old = modelview;
    for r in 0..3 {
        for c in 0..4 {
            modelview[r][c] = (0..3).map(|k| how.turn[r][k] * old[k][c]).sum::<f32>() + if c == 3 { how.shift[r] } else { 0.0 };
        }
    }
    for r in 0..4 {
        numbers[r * 4..r * 4 + 4].copy_from_slice(&modelview[r]);
    }
    for (p, projection) in projections.iter().enumerate() {
        let mut projected = product(projection, &modelview);
        // Our lens: wider than the mirror's. Lying on its side, the picture's x is the screen's up and down and its
        // y the screen's across.
        let across = 1.0 / (wide().to_radians() / 2.0).tan();
        let up_down = across * shape();
        // A test (owner 2026-11-14): with the mirrors' high quality, things far off came out stretched down from the
        // horizon; lit afterwards with the mirror's own lens, maybe. Its own lens kept there, to tell.
        let lens: &[(usize, f32)] = if crate::mirror::high_quality() { &[] } else { &[(0, up_down), (1, across)] };
        for &(row, wanted) in lens {
            let had = projection[row][row];
            if had.abs() > 1e-3 {
                projected[row] = projected[row].map(|v| v * wanted / had);
            }
        }
        for r in 0..4 {
            let at = (4 + p * 4 + r) * 4;
            numbers[at..at + 4].copy_from_slice(&projected[r]);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block as the owner's capture of 2026-11-14 had it (the right mirror's sky): its projections kept.
    #[test]
    fn a_block_is_seen_from_the_moved_camera() {
        let r = [[-0.32335f32, -0.91117, 0.25538], [0.65763, -0.41043, -0.63172], [0.68042, -0.03632, 0.73192]];
        let t = [1.0f32, 2.0, -30.0];
        let mut numbers = [0f32; 48];
        let (sx, sy, a, b) = (1.92098f32, 0.96049, 0.00167, 0.10017);
        let p: M4 = [[sx, 0.0, 0.0, 0.0], [0.0, sy, 0.0, 0.0], [0.0, 0.0, a, b], [0.0, 0.0, -1.0, 0.0]];
        let mv: M4 = [
            [r[0][0], r[0][1], r[0][2], t[0]],
            [r[1][0], r[1][1], r[1][2], t[1]],
            [r[2][0], r[2][1], r[2][2], t[2]],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let pmv = product(&p, &mv);
        for row in 0..4 {
            numbers[row * 4..row * 4 + 4].copy_from_slice(&mv[row]);
            numbers[16 + row * 4..16 + row * 4 + 4].copy_from_slice(&pmv[row]);
            numbers[32 + row * 4..32 + row * 4 + 4].copy_from_slice(&pmv[row]);
        }
        let by = [0.5, -0.25, 3.0];
        assert!(camera(&mut numbers, &Move { turn: IDENTITY, shift: by }));
        assert!((numbers[11] - (t[2] + 3.0)).abs() < 1e-4, "the object nearer");
        let mut moved = mv;
        for r in 0..3 {
            moved[r][3] += by[r];
        }
        // Our lens: across (view y) and up and down (view x), wider than the mirror's.
        let across = 1.0 / (WIDE.to_radians() / 2.0).tan();
        let mut lens = p;
        lens[0][0] = across * SCREEN_SHAPE;
        lens[1][1] = across;
        let expected = product(&lens, &moved);
        for row in 0..4 {
            for col in 0..4 {
                assert!((numbers[16 + row * 4 + col] - expected[row][col]).abs() < 1e-3, "{row},{col}");
            }
        }
    }

    #[test]
    fn the_players_moves_are_kept_by_truck_and_trailer() {
        use westplay_shared::{
            CAMERA_NUDGE_BACK, CAMERA_NUDGE_BACK_FAR, CAMERA_NUDGE_DOWN, CAMERA_NUDGE_LEFT, CAMERA_NUDGE_RESET,
            CAMERA_NUDGE_RIGHT, CAMERA_NUDGE_ROLL_LEFT, CAMERA_NUDGE_ROLL_RIGHT, CAMERA_NUDGE_TILT_UP,
            CAMERA_NUDGE_TURN_LEFT, CAMERA_NUDGE_UP, CAMERA_NUDGE_WIDER,
        };
        let moved = nudge(nudge(nudge([0.0; 7], CAMERA_NUDGE_BACK), CAMERA_NUDGE_BACK), CAMERA_NUDGE_UP);
        assert_eq!(moved, [0.1, 0.05, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(nudge(moved, CAMERA_NUDGE_DOWN), [0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(nudge(moved, CAMERA_NUDGE_RESET), [0.0; 7]);
        // Long trailers: can move up to 25 m back (including 50 cm quick steps).
        assert_eq!(nudge([6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], CAMERA_NUDGE_BACK_FAR)[0], 6.5);
        assert_eq!(
            nudge([NUDGE_BACK_MOST, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], CAMERA_NUDGE_BACK),
            [NUDGE_BACK_MOST, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
        // Sideways shift and lateral tilt (roll).
        let shifted = nudge(nudge(moved, CAMERA_NUDGE_RIGHT), CAMERA_NUDGE_ROLL_LEFT);
        assert_eq!(shifted[5], NUDGE_STEP);
        assert_eq!(shifted[6], -ROLL_STEP);
        assert_eq!(nudge(shifted, CAMERA_NUDGE_LEFT)[5], 0.0);
        assert_eq!(nudge(shifted, CAMERA_NUDGE_ROLL_RIGHT)[6], 0.0);
        // Turned: up, left, wider; each stops at its most.
        let turned = nudge(nudge(nudge(moved, CAMERA_NUDGE_TILT_UP), CAMERA_NUDGE_TURN_LEFT), CAMERA_NUDGE_WIDER);
        assert_eq!(turned, [0.1, 0.05, TILT_STEP, -TURN_STEP, WIDE_STEP, 0.0, 0.0]);
        assert_eq!(nudge([0.0, 0.0, 0.0, 0.0, WIDE_MOST, 0.0, 0.0], CAMERA_NUDGE_WIDER)[4], WIDE_MOST);
        // Kept as lines, read back the same; the old lines (2 or 5 numbers) still read.
        let mut by_key = std::collections::BTreeMap::new();
        by_key.insert("trailer:-5.18:3.09:1.12".to_owned(), [12.5, -0.25, 4.0, -6.0, 20.0, 0.35, -3.0]);
        by_key.insert("truck:vehicle.scania.s_2016".to_owned(), [0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(read_nudges(&write_nudges(&by_key)), by_key);
        assert_eq!(
            read_nudges("truck:vehicle.daf.xf 0.5 -0.25\n")["truck:vehicle.daf.xf"],
            [0.5, -0.25, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(
            read_nudges("truck:vehicle.daf.xf 0.5 -0.25 1 2 5\n")["truck:vehicle.daf.xf"],
            [0.5, -0.25, 1.0, 2.0, 5.0, 0.0, 0.0]
        );
        assert!(read_nudges("nonsense\n x y z\n").is_empty());
        // A trailer by its layout, the truck alone by its model.
        let t = westplay_shared::Telemetry {
            trailer_connected: true,
            trailer_hook_z: -5.178,
            trailer_axle_z: 3.0868,
            trailer_half_width: 1.1228,
            ..westplay_shared::Telemetry::default()
        };
        assert_eq!(nudge_key(&t, "vehicle.x"), "trailer:-5.18:3.09:1.12");
        assert_eq!(nudge_key(&westplay_shared::Telemetry::default(), "vehicle.x"), "truck:vehicle.x");
    }

    #[test]
    fn the_camera_is_never_inside_the_trailer() {
        // The owner's random trailer (2026-11-20): the axles' guess 7.45, the rays 6.47: behind the guess.
        assert!((trailer_camera_end(6.47, 7.45) - 7.75).abs() < 1e-4);
        // The rays further back than the guess: theirs.
        assert!((trailer_camera_end(8.5, 7.45) - 8.5).abs() < 1e-4);
        assert!((trailer_camera_end(0.0, 7.45) - 7.75).abs() < 1e-4);
        assert!((trailer_camera_end(6.0, 0.0) - 6.0).abs() < 1e-4);
        assert!((trailer_camera_end(0.0, 0.0) - TRAILER_PAST_ORIGIN).abs() < 1e-4);
    }

    #[test]
    fn the_camera_moves_level() {
        // Looking back (+z in the world) and 20 degrees down: view -z is (0, -sin, cos) in the world.
        let (s, c) = (20f32.to_radians().sin(), 20f32.to_radians().cos());
        let turn = [[-1.0, 0.0, 0.0], [0.0, c, s], [0.0, s, -c]];
        let by = shift(&turn);
        // Back in the world: an object FORWARD metres behind is now at the camera, in view space too.
        let world = [0.0, 0.0, FORWARD];
        let seen: [f32; 3] = std::array::from_fn(|r| (0..3).map(|k| turn[r][k] * world[k]).sum::<f32>() + by[r]);
        assert!(seen.iter().all(|x| x.abs() < 1e-4), "{seen:?}");
    }

    #[test]
    fn a_turned_camera_sees_the_world_turned() {
        // The mirror looking south (+z), ours looking west (-x): an object 5 m west is straight ahead of ours.
        let mirror = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
        let ours = [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let how = Move { turn: mul3(&ours, &transpose(&mirror)), shift: [0.0; 3] };
        let world = [-5.0, 0.0, 0.0];
        let seen_by_mirror = apply(&mirror, world);
        let seen = apply(&how.turn, seen_by_mirror);
        assert!((seen[2] + 5.0).abs() < 1e-5 && seen[0].abs() < 1e-5, "{seen:?}");
    }

    #[test]
    fn our_camera_is_rolled_as_the_mirror() {
        // A mirror turn rolled about 90 degrees: looking along the same axis, ours is it rolled a quarter turn
        // exactly (its own slight tilt left out), still a turn, near the mirror's.
        let mirror = [[-0.32335f32, -0.91117, 0.25538], [0.65763, -0.41043, -0.63172], [0.68042, -0.03632, 0.73192]];
        let ours = rolled_as(&mirror, mirror[2], [0.0, 1.0, 0.0]);
        assert_eq!(ours[2], mirror[2]);
        assert!((det(&ours) - det(&mirror)).abs() < 1e-3);
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        assert!(dot(ours[0], mirror[0]) > 0.9 && dot(ours[1], mirror[1]) > 0.9, "{ours:?}");
        // Upright about the world's up: its first row level (a quarter turn exactly).
        assert!(ours[0][1].abs() > 0.99 || ours[1][1].abs() > 0.99, "{ours:?}");
        // Looking elsewhere, still a turn with the same determinant.
        let other = rolled_as(&mirror, normal([0.0, 0.3, 1.0]), [0.0, 1.0, 0.0]);
        assert!((det(&other) - det(&mirror)).abs() < 1e-3);
    }

    #[test]
    fn a_flat_picture_is_left() {
        let mut numbers = [0f32; 48];
        for i in 0..4 {
            numbers[i * 5] = 1.0;
        }
        assert!(!camera(&mut numbers, &Move { turn: IDENTITY, shift: [0.0, 0.0, 1.0] }));
    }
}

#[cfg(test)]
mod tilt_tests {
    use super::*;

    #[test]
    fn level_axes_are_the_headings() {
        let axes = tilted_axes(0.3, 0.0, 0.0);
        let behind = back(0.3, (1.0, 1.0));
        for i in 0..3 {
            assert!((axes[2][i] - behind[i]).abs() < 1e-6);
        }
        assert!((axes[1][1] - 1.0).abs() < 1e-6);
        assert!((axes[0][0] - behind[2]).abs() < 1e-6 && (axes[0][2] + behind[0]).abs() < 1e-6);
    }

    #[test]
    fn a_pitched_truck_still_has_square_axes() {
        let axes = tilted_axes(0.6, 0.02, -0.01);
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        assert!(dot(axes[0], axes[1]).abs() < 1e-5 && dot(axes[1], axes[2]).abs() < 1e-5);
        assert!(axes[1][1] < 0.999);
    }
}

#[cfg(test)]
mod look_tests {
    use super::*;

    #[test]
    fn the_logged_town_turn_is_not_a_mirrors() {
        // Logged 2026-11-14 in a town: houses' turn, looking 130 degrees off the truck's back.
        let houses = [[0.99293846, -0.014178419, -0.117789835], [-0.002288539, -0.9949383, 0.10046911], [-0.11861858, -0.0994899, -0.9879437]];
        assert!(!looks_back(&houses, back(0.6309, (1.0, 1.0))));
    }

    #[test]
    fn a_view_looking_back_is() {
        let behind = back(0.6309, (1.0, 1.0));
        // Third row minus the look; up row the world's, upside down as the game's mirrors are.
        let right = [behind[2], 0.0, -behind[0]];
        let view = [right, [0.0, -1.0, 0.0], [-behind[0], 0.0, -behind[2]]];
        assert!(looks_back(&view, behind));
    }
}

#[cfg(test)]
mod look_tests_2 {
    use super::*;

    #[test]
    fn the_towns_turn_is_refused_whichever_way_the_truck_faces() {
        // Logged 2026-11-14 at heading 0.13: the town's houses again (the same look as at 0.63), 40 degrees off the
        // back, taken with the first filter (50); the true mirror looked 8 off.
        // Since refused for the truck's parts not agreeing (`voted_turn`): 45 degrees let a DAF XD's mirror through,
        // 26 off its back (owner 2026-11-14).
        let daf = [[-0.889, 0.068, -0.453], [-0.182, -0.960, 0.212], [-0.420, 0.271, 0.866]];
        assert!(looks_back(&daf, back(0.3683, (1.0, 1.0))));
        let mirror = [[0.596, 0.0, -0.803], [0.0, -1.0, 0.0], [-0.803, 0.0, -0.596]];
        assert!(looks_back(&mirror, back(0.13, (1.0, 1.0))));
    }
}

/// A probe of what's drawn into our camera's picture (owner 2026-11-14: a dark band across its top, not the truck's
/// parts): once a game, after our camera's been on 5 s, every block bound and draw made into our mirror over a few
/// frames, to `logs\retrovisor\camera-draws.txt`.
static PROBE_STATE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
static PROBE_ON_FOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static PROBE_PRESENTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static PROBE_TEXT: Mutex<String> = Mutex::new(String::new());

fn note(line: impl FnOnce() -> String) {
    if PROBE_STATE.load(Ordering::Relaxed) != 1 || !crate::mirror::drawing_ours() {
        return;
    }
    let mut text = PROBE_TEXT.lock().unwrap_or_else(|e| e.into_inner());
    if text.len() < 600_000 {
        text.push_str(&line());
        text.push('\n');
    }
}

/// `VSSetConstantBuffers` (no offsets), noted in the probe.
pub fn note_vs_cb(start: u32, count: u32) {
    note(|| format!("VSSetConstantBuffers b{start} x{count}"));
}

/// Whether the probe of what's drawn into our camera runs (off from 2026-11-18: its file written is a hitch).
const CAMERA_PROBE: bool = false;

fn probe_present(on: bool) {
    if !CAMERA_PROBE {
        return;
    }
    match PROBE_STATE.load(Ordering::Relaxed) {
        0 => {
            if !on {
                PROBE_ON_FOR.store(0, Ordering::Relaxed);
            } else if PROBE_ON_FOR.fetch_add(1, Ordering::Relaxed) + 1 >= 300 {
                PROBE_STATE.store(1, Ordering::Relaxed);
            }
        }
        1 => {
            let mut text = PROBE_TEXT.lock().unwrap_or_else(|e| e.into_inner());
            text.push_str("--- present\n");
            if PROBE_PRESENTS.fetch_add(1, Ordering::Relaxed) + 1 >= 4 {
                PROBE_STATE.store(2, Ordering::Relaxed);
                let dir = crate::log::log_dir().join("retrovisor");
                let _ = std::fs::create_dir_all(&dir);
                match std::fs::write(dir.join("camera-draws.txt"), std::mem::take(&mut *text)) {
                    Ok(()) => log!("retrovisor: what's drawn into our camera's picture written (camera-draws.txt)"),
                    Err(e) => log!("retrovisor: camera-draws.txt not written: {e}"),
                }
            }
        }
        _ => {}
    }
}
