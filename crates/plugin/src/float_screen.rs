//! MaxDevMod's own floating screen in the cab (owner 2026-11-20: "a screen of our own that works in any truck"; quick
//! jobs don't let the tablet accessory be added). No accessory and no game mod: the plugin draws the app's screen
//! frame itself, as a flat screen standing in the cab (docs/tela-flutuante/PLANO.md).
//!
//! Where the cab is, seen from the game's camera: every draw of the main view gets, in the vertex shader's b0 (a
//! ring buffer, `VSSetConstantBuffers1`), a block with the object's modelview (rows 0-3, camera-relative: column 3
//! the object's place seen from the camera) and that times the projection (rows 4-7): the same blocks the mirrors'
//! draws have (`rear_camera`). The game draws the cab's interior in the truck's axes with its origin at the driver's
//! head (the probe of 2026-11-20; at first the truck's origin was assumed, and nothing was found), and in the cab view
//! the camera is at the head: so a plain block whose origin is at the camera (within [`HEAD_NEAR`]) is one of the
//! cab's. Its modelview is the interior's space seen from the camera, and the
//! projection comes out of the block (P = PMV · MV⁻¹). The cab's many parts share it: the most counted in a frame
//! wins ([`on_vs_constant_buffers1`]). None found (the camera outside the cab, a menu): nothing is drawn.
//!
//! At each present the screen is drawn over the game's picture ([`on_present`]): a quad of [`WIDTH`] in the truck's
//! space, at its place ([`Pose`]), times the cab's modelview and the projection, with the app's screen frame on it
//! and a dark bezel round it, in a device context state of its own (as the phone, `handset`). It's drawn after the
//! game's light and tone (so its colours are the app's own, as a lit screen is), without the game's depth: what's
//! in front of it in the cab (the steering wheel) doesn't hide it yet.
//!
//! Its place: where it goes by itself, in front of the driver over the dashboard's middle and turned to face the
//! head ([`default_place`]), moved by the player from the phone's Tela app (`HostBlock::float_screen`), kept for
//! each truck in `Documents\WestPlay\<game>\tela-flutuante.txt` with whether it's on.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

use westplay_shared::{SCREEN_HEIGHT, SCREEN_WIDTH};
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D_SRV_DIMENSION_TEXTURE2D, ID3DBlob};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
    D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_CREATE_DEVICE_SINGLETHREADED, D3D11_CULL_NONE,
    D3D11_DEVICE_CONTEXT_IMMEDIATE, D3D11_FILL_SOLID, D3D11_FILTER_ANISOTROPIC, D3D11_MAP_WRITE_NO_OVERWRITE,
    D3D11_MAPPED_SUBRESOURCE, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC, D3D11_RENDER_TARGET_VIEW_DESC,
    D3D11_RENDER_TARGET_VIEW_DESC_0, D3D11_RTV_DIMENSION_TEXTURE2D, D3D11_TEX2D_RTV, D3D11_SAMPLER_DESC,
    D3D11_SDK_VERSION, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_SRV,
    D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT, ID3D11BlendState,
    ID3D11Buffer, ID3D11Device, ID3D11Device1, ID3D11DeviceContext, ID3D11DeviceContext1, ID3D11PixelShader,
    ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D,
    ID3D11VertexShader, ID3DDeviceContextState,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::{Interface, PCSTR, s};

use crate::frame::{FrameKind, FrameSource};
use crate::{bridge, log};

/// The screen's width in the cab (metres; its height the frame's shape): about a 13-inch screen.
const WIDTH: f32 = 0.30;
/// A version of the picture that's the one composed with the reversing camera (`mirror::composed`'s count under it).
const COMPOSED_VERSION: u64 = 1 << 63;
/// The bezel round it (metres), the chin under it with the logo, and the corners' roundness, outside and of the
/// picture (owner 2026-11-20: "round the frame's corners a little, and my logo under the screen, as its brand").
const BEZEL: f32 = 0.010;
const CHIN: f32 = 0.026;
const CORNER: f32 = 0.014;
const PICTURE_CORNER: f32 = 0.005;
/// The logo's height in the chin, of the chin's.
const LOGO_OF_CHIN: f32 = 0.78;
/// MaxDevMod's logo (assets/logo-reference.png at half size): u32 width and height, then premultiplied BGRA.
const LOGO: &[u8] = include_bytes!("../assets/float_logo.raw");
/// Where it goes by itself, from the driver's head (the truck's space; metres): this far ahead, this far down, and
/// this part of the way across from the head to the cab's middle.
const AHEAD: f32 = 0.70;
const DOWN: f32 = 0.28;
const TO_MIDDLE: f32 = 0.6;
/// A block is the cab's when its modelview puts its origin this close to the camera (metres): the game draws the
/// cab's interior in the cab's own axes with its origin at the driver's head (the probe, owner 2026-11-20: the
/// interior's blocks 1 to 2 cm from the camera, the steering wheel's 66 cm ahead of it, turned as it is).
const HEAD_NEAR: f32 = 0.15;
/// The cab's blocks have this many constants at most (the sky's, also at the camera, has 48), and a frame's cab is
/// taken only with this many of its blocks counted (the cab has dozens).
const CAB_CONSTANTS: u32 = 24;
const MIN_VOTES: u32 = 6;
/// Farther from the head than [`HEAD_NEAR`] (up to [`FOLLOW_FARTHEST`]: the seat moved), a block is taken as the
/// cab's only with this many of its blocks agreeing.
const FAR_VOTES: u32 = 12;
/// A block also counts as the cab's when it's where the cab was a frame ago (owner 2026-11-20, third test: looking at
/// the passenger's seat only one of the cab's blocks was drawn, under [`MIN_VOTES`]; with the seat moved back, the
/// camera went past [`HEAD_NEAR`] from the head's place): its turn within [`FOLLOW_TURN`] (each number of it) and its
/// place within [`FOLLOW_MOVE`] metres of the last frame's cab, however few and however far from the camera (up to
/// [`FOLLOW_FARTHEST`]). Not seen for [`FOLLOW_FOR`] presents, it's looked for anew.
const FOLLOW_TURN: f32 = 0.08;
const FOLLOW_MOVE: f32 = 0.08;
const FOLLOW_FARTHEST: f32 = 0.8;
const FOLLOW_FOR: u64 = 20;
/// With the seat moved the head's away from its default place (the SDK's head offset), and the cab's origin with it:
/// a block whose origin is where the offset puts the cab's, within this (metres), is the cab's too.
const OFFSET_NEAR: f32 = 0.10;
/// Not found at all, the last cab is kept this many presents (a blink hidden).
const HOLD_FOR: u64 = 4;
/// Blocks read in a frame at most, and how many votes for one modelview end the frame's reading early.
/// The blocks are read in a window of the frame's main-view blocks (by their order in it): the cab's come late, past
/// the world's (owner 2026-11-20, first test: the first 600 only, the cab found in a few frames out of 600). Not found,
/// the window moves on each frame, over the whole frame in a few; found, it's kept round where the cab's first came.
const READS_PER_FRAME: u32 = 1500;
/// The window's size: round the cab when it was found last frame, and while it's looked for (owner 2026-11-20: back
/// from the outside camera, with three times the blocks, the screen took long to come back: the window went on from
/// where the cab had been, the cab behind it).
const READS_FOUND: u32 = 900;
const READS_LOOKING: u32 = 1500;
static BUDGET: AtomicU32 = AtomicU32::new(READS_LOOKING);
static WAS_FOUND: AtomicBool = AtomicBool::new(false);
/// How far before the cab's first block the window starts, found.
const BEFORE_CAB: u32 = 300;
/// One press: this far (metres) or turned this much (degrees); and the most either way.
const MOVE_STEP: f32 = 0.02;
const MOVE_MOST: f32 = 1.5;
const TURN_STEP: f32 = 2.0;
const TURN_MOST: f32 = 90.0;
/// The cab counts as seen lately within this many presents (the telemetry's `float_seen`).
const SEEN_FOR: u64 = 30;

type M4 = [[f32; 4]; 4];

/// Whether the player switched it on (kept in the file; read at the first telemetry).
static ON: AtomicBool = AtomicBool::new(false);
/// The main view's targets are bound now (a big, wide target with a depth buffer).
static MAIN_VIEW: AtomicBool = AtomicBool::new(false);
/// Only the pass drawing the scene's surfaces is read (owner 2026-11-20: back from the outside camera the screen took
/// long to come back; read in windows of all the passes, the cab's main draws were missed): the one with several
/// targets at once (the game's G-buffer: ETS2 1.62's has four), every block of it, each frame, while such a pass
/// was drawn last frame; a frame after one without it, every main-view pass is read as before (in windows). (At first
/// it gave up for good after 10 s without it: in the game's menu and loading, before any truck, it always did.)
static GBUFFER_SEEN_AT: AtomicU64 = AtomicU64::new(0);
const GBUFFER_TARGETS: u32 = 3;

/// Whether the G-buffer was drawn this frame or the last.
fn gbuffer_only() -> bool {
    let seen = GBUFFER_SEEN_AT.load(Ordering::Relaxed);
    seen != 0 && PRESENTS.load(Ordering::Relaxed).saturating_sub(seen) <= 1
}
/// Where the head is now, looking round included (f32 bits; the truck's space), for the blocks' test: the camera is
/// there.
static HEAD: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];
/// Presents counted, and the one the cab was last found in.
static PRESENTS: AtomicU64 = AtomicU64::new(0);
static FOUND_AT: AtomicU64 = AtomicU64::new(0);
/// Blocks read this frame.
static READS: AtomicU32 = AtomicU32::new(0);
/// The head's offset from its default place now (f32 bits; the cab's space).
static HEAD_OFFSET: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];
/// How the cab was had, since the last log line: by its blocks, (unused), held.
static FOUND_BY: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
/// The last cab drawn: its modelview, the projection, and the present.
static LAST_DRAWN: Mutex<Option<(M4, M4, u64)>> = Mutex::new(None);
/// This frame's blocks within 3 m of the camera, for [`diagnose`].
static NEAR_BLOCKS: Mutex<Vec<[f32; 32]>> = Mutex::new(Vec::new());
/// The key of the block the cab was taken from last frame.
static LAST_KEY: Mutex<Option<[i32; 12]>> = Mutex::new(None);
/// The cab's modelview last found (rows 0 to 2), and the present it was found at.
static LAST_CAB: Mutex<Option<([[f32; 4]; 3], u64)>> = Mutex::new(None);
/// This frame's main-view blocks counted (each block's order), the window's start, the first cab block's order this
/// frame (u32::MAX: none), and how many blocks the last frame had.
static ORDER: AtomicU32 = AtomicU32::new(0);
static WINDOW: AtomicU32 = AtomicU32::new(0);
static FIRST_CAB: AtomicU32 = AtomicU32::new(u32::MAX);
static LAST_TOTAL: AtomicU32 = AtomicU32::new(0);
/// This frame's votes: a modelview (rounded) and its block, counted.
static VOTES: LazyLock<Mutex<std::collections::HashMap<[i32; 12], (u32, [f32; 32], bool)>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
/// Diagnostics, written every 10 s while on: main-view blocks seen, read, the cab's, the nearest head miss (cm).
static STATS: Mutex<Stats> = Mutex::new(Stats { blocks: 0, read: 0, cab: 0, nearest: f32::MAX, drawn: 0, frames: 0 });

