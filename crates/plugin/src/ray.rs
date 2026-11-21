//! Rays through the game's own physics (PhysX 3.4, under the game's own layer), for the parking sensors: what
//! the truck itself would bump into behind it (walls, posts, the map's scenery, the traffic, trailers), on any
//! map, which WestPlay's map can't tell (owner 2026-11-13: "the truck bumps into it: use that").
//!
//! The game's own ray (`casing`, found by its code's shape, the same in ETS2 1.61 and ATS 1.62: ETS2
//! `0x1635990`, ATS `0x16967e0`) takes its physics world, a hit callback, a place (the game's: local x, y, z in a
//! 512 m sector, and the sector), a direction, a distance and a mask of collision layers; it turns the place into
//! PhysX's coordinates (less the world's origin, `+0x110`) and casts in the world's scene (`+0x178`), through
//! `PxScene::raycast` (its vtable's `0x2f0`).
//!
//! Casting from the plugin's own place in the frame closed the game twice in the Modo Personagem (2026-11-05,
//! docs/personagem/antigo/ray.rs.txt: a scene found by looking through pointers, cast at the frame's end). So
//! here no scene is looked for and no moment chosen: the casing is hooked, and the plugin's rays go only from
//! inside one of the game's own calls, right after it, in the same world and on the same thread, where the game
//! itself casts. The scene is checked first (its vtable's raycast the one found), then a ray down from over the
//! truck must meet the ground near it, else no rays in that world.
//!
//! And a guard against closing the game again: before the first ray, `rays.txt` (the game's WestPlay folder)
//! says "testing"; after [`CONFIRM_BATCHES`] batches cast, "ok". Found saying "testing" at the next start (the
//! game closed meanwhile), the rays are tried again, up to [`STRIKES`] closes in a row; then it says "off" and they
//! stay off until another version of the plugin (or deleting the file) tries again (owner 2026-11-18: off after one
//! close, the sensors saw nothing on a protected map, Eldorado's, whose buildings the app can't read).
//!
//! Not under TruckersMP (no hook in the game's code there).

use std::cell::Cell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentThreadId};
use windows::core::w;

use crate::log;
use crate::navigation::Image;
use westplay_shared::Telemetry;

/// The casing's start: `push rbp; lea rbp, [rsp-3Fh]; sub rsp, 0B0h; movss xmm3, [r9]; mov r10, rcx; movss xmm4,
/// [r9+4]; movaps xmm2, xmm3; movss xmm5, [r9+8]` (the direction's length first).
const CASING: &str = "40 55 48 8D 6C 24 C1 48 81 EC B0 00 00 00 F3 41 0F 10 19 4C 8B D1 F3 41 0F 10 61 04 0F 28 D3 \
                      F3 41 0F 10 69 08";
/// In it, the cast: `mov rcx, [r10+scene]; lea r8, [rbp+7]; mov [rsp+30h], r8; mov eax, 407h; ...; call
/// [rax+slot]` (the scene's raycast, PhysX's default hit flags).
const CASING_CALL: &str = "49 8B 8A ?? ?? ?? ?? 4C 8D 45 07 4C 89 44 24 30 B8 07 04 00 00 4C 8D 45 DF 66 89 45 DF \
                           4C 89 44 24 28 4C 8D 45 E7 48 8B 01 48 89 54 24 20 48 8D 55 F7 FF 90 ?? ?? ?? ??";
const CALL_AT: usize = 0x171;
/// `PxScene::raycast`'s first bytes (NpScene's; another class's starts so too: both are kept).
const PX_RAYCAST: &str = "F3 0F 11 5C 24 20 4C 8B DC 53 48 83 EC 70 0F AE 9C 24 88 00 00 00 8B 9C 24 88 00 00 00 \
                          4C 8B D1 C7 84 24 88 00 00 00 C0 9F 00 00 0F AE 94 24 88 00 00 00 0F 18 02 41 0F 18 00";

/// Collision layers (docs/personagem/INVESTIGACAO.md §7.1): 1 and 2 the fixed world (roads, buildings, the map's
/// models), 4 the traffic, 8 and 10 trailers. Not 6 and 7 (the player's truck) nor the people's.
const SOLID: u32 = 1 << 1 | 1 << 2 | 1 << 4 | 1 << 8 | 1 << 10;
const FIXED_WORLD: u32 = 1 << 1 | 1 << 2;
const TRAILERS: u32 = 1 << 8 | 1 << 10;

/// The trailer's rear as it really is, not as its wheels put it (`reversing::rear_overhang`'s guess; owner
/// 2026-11-13, a harvester on a low-loader: "with the trailer it beeps only when I'm already on it"): rays from
/// this far behind the guessed rear, forward at it, at these heights over the ground and across its width
/// (fractions of its half width), only at trailers. The furthest back they meet (its load included) is its rear.
const CALIBRATE_FROM: f64 = 8.0;
const CALIBRATE_INTO: f64 = 6.0;
const CALIBRATE_HEIGHTS: [f64; 5] = [0.5, 1.0, 1.5, 2.2, 3.0];
const CALIBRATE_ACROSS: [f64; 3] = [-0.6, 0.0, 0.6];
/// Batches measured, how far apart the middle of them may be (metres), how far from the guess the rear may be
/// found (behind it positive), and how many times it's measured again before the guess is kept.
const CALIBRATE_SAMPLES: usize = 15;
const CALIBRATE_SPREAD: f64 = 0.3;
const CALIBRATE_WITHIN: (f64, f64) = (-4.0, 6.0);
const CALIBRATE_TRIES: u32 = 3;
/// A hit whose surface faces up more than this is the ground (or a ramp), not something in the way.
const GROUND_NORMAL: f32 = 0.7;

/// The self-test: a ray down from this far over the truck's placement, this long; the ground must be within.
const TEST_OVER: f64 = 3.0;
const TEST_LENGTH: f64 = 10.0;
const GROUND_WITHIN: (f64, f64) = (1.0, 6.0);
/// The sensors' rays: this far over the ground, starting this far behind the rear.
const SENSOR_HEIGHT: f64 = 0.6;
const BEHIND: f64 = 0.05;

/// The rear's four sensors, as the page's (ui/src/cabin/reversing/sensor.ts `SENSORS`): across the rear (-1 its
/// left corner, 1 its right), which way the middle of each looks out from straight back (radians, right
/// positive), half its cone, how far it sees.
const SENSORS: [(f64, f64, f64, f64); 4] =
    [(-0.92, -0.35, 0.6, 2.0), (-0.35, 0.0, 0.5, 3.0), (0.35, 0.0, 0.5, 3.0), (0.92, 0.35, 0.6, 2.0)];
/// Rays across each sensor's cone.
const RAYS_PER_SENSOR: usize = 3;

/// How far the truck may have moved (metres) and its readings still hold, no new rays cast.
const STILL: f64 = 0.05;
/// Batches cast before the guard says "ok".
// 120, not 600 (owner 2026-11-14): batches count only while reversing, so 600 took minutes of it, and twice the game
// closed before then without the plugin told (no shutdown): the sensor was turned off though the rays were fine.
const CONFIRM_BATCHES: u32 = 120;
/// Frames a batch's readings are kept for, the game not casting meanwhile.
const READINGS_FOR: u64 = 6;

/// How the rays are, for the page (`Telemetry::rays`).
pub const NOT_YET: u8 = 0;
pub const WORKING: u8 = 1;
pub const OFF_AFTER_CLOSE: u8 = 2;
pub const UNAVAILABLE: u8 = 3;

/// The game's place: local x, y, z (metres) in a 512 m sector, and the sector's x and z.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Place {
    local: [f32; 3],
    sector: [i16; 2],
}

const SECTOR: f64 = 512.0;

impl Place {
    fn at(world: [f64; 3]) -> Self {
        let sector = |v: f64| (v / SECTOR).round() as i16;
        let (sx, sz) = (sector(world[0]), sector(world[2]));
        Place {
            local: [
                (world[0] - f64::from(sx) * SECTOR) as f32,
                world[1] as f32,
                (world[2] - f64::from(sz) * SECTOR) as f32,
            ],
            sector: [sx, sz],
        }
    }
}

/// `PxRaycastHit` (PhysX 3.4, 64-bit, as MSVC lays it out): `PxQueryHit` (actor, shape, face) padded to 24 bytes
/// (MSVC puts a derived class's fields after its base's padding), then `PxLocationHit`'s flags, position, normal
/// and distance, then u, v. Seen in the game 2026-11-13: read without the padding, a ray down read the ground's
/// normal (0, 1, 0) four bytes off, as the distance and the normal.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Hit {
    actor: usize,
    shape: usize,
    face: u32,
    _query_hit_padding: u32,
    flags: u16,
    position: [f32; 3],
    normal: [f32; 3],
    distance: f32,
    uv: [f32; 2],
}

/// `PxHitCallback<PxRaycastHit>` (no room for touches: the nearest blocking hit only), and room after it for the
/// casing's own two fields (it writes the world at `+0x68` and the distance at `+0x70`, as the game's own
/// callback has them).
#[repr(C)]
struct Callback {
    vtable: *const CallbackVtable,
    block: Hit,
    has_block: bool,
    touches: *mut Hit,
    most_touches: u32,
    touches_found: u32,
    game: [u8; 0x60],
}

#[repr(C)]
struct CallbackVtable {
    process_touches: unsafe extern "system" fn(*mut Callback, *const Hit, u32) -> bool,
    finalize_query: unsafe extern "system" fn(*mut Callback),
    drop: unsafe extern "system" fn(*mut Callback, u32) -> *mut c_void,
}

unsafe extern "system" fn process_touches(_: *mut Callback, _: *const Hit, _: u32) -> bool {
    false
}
unsafe extern "system" fn finalize_query(_: *mut Callback) {}
unsafe extern "system" fn drop_callback(_: *mut Callback, _: u32) -> *mut c_void {
    std::ptr::null_mut()
}
static CALLBACK_VTABLE: CallbackVtable = CallbackVtable { process_touches, finalize_query, drop: drop_callback };

/// The casing: world, callback, place, direction, distance, layers, filter callback (none) → hit.
type Casing =
    unsafe extern "system" fn(*mut u8, *mut Callback, *const Place, *const [f32; 3], f32, u32, *mut c_void) -> bool;

/// What was found in the game's code.
struct Found {
    casing: usize,
    scene_at: usize,
    raycast_slot: usize,
    raycasts: Vec<usize>,
}

/// One ray: from (world metres), along (a unit vector), how far.
#[derive(Clone, Copy, Debug)]
struct Ray {
    from: [f64; 3],
    along: [f64; 3],
    length: f64,
}

/// What the frame wants cast, made at the frame's end.
#[derive(Clone, Debug)]
struct Request {
    frame: u64,
    /// The truck's placement, for the ground check of a world not checked yet.
    truck: [f64; 3],
    /// The rear sensors' rays (in [`SENSORS`]' order, [`RAYS_PER_SENSOR`] each), from the ground as it was found.
    sensors: Vec<Ray>,
    /// The rays measuring the trailer's rear (none once it's known), and the trailer they're for.
    calibrate: Vec<Ray>,
    trailer: [u32; 4],
}

/// The trailer's rear as measured: for which trailer (its layout and coupling), each batch's finding (how far
/// behind the guessed rear its furthest back surface is, metres), and the result.
#[derive(Debug)]
struct Calibration {
    trailer: [u32; 4],
    samples: Vec<f64>,
    tries: u32,
    behind: Option<f64>,
}