struct Stats {
    blocks: u64,
    read: u64,
    cab: u64,
    nearest: f32,
    drawn: u64,
    frames: u64,
}

/// Whether the floating screen is on and showing (the cab seen lately): the mouse goes to it with the interaction key
/// as it does to the truck's own screen (`bridge::usable`).
pub fn showing() -> bool {
    ON.load(Ordering::Relaxed)
        && powered()
        && PRESENTS.load(Ordering::Relaxed).saturating_sub(FOUND_AT.load(Ordering::Relaxed)) <= SEEN_FOR
        && FOUND_AT.load(Ordering::Relaxed) != 0
}

/// The picture held when it's black (the truck's electrics off), as a version no frame has.
const BLACK_VERSION: u64 = u64::MAX - 1;

/// Whether the truck's electrics are on (not known yet: on, as the truck's own screen takes it).
fn powered() -> bool {
    bridge::get().is_none_or(|b| b.plugin.telemetry.read().is_none_or(|t| t.electric_enabled))
}

// ---------------------------------------------------------------------------------------------------------------
// Its place, kept per truck.

/// The player's move from where it goes by itself: right, up, forward (metres), turned right, tilted up, rolled right
/// (degrees), and its size's change (percent: -20 is 80% of [`WIDTH`]).
type Pose = [f32; 7];
/// The size's change a press, and the least and most.
const SIZE_STEP: f32 = 5.0;
const SIZE_LEAST: f32 = -60.0;
const SIZE_MOST: f32 = 100.0;

#[derive(Default)]
struct Kept {
    /// The host's count of presses last seen (`None`: none yet, the ones before the plugin came not taken).
    seen: Option<u32>,
    loaded: bool,
    by_truck: std::collections::BTreeMap<String, Pose>,
    /// The truck last told (its key).
    truck: Option<String>,
}

static KEPT: Mutex<Kept> =
    Mutex::new(Kept { seen: None, loaded: false, by_truck: std::collections::BTreeMap::new(), truck: None });
/// The pose in use now (f32 bits), for the drawing.
static POSE_NOW: [AtomicU32; 7] = [const { AtomicU32::new(0) }; 7];

fn kept_file() -> std::path::PathBuf {
    crate::log::game_dir().join("tela-flutuante.txt")
}

/// `ligada 0|1`, then one line per truck: `key right up forward turn tilt roll size` (before the size, six numbers).
fn read_kept(text: &str) -> (bool, std::collections::BTreeMap<String, Pose>) {
    let mut on = false;
    let mut by_truck = std::collections::BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("ligada ") {
            on = value.trim() == "1";
            continue;
        }
        let mut words: Vec<&str> = line.split(' ').collect();
        let mut numbers = Vec::new();
        while numbers.len() < 7 && words.len() > 1 {
            let Some(number) = words.last().and_then(|w| w.parse::<f32>().ok()).filter(|n| n.is_finite()) else {
                break;
            };
            numbers.insert(0, number);
            words.pop();
        }
        let key = words.join(" ");
        if numbers.len() >= 6 && !key.is_empty() {
            by_truck.insert(key, std::array::from_fn(|i| numbers.get(i).copied().unwrap_or(0.0)));
        }
    }
    (on, by_truck)
}

fn write_kept(on: bool, by_truck: &std::collections::BTreeMap<String, Pose>) -> String {
    let mut text = format!("ligada {}\n", u8::from(on));
    for (key, p) in by_truck {
        text.push_str(&format!("{key} {} {} {} {} {} {} {}\n", p[0], p[1], p[2], p[3], p[4], p[5], p[6]));
    }
    text
}

/// A press on the Tela app: the pose after it (`None`: not a move).
fn moved(now: Pose, press: u32) -> Option<Pose> {
    use westplay_shared::{
        FLOAT_BACK, FLOAT_BIGGER, FLOAT_DOWN, FLOAT_FORWARD, FLOAT_LEFT, FLOAT_RESET, FLOAT_RIGHT, FLOAT_ROLL_LEFT,
        FLOAT_ROLL_RIGHT, FLOAT_SMALLER, FLOAT_TILT_DOWN, FLOAT_TILT_UP, FLOAT_TURN_LEFT, FLOAT_TURN_RIGHT, FLOAT_UP,
    };
    let percent = |v: f32| v.clamp(SIZE_LEAST, SIZE_MOST).round();
    let metres = |v: f32| (v.clamp(-MOVE_MOST, MOVE_MOST) * 1000.0).round() / 1000.0;
    let degrees = |v: f32| (v.clamp(-TURN_MOST, TURN_MOST) * 10.0).round() / 10.0;
    let mut p = now;
    match press {
        FLOAT_RIGHT => p[0] = metres(p[0] + MOVE_STEP),
        FLOAT_LEFT => p[0] = metres(p[0] - MOVE_STEP),
        FLOAT_UP => p[1] = metres(p[1] + MOVE_STEP),
        FLOAT_DOWN => p[1] = metres(p[1] - MOVE_STEP),
        FLOAT_FORWARD => p[2] = metres(p[2] + MOVE_STEP),
        FLOAT_BACK => p[2] = metres(p[2] - MOVE_STEP),
        FLOAT_TURN_RIGHT => p[3] = degrees(p[3] + TURN_STEP),
        FLOAT_TURN_LEFT => p[3] = degrees(p[3] - TURN_STEP),
        FLOAT_TILT_UP => p[4] = degrees(p[4] + TURN_STEP),
        FLOAT_TILT_DOWN => p[4] = degrees(p[4] - TURN_STEP),
        FLOAT_ROLL_RIGHT => p[5] = degrees(p[5] + TURN_STEP),
        FLOAT_ROLL_LEFT => p[5] = degrees(p[5] - TURN_STEP),
        FLOAT_BIGGER => p[6] = percent(p[6] + SIZE_STEP),
        FLOAT_SMALLER => p[6] = percent(p[6] - SIZE_STEP),
        FLOAT_RESET => p = [0.0; 7],
        _ => return None,
    }
    Some(p)
}

/// Once a frame, as the telemetry's published: the Tela app's presses taken, and whether it's on, drawn lately, and
/// its move in this truck (for the app's page).
pub fn published(_t: &westplay_shared::Telemetry) -> (bool, bool, [f32; 6]) {
    let Some(bridge) = bridge::get() else { return (false, false, [0.0; 6]) };
    let truck = bridge.plugin.truck.read().unwrap_or_default();
    let key = format!("truck:{}", truck.id.as_str());
    let pressed = bridge.host.float_screen.load(Ordering::Acquire);
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    if !kept.loaded {
        kept.loaded = true;
        if let Ok(text) = std::fs::read_to_string(kept_file()) {
            let (on, by_truck) = read_kept(&text);
            ON.store(on, Ordering::Relaxed);
            kept.by_truck = by_truck;
            log!("tela flutuante: {} ({} trucks kept)", if on { "on" } else { "off" }, kept.by_truck.len());
        }
    }
    // Another truck: off (owner 2026-11-20: "each time the truck changes it stays off; whoever wants it switches it
    // on": its place differs in each truck). Not the first one told (the game starting).
    if !truck.id.as_str().is_empty() {
        let before = kept.truck.replace(key.clone());
        if before.is_some_and(|before| before != key) && ON.swap(false, Ordering::Relaxed) {
            log!("tela flutuante: switched off (another truck: {key})");
            if let Err(e) = std::fs::write(kept_file(), write_kept(false, &kept.by_truck)) {
                log!("tela flutuante: not kept ({e})");
            }
        }
    }
    let mut pose = kept.by_truck.get(&key).copied().unwrap_or([0.0; 7]);
    if let Some(seen) = kept.seen.replace(pressed)
        && seen != pressed
        && pressed != 0
    {
        let press = pressed & 0xff;
        if press == westplay_shared::FLOAT_TOGGLE {
            let on = !ON.load(Ordering::Relaxed);
            ON.store(on, Ordering::Relaxed);
            log!("tela flutuante: switched {}", if on { "on" } else { "off" });
        } else if let Some(p) = moved(pose, press) {
            pose = p;
            if p == [0.0; 7] {
                kept.by_truck.remove(&key);
            } else {
                kept.by_truck.insert(key.clone(), p);
            }
            log!("tela flutuante: moved for {key}: {p:?} (right, up, forward, turn, tilt, roll, size %)");
        }
        if let Err(e) = std::fs::write(kept_file(), write_kept(ON.load(Ordering::Relaxed), &kept.by_truck)) {
            log!("tela flutuante: not kept ({e})");
        }
    }
    for (slot, value) in POSE_NOW.iter().zip(pose) {
        slot.store(value.to_bits(), Ordering::Relaxed);
    }
    (ON.load(Ordering::Relaxed), showing(), std::array::from_fn(|i| pose[i]))
}

fn pose_now() -> Pose {
    POSE_NOW.each_ref().map(|slot| f32::from_bits(slot.load(Ordering::Relaxed)))
}