impl Calibration {
    /// Takes in one batch's finding; once there are enough, the rear (their middle, if they agree).
    fn add(&mut self, behind: f64) {
        if self.behind.is_some() || self.tries >= CALIBRATE_TRIES {
            return;
        }
        self.samples.push(behind);
        if self.samples.len() < CALIBRATE_SAMPLES {
            return;
        }
        self.samples.sort_by(f64::total_cmp);
        let n = self.samples.len();
        let (low, middle, high) = (self.samples[n / 5], self.samples[n / 2], self.samples[n - 1 - n / 5]);
        if high - low <= CALIBRATE_SPREAD {
            self.behind = Some(middle);
            log!("ray: the trailer's rear measured {middle:+.2} m from the guess ({low:+.2} to {high:+.2})");
        } else {
            self.tries += 1;
            log!(
                "ray: the trailer's rear measured from {low:+.2} to {high:+.2} m: not steady, measured again{}",
                if self.tries >= CALIBRATE_TRIES { " no more (the guess kept)" } else { "" }
            );
        }
        self.samples.clear();
    }
}

/// The last batch's readings: each sensor's nearest thing (metres; `None`: nothing in its range).
#[derive(Clone, Copy, Debug, Default)]
struct Readings {
    frame: u64,
    sensors: [Option<f32>; 4],
    /// Where the truck was (its placement), for keeping them while it stands still.
    truck: [f64; 3],
}

struct State {
    found: Option<Found>,
    looked: bool,
    hooked: bool,
    guard: Guard,
    /// The closes in a row while the rays were tested, before this start (`STRIKES`).
    strikes: u32,
    /// Worlds checked (the ground found under the truck: how far under its placement), or found wrong.
    worlds: Vec<(usize, Option<f64>)>,
    request: Option<Request>,
    readings: Readings,
    calibration: Calibration,
    batches: u32,
    /// For the log: the game's own rays since the last line, and when that was.
    logged_at: Option<Instant>,
}

static STATE: Mutex<State> = Mutex::new(State {
    found: None,
    looked: false,
    hooked: false,
    guard: Guard::Unread,
    strikes: 0,
    worlds: Vec::new(),
    request: None,
    readings: Readings { frame: 0, sensors: [None; 4], truck: [0.0; 3] },
    calibration: Calibration { trailer: [0; 4], samples: Vec::new(), tries: 0, behind: None },
    batches: 0,
    logged_at: None,
});

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static GAME_RAYS: AtomicU32 = AtomicU32::new(0);
static GAME_THREAD: AtomicU32 = AtomicU32::new(0);
static THREADS_LOGGED: AtomicU32 = AtomicU32::new(0);
static FRAME: AtomicU64 = AtomicU64::new(0);
static STATUS: AtomicU8 = AtomicU8::new(NOT_YET);

thread_local! {
    /// Set while the plugin's own rays are cast (they go through the original, not the hook; this is a second
    /// guard against casting from inside a cast).
    static CASTING: Cell<bool> = const { Cell::new(false) };
}

/// The guard file's word.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Guard {
    Unread,
    /// No file, or "ok": rays may go.
    Clear,
    /// "testing" written this session (until [`CONFIRM_BATCHES`]).
    Testing,
    Confirmed,
    /// "off": the game closed while they were tested.
    Off,
}

fn guard_file() -> PathBuf {
    crate::log::game_dir().join("rays.txt")
}

/// Closes of the game in a row while the rays were tested before they're left off.
const STRIKES: u32 = 3;

/// This plugin's version, in the guard file's "off" (another version tries the rays again).
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The guard as the file says, and the closes in a row it counts ([`STRIKES`]).
fn read_guard() -> (Guard, u32) {
    let text = std::fs::read_to_string(guard_file()).unwrap_or_default();
    let (guard, strikes, write) = guard_from(&text, VERSION);
    if let Some(write) = write {
        write_guard(&write);
    }
    match guard {
        Guard::Off => log!(
            "ray: off, as {:?} says ({strikes} closes in a row; another version, or deleting it, tries again)",
            guard_file()
        ),
        _ if strikes > 0 => log!(
            "ray: the game closed {strikes} times in a row while the rays were tested: tried again (left off at {STRIKES})"
        ),
        _ => {}
    }
    (guard, strikes)
}

/// The guard from the file's text, for this version: the guard, the closes in a row, and what to write there now.
/// "testing N" (N closes before): one more; "retry N": N; "off N vX" of another version, or an "off" of before the
/// closes were counted: tried again.
fn guard_from(text: &str, version: &str) -> (Guard, u32, Option<String>) {
    let number = |rest: &str| rest.split_whitespace().next().and_then(|n| n.trim_end_matches(':').parse::<u32>().ok()).unwrap_or(0);
    if text.is_empty() || text.starts_with("ok") {
        return (Guard::Clear, 0, None);
    }
    if let Some(rest) = text.strip_prefix("testing") {
        let strikes = number(rest) + 1;
        if strikes >= STRIKES {
            let off = format!(
                "off {strikes} v{version}: the game closed {strikes} times in a row while WestPlay's physics rays were being tested; delete this file to try again"
            );
            return (Guard::Off, strikes, Some(off));
        }
        return (Guard::Clear, strikes, Some(format!("retry {strikes}: the game closed while the rays were tested")));
    }
    if let Some(rest) = text.strip_prefix("retry") {
        return (Guard::Clear, number(rest), None);
    }
    if let Some(rest) = text.strip_prefix("off") {
        let of_this = rest.split_whitespace().nth(1).is_some_and(|v| v.trim_end_matches(':') == format!("v{version}"));
        if of_this {
            return (Guard::Off, number(rest).max(STRIKES), None);
        }
        // Another version's, or one of before the closes were counted (off after one close): tried again.
        return (Guard::Clear, 0, Some("retry 0: off for another version, tried again".to_owned()));
    }
    (Guard::Clear, 0, None)
}

fn write_guard(word: &str) {
    if let Err(e) = std::fs::write(guard_file(), format!("{word}\n")) {
        log!("ray: the guard file {:?} unwritable ({e})", guard_file());
    }
}