/// Where the screen goes by itself, from the head's default place (the truck's space: x right, y up, z back): its
/// middle, and its turn (degrees about y, its face's tilt up) to face the head.
fn default_place(head: [f32; 3]) -> ([f32; 3], f32, f32) {
    let centre = [head[0] * (1.0 - TO_MIDDLE), head[1] - DOWN, head[2] - AHEAD];
    let to_head = [head[0] - centre[0], head[1] - centre[1], head[2] - centre[2]];
    let turn = to_head[0].atan2(to_head[2]).to_degrees();
    let tilt = to_head[1].atan2((to_head[0] * to_head[0] + to_head[2] * to_head[2]).sqrt()).to_degrees();
    (centre, turn, tilt)
}

/// The screen's own space (x its right, y its up, its face looking along +z) in the cab interior's (the truck's axes,
/// its origin at the head's default place).
fn placement(head: [f32; 3], pose: Pose) -> M4 {
    let (centre, turn, tilt) = default_place(head);
    let at = [centre[0] + pose[0], centre[1] + pose[1], centre[2] - pose[2]];
    let (yaw, pitch, roll) = ((turn + pose[3]).to_radians(), -(tilt + pose[4]).to_radians(), (-pose[5]).to_radians());
    let ry = [[yaw.cos(), 0.0, yaw.sin()], [0.0, 1.0, 0.0], [-yaw.sin(), 0.0, yaw.cos()]];
    let rx = [[1.0, 0.0, 0.0], [0.0, pitch.cos(), -pitch.sin()], [0.0, pitch.sin(), pitch.cos()]];
    let rz = [[roll.cos(), -roll.sin(), 0.0], [roll.sin(), roll.cos(), 0.0], [0.0, 0.0, 1.0]];
    let r = mul3(&mul3(&ry, &rx), &rz);
    // Its size: the whole frame scaled (the picture, its bezel and its logo).
    let size = 1.0 + pose[6].clamp(SIZE_LEAST, SIZE_MOST) / 100.0;
    let r = r.map(|row| row.map(|v| v * size));
    // The interior's space has its origin at the head's default place.
    let at = [at[0] - head[0], at[1] - head[1], at[2] - head[2]];
    [
        [r[0][0], r[0][1], r[0][2], at[0]],
        [r[1][0], r[1][1], r[1][2], at[1]],
        [r[2][0], r[2][1], r[2][2], at[2]],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

// ---------------------------------------------------------------------------------------------------------------
// The cab, from the game's draws.

/// `OMSetRenderTargets`: whether the main view's targets are bound (a big HDR target and a depth buffer).
///
/// # Safety
/// The game's live context and arguments: `count` views, each live or null.
pub unsafe fn on_set_render_targets(context: *mut c_void, count: u32, views: *const *mut c_void, depth: *mut c_void) {
    MAIN_VIEW.store(false, Ordering::Relaxed);
    if PROBING.load(Ordering::Relaxed) {
        // SAFETY: as the caller says.
        let said = unsafe { describe_targets(count, views, depth) };
        let mut probe = PROBE.lock().unwrap_or_else(|e| e.into_inner());
        probe.passes += 1;
        probe.target = format!("pass {} {said}", probe.passes);
    }
    if !ON.load(Ordering::Relaxed) || views.is_null() || count == 0 || count > 8 || depth.is_null() {
        return;
    }
    // SAFETY: as the caller says.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return;
        }
        let Some(view) = ID3D11RenderTargetView::from_raw_borrowed(&*views) else { return };
        let Ok(texture) = view.GetResource().and_then(|r| r.cast::<ID3D11Texture2D>()) else { return };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc);
        // Any format (the cab's pass may not be the HDR one): the head's test tells the cab's blocks anyway. Not the
        // mirrors' (tall), not a shadow's (no target).
        let main = desc.Width >= 640 && desc.Width > desc.Height && desc.ArraySize == 1;
        let gbuffer = main && count >= GBUFFER_TARGETS;
        if gbuffer {
            GBUFFER_SEEN_AT.store(PRESENTS.load(Ordering::Relaxed).max(1), Ordering::Relaxed);
        }
        MAIN_VIEW.store(if gbuffer_only() { gbuffer } else { main }, Ordering::Relaxed);
    }
}

/// `VSSetConstantBuffers` (bound whole): written by the probe only.
///
/// # Safety
/// The game's live context and arguments: `count` buffers.
pub unsafe fn on_vs_constant_buffers(context: *mut c_void, start: u32, count: u32, buffers: *const *mut c_void) {
    if PROBING.load(Ordering::Relaxed) {
        // SAFETY: as the caller says.
        unsafe { probe_block(context, start, count, buffers, std::ptr::null(), std::ptr::null()) };
    }
}

/// `VSSetConstantBuffers1`: a main-view draw's block read, and counted if it's the cab's.
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
    if PROBING.load(Ordering::Relaxed) {
        // SAFETY: as the caller says.
        unsafe { probe_block(context, start, count, buffers, firsts, counts) };
    }
    if !ON.load(Ordering::Relaxed) || !MAIN_VIEW.load(Ordering::Relaxed) {
        return;
    }
    if start != 0 || count == 0 || buffers.is_null() || firsts.is_null() {
        return;
    }
    // SAFETY: the game's first buffer and its range (the caller's contract).
    let (raw, first, constants) = unsafe { (*buffers, *firsts, if counts.is_null() { 0 } else { *counts }) };
    if raw.is_null() || !(8..=CAB_CONSTANTS).contains(&constants) {
        return;
    }
    STATS.lock().unwrap_or_else(|e| e.into_inner()).blocks += 1;
    let order = ORDER.fetch_add(1, Ordering::Relaxed);
    let (window, budget) = if gbuffer_only() { (0, READS_PER_FRAME) } else { (WINDOW.load(Ordering::Relaxed), BUDGET.load(Ordering::Relaxed)) };
    if order < window || READS.fetch_add(1, Ordering::Relaxed) >= budget {
        return;
    }
    // SAFETY: the game's live context and buffer; the block is within the bound range (8 constants of it).
    let numbers = unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        let Some(buffer) = ID3D11Buffer::from_raw_borrowed(&raw) else { return };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        if context.Map(buffer, 0, D3D11_MAP_WRITE_NO_OVERWRITE, 0, Some(&mut mapped)).is_err() {
            return;
        }
        let mut numbers = [0f32; 32];
        std::ptr::copy_nonoverlapping((mapped.pData as *const f32).add(first as usize * 4), numbers.as_mut_ptr(), 32);
        context.Unmap(buffer, 0);
        numbers
    };
    let miss = cab_miss(&numbers, [0.0; 3]);
    let mut stats = STATS.lock().unwrap_or_else(|e| e.into_inner());
    stats.read += 1;
    let Some(miss) = miss else { return };
    stats.nearest = stats.nearest.min(miss);
    // Only a block seen through a camera's lens: not one of the screen-wide passes, whose blocks are plain too (owner
    // 2026-11-20, fourth test: the screen followed one with no turn at all and went out of sight).
    if !seen_through_lens(&numbers) {
        return;
    }
    if miss < 3.0 {
        let mut near = NEAR_BLOCKS.lock().unwrap_or_else(|e| e.into_inner());
        if near.len() < READS_PER_FRAME as usize {
            near.push(numbers);
        }
    }
    let follows = miss <= FOLLOW_FARTHEST && follows_last(&numbers);
    let offset = HEAD_OFFSET.each_ref().map(|h| f32::from_bits(h.load(Ordering::Relaxed)));
    if miss > FOLLOW_FARTHEST || (miss > HEAD_NEAR && !follows && !at_offset(&numbers, offset) && !seat_moved()) {
        return;
    }
    stats.cab += 1;
    drop(stats);
    FIRST_CAB.fetch_min(order, Ordering::Relaxed);
    let key: [i32; 12] = std::array::from_fn(|i| (numbers[i] * 100.0).round() as i32);
    let mut votes = VOTES.lock().unwrap_or_else(|e| e.into_inner());
    let entry = votes.entry(key).or_insert((0, numbers, false));
    entry.0 += 1;
    entry.2 |= follows;
}

/// Once a second while the cab is found (owner 2026-11-20, eighth test: the screen rose and fell as he aimed, and
/// stayed still as the cab swayed): the cab's block (its place from the camera, and where it looks), the head's
/// offset, and the other blocks within 3 m turned as it is (within 0.01), each as where it is from the cab's block in
/// the cab's own axes. Taken as the cab is, those stay the same as the view turns and the cab sways.
fn diagnose(cab: &M4, near: &[[f32; 32]], presents: u64) {
    let offset = HEAD_OFFSET.each_ref().map(|h| f32::from_bits(h.load(Ordering::Relaxed)));
    let mut parts: Vec<([f32; 3], u32)> = Vec::new();
    let mut turned_otherwise = 0u32;
    for numbers in near {
        let m = rows(numbers, 0);
        let same = (0..3).all(|r| (0..3).all(|c| (m[r][c] - cab[r][c]).abs() < 0.01));
        if !same {
            turned_otherwise += 1;
            continue;
        }
        let d = [m[0][3] - cab[0][3], m[1][3] - cab[1][3], m[2][3] - cab[2][3]];
        // In the cab's axes: R^T d.
        let in_cab: [f32; 3] = std::array::from_fn(|c| (0..3).map(|r| cab[r][c] * d[r]).sum());
        match parts.iter_mut().find(|(p, _)| (0..3).all(|i| (p[i] - in_cab[i]).abs() < 0.005)) {
            Some(part) => part.1 += 1,
            None => parts.push((in_cab, 1)),
        }
    }
    parts.sort_by(|a, b| b.1.cmp(&a.1));
    let listed: Vec<String> =
        parts.iter().take(8).map(|(p, n)| format!("[{:.3} {:.3} {:.3}]x{n}", p[0], p[1], p[2])).collect();
    log!(
        "tela flutuante: diag {presents}: cab block at [{:.3} {:.3} {:.3}] looking [{:.2} {:.2} {:.2}], head offset [{:.3} {:.3} {:.3}]; same-turned parts {}; turned otherwise {turned_otherwise}",
        cab[0][3],
        cab[1][3],
        cab[2][3],
        -cab[2][0],
        -cab[2][1],
        -cab[2][2],
        offset[0],
        offset[1],
        offset[2],
        listed.join(" ")
    );
}

/// Whether the cab was last found away from the head (the seat moved): blocks that far are counted then.
fn seat_moved() -> bool {
    LAST_CAB
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some_and(|(last, _)| (last[0][3].powi(2) + last[1][3].powi(2) + last[2][3].powi(2)).sqrt() > HEAD_NEAR * 0.7)
}