/// Whether TruckersMP's client is in the game.
fn under_truckersmp() -> bool {
    // SAFETY: plain queries of the process's modules by name.
    crate::truckersmp::loaded()
        || unsafe { GetModuleHandleW(w!("core_ets2mp.dll")).is_ok() || GetModuleHandleW(w!("core_atsmp.dll")).is_ok() }
}

fn pattern(text: &str) -> Vec<Option<u8>> {
    text.split_whitespace().map(|t| if t == "??" { None } else { u8::from_str_radix(t, 16).ok() }).collect()
}

/// Reads a `T` at `at` in the game's memory, if it's readable there.
///
/// # Safety
/// `T` is plain data (any bytes are a valid `T`).
unsafe fn read<T: Copy + Default>(at: usize) -> Option<T> {
    let mut value = T::default();
    let mut got = 0usize;
    // SAFETY: reads into `value`, its own size; the process's own memory, through the system's checked copy.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            at as *const _,
            (&mut value as *mut T).cast(),
            size_of::<T>(),
            Some(&mut got),
        )
    };
    (ok.is_ok() && got == size_of::<T>()).then_some(value)
}

/// The scene's offset in the world and the raycast's slot in the scene's vtable, from the casing's cast
/// ([`CASING_CALL`]'s bytes: `mov rcx, [r10+scene]` at its start, `call [rax+slot]` at its end).
fn read_call(call: &[u8]) -> Option<(usize, usize)> {
    let pattern = pattern(CASING_CALL);
    if call.len() < pattern.len() || !call.iter().zip(&pattern).all(|(b, p)| p.is_none_or(|p| *b == p)) {
        return None;
    }
    let u32_at = |at: usize| u32::from_le_bytes(call[at..at + 4].try_into().unwrap()) as usize;
    Some((u32_at(3), u32_at(pattern.len() - 4)))
}

/// Finds the casing and PhysX's raycast in the game's code.
fn locate() -> Option<Found> {
    // SAFETY: no name: the game's own executable, mapped for the process's life.
    let base = unsafe { GetModuleHandleW(None) }.ok()?.0 as usize;
    // SAFETY: `base` is the game's executable, as mapped.
    let image = unsafe { Image::of(base) }?;
    let Some(casing) = image.unique_masked(&pattern(CASING)) else {
        log!("ray: not found: the game's physics ray (the sensors go by the map only)");
        return None;
    };
    let Some((scene_at, raycast_slot)) = image.code_at(casing + CALL_AT, 56).and_then(read_call) else {
        log!("ray: the game's physics ray isn't as known (the sensors go by the map only)");
        return None;
    };
    let raycasts = image.all_masked(&pattern(PX_RAYCAST));
    if raycasts.is_empty() || raycasts.len() > 4 || scene_at > 0x10000 || raycast_slot > 0x1000 {
        log!("ray: PhysX's raycast not found as known (the sensors go by the map only)");
        return None;
    }
    log!(
        "ray: found the game's physics ray at +{:#x} (scene at +{scene_at:#x}, raycast slot {raycast_slot:#x})",
        casing - base
    );
    Some(Found { casing, scene_at, raycast_slot, raycasts })
}

/// The third-person camera's ray (owner 2026-11-19: "when the camera comes near a thing, zoom in on the person;
/// getting out with something near it, the camera ends up inside things"): what the walking frame asks (from the
/// person's chest to where the camera would be), and the last answer (how far to the first thing; `None`: nothing),
/// with the frame each was for. Cast as the sensors' are, in a world already checked.
struct CameraRay {
    ask: Option<(u64, Ray)>,
    answer: Option<(u64, Option<f32>)>,
}

static CAMERA: Mutex<CameraRay> = Mutex::new(CameraRay { ask: None, answer: None });
/// What the camera's ray meets: the fixed world, the traffic, trailers, and the player's own truck (6, 7).
const CAMERA_LAYERS: u32 = SOLID | 1 << 6 | 1 << 7;
/// Frames an answer is kept for (the game casts about 50 times a second on foot).
const CAMERA_FRESH: u64 = 10;
/// Camera rays cast (this session), and how many before the guard says ok for them (about a minute on foot).
static CAMERA_CASTS: AtomicU32 = AtomicU32::new(0);
static CAMERA_TESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const CAMERA_CONFIRM: u32 = 3000;

/// Asks for the camera's ray this frame: from `from` to `to` (world metres).
pub fn camera_ask(from: [f64; 3], to: [f64; 3]) {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if !(0.05..50.0).contains(&length) {
        return;
    }
    let along = d.map(|v| v / length);
    if let Ok(mut camera) = CAMERA.lock() {
        camera.ask = Some((FRAME.load(Ordering::Relaxed), Ray { from, along, length }));
    }
}

/// The camera's ray's last answer, if fresh: `Some(None)` nothing in the way, `Some(Some(d))` a thing `d` metres
/// along it; `None` no answer (no rays: the camera as it was).
pub fn camera_answer() -> Option<Option<f32>> {
    let camera = CAMERA.lock().ok()?;
    let (frame, answer) = camera.answer?;
    (FRAME.load(Ordering::Relaxed).saturating_sub(frame) <= CAMERA_FRESH).then_some(answer)
}

/// Not walking in third person: no more rays for the camera.
pub fn camera_stop() {
    if let Ok(mut camera) = CAMERA.lock() {
        camera.ask = None;
        camera.answer = None;
    }
}

/// The camera's ray, if asked, in `world` if it's a checked one (the ground found under the truck in it).
///
/// # Safety
/// As [`after_game_ray`].
unsafe fn cast_camera(state: &State, world: usize, original: Casing) {
    if !state.worlds.iter().any(|(w, ground)| *w == world && ground.is_some()) {
        return;
    }
    let Ok(mut camera) = CAMERA.try_lock() else { return };
    let Some((frame, ray)) = camera.ask.take() else { return };
    // SAFETY: as this function's contract says.
    let hit = unsafe { cast(original, world, ray, CAMERA_LAYERS) }.map(|(d, _)| d);
    camera.answer = Some((frame, hit));
}