/// Whether a block's origin is where the head's offset puts the cab's: seen from a head moved by `offset` (the cab's
/// space), the cab's origin is at -R·offset.
fn at_offset(numbers: &[f32; 32], offset: [f32; 3]) -> bool {
    let away = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
    if away < 0.05 {
        return false;
    }
    let miss: f32 = (0..3)
        .map(|r| (numbers[r * 4 + 3] + (0..3).map(|c| numbers[r * 4 + c] * offset[c]).sum::<f32>()).powi(2))
        .sum::<f32>()
        .sqrt();
    miss < OFFSET_NEAR
}

/// Whether a block is drawn through a camera's lens: its modelview turned (not exactly none), and the projection
/// under it a perspective one looking down -z.
fn seen_through_lens(numbers: &[f32; 32]) -> bool {
    let modelview = rows(numbers, 0);
    let unturned = (0..3).all(|r| (0..3).all(|c| (modelview[r][c] - if r == c { 1.0 } else { 0.0 }).abs() < 1e-5));
    if unturned {
        return false;
    }
    let Some(inverse) = affine_inverse(&modelview) else { return false };
    let projection = product(&rows(numbers, 4), &inverse);
    (projection[3][2] + 1.0).abs() < 0.05 && projection[0][0] > 0.2 && projection[1][1] > 0.2
}

/// Whether a block is where the cab was found lately (its turn and place close to it).
fn follows_last(numbers: &[f32; 32]) -> bool {
    let Some((last, at)) = *LAST_CAB.lock().unwrap_or_else(|e| e.into_inner()) else { return false };
    if PRESENTS.load(Ordering::Relaxed).saturating_sub(at) > FOLLOW_FOR {
        return false;
    }
    let turned = (0..3).all(|r| (0..3).all(|c| (numbers[r * 4 + c] - last[r][c]).abs() < FOLLOW_TURN));
    let moved: f32 = (0..3).map(|r| (numbers[r * 4 + 3] - last[r][3]).powi(2)).sum::<f32>().sqrt();
    turned && moved < FOLLOW_MOVE
}

/// How far from the camera a block's modelview puts the head's place (metres), if the block is a plain one (a
/// modelview with a turn and no scale, and a projection under it that isn't nothing).
fn cab_miss(numbers: &[f32; 32], head: [f32; 3]) -> Option<f32> {
    let m = rows(numbers, 0);
    if m[3] != [0.0, 0.0, 0.0, 1.0] {
        return None;
    }
    let unit = (0..3).all(|r| ((0..3).map(|c| m[r][c] * m[r][c]).sum::<f32>() - 1.0).abs() < 0.02);
    if !unit {
        return None;
    }
    let p: [f32; 3] = std::array::from_fn(|r| (0..3).map(|c| m[r][c] * head[c]).sum::<f32>() + m[r][3]);
    Some((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
}

/// This frame's cab: its modelview and the projection (from the most counted block), the votes cleared.
fn take_cab() -> Option<(M4, M4)> {
    let mut votes = VOTES.lock().unwrap_or_else(|e| e.into_inner());
    // The one taken last frame while it's still there (owner 2026-11-20: in ATS the screen trembled, two of the cab's
    // blocks a centimetre apart taken in turns); else the most counted: enough of them, or where the cab was last
    // frame (then it's it, however few).
    let last_key = *LAST_KEY.lock().unwrap_or_else(|e| e.into_inner());
    let best = votes
        .iter()
        .filter(|(_, (count, numbers, follows))| {
            let near = cab_miss(numbers, [0.0; 3]).is_some_and(|miss| miss <= HEAD_NEAR);
            // Away from the head (the seat moved) only with many of the cab's blocks agreeing.
            (near && *count >= MIN_VOTES) || *follows || *count >= FAR_VOTES
        })
        .max_by_key(|(key, (count, _, follows))| (Some(**key) == last_key, *count >= MIN_VOTES, *follows, *count))
        .map(|(key, (_, numbers, _))| (*key, *numbers));
    *LAST_KEY.lock().unwrap_or_else(|e| e.into_inner()) = best.map(|(key, _)| key);
    let best = best.map(|(_, numbers)| numbers);
    votes.clear();
    READS.store(0, Ordering::Relaxed);
    // The next frame's window: round the cab's first block, or on past this one's.
    let total = ORDER.swap(0, Ordering::Relaxed);
    if total > 0 {
        LAST_TOTAL.store(total, Ordering::Relaxed);
    }
    let first = FIRST_CAB.swap(u32::MAX, Ordering::Relaxed);
    let window = WINDOW.load(Ordering::Relaxed);
    let budget = BUDGET.load(Ordering::Relaxed);
    let found_now = first != u32::MAX;
    let next = if found_now {
        first.saturating_sub(BEFORE_CAB)
    } else if WAS_FOUND.load(Ordering::Relaxed) || window + budget >= LAST_TOTAL.load(Ordering::Relaxed) {
        // Just lost, or past the frame's end: from the frame's start again.
        0
    } else {
        window + budget - BEFORE_CAB
    };
    WINDOW.store(next, Ordering::Relaxed);
    WAS_FOUND.store(found_now, Ordering::Relaxed);
    BUDGET.store(if found_now { READS_FOUND } else { READS_LOOKING }, Ordering::Relaxed);
    drop(votes);
    let presents = PRESENTS.load(Ordering::Relaxed);
    // On foot (the Modo Personagem), out of the cab: no screen (owner 2026-11-20: it stood in the road in front of
    // the person, the person's camera taken for the head), and nothing learned or kept for when the cab is back.
    if crate::character::walking() {
        *LAST_DRAWN.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *LAST_CAB.lock().unwrap_or_else(|e| e.into_inner()) = None;
        NEAR_BLOCKS.lock().unwrap_or_else(|e| e.into_inner()).clear();
        return None;
    }
    let near = std::mem::take(&mut *NEAR_BLOCKS.lock().unwrap_or_else(|e| e.into_inner()));
    let found = match best {
        Some(numbers) => {
            FOUND_BY[0].fetch_add(1, Ordering::Relaxed);
            let modelview = rows(&numbers, 0);
            if presents % 60 == 0 {
                diagnose(&modelview, &near, presents);
            }
            let projection = product(&rows(&numbers, 4), &affine_inverse(&modelview)?);
            Some((modelview, projection))
        }
        // The cab's parts aren't used to find it (owner 2026-11-20, seventh test: by them the screen went to the bed,
        // rose by itself, and stood outside in the outside cameras): not found, it's held a moment, then not drawn.
        None => None,
    };
    let found = found.filter(|(_, projection)| projection[3][2] < -0.5);
    if let Some((modelview, projection)) = found {
        *LAST_CAB.lock().unwrap_or_else(|e| e.into_inner()) = Some(([modelview[0], modelview[1], modelview[2]], presents));
        *LAST_DRAWN.lock().unwrap_or_else(|e| e.into_inner()) = Some((modelview, projection, presents));
        return Some((modelview, projection));
    }
    // Lost: the last one a moment longer.
    let last = *LAST_DRAWN.lock().unwrap_or_else(|e| e.into_inner());
    let held = last.filter(|(_, _, at)| presents.saturating_sub(*at) <= HOLD_FOR);
    if held.is_some() {
        FOUND_BY[2].fetch_add(1, Ordering::Relaxed);
    }
    held.map(|(modelview, projection, _)| (modelview, projection))
}

fn rows(numbers: &[f32; 32], first: usize) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| numbers[(first + r) * 4 + c]))
}

fn product(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| a[r][k] * b[k][c]).sum()))
}

fn mul3(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

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
        out[row][..3].copy_from_slice(&r[row]);
        out[row][3] = -(r[row][0] * t[0] + r[row][1] * t[1] + r[row][2] * t[2]);
    }
    out[3][3] = 1.0;
    Some(out)
}

// ---------------------------------------------------------------------------------------------------------------
// The probe (owner 2026-11-20, second test: the cab never found in the main view, the nearest block's head 3 m off):
// one frame in 1200 (20 s), at most PROBES a game, every vertex-shader b0 block of the frame is read and written to
// logs/tela-flutuante-N.txt with the pass's targets, to see how the game gives the cab. It stutters that frame.

/// 0 since the cab was found from it (2026-11-20 12:30): no more stutters.
const PROBES: u32 = 0;
const PROBE_EVERY: u64 = 1200;
/// Blocks written in full in a pass at most (the nearest ones are written whatever).
const PROBE_FULL_PER_PASS: u32 = 6;
static PROBING: AtomicBool = AtomicBool::new(false);
static PROBES_TAKEN: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
struct Probe {
    passes: u32,
    target: String,
    text: String,
    head: [f32; 3],
    written_in_pass: (u32, u32),
    blocks: u32,
}

static PROBE: Mutex<Probe> = Mutex::new(Probe {
    passes: 0,
    target: String::new(),
    text: String::new(),
    head: [0.0; 3],
    written_in_pass: (0, 0),
    blocks: 0,
});

/// The targets bound: each one's size and format, and the depth buffer's.
///
/// # Safety
/// `count` views, each live or null; `depth` live or null.
unsafe fn describe_targets(count: u32, views: *const *mut c_void, depth: *mut c_void) -> String {
    let mut said = String::new();
    // SAFETY: as the caller says.
    unsafe {
        if !views.is_null() && count <= 8 {
            for i in 0..count as usize {
                let raw = *views.add(i);
                let Some(view) = ID3D11RenderTargetView::from_raw_borrowed(&raw) else {
                    said.push_str(" -");
                    continue;
                };
                if let Ok(texture) = view.GetResource().and_then(|r| r.cast::<ID3D11Texture2D>()) {
                    let mut desc = D3D11_TEXTURE2D_DESC::default();
                    texture.GetDesc(&mut desc);
                    said.push_str(&format!(" {}x{} f{} a{}", desc.Width, desc.Height, desc.Format.0, desc.ArraySize));
                }
            }
        } else {
            said.push_str(&format!(" (count {count})"));
        }
        if let Some(view) = windows::Win32::Graphics::Direct3D11::ID3D11DepthStencilView::from_raw_borrowed(&depth)
            && let Ok(texture) = view.GetResource().and_then(|r| r.cast::<ID3D11Texture2D>())
        {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc);
            said.push_str(&format!(" depth {}x{} f{}", desc.Width, desc.Height, desc.Format.0));
        } else {
            said.push_str(" no depth");
        }
    }
    said
}

/// # Safety
/// As [`on_vs_constant_buffers1`].
unsafe fn probe_block(
    context: *mut c_void,
    start: u32,
    count: u32,
    buffers: *const *mut c_void,
    firsts: *const u32,
    counts: *const u32,
) {
    if buffers.is_null() || count == 0 {
        return;
    }
    // SAFETY: as the caller says; each block read within its bound range.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return;
        }
        let mut probe = PROBE.lock().unwrap_or_else(|e| e.into_inner());
        if probe.written_in_pass.0 != probe.passes {
            probe.written_in_pass = (probe.passes, 0);
            let target = probe.target.clone();
            probe.text.push_str(&format!("\n== {target}\n"));
        }
        for slot in 0..count.min(4) as usize {
            let raw = *buffers.add(slot);
            let Some(buffer) = ID3D11Buffer::from_raw_borrowed(&raw) else { continue };
            let first = if firsts.is_null() { 0 } else { *firsts.add(slot) };
            // Bound whole (`VSSetConstantBuffers`): its own size.
            let constants = if counts.is_null() {
                let mut desc = windows::Win32::Graphics::Direct3D11::D3D11_BUFFER_DESC::default();
                buffer.GetDesc(&mut desc);
                desc.ByteWidth / 16
            } else {
                *counts.add(slot)
            };
            if constants < 8 {
                if probe.written_in_pass.1 < 2 {
                    probe.text.push_str(&format!("b{} {constants} constants (skipped)\n", start as usize + slot));
                }
                continue;
            }
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            if context.Map(buffer, 0, D3D11_MAP_WRITE_NO_OVERWRITE, 0, Some(&mut mapped)).is_err() {
                if probe.written_in_pass.1 < PROBE_FULL_PER_PASS {
                    probe.written_in_pass.1 += 1;
                    probe.text.push_str(&format!("b{} map failed\n", start as usize + slot));
                }
                continue;
            }
            let wanted = (constants.min(16) * 4) as usize;
            let mut numbers = vec![0f32; wanted];
            std::ptr::copy_nonoverlapping((mapped.pData as *const f32).add(first as usize * 4), numbers.as_mut_ptr(), wanted);
            context.Unmap(buffer, 0);
            probe.blocks += 1;
            let mut block = [0f32; 32];
            block.copy_from_slice(&numbers[..32]);
            let miss = cab_miss(&block, probe.head);
            let t = [numbers[3], numbers[7], numbers[11]];
            let near = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
            let full = probe.written_in_pass.1 < PROBE_FULL_PER_PASS || near < 4.0 || miss.is_some_and(|m| m < 1.0);
            if full && probe.blocks < 4000 {
                probe.written_in_pass.1 += 1;
                let rows: Vec<String> =
                    numbers.chunks(4).map(|r| format!("[{:.4} {:.4} {:.4} {:.4}]", r[0], r[1], r[2], r[3])).collect();
                let miss = miss.map_or("-".to_string(), |m| format!("{:.2}", m));
                probe.text.push_str(&format!(
                    "b{} {constants}c +{first} |t| {near:.2} headmiss {miss}: {}\n",
                    start as usize + slot,
                    rows.join(" ")
                ));
            }
        }
    }
}