/// The game's casing, hooked: the game's own ray, then the plugin's (if a frame asked), from inside it.
unsafe extern "system" fn casing_detour(
    world: *mut u8,
    callback: *mut Callback,
    place: *const Place,
    along: *const [f32; 3],
    length: f32,
    layers: u32,
    filter: *mut c_void,
) -> bool {
    // SAFETY: the trampoline to the game's own casing, with the game's own arguments.
    let original: Casing = unsafe { std::mem::transmute::<usize, Casing>(ORIGINAL.load(Ordering::Acquire)) };
    // SAFETY: as the game called it.
    let hit = unsafe { original(world, callback, place, along, length, layers, filter) };
    GAME_RAYS.fetch_add(1, Ordering::Relaxed);
    if !CASTING.with(Cell::get) {
        // SAFETY: on the thread and in the world the game cast in, right after it (this module's argument).
        crate::hooks::guarded(|| unsafe { after_game_ray(world as usize, original) });
    }
    hit
}

/// Casts what the frame asked for, if anything, in `world` (the game's own, as it just cast in it).
///
/// # Safety
/// From inside the game's own call of its casing, with that call's world; `original` the casing's trampoline.
unsafe fn after_game_ray(world: usize, original: Casing) {
    let Ok(mut state) = STATE.try_lock() else { return };
    if matches!(state.guard, Guard::Off | Guard::Unread) {
        return;
    }
    // The third-person camera's ray, in a world already checked (the scene's raycast the one found).
    // SAFETY: reads guarded; as this function's contract says.
    if unsafe { scene_is_known(&state, world) } && CAMERA.try_lock().is_ok_and(|c| c.ask.is_some()) {
        // Under the guard as the sensors' rays: "testing" until enough of them went without trouble (a camera's
        // ray, new, may close the game where the sensors' never did: owner 2026-11-05, the first one did twice).
        if matches!(state.guard, Guard::Clear | Guard::Confirmed) && !CAMERA_TESTED.load(Ordering::Relaxed) {
            write_guard(&format!("testing {}", state.strikes));
            state.guard = Guard::Testing;
            log!("ray: the camera's rays through the game's physics (guard says testing)");
        }
        CASTING.with(|c| c.set(true));
        // SAFETY: as this function's contract says.
        unsafe { cast_camera(&state, world, original) };
        CASTING.with(|c| c.set(false));
        let casts = CAMERA_CASTS.fetch_add(1, Ordering::Relaxed) + 1;
        if casts == CAMERA_CONFIRM && state.guard == Guard::Testing {
            CAMERA_TESTED.store(true, Ordering::Relaxed);
            write_guard("ok: WestPlay's physics rays worked (the camera's too)");
            state.guard = Guard::Confirmed;
            log!("ray: {CAMERA_CONFIRM} camera rays cast without trouble: the guard says ok");
        }
    }
    if state.request.is_none() {
        return;
    }
    // SAFETY: a plain query of the calling thread.
    let thread = unsafe { GetCurrentThreadId() };
    if GAME_THREAD.swap(thread, Ordering::Relaxed) != thread && THREADS_LOGGED.fetch_add(1, Ordering::Relaxed) < 4 {
        log!("ray: the game casts its rays on thread {thread}");
    }
    // SAFETY: reads guarded.
    if !unsafe { scene_is_known(&state, world) } {
        return;
    }
    let request = state.request.clone().expect("checked above");
    if state.guard == Guard::Clear {
        write_guard(&format!("testing {}", state.strikes));
        state.guard = Guard::Testing;
        log!("ray: the first rays through the game's physics (guard {:?} says testing)", guard_file());
    }
    CASTING.with(|c| c.set(true));
    // The world's ground under the truck, once: the check that it's the world the truck is in.
    let ground = match state.worlds.iter().find(|(w, _)| *w == world) {
        Some((_, ground)) => *ground,
        None => {
            let from = [request.truck[0], request.truck[1] + TEST_OVER, request.truck[2]];
            // SAFETY: as this function's contract says.
            let down = unsafe {
                cast(original, world, Ray { from, along: [0.0, -1.0, 0.0], length: TEST_LENGTH }, FIXED_WORLD)
            };
            let ground = down
                .map(|(d, _)| f64::from(d))
                .filter(|d| (GROUND_WITHIN.0..=GROUND_WITHIN.1).contains(d))
                .map(|d| d - TEST_OVER);
            match ground {
                Some(under) => log!("ray: world {world:#x}: the ground {:.2} m under the truck's placement", -under),
                None => {
                    log!("ray: world {world:#x}: no ground under the truck ({down:?}): not the truck's, no rays in it")
                }
            }
            if state.worlds.len() < 16 {
                state.worlds.push((world, ground));
            }
            ground
        }
    };
    if let Some(under) = ground {
        let mut sensors = [None; 4];
        for (index, ray) in request.sensors.iter().enumerate() {
            let ray = Ray { from: [ray.from[0], ray.from[1] + under, ray.from[2]], ..*ray };
            // SAFETY: as this function's contract says.
            let reading = unsafe { cast(original, world, ray, SOLID) }
                .filter(|(_, normal)| normal[1] < GROUND_NORMAL)
                .map(|(d, _)| d);
            let sensor = &mut sensors[index / RAYS_PER_SENSOR];
            *sensor = match (*sensor, reading) {
                (Some(a), Some(b)) => Some(f32::min(a, b)),
                (a, b) => a.or(b),
            };
        }
        state.readings = Readings { frame: request.frame, sensors, truck: request.truck };
        // The trailer's rear: the furthest back any of the rays met it (each ray from behind, forward at it).
        let mut behind: Option<f64> = None;
        for ray in &request.calibrate {
            let ray = Ray { from: [ray.from[0], ray.from[1] + under, ray.from[2]], ..*ray };
            // SAFETY: as this function's contract says.
            if let Some((d, _)) = unsafe { cast(original, world, ray, TRAILERS) } {
                let found = CALIBRATE_FROM - f64::from(d);
                behind = Some(behind.map_or(found, |b: f64| b.max(found)));
            }
        }
        if let Some(behind) = behind.filter(|b| (CALIBRATE_WITHIN.0..=CALIBRATE_WITHIN.1).contains(b))
            && state.calibration.trailer == request.trailer
        {
            state.calibration.add(behind);
        }
        state.request = None;
        state.batches += 1;
        if state.batches == CONFIRM_BATCHES && state.guard == Guard::Testing {
            write_guard("ok: WestPlay's physics rays worked");
            state.guard = Guard::Confirmed;
            log!("ray: {CONFIRM_BATCHES} batches cast without trouble: the guard says ok");
        }
    }
    CASTING.with(|c| c.set(false));
}

/// Whether `world`'s scene is a PhysX scene whose raycast is the one found (else no ray in it).
///
/// # Safety
/// Reads guarded.
unsafe fn scene_is_known(state: &State, world: usize) -> bool {
    let Some(found) = state.found.as_ref() else { return false };
    // SAFETY: each read guarded.
    unsafe {
        let Some(scene) = read::<usize>(world + found.scene_at).filter(|&s| s > 0x10000) else { return false };
        let Some(vtable) = read::<usize>(scene) else { return false };
        read::<usize>(vtable + found.raycast_slot).is_some_and(|f| found.raycasts.contains(&f))
    }
}

/// One ray through the game's casing: how far to the first thing on `layers`, and its surface's normal.
///
/// # Safety
/// As [`after_game_ray`].
unsafe fn cast(original: Casing, world: usize, ray: Ray, layers: u32) -> Option<(f32, [f32; 3])> {
    let place = Place::at(ray.from);
    let along = ray.along.map(|v| v as f32);
    let mut callback = Callback {
        vtable: &CALLBACK_VTABLE,
        block: Hit::default(),
        has_block: false,
        touches: std::ptr::null_mut(),
        most_touches: 0,
        touches_found: 0,
        game: [0; 0x60],
    };
    // SAFETY: the game's casing with its own arguments' types, from inside its own call (the contract).
    let hit = unsafe {
        original(world as *mut u8, &mut callback, &place, &along, ray.length as f32, layers, std::ptr::null_mut())
    };
    let d = callback.block.distance;
    (hit && callback.has_block && d.is_finite() && (0.0..=ray.length as f32 + 0.01).contains(&d))
        .then_some((d, callback.block.normal))
}

/// The rear's sensors' rays for the rig as the telemetry has it (heights from its placement: the ground's found
/// by the self-test, added when cast).
fn sensor_rays(t: &Telemetry) -> Vec<Ray> {
    let (x, y, z, heading, rear_z, half_width) = if t.trailer_connected && t.trailer_half_width > 0.0 {
        (t.trailer_x, t.trailer_y, t.trailer_z, t.trailer_heading, t.trailer_rear_z, t.trailer_half_width)
    } else {
        (t.x, t.y, t.z, t.heading, t.rear_z, t.half_width)
    };
    let h = f64::from(heading) * std::f64::consts::TAU;
    // The game's heading turns counterclockwise from north (-z): back is (sin, cos), right (cos, -sin).
    let back = [h.sin(), h.cos()];
    let right = [h.cos(), -h.sin()];
    let half = f64::from(half_width.max(1.0));
    let rear = [x + back[0] * (f64::from(rear_z) + BEHIND), z + back[1] * (f64::from(rear_z) + BEHIND)];
    let mut rays = Vec::with_capacity(SENSORS.len() * RAYS_PER_SENSOR);
    for (across, aim, half_cone, range) in SENSORS {
        let at = [rear[0] + right[0] * half * across, rear[1] + right[1] * half * across];
        for ray in 0..RAYS_PER_SENSOR {
            let angle = aim + half_cone * ((2 * ray) as f64 / (RAYS_PER_SENSOR - 1) as f64 - 1.0) * 0.8;
            let along =
                [back[0] * angle.cos() + right[0] * angle.sin(), 0.0, back[1] * angle.cos() + right[1] * angle.sin()];
            rays.push(Ray { from: [at[0], y + SENSOR_HEIGHT, at[1]], along, length: range });
        }
    }
    rays
}

/// The rays measuring the trailer's rear (heights from its placement), from behind its guessed rear, forward.
fn calibration_rays(t: &Telemetry) -> Vec<Ray> {
    let h = f64::from(t.trailer_heading) * std::f64::consts::TAU;
    let back = [h.sin(), h.cos()];
    let right = [h.cos(), -h.sin()];
    let half = f64::from(t.trailer_half_width.max(1.0));
    let out = f64::from(t.trailer_rear_z) + CALIBRATE_FROM;
    let mut rays = Vec::with_capacity(CALIBRATE_HEIGHTS.len() * CALIBRATE_ACROSS.len());
    for height in CALIBRATE_HEIGHTS {
        for across in CALIBRATE_ACROSS {
            let from = [
                t.trailer_x + back[0] * out + right[0] * half * across,
                t.trailer_y + height,
                t.trailer_z + back[1] * out + right[1] * half * across,
            ];
            rays.push(Ray { from, along: [-back[0], 0.0, -back[1]], length: CALIBRATE_FROM + CALIBRATE_INTO });
        }
    }
    rays
}

/// Which trailer is coupled (its layout and its coupling), to measure its rear again when it changes.
fn trailer_key(t: &Telemetry) -> [u32; 4] {
    if !t.trailer_connected || t.trailer_half_width <= 0.0 {
        return [0; 4];
    }
    [t.trailer_rear_z.to_bits(), t.trailer_hook_z.to_bits(), t.trailer_half_width.to_bits(), 1]
}