/// At each present: the probe's frame armed, and the last one written.
fn probe_present(presents: u64, head: [f32; 3]) {
    if PROBING.swap(false, Ordering::Relaxed) {
        let mut probe = PROBE.lock().unwrap_or_else(|e| e.into_inner());
        let n = PROBES_TAKEN.fetch_add(1, Ordering::Relaxed) + 1;
        let truck = crate::telemetry::truck();
        let header = format!(
            "tela flutuante probe {n}: head (now) {:?}, head default {head:?}, {} passes, {} blocks read\n",
            truck.head, probe.passes, probe.blocks
        );
        let path = crate::log::log_dir().join(format!("tela-flutuante-{n}.txt"));
        match std::fs::write(&path, format!("{header}{}", probe.text)) {
            Ok(()) => log!("tela flutuante: probe {n} written ({} blocks) to {}", probe.blocks, path.display()),
            Err(e) => log!("tela flutuante: probe {n} not written: {e}"),
        }
        *probe = Probe::default();
        return;
    }
    if ON.load(Ordering::Relaxed)
        && PROBES_TAKEN.load(Ordering::Relaxed) < PROBES
        && presents % PROBE_EVERY == PROBE_EVERY / 2
        && !crate::telemetry::paused()
    {
        let mut probe = PROBE.lock().unwrap_or_else(|e| e.into_inner());
        *probe = Probe::default();
        probe.head = crate::telemetry::truck().head;
        PROBING.store(true, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Drawing it.

// The quad covers the frame (`outer`: left, bottom, right, top, metres in the screen's own space); the picture is
// `picture.xy` wide and tall round the middle; corners rounded `picture.z` outside and `picture.w` of the picture;
// the logo in `logo` (left, top, width, height).
const SHADERS: &str = r"
cbuffer Place : register(b0) { row_major float4x4 mvp; float4 outer; float4 picture; float4 logo; };
struct Vertex { float4 position : SV_Position; float2 local : TEXCOORD0; };
Vertex vs(uint id : SV_VertexID) {
    float2 corner = float2(id & 1, id >> 1);
    float2 local = lerp(outer.xy, outer.zw, corner);
    Vertex v;
    v.position = mul(mvp, float4(local, 0, 1));
    v.local = local;
    return v;
}
Texture2D screen : register(t0);
Texture2D brand : register(t1);
SamplerState smooth : register(s0);
float rounded(float2 p, float2 half_size, float r) {
    float2 q = abs(p) - half_size + r;
    return length(max(q, 0)) + min(max(q.x, q.y), 0) - r;
}
float4 ps(Vertex v) : SV_Target {
    float2 p = v.local;
    float2 middle = (outer.xy + outer.zw) * 0.5;
    float d = rounded(p - middle, (outer.zw - outer.xy) * 0.5, picture.z);
    float edge = max(fwidth(d), 1e-5);
    float alpha = saturate(0.5 - d / edge);
    // The bezel: dark, a thin lighter rim at its edge.
    float3 color = lerp(float3(0.035, 0.035, 0.04), float3(0.16, 0.16, 0.17), saturate(1 + d / 0.0015));
    float2 logo_uv = float2((p.x - logo.x) / logo.z, (logo.y - p.y) / logo.w);
    float4 mark = brand.Sample(smooth, saturate(logo_uv));
    float in_logo = all(logo_uv >= 0) && all(logo_uv <= 1) ? 1 : 0;
    color = color * (1 - mark.a * in_logo) + mark.rgb * in_logo;
    float2 uv = float2(0.5 + p.x / picture.x, 0.5 - p.y / picture.y);
    float3 shown = screen.Sample(smooth, saturate(uv)).rgb;
    float inside = rounded(p, picture.xy * 0.5, picture.w);
    float cover = saturate(0.5 - inside / max(fwidth(inside), 1e-5));
    color = lerp(color, shown, cover);
    return float4(color * alpha, alpha);
}
";

struct Drawing {
    device: ID3D11Device,
    frames: FrameSource,
    state: ID3DDeviceContextState,
    vertex: ID3D11VertexShader,
    pixel: ID3D11PixelShader,
    place: ID3D11Buffer,
    blend: ID3D11BlendState,
    rasterizer: ID3D11RasterizerState,
    sampler: ID3D11SamplerState,
    /// The app's picture copied with its smaller sizes (mipmaps; owner 2026-11-20: "at the normal view it's a bit
    /// blurry, zoomed in it's sharp": 1280 x 720 shrunk to some 370 x 210 pixels without them it shimmered): its size,
    /// the format it's viewed in, the texture and its view, and the frame's version it holds.
    view: Option<((u32, u32), DXGI_FORMAT, ID3D11Texture2D, ID3D11ShaderResourceView, u64)>,
    /// The logo's texture, and its view for the format it was made for.
    logo: ID3D11Texture2D,
    logo_view: Option<(DXGI_FORMAT, ID3D11ShaderResourceView)>,
}

// SAFETY: used only on the game's render thread (Present), the lock only keeping it there.
unsafe impl Send for Drawing {}

static DRAWING: Mutex<Option<Result<Drawing, ()>>> = Mutex::new(None);
/// After a failed draw, the present it's tried again at (owner 2026-11-21, ETS2 1.62, old Actros: one "invalid
/// parameter" as the game came back from loading, and the screen never drawn again that session).
static RETRY_AT: AtomicU64 = AtomicU64::new(0);
/// Failed draws in a row, for the log and how long to wait.
static FAILURES: AtomicU64 = AtomicU64::new(0);
/// The failures said in the log (then quiet).
const FAILURES_SAID: u64 = 5;

/// At each of the game's presents: this frame's cab taken, and the screen drawn in it while it's on.
///
/// # Safety
/// The game's live swap chain, in its `Present`, on its render thread.
pub unsafe fn on_present(swapchain: *mut c_void) {
    let presents = PRESENTS.fetch_add(1, Ordering::Relaxed) + 1;
    let head = crate::telemetry::head_default();
    let now = crate::telemetry::truck().head;
    for (slot, value) in HEAD.iter().zip(now) {
        slot.store(value.to_bits(), Ordering::Relaxed);
    }
    for (i, slot) in HEAD_OFFSET.iter().enumerate() {
        slot.store((now[i] - head[i]).to_bits(), Ordering::Relaxed);
    }
    let cab = take_cab();
    probe_present(presents, head);
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    if cab.is_some() {
        FOUND_AT.store(presents, Ordering::Relaxed);
    }
    log_stats(presents, cab.is_some());
    let Some((modelview, projection)) = cab else { return };
    if crate::telemetry::paused() || !bridge::host_alive() {
        return;
    }
    let place = product(&modelview, &placement(head, pose_now()));
    *SEEN_AT.lock().unwrap_or_else(|e| e.into_inner()) = [place[0][3], place[1][3], place[2][3]];
    let mvp = product(&projection, &place);
    // SAFETY: the game's live swap chain, on its render thread (the caller's contract).
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let mut drawing = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(&*drawing, Some(Ok(made)) if made.device != device) {
            *drawing = None;
        }
        // Failed before: made anew once its wait is over (a second up to half a minute, longer the more it fails).
        if matches!(&*drawing, Some(Err(()))) && presents >= RETRY_AT.load(Ordering::Relaxed) {
            *drawing = None;
        }
        if drawing.is_none() {
            *drawing = Some(Drawing::new(&device).map_err(|e| log!("tela flutuante: can't be drawn: {e}")));
        }
        let Some(Ok(made)) = drawing.as_mut() else { return };
        // The truck's electrics off: the frame stays, its picture black (owner 2026-11-21: "not the screen gone,
        // the picture on it: black with the truck off, and on again as our screen comes on, MaxDevMod first").
        match made.draw(swapchain, &mvp, !powered()) {
            Ok(drawn) => {
                if drawn && FAILURES.swap(0, Ordering::Relaxed) > 0 {
                    log!("tela flutuante: drawn again");
                }
                STATS.lock().unwrap_or_else(|e| e.into_inner()).drawn += u64::from(drawn);
                // Drawn in the cab: the app's told the screen's in view, so it sends its frames (owner 2026-11-21, old
                // Actros: its own screen isn't WestPlay's, the app sent none and the floating screen stayed black).
                if drawn && let Some(bridge) = bridge::get() {
                    let plugin = &bridge.plugin;
                    plugin.cab_frame.store(plugin.heartbeat.load(Ordering::Relaxed), Ordering::Release);
                }
            }
            Err(e) => {
                let failures = FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
                if failures <= FAILURES_SAID {
                    log!("tela flutuante: not drawn ({failures}): {e}; tried again in a moment");
                }
                RETRY_AT.store(presents + (60 * failures).min(1800), Ordering::Relaxed);
                *drawing = Some(Err(()));
            }
        }
    }
}

/// Where the screen's middle was last seen from the camera (metres: right, up, back), for the log.
static SEEN_AT: Mutex<[f32; 3]> = Mutex::new([0.0; 3]);

/// Every 10 s while on: what the draws gave (for telling, from the log, why it doesn't show).
fn log_stats(presents: u64, found: bool) {
    let mut stats = STATS.lock().unwrap_or_else(|e| e.into_inner());
    stats.frames += u64::from(found);
    if presents % 600 != 0 {
        return;
    }
    let nearest = if stats.nearest == f32::MAX { "none".to_string() } else { format!("{:.0} cm", stats.nearest * 100.0) };
    log!(
        "tela flutuante: 10 s: main-view blocks {}, read {}, the cab's {}, nearest head {nearest}; cab found in {} frames, drawn {}; window at {} of {}; screen seen at {:.2?}; by its blocks {}, held {}; G-buffer only {}",
        stats.blocks,
        stats.read,
        stats.cab,
        stats.frames,
        stats.drawn,
        WINDOW.load(Ordering::Relaxed),
        LAST_TOTAL.load(Ordering::Relaxed),
        *SEEN_AT.lock().unwrap_or_else(|e| e.into_inner()),
        FOUND_BY[0].swap(0, Ordering::Relaxed),
        FOUND_BY[2].swap(0, Ordering::Relaxed),
        gbuffer_only(),
    );
    *stats = Stats { blocks: 0, read: 0, cab: 0, nearest: f32::MAX, drawn: 0, frames: 0 };
}

impl Drawing {
    /// # Safety
    /// The game's live device, on its render thread.
    unsafe fn new(device: &ID3D11Device) -> windows::core::Result<Self> {
        // SAFETY: the game's live device (the caller's contract); every description is a local, the shaders our own.
        unsafe {
            let device1: ID3D11Device1 = device.cast()?;
            let flags = if device.GetCreationFlags() & D3D11_CREATE_DEVICE_SINGLETHREADED.0 != 0 { 1 } else { 0 };
            let mut state = None;
            device1.CreateDeviceContextState(
                flags,
                &[device.GetFeatureLevel()],
                D3D11_SDK_VERSION,
                &ID3D11Device::IID,
                None,
                Some(&mut state),
            )?;
            let vertex_code = compile(s!("vs"), s!("vs_4_0"))?;
            let pixel_code = compile(s!("ps"), s!("ps_4_0"))?;
            let mut vertex = None;
            device.CreateVertexShader(blob_bytes(&vertex_code), None, Some(&mut vertex))?;
            let mut pixel = None;
            device.CreatePixelShader(blob_bytes(&pixel_code), None, Some(&mut pixel))?;
            let mut place = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: 112,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut place),
            )?;
            let mut blend_desc = D3D11_BLEND_DESC::default();
            blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: true.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            let mut blend = None;
            device.CreateBlendState(&blend_desc, Some(&mut blend))?;
            let mut rasterizer = None;
            device.CreateRasterizerState(
                &D3D11_RASTERIZER_DESC {
                    FillMode: D3D11_FILL_SOLID,
                    CullMode: D3D11_CULL_NONE,
                    // The game's depth range isn't ours to keep: nothing cut off by it.
                    DepthClipEnable: false.into(),
                    MultisampleEnable: true.into(),
                    AntialiasedLineEnable: false.into(),
                    ..Default::default()
                },
                Some(&mut rasterizer),
            )?;
            let mut sampler = None;
            device.CreateSamplerState(
                &D3D11_SAMPLER_DESC {
                    Filter: D3D11_FILTER_ANISOTROPIC,
                    MaxAnisotropy: 8,
                    AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                    MaxLOD: f32::MAX,
                    // A little sharper than the mipmaps alone (the letters stay crisp, not soft).
                    MipLODBias: -0.5,
                    ..Default::default()
                },
                Some(&mut sampler),
            )?;
            let none = || windows::core::Error::empty();
            let logo = logo_texture(device)?;
            log!("tela flutuante: ready to be drawn");
            Ok(Self {
                device: device.clone(),
                frames: FrameSource::of(FrameKind::Screen),
                state: state.ok_or_else(none)?,
                vertex: vertex.ok_or_else(none)?,
                pixel: pixel.ok_or_else(none)?,
                place: place.ok_or_else(none)?,
                blend: blend.ok_or_else(none)?,
                rasterizer: rasterizer.ok_or_else(none)?,
                sampler: sampler.ok_or_else(none)?,
                view: None,
                logo,
                logo_view: None,
            })
        }
    }

    /// Draws the app's screen frame at `mvp` (the screen's own space to clip space) over the back buffer; `false`:
    /// no frame of the app's yet.
    ///
    /// # Safety
    /// As [`on_present`]: the game's swap chain, of this drawing's device.
    unsafe fn draw(&mut self, swapchain: &IDXGISwapChain, mvp: &M4, black: bool) -> windows::core::Result<bool> {
        // SAFETY: the game's live device, context and back buffer, on its render thread; the game's context state is
        // swapped back as it was before this returns.
        unsafe {
            let context = self.device.GetImmediateContext()?;
            // No picture from the app (closed, starting): the frame still there, black (owner 2026-11-21: it's gone
            // only when switched off in the app).
            let latest = self.frames.update(&self.device, &context);
            let black = black || latest.is_none();
            let back: ID3D11Texture2D = swapchain.GetBuffer(0).map_err(|e| step(e, "the back buffer"))?;
            let mut back_desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut back_desc);
            let format = crate::handset::view_format(back_desc.Format);
            let size = match &latest {
                Some((frame, _)) => {
                    let mut frame_desc = D3D11_TEXTURE2D_DESC::default();
                    frame.GetDesc(&mut frame_desc);
                    (frame_desc.Width, frame_desc.Height)
                }
                None => self.view.as_ref().map_or((SCREEN_WIDTH, SCREEN_HEIGHT), |(size, ..)| *size),
            };
            if !matches!(&self.view, Some((made_size, made, ..)) if *made_size == size && *made == format) {
                let mut texture = None;
                self.device.CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: size.0,
                        Height: size.1,
                        MipLevels: 0,
                        ArraySize: 1,
                        Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_TYPELESS,
                        SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: (windows::Win32::Graphics::Direct3D11::D3D11_BIND_SHADER_RESOURCE.0
                            | windows::Win32::Graphics::Direct3D11::D3D11_BIND_RENDER_TARGET.0)
                            as u32,
                        MiscFlags: windows::Win32::Graphics::Direct3D11::D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
                        ..Default::default()
                    },
                    None,
                    Some(&mut texture),
                )
                .map_err(|e| step(e, "the picture's texture"))?;
                let texture = texture.ok_or_else(windows::core::Error::empty)?;
                let mut view = None;
                self.device.CreateShaderResourceView(
                    &texture,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: format,
                        ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: u32::MAX },
                        },
                    }),
                    Some(&mut view),
                )
                .map_err(|e| step(e, "the picture's view"))?;
                self.view = Some((size, format, texture, view.ok_or_else(windows::core::Error::empty)?, u64::MAX));
                log!("tela flutuante: the picture kept with its mipmaps ({}x{})", size.0, size.1);
            }
            // In reverse, the picture composed with the reversing camera (owner 2026-11-20: only the guide lines showed,
            // on black): the one the truck's screen shows (`mirror::composed`), the same size as the frame.
            let source = latest.map(|(frame, version)| {
                crate::mirror::note_frame_if_none(&frame);
                match crate::mirror::composed() {
                    Some((composed, count)) => (composed, COMPOSED_VERSION | count),
                    None => (frame, version),
                }
            });
            if let Some((_, _, texture, view, held)) = &mut self.view {
                if black {
                    // The picture black (once), its smaller sizes with it.
                    if *held != BLACK_VERSION {
                        // Typeless: its view says the format (without one, "invalid parameter" at every ignition off,
                        // and the frame gone with the truck off: owner 2026-11-21).
                        let mut target = None;
                        self.device
                            .CreateRenderTargetView(
                                &*texture,
                                Some(&D3D11_RENDER_TARGET_VIEW_DESC {
                                    Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
                                    ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                                    Anonymous: D3D11_RENDER_TARGET_VIEW_DESC_0 {
                                        Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 },
                                    },
                                }),
                                Some(&mut target),
                            )
                            .map_err(|e| step(e, "the black picture's target"))?;
                        if let Some(target) = target {
                            context.ClearRenderTargetView(&target, &[0.0, 0.0, 0.0, 1.0]);
                        }
                        context.GenerateMips(&*view);
                        *held = BLACK_VERSION;
                    }
                } else if let Some((source, version)) = &source
                    && *held != *version
                {
                    let version = *version;
                    context.CopySubresourceRegion(&*texture, 0, 0, 0, 0, source, 0, None);
                    context.GenerateMips(&*view);
                    *held = version;
                }
            }
            if !matches!(&self.logo_view, Some((made, _)) if *made == format) {
                let mut view = None;
                self.device.CreateShaderResourceView(
                    &self.logo,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: format,
                        ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
                        },
                    }),
                    Some(&mut view),
                )
                .map_err(|e| step(e, "the logo's view"))?;
                self.logo_view = Some((format, view.ok_or_else(windows::core::Error::empty)?));
            }
            let Some((_, _, _, view, _)) = &self.view else { return Ok(false) };
            let Some((_, logo_view)) = &self.logo_view else { return Ok(false) };
            let mut target = None;
            self.device.CreateRenderTargetView(&back, None, Some(&mut target)).map_err(|e| step(e, "the back buffer's target"))?;
            let target = target.ok_or_else(windows::core::Error::empty)?;
            self.render(&context, &target, back_desc.Width as f32, back_desc.Height as f32, mvp, view, logo_view)?;
            Ok(true)
        }
    }

    /// Draws the frame at `mvp` into `target` (`width` x `height`), with the picture `view` and the logo's
    /// `logo_view`, in the drawing's own context state (the context's swapped back after).
    ///
    /// # Safety
    /// This drawing's device's live immediate context, on its thread; the views of its device.
    #[allow(clippy::too_many_arguments)]
    unsafe fn render(
        &self,
        context: &ID3D11DeviceContext,
        target: &ID3D11RenderTargetView,
        width: f32,
        height: f32,
        mvp: &M4,
        view: &ID3D11ShaderResourceView,
        logo_view: &ID3D11ShaderResourceView,
    ) -> windows::core::Result<()> {
        // SAFETY: as the caller says; the context's state is swapped back as it was before this returns.
        unsafe {
            let (back_width, back_height) = (width, height);
            let height = WIDTH * SCREEN_HEIGHT as f32 / SCREEN_WIDTH as f32;
            let mut place = [0f32; 28];
            for r in 0..4 {
                place[r * 4..r * 4 + 4].copy_from_slice(&mvp[r]);
            }
            place[16..28].copy_from_slice(&frame_layout(height));
            context.UpdateSubresource(&self.place, 0, None, place.as_ptr().cast(), 0, 0);
            let context1: ID3D11DeviceContext1 = context.cast().map_err(|e| step(e, "the context"))?;
            let mut game = None;
            context1.SwapDeviceContextState(&self.state, Some(&mut game));
            context.OMSetRenderTargets(Some(&[Some(target.clone())]), None);
            context.OMSetBlendState(&self.blend, None, 0xffff_ffff);
            context.RSSetState(&self.rasterizer);
            context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: back_width,
                Height: back_height,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
            context.IASetInputLayout(None);
            context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            context.VSSetShader(&self.vertex, None);
            context.VSSetConstantBuffers(0, Some(&[Some(self.place.clone())]));
            context.PSSetShader(&self.pixel, None);
            // The frame's layout is the pixel shader's too (owner 2026-11-20: without it the frame came out empty).
            context.PSSetConstantBuffers(0, Some(&[Some(self.place.clone())]));
            context.PSSetShaderResources(0, Some(&[Some(view.clone()), Some(logo_view.clone())]));
            context.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            context.Draw(4, 0);
            // Let go of the game's back buffer: it's resized only once nothing holds it.
            context.OMSetRenderTargets(Some(&[None]), None);
            context.PSSetShaderResources(0, Some(&[None, None]));
            if let Some(game) = &game {
                context1.SwapDeviceContextState(game, None);
            }
            Ok(())
        }
    }
}