/// At each frame's end (the game's main thread): asks for the rays this frame wants, and puts the last readings
/// into `t` (`ray_sensors`, `rays`).
pub fn frame(t: &mut Telemetry) {
    let frame = FRAME.fetch_add(1, Ordering::Relaxed) + 1;
    t.ray_sensors = [-1.0; 4];
    let Ok(mut state) = STATE.lock() else { return };
    if state.guard == Guard::Unread {
        (state.guard, state.strikes) = read_guard();
    }
    let truck_known = t.half_width > 0.0;
    if !state.looked && truck_known {
        state.looked = true;
        if under_truckersmp() {
            log!("ray: not under TruckersMP (the sensors go by the map only)");
        } else if state.guard != Guard::Off {
            state.found = locate();
            if let Some(found) = state.found.as_ref() {
                // SAFETY: the casing's start, and a detour of its signature calling on through the trampoline.
                match unsafe { crate::hooks::hook_now(found.casing, casing_detour as *mut c_void, &ORIGINAL, "ray") } {
                    Ok(()) => state.hooked = true,
                    Err(e) => log!("ray: not hooked ({e})"),
                }
            }
        }
    }
    let status = match (state.guard, state.hooked) {
        (Guard::Off, _) => OFF_AFTER_CLOSE,
        (_, false) if state.looked => UNAVAILABLE,
        (_, false) => NOT_YET,
        _ if state.worlds.iter().any(|(_, ground)| ground.is_some()) => WORKING,
        _ => NOT_YET,
    };
    STATUS.store(status, Ordering::Relaxed);
    t.rays = status;
    if !state.hooked || state.guard == Guard::Off {
        return;
    }
    let reversing = t.gear < 0 || t.displayed_gear < 0;
    let checked = state.worlds.iter().any(|(_, ground)| ground.is_some());
    // The trailer's rear: measured once per trailer coupled (any gear: the game casts while the truck moves), and
    // where it is from then on, for the sensors, the camera and the lines on the page.
    let trailer = trailer_key(t);
    if state.calibration.trailer != trailer {
        state.calibration = Calibration { trailer, samples: Vec::new(), tries: 0, behind: None };
    }
    let calibrating =
        trailer != [0; 4] && state.calibration.behind.is_none() && state.calibration.tries < CALIBRATE_TRIES;
    let calibrate = if calibrating { calibration_rays(t) } else { Vec::new() };
    if let Some(behind) = state.calibration.behind.filter(|_| trailer != [0; 4]) {
        t.trailer_rear_z += behind as f32;
    }
    state.request = (reversing || !checked || calibrating).then(|| Request {
        frame,
        truck: [t.x, t.y, t.z],
        sensors: if reversing { sensor_rays(t) } else { Vec::new() },
        calibrate,
        trailer,
    });
    // Kept while fresh, or while the truck stands still in reverse: the game casts no rays of its own then (seen
    // 2026-11-13: none a second, stopped), and a rear stopped near a wall must go on beeping.
    let moved = (0..3).map(|i| (state.readings.truck[i] - [t.x, t.y, t.z][i]).abs()).fold(0.0, f64::max);
    let still = reversing && state.readings.frame > 0 && moved < STILL;
    if state.readings.frame + READINGS_FOR >= frame || still {
        t.ray_sensors = state.readings.sensors.map(|s| s.unwrap_or(-1.0));
    }
    // A line now and then: the game's own rays (whether there's a moment to cast in at all), and the readings.
    let now = Instant::now();
    let every = if reversing { 1.0 } else { 30.0 };
    if state.logged_at.is_none_or(|at| now.duration_since(at).as_secs_f64() >= every) {
        let since = state.logged_at.map_or(0.0, |at| now.duration_since(at).as_secs_f64());
        state.logged_at = Some(now);
        let game_rays = GAME_RAYS.swap(0, Ordering::Relaxed);
        if since > 0.0 {
            log!(
                "ray: the game cast {game_rays} rays in {since:.0} s (thread {}); {}",
                GAME_THREAD.load(Ordering::Relaxed),
                if reversing { format!("rear sensors {:?}", t.ray_sensors) } else { format!("status {status}") }
            );
        }
    }
}

/// The plugin unloading (the game closing normally): rays being tested didn't close it, so the guard says ok.
pub fn shutdown() {
    let Ok(state) = STATE.lock() else { return };
    if state.guard == Guard::Testing {
        write_guard("ok: the game closed normally while WestPlay's physics rays were tested");
        log!("ray: the game closes normally: the guard says ok");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guard_tries_again_after_a_close_and_is_off_after_three() {
        assert_eq!(guard_from("", "2.3.4").0, Guard::Clear);
        assert_eq!(guard_from("ok: worked
", "2.3.4").0, Guard::Clear);
        let (guard, strikes, write) = guard_from("testing 0
", "2.3.4");
        assert_eq!((guard, strikes), (Guard::Clear, 1));
        assert!(write.is_some_and(|w| w.starts_with("retry 1")));
        assert_eq!(guard_from("testing
", "2.3.4").1, 1);
        assert_eq!(guard_from("retry 1: closed
", "2.3.4").0, Guard::Clear);
        assert_eq!(guard_from("retry 1: closed
", "2.3.4").1, 1);
        assert_eq!(guard_from("testing 1
", "2.3.4").1, 2);
        let (guard, strikes, write) = guard_from("testing 2
", "2.3.4");
        assert_eq!((guard, strikes), (Guard::Off, 3));
        let write = write.unwrap();
        assert!(write.starts_with("off 3 v2.3.4"));
        assert_eq!(guard_from(&write, "2.3.4").0, Guard::Off);
        assert_eq!(guard_from(&write, "2.3.5").0, Guard::Clear);
        // The file of before the closes were counted (off after one close): tried again.
        let old = "off: the game closed while WestPlay's physics rays were being tested; delete this file to try again
";
        assert_eq!(guard_from(old, "2.3.4").0, Guard::Clear);
    }

    #[test]
    fn the_callback_is_laid_out_as_physx_and_the_casing_have_it() {
        assert_eq!(size_of::<Hit>(), 64);
        assert_eq!(std::mem::offset_of!(Hit, flags), 24);
        assert_eq!(std::mem::offset_of!(Hit, position), 28);
        assert_eq!(std::mem::offset_of!(Hit, normal), 40);
        assert_eq!(std::mem::offset_of!(Hit, distance), 52);
        assert_eq!(std::mem::offset_of!(Hit, uv), 56);
        assert_eq!(std::mem::offset_of!(Callback, block), 8);
        assert_eq!(std::mem::offset_of!(Callback, has_block), 72);
        assert_eq!(std::mem::offset_of!(Callback, touches), 80);
        assert_eq!(std::mem::offset_of!(Callback, touches_found), 92);
        // The casing writes up to +0x74.
        assert!(size_of::<Callback>() >= 0x78);
        assert_eq!(size_of::<Place>(), 16);
        assert_eq!(pattern(CASING).len(), 37);
        assert_eq!(pattern(CASING_CALL).len(), 56);
    }

    #[test]
    fn the_casings_cast_reads_as_in_ets2() {
        // ETS2 1.61's bytes at the casing + 0x171 (dumpbin): the scene at +0x178, its raycast at slot 0x2f0.
        let call = pattern(
            "49 8B 8A 78 01 00 00 4C 8D 45 07 4C 89 44 24 30 B8 07 04 00 00 4C 8D 45 DF 66 89 45 DF 4C 89 44 24 28 \
             4C 8D 45 E7 48 8B 01 48 89 54 24 20 48 8D 55 F7 FF 90 F0 02 00 00",
        )
        .into_iter()
        .map(Option::unwrap)
        .collect::<Vec<u8>>();
        assert_eq!(read_call(&call), Some((0x178, 0x2f0)));
        let mut other = call.clone();
        other[8] = 0x4d;
        assert_eq!(read_call(&other), None);
        assert_eq!(read_call(&call[..50]), None);
    }

    #[test]
    fn the_trailers_rear_is_the_middle_of_steady_measures_only() {
        let mut calibration = Calibration { trailer: [1; 4], samples: Vec::new(), tries: 0, behind: None };
        for at in 0..CALIBRATE_SAMPLES {
            calibration.add(2.0 + if at % 2 == 0 { 0.05 } else { -0.05 });
        }
        assert!(calibration.behind.is_some_and(|b| (b - 2.0).abs() < 0.06));
        // Scattered (another trailer behind now and then): measured again, then the guess kept.
        let mut calibration = Calibration { trailer: [1; 4], samples: Vec::new(), tries: 0, behind: None };
        for at in 0..CALIBRATE_SAMPLES * CALIBRATE_TRIES as usize + 5 {
            calibration.add(if at % 2 == 0 { 0.0 } else { 3.0 });
        }
        assert_eq!(calibration.behind, None);
        assert_eq!(calibration.tries, CALIBRATE_TRIES);
    }

    #[test]
    fn the_trailer_is_measured_from_behind_its_guessed_rear_forward() {
        let t = Telemetry {
            trailer_connected: true,
            trailer_half_width: 1.3,
            trailer_rear_z: 5.0,
            trailer_z: 10.0,
            ..Default::default()
        };
        let rays = calibration_rays(&t);
        assert_eq!(rays.len(), 15);
        for ray in &rays {
            // Facing north: back is +z; from 8 m behind the guessed rear (10 + 5 + 8), forward (-z).
            assert!((ray.from[2] - 23.0).abs() < 1e-9 && (ray.along[2] + 1.0).abs() < 1e-9, "{ray:?}");
        }
        assert_eq!(trailer_key(&Telemetry { trailer_connected: false, ..t }), [0; 4]);
        assert_ne!(trailer_key(&t), [0; 4]);
    }

    #[test]
    fn the_sensors_never_ask_for_the_players_own_truck_or_people() {
        for layer in [1, 2, 4, 8, 10] {
            assert_ne!(SOLID & 1 << layer, 0);
        }
        for layer in [3, 6, 7, 9, 16, 18] {
            assert_eq!(SOLID & 1 << layer, 0);
        }
    }

    #[test]
    fn a_place_is_its_sector_and_whats_left() {
        let place = Place::at([1300.0, 12.5, -700.0]);
        assert_eq!(place.sector, [3, -1]);
        assert!((place.local[0] - (1300.0 - 1536.0) as f32).abs() < 1e-3);
        assert!((place.local[2] - (-700.0 + 512.0) as f32).abs() < 1e-3);
        assert_eq!(place.local[1], 12.5);
    }

    #[test]
    fn the_rays_go_back_from_the_rear_and_fan_out_at_the_corners() {
        // A truck facing north (back is +z), its rear 5 m back.
        let t = Telemetry { half_width: 1.25, rear_z: 5.0, ..Default::default() };
        let rays = sensor_rays(&t);
        assert_eq!(rays.len(), 12);
        for ray in &rays {
            assert!((ray.from[2] - 5.05).abs() < 1e-9);
            assert!(ray.along[2] > 0.6, "{ray:?}");
            assert!((ray.along[0].hypot(ray.along[2]) - 1.0).abs() < 1e-9);
        }
        // The left corner's middle ray looks out to the left (-x), the right's to the right.
        assert!(rays[1].along[0] < -0.2 && rays[1].from[0] < -1.0);
        assert!(rays[10].along[0] > 0.2 && rays[10].from[0] > 1.0);
        // With a trailer, from its rear.
        let t =
            Telemetry { trailer_connected: true, trailer_half_width: 1.3, trailer_rear_z: 7.0, trailer_z: 10.0, ..t };
        assert!((sensor_rays(&t)[4].from[2] - 17.05).abs() < 1e-9);
    }
}