/// `e` said with the step it came from (for the log).
fn step(e: windows::core::Error, what: &str) -> windows::core::Error {
    // Said here: the error's own text keeps only the system's words, not which step (2026-11-21 log).
    if FAILURES.load(Ordering::Relaxed) < FAILURES_SAID {
        log!("tela flutuante: failed at {what}: {e}");
    }
    e
}

/// The frame's layout for the shader, in the screen's own space (metres; the picture `height` tall round the middle):
/// the frame (left, bottom, right, top), the picture's size and the corners, the logo (left, top, width, height).
fn frame_layout(height: f32) -> [f32; 12] {
    let (logo_w, logo_h) = logo_size();
    let tall = CHIN * LOGO_OF_CHIN;
    let wide = tall * logo_w as f32 / logo_h.max(1) as f32;
    let logo_top = -height / 2.0 - (CHIN - tall) / 2.0;
    [
        -WIDTH / 2.0 - BEZEL,
        -height / 2.0 - CHIN,
        WIDTH / 2.0 + BEZEL,
        height / 2.0 + BEZEL,
        WIDTH,
        height,
        CORNER,
        PICTURE_CORNER,
        -wide / 2.0,
        logo_top,
        wide,
        tall,
    ]
}

fn logo_size() -> (u32, u32) {
    let word = |at: usize| u32::from_le_bytes([LOGO[at], LOGO[at + 1], LOGO[at + 2], LOGO[at + 3]]);
    (word(0), word(4))
}

/// The logo as a texture (BGRA, typeless: viewed as the frame is, sRGB or not).
///
/// # Safety
/// The game's live device.
unsafe fn logo_texture(device: &ID3D11Device) -> windows::core::Result<ID3D11Texture2D> {
    let (width, height) = logo_size();
    let pixels = &LOGO[8..];
    if pixels.len() < (width * height * 4) as usize {
        return Err(windows::core::Error::empty());
    }
    let mut texture = None;
    // SAFETY: the description and the pixels are locals of the sizes given.
    unsafe {
        device.CreateTexture2D(
            &D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_TYPELESS,
                SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: windows::Win32::Graphics::Direct3D11::D3D11_USAGE_IMMUTABLE,
                BindFlags: windows::Win32::Graphics::Direct3D11::D3D11_BIND_SHADER_RESOURCE.0 as u32,
                ..Default::default()
            },
            Some(&windows::Win32::Graphics::Direct3D11::D3D11_SUBRESOURCE_DATA {
                pSysMem: pixels.as_ptr().cast(),
                SysMemPitch: width * 4,
                SysMemSlicePitch: 0,
            }),
            Some(&mut texture),
        )?;
    }
    texture.ok_or_else(windows::core::Error::empty)
}

fn compile(entry: PCSTR, target: PCSTR) -> windows::core::Result<ID3DBlob> {
    let mut code = None;
    let mut errors = None;
    // SAFETY: the source is a static string of the length given; the blobs out are locals.
    let compiled = unsafe {
        D3DCompile(
            SHADERS.as_ptr().cast(),
            SHADERS.len(),
            s!("westplay-float-screen"),
            None,
            None,
            entry,
            target,
            0,
            0,
            &mut code,
            Some(&mut errors),
        )
    };
    if let Err(e) = compiled {
        let said = errors.map(|blob| String::from_utf8_lossy(blob_bytes(&blob)).into_owned()).unwrap_or_default();
        return Err(windows::core::Error::new(e.code(), said));
    }
    code.ok_or_else(windows::core::Error::empty)
}

fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
    // SAFETY: the blob's buffer is its own, alive and of its size while the blob is.
    unsafe { std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_shaders_compile() {
        compile(s!("vs"), s!("vs_4_0")).unwrap();
        compile(s!("ps"), s!("ps_4_0")).unwrap();
    }

    /// A block of the cab's interior as the probe had it (its origin at the head, 1.4 cm off), and the steering
    /// wheel's (66 cm ahead): only the first is the cab's.
    #[test]
    fn the_cabs_blocks_are_at_the_head() {
        let interior: [f32; 16] = [
            0.9878, -0.0149, 0.1549, -0.0022, 0.0757, 0.9158, -0.3943, 0.0057, -0.1360, 0.4013, 0.9058, -0.0128, 0.0,
            0.0, 0.0, 1.0,
        ];
        let mut numbers = [0f32; 32];
        numbers[..16].copy_from_slice(&interior);
        assert!(cab_miss(&numbers, [0.0; 3]).unwrap() < HEAD_NEAR);
        // Its projection under it, as the probe had it: seen through the lens; a block with no turn isn't.
        let pmv: [f32; 16] = [
            0.9314, -0.0141, 0.1460, -0.0021, 0.1268, 1.5352, -0.6610, 0.0096, -0.0009, 0.0027, 0.0061, 0.1006, 0.1360,
            -0.4013, -0.9058, 0.0128,
        ];
        numbers[16..].copy_from_slice(&pmv);
        assert!(seen_through_lens(&numbers));
        let mut plain = [0f32; 32];
        for i in 0..4 {
            plain[i * 5] = 1.0;
            plain[16 + i * 5] = 1.0;
        }
        assert!(!seen_through_lens(&plain));
        let wheel: [f32; 16] = [
            0.9878, 0.0949, 0.1233, -0.0832, 0.0756, 0.3997, -0.9135, -0.1093, -0.1360, 0.9117, 0.3877, -0.6649, 0.0,
            0.0, 0.0, 1.0,
        ];
        numbers[..16].copy_from_slice(&wheel);
        assert!(cab_miss(&numbers, [0.0; 3]).unwrap() > HEAD_NEAR);
    }

    /// Where it goes by itself: ahead of the head, lower, towards the middle, its face turned to the head.
    #[test]
    fn it_stands_in_front_of_the_driver_facing_the_head() {
        let head = [-0.6f32, 2.1, -1.2];
        let m = placement(head, [0.0; 7]);
        let centre = [m[0][3] + head[0], m[1][3] + head[1], m[2][3] + head[2]];
        assert!(centre[2] < head[2] - 0.5, "ahead (forward is -z)");
        assert!(centre[1] < head[1] && centre[0] > head[0]);
        // Its face (+z of its own) points at the head.
        let face = [m[0][2], m[1][2], m[2][2]];
        let to_head: [f32; 3] = std::array::from_fn(|i| head[i] - centre[i]);
        let length = (to_head.iter().map(|v| v * v).sum::<f32>()).sqrt();
        let along: f32 = (0..3).map(|i| face[i] * to_head[i] / length).sum();
        assert!(along > 0.999, "{along}");
        // Its right (+x of its own) is level.
        assert!(m[1][0].abs() < 1e-4);
    }

    /// The frame drawn on a software device (WARP), straight ahead of the camera: the picture's middle shows the
    /// picture, the chin is drawn, outside the frame is left alone.
    #[test]
    fn it_is_drawn() {
        use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP;
        use windows::Win32::Graphics::Direct3D11::{
            D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_FLAG,
            D3D11_MAP_READ, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_STAGING, D3D11CreateDevice,
        };
        use windows::Win32::Graphics::Dxgi::Common::{
            DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC,
        };
        unsafe {
            let mut device = None;
            let mut context = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .unwrap();
            let (device, context): (ID3D11Device, ID3D11DeviceContext) = (device.unwrap(), context.unwrap());
            let drawing = Drawing::new(&device).unwrap();
            let size = 256u32;
            let texture = |format, bind: u32, data: Option<&[u8]>| {
                let mut made = None;
                let init = data.map(|d| D3D11_SUBRESOURCE_DATA { pSysMem: d.as_ptr().cast(), SysMemPitch: size * 4, SysMemSlicePitch: 0 });
                device
                    .CreateTexture2D(
                        &D3D11_TEXTURE2D_DESC {
                            Width: size,
                            Height: size,
                            MipLevels: 1,
                            ArraySize: 1,
                            Format: format,
                            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                            Usage: D3D11_USAGE_DEFAULT,
                            BindFlags: bind,
                            ..Default::default()
                        },
                        init.as_ref().map(|i| i as *const _),
                        Some(&mut made),
                    )
                    .unwrap();
                made.unwrap()
            };
            // The picture: all green.
            let green: Vec<u8> = (0..size * size).flat_map(|_| [0u8, 255, 0, 255]).collect();
            let picture = texture(DXGI_FORMAT_B8G8R8A8_UNORM, D3D11_BIND_SHADER_RESOURCE.0 as u32, Some(&green));
            let target = texture(DXGI_FORMAT_R8G8B8A8_UNORM, D3D11_BIND_RENDER_TARGET.0 as u32, None);
            let mut view = None;
            device.CreateShaderResourceView(&picture, None, Some(&mut view)).unwrap();
            let mut logo_view = None;
            device
                .CreateShaderResourceView(
                    &drawing.logo,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
                        },
                    }),
                    Some(&mut logo_view),
                )
                .unwrap();
            let mut rtv = None;
            device.CreateRenderTargetView(&target, None, Some(&mut rtv)).unwrap();
            // The screen 0.6 m straight ahead, through the game's lens (the probe's).
            let projection: M4 = [[0.943, 0.0, 0.0, 0.0], [0.0, 1.676, 0.0, 0.0], [0.0, 0.0, 0.0066, 0.1], [0.0, 0.0, -1.0, 0.0]];
            let place: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, -0.6], [0.0, 0.0, 0.0, 1.0]];
            let mvp = product(&projection, &place);
            drawing
                .render(&context, rtv.as_ref().unwrap(), size as f32, size as f32, &mvp, view.as_ref().unwrap(), logo_view.as_ref().unwrap())
                .unwrap();
            let staging = {
                let mut made = None;
                device
                    .CreateTexture2D(
                        &D3D11_TEXTURE2D_DESC {
                            Width: size,
                            Height: size,
                            MipLevels: 1,
                            ArraySize: 1,
                            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                            Usage: D3D11_USAGE_STAGING,
                            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                            ..Default::default()
                        },
                        None,
                        Some(&mut made),
                    )
                    .unwrap();
                made.unwrap()
            };
            context.CopyResource(&staging, &target);
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).unwrap();
            let pixel = |x: u32, y: u32| {
                let at = (mapped.pData as *const u8).add((y * mapped.RowPitch + x * 4) as usize);
                [*at, *at.add(1), *at.add(2), *at.add(3)]
            };
            if let Ok(path) = std::env::var("FLOAT_SCREEN_DUMP") {
                let mut bytes = Vec::new();
                for y in 0..size {
                    for x in 0..size {
                        bytes.extend_from_slice(&pixel(x, y));
                    }
                }
                std::fs::write(path, bytes).unwrap();
            }
            let middle = pixel(size / 2, size / 2);
            let corner = pixel(2, 2);
            assert_eq!(corner, [0, 0, 0, 0], "outside the frame left alone");
            assert!(middle[1] > 200 && middle[0] < 30 && middle[3] > 200, "the picture in the middle: {middle:?}");
        }
    }

    /// A block moved by the head's offset (the seat moved) is told as the cab's.
    #[test]
    fn the_cab_is_found_with_the_seat_moved() {
        let (s, c) = 40f32.to_radians().sin_cos();
        let cab: M4 = [[c, 0.0, -s, 0.01], [0.0, 1.0, 0.0, -0.02], [s, 0.0, c, 0.03], [0.0, 0.0, 0.0, 1.0]];
        let block = |m: &M4| {
            let mut numbers = [0f32; 32];
            for r in 0..4 {
                numbers[r * 4..r * 4 + 4].copy_from_slice(&m[r]);
            }
            numbers
        };
        // The head moved 30 cm back (the seat): the cab's origin 30 cm ahead, turned.
        let offset = [0.0, 0.0, 0.3];
        let mut moved = cab;
        for r in 0..3 {
            moved[r][3] = -(0..3).map(|k| cab[r][k] * offset[k]).sum::<f32>();
        }
        assert!(at_offset(&block(&moved), offset));
        assert!(!at_offset(&block(&cab), offset));
    }

    /// The picture's copy with its mipmaps can be made and filled, viewed sRGB or not (as `draw` does).
    #[test]
    fn the_picture_gets_its_mipmaps() {
        use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP;
        use windows::Win32::Graphics::Direct3D11::{
            D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CREATE_DEVICE_FLAG, D3D11_RESOURCE_MISC_GENERATE_MIPS,
            D3D11CreateDevice,
        };
        use windows::Win32::Graphics::Dxgi::Common::{
            DXGI_FORMAT_B8G8R8A8_TYPELESS, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_SAMPLE_DESC,
        };
        unsafe {
            let (mut device, mut context) = (None, None);
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .unwrap();
            let (device, context): (ID3D11Device, ID3D11DeviceContext) = (device.unwrap(), context.unwrap());
            let make = |mips: u32, bind: u32, misc: u32| {
                let mut made = None;
                device
                    .CreateTexture2D(
                        &D3D11_TEXTURE2D_DESC {
                            Width: SCREEN_WIDTH,
                            Height: SCREEN_HEIGHT,
                            MipLevels: mips,
                            ArraySize: 1,
                            Format: DXGI_FORMAT_B8G8R8A8_TYPELESS,
                            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                            Usage: D3D11_USAGE_DEFAULT,
                            BindFlags: bind,
                            MiscFlags: misc,
                            ..Default::default()
                        },
                        None,
                        Some(&mut made),
                    )
                    .unwrap();
                made.unwrap()
            };
            let frame = make(1, D3D11_BIND_SHADER_RESOURCE.0 as u32, 0);
            for format in [DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_FORMAT_B8G8R8A8_UNORM] {
                let mipped = make(
                    0,
                    (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
                    D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
                );
                let mut view = None;
                device
                    .CreateShaderResourceView(
                        &mipped,
                        Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                            Format: format,
                            ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                                Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: u32::MAX },
                            },
                        }),
                        Some(&mut view),
                    )
                    .unwrap();
                context.CopySubresourceRegion(&mipped, 0, 0, 0, 0, &frame, 0, None);
                context.GenerateMips(view.as_ref().unwrap());
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                mipped.GetDesc(&mut desc);
                assert!(desc.MipLevels > 8, "{}", desc.MipLevels);
                // Made black (the truck off): a typeless texture's target needs its format said.
                let mut bare = None;
                assert!(device.CreateRenderTargetView(&mipped, None, Some(&mut bare)).is_err());
                let mut target = None;
                device
                    .CreateRenderTargetView(
                        &mipped,
                        Some(&D3D11_RENDER_TARGET_VIEW_DESC {
                            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                            Anonymous: D3D11_RENDER_TARGET_VIEW_DESC_0 { Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 } },
                        }),
                        Some(&mut target),
                    )
                    .unwrap();
                context.ClearRenderTargetView(target.as_ref().unwrap(), &[0.0, 0.0, 0.0, 1.0]);
            }
        }
    }

    /// The logo is read whole, and sits in the chin, under the picture, within the frame.
    #[test]
    fn the_logo_sits_in_the_chin() {
        let (w, h) = logo_size();
        assert_eq!(LOGO.len(), 8 + (w * h * 4) as usize);
        let height = WIDTH * 9.0 / 16.0;
        let [left, bottom, right, _, _, _, _, _, x, top, wide, tall] = frame_layout(height);
        assert!(top < -height / 2.0 && top - tall > bottom, "in the chin");
        assert!(x > left && x + wide < right, "within the frame");
    }

    #[test]
    fn its_place_is_kept_and_read_back() {
        let mut by_truck = std::collections::BTreeMap::new();
        by_truck.insert("truck:vehicle.scania.s_2016".to_string(), [0.02, -0.04, 0.1, 4.0, -2.0, 0.0, -20.0]);
        by_truck.insert("truck:vehicle.man.tgx".to_string(), [0.32, -0.6, 0.74, 26.0, 0.0, -2.0, 0.0]);
        let (on, read) = read_kept(&write_kept(true, &by_truck));
        assert!(on);
        assert_eq!(read, by_truck);
        assert_eq!(moved([0.0; 7], westplay_shared::FLOAT_FORWARD).unwrap()[2], MOVE_STEP);
        assert_eq!(moved([0.3; 7], westplay_shared::FLOAT_RESET).unwrap(), [0.0; 7]);
        assert_eq!(moved([0.0; 7], westplay_shared::FLOAT_TOGGLE), None);
        assert_eq!(moved([0.0; 7], westplay_shared::FLOAT_SMALLER).unwrap()[6], -SIZE_STEP);
        // A line kept before the size (six numbers) is read with the size as it was.
        let (_, old) = read_kept("ligada 1
truck:vehicle.man.tgx 0.32 -0.6 0.74 26 0 -2
");
        assert_eq!(old["truck:vehicle.man.tgx"], [0.32, -0.6, 0.74, 26.0, 0.0, -2.0, 0.0]);
    }
}
