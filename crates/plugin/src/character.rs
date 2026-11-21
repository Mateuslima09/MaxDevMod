//! Modo Personagem, test build (path 1 in docs/personagem/INVESTIGACAO.md): walking about on foot with the game's
//! own walking camera (`walking_camera`, the one its truck-configuration scenes use), on a key, single player only.
//!
//! F8, with the truck stopped on its parking brake: camera 0 (the game's `debug_camera`, `g_developer`) is put on
//! screen first if it isn't (its key, 0, given to the game's window as the player's presses are); then the plugin
//! makes a walking camera as the configuration scene does (the game's factory, then its start,
//! given the road's physics world instead of the scene's, which makes its capsule body there), standing outside
//! the driver's door, and switches the game's input to it (the camera's own "activate"). Then, each frame, it
//! steps the walking camera (its own update: its body on the ground) and puts camera 0 where it is, so the game
//! draws the world from there. Camera 0 keeps the mouse (the view turns with it); the walking camera is turned
//! where camera 0 looks before each step, so walking goes where the player looks. The walking keys (W, A, S, D,
//! Shift, Space, Ctrl) are kept from the game meanwhile (`keyboard`, `input`), so the truck neither steers,
//! speeds up nor brakes with them, and are given to the walking camera's own controls instead (each control's
//! state: 2 while its key is down). The camera keys (1 to 9, 0) are kept from the game too: another camera would
//! end walking. F8 again by the driver's door (or held two seconds anywhere, or the truck
//! moving) gives the input back as it was, lets the walking camera go and goes back to the cab (the camera 1 key). The mouse turns the view slower than the game's
//! while walking (`input` scales it: the app's setting, 50% at first; `-` and `=` while walking take it down and
//! up, kept by the app). Only with the Modo Personagem on in the app (its Personagem page; off by default).
//!
//! Every piece is found by the shape of the game's code, as `navigation` does: the configuration scene's call
//! sequence (factory, start, smart-pointer assignment), the game creating its road physics world (the system's
//! and the world's offsets in its core object), and the core object's global. The cameras are told by their
//! class names (each object's `get_class`, the game's own class descriptors). Anything not found, not as
//! described, or under TruckersMP, and nothing is done; the log says why. Worked out on ETS2 1.61.1.9; the same
//! shapes are in ATS 1.62.
//!
//! Placements (`core_camera` + 0x40) are the game's: local x, y, z (floats) in a 512 m sector, the sector's x
//! and z (two i16), then a rotation quaternion (w, x, y, z).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::time::Instant;

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MAPVK_VSC_TO_VK, MapVirtualKeyW, VIRTUAL_KEY, VK_CONTROL, VK_F8, VK_LCONTROL, VK_LSHIFT,
    VK_OEM_MINUS, VK_OEM_PLUS, VK_RCONTROL, VK_RSHIFT, VK_SHIFT, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_KEYDOWN, WM_KEYUP};
use windows::core::w;

use westplay_shared::{CHARACTER_PEOPLE, CHARACTER_SENSITIVITY};

use crate::log;
use crate::navigation::Image;

mod climb;
mod freight;
mod meter;
mod notice;
mod person;
mod probe;
mod refuel;
mod steps;

pub(crate) use steps::{BrakeSound, brake as brake_sound};

/// The configuration scene starting its walking camera: `mov r8, [rsi+world]; lea rdx, [rsi+placement];
/// mov rcx, r15; call start; mov eax, 0xdfffffff; lock xadd [rdi], eax; mov rdx, r15; mov rcx, r14; call assign`.
const START_SITE: &str = "4C 8B 86 ?? ?? ?? ?? 48 8D 96 ?? ?? ?? ?? 49 8B CF E8 ?? ?? ?? ?? B8 FF FF FF DF F0 0F C1 07 \
                          49 8B D7 49 8B CE E8 ?? ?? ?? ??";
/// Just before it, making the camera: `mov eax, [mode]; cmp eax, 4; je ..; lea rcx, [table]; mov rcx,
/// [rcx+rax*8]; test rcx, rcx; je ..; call factory; mov r15, rax`.
const FACTORY_SITE: &str = "8B 05 ?? ?? ?? ?? 83 F8 04 0F 84 ?? ?? ?? ?? 48 8D 0D ?? ?? ?? ?? 48 8B 0C C1 48 85 C9 \
                            0F 84 ?? ?? ?? ?? E8 ?? ?? ?? ?? 4C 8B F8";
/// The game creating its road physics world: `mov rcx, [rdi+system]; movss [..], xmm0; call create_world;
/// mov rcx, [rdi+world]; mov rbx, rax; test rcx, rcx; je ..; mov r8, [rcx]; mov edx, r15d; call [r8];
/// mov [rdi+world], rbx`.
const WORLD_SITE: &str = "48 8B 8F ?? ?? ?? ?? F3 0F 11 05 ?? ?? ?? ?? E8 ?? ?? ?? ?? 48 8B 8F ?? ?? ?? ?? 48 8B D8 \
                          48 85 C9 74 09 4C 8B 01 41 8B D7 41 FF 10 48 89 9F ?? ?? ?? ??";
/// Code reading the core object's global and its physics system, then creating a world from it: `mov rax,
/// [core]; mov r14, [rax+system]; mov rcx, r14; call create_world`.
const CORE_SITE: &str = "48 8B 05 ?? ?? ?? ?? 4C 8B B0 ?? ?? ?? ?? 49 8B CE E8 ?? ?? ?? ??";
/// The walking camera's "activate" (its vtable's slot 29): `mov rdx, [input]; mov rax, [mode]; mov
/// [rdx+context], rax; movzx eax, byte [rcx+..]; cmp [rdx+flag], al; je ..; mov rcx, rdx; mov [rdx+flag], al;
/// jmp apply; ret`. Read to put the input back as it was.
const ACTIVATE: &str = "48 8B 15 ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 48 89 82 ?? ?? ?? ?? 0F B6 81 ?? ?? ?? ?? \
                        38 82 ?? ?? ?? ?? 74 0E 48 8B CA 88 82 ?? ?? ?? ?? E9 ?? ?? ?? ?? C3";

/// The walking camera's start putting its capsule body on its collision layer: `mov rcx, [rbx+body]; mov edx,
/// 0x12; call set_layer` (layer 18, which collides with the player's truck and trailer and so damages them:
/// docs/personagem/INVESTIGACAO.md §7.1).
const LAYER_SITE: &str = "48 8B 8B ?? ?? ?? ?? BA 12 00 00 00 E8 ?? ?? ?? ??";
/// Its making the capsule's shape: `call create_shape; movss xmm3, [rbx+..]; lea rcx, [rbp+..]; movss xmm4,
/// [rbx+..]; xorps xmm1, xmm1; mov [rbx+shape], rax`.
const SHAPE_SITE: &str =
    "E8 ?? ?? ?? ?? F3 0F 10 9B ?? ?? ?? ?? 48 8D 4D ?? F3 0F 10 A3 ?? ?? ?? ?? 0F 57 C9 48 89 83 ?? ?? ?? ??";
/// Its first lines: `mov [rcx+world], r8; mov rdi, [r8+system]` (the world's physics system).
const SYSTEM_SITE: &str = "4C 89 81 ?? ?? ?? ?? 49 8B B8 ?? ?? ?? ??";
/// The shape's margin, by the system's kind: `movsxd rax, [rdi+kind]; lea rcx, [margins]`.
const MARGIN_SITE: &str = "48 63 47 ?? 48 8D 0D ?? ?? ?? ??";
/// The game's set_layer itself: `and dword [rcx+field], 0xfffff03f` (bits 6..11 of the body's collision field),
/// then, for each shape, `mov rcx, [rbx]; mov edx, edi; mov rax, [rcx]; call [rax+slot]` (the shape's own).
const SET_LAYER: &str = "48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20 81 A1 ?? ?? ?? ?? 3F F0 FF FF";
const SET_SHAPE_LAYER: &str = "48 8B 0B 8B D7 48 8B 01 FF 90 ?? ?? ?? ??";
/// The walking camera's destructor letting its shape and body go: `mov rax, [rbx+world]; mov rdx, [rbx+shape];
/// mov [rsp+30h], rdi; mov rdi, [rax+system]; test rdx, rdx; je ..; mov rax, [rdi]; mov rcx, rdi; call
/// [rax+destroy_shape]`.
const RELEASE_SITE: &str = "48 8B 83 ?? ?? ?? ?? 48 8B 93 ?? ?? ?? ?? 48 89 7C 24 30 48 8B B8 ?? ?? ?? ?? 48 85 D2 74 13 \
                            48 8B 07 48 8B CF FF 90 ?? ?? ?? ??";
/// The layers (the road world's collision table, read in the game: INVESTIGACAO.md §7.1). 9: the ground and the
/// world's fixed things, no vehicle at all (4 traffic, 6 and 7 the player's truck, 8 the trailer), so no damage.
/// 3: only 6, 8 and 20, and the truck's and trailer's contact listeners skip it for damage: a second shape on
/// it bumps into the truck without damaging it (owner 2026-11-04: "bump, but no damage").
const BODY_LAYER: u32 = 9;
const BUMP_LAYER: u32 = 3;
/// 22: meets the traffic (4) and not the player's truck (6, 7) nor the trailer (8): a third shape on it bumps into
/// the game's cars (owner 2026-11-19: "the game's cars have no physics; my person goes inside them"), the truck's
/// listener never told (no damage there).
const TRAFFIC_LAYER: u32 = 22;
/// A body whose shapes have layers of their own (what the game's attach writes when they differ).
const MIXED_LAYERS: u32 = 0x20;
/// The capsule shape's turn in its body (the start's own: (0.7071, 0.7071, 0, 0)).
const SHAPE_TURN: u32 = 0x3f35_04f3;

/// Where a camera keeps its placement (the `core_camera` part of it).
const PLACEMENT: usize = 0x40;
/// The cameras' virtual functions used: `get_class`, the update (`this`, seconds), and "activate".
const CLASS_SLOT: usize = 5;
const UPDATE_SLOT: usize = 13;
const ACTIVATE_SLOT: usize = 29;
/// The walking camera's controls (`camwalk_*`): where it keeps each one's object, from +0x3f0 every 0x10 bytes,
/// in the order forward, back, left, right, run, jump, crouch (as its movement reads them); the key giving each
/// (the player's controls.sii: W, S, A, D, left Shift, Space, left Ctrl), as a virtual key and a scan code.
const CONTROLS: usize = 0x3f0;
const CONTROL_KEYS: [(u16, u32); 7] = [
    (b'W' as u16, 0x11),
    (b'S' as u16, 0x1f),
    (b'A' as u16, 0x1e),
    (b'D' as u16, 0x20),
    (VK_LSHIFT.0, 0x2a),
    (VK_SPACE.0, 0x39),
    (VK_LCONTROL.0, 0x1d),
];
/// A control's state (2: down, 0: up), in its object.
const CONTROL_STATE: usize = 0xf8;
const DOWN: u32 = 2;
/// Where the walking camera's view turns (radians about the vertical, 0..2π): its own heading, which its
/// steps go by.
const YAW: usize = 0x66c;
/// The walking camera's mode (the scene sets 0 unless the game is in its bus mode): +0x6a0.
const WALKING_MODE: usize = 0x6a0;
/// The camera manager's list of cameras: data, count.
const MANAGER_LIST: usize = 0x38;
/// How far into the core object (`game_ctrl`, 0x4750 bytes in 1.61) its camera manager is looked for.
const CORE_SIZE: usize = 0x4750;
/// A sector's side, in metres.
const SECTOR: f64 = 512.0;
/// Where a person's eyes are over the ground, and how far out of the cab the door's outside is.
const EYE_HEIGHT: f64 = 1.8;
const DOOR_OUT: f32 = 1.0;
/// How far ahead of the driver's head the door's step is (metres; owner 2026-11-05: 0.3 behind it, the climb
/// started "too far back", behind the door).
const DOOR_AHEAD: f32 = 0.25;
/// How near the driver's door (along the ground, metres) F8 gets the player back in the cab; and how long F8 held
/// gets them back from anywhere.
const DOOR_REACH: f64 = 1.5;
const HOLD_TO_LEAVE: std::time::Duration = std::time::Duration::from_secs(2);
/// The truck counts as stopped under this speed (m/s), and walking ends past this one.
const STOPPED: f32 = 0.2;
const MOVING: f32 = 1.0;

type Factory = unsafe extern "system" fn(*mut u8) -> *mut u8;
type Start = unsafe extern "system" fn(*mut u8, *const Placement, *mut u8);
type Assign = unsafe extern "system" fn(*mut *mut u8, *mut u8);
type Update = unsafe extern "system" fn(*mut u8, f32);
type Activate = unsafe extern "system" fn(*mut u8);
type Apply = unsafe extern "system" fn(*mut u8);
type SetLayer = unsafe extern "system" fn(*mut u8, u32);
/// (system, body, the shape's two sizes as its +0x1c and +0x18 keep them, margin, its place in the body).
type CreateShape = unsafe extern "system" fn(*mut u8, *mut u8, f32, f32, f32, *const Placement) -> *mut u8;
type SetShapeLayer = unsafe extern "system" fn(*mut u8, u32);
type DestroyShape = unsafe extern "system" fn(*mut u8, *mut u8);

/// The walking camera's capsule body, as its start makes it: what's needed to put it on other collision layers.
struct Physics {
    /// Offsets in the walking camera (its body, its shape, its world) and in the world (its physics system).
    body: usize,
    shape: usize,
    world: usize,
    system: usize,
    set_layer: SetLayer,
    /// The body's collision field (its layer in bits 6..11), and the shape's virtual setting a shape's layer.
    layer_field: usize,
    shape_layer_slot: usize,
    create_shape: CreateShape,
    /// The margins by the system's kind (its field at `kind`), and the system's virtual letting a shape go.
    kind: usize,
    margins: usize,
    destroy_shape_slot: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Placement {
    local: [f32; 3],
    sector: [i16; 2],
    rotation: [f32; 4],
}

impl Placement {
    /// The world position (metres).
    fn world(&self) -> [f64; 3] {
        [
            f64::from(self.sector[0]) * SECTOR + f64::from(self.local[0]),
            f64::from(self.local[1]),
            f64::from(self.sector[1]) * SECTOR + f64::from(self.local[2]),
        ]
    }

    /// At a world position, turned by `heading` (turns) about the vertical.
    fn at(world: [f64; 3], heading: f32) -> Self {
        let sector = |v: f64| (v / SECTOR).round() as i16;
        let (sx, sz) = (sector(world[0]), sector(world[2]));
        let half = heading * std::f32::consts::PI;
        Placement {
            local: [
                (world[0] - f64::from(sx) * SECTOR) as f32,
                world[1] as f32,
                (world[2] - f64::from(sz) * SECTOR) as f32,
            ],
            sector: [sx, sz],
            rotation: [half.cos(), 0.0, half.sin(), 0.0],
        }
    }
}

/// Where a rotation (w, x, y, z) looks, turned about the vertical: radians, 0..2π, as the walking camera keeps
/// its heading (its own rotation is made from it the same way: a turn of θ is (cos θ/2, 0, sin θ/2, 0)).
fn yaw_of([w, x, y, z]: [f32; 4]) -> f32 {
    (2.0 * (x * z + w * y)).atan2(1.0 - 2.0 * (x * x + y * y)).rem_euclid(std::f32::consts::TAU)
}

/// What was found in the game's code.
struct Game {
    image: Image,
    factory: Factory,
    /// The global the scene indexes its factory's argument by, and the table it indexes.
    allocator_mode: usize,
    allocator_table: usize,
    start: Start,
    assign: Assign,
    /// The global holding the core object, and the road physics world's offset in it.
    core: usize,
    world: usize,
    /// The capsule's layers (none found: walking goes on with the game's layer, as before).
    physics: Option<Physics>,
    /// The game's people (none found: F9 does nothing).
    people: Option<person::Game>,
    /// The game's notices in the screen's corner (none found: the log only).
    notices: Option<notice::Notices>,
}

// SAFETY: addresses in the game's executable, used only on its main thread.
unsafe impl Send for Game {}

/// The input as it was before walking (to put it back).
struct Input {
    input: usize,
    context: usize,
    context_was: u64,
    flag: usize,
    flag_was: u8,
    apply: Apply,
}

struct Walking {
    /// Measurements for the walk's timing, in the log only (`meter`: TRANSICOES.md 7.7, test B).
    meter: meter::Meter,
    /// The walking camera (held as a smart pointer of the game's: one reference).
    camera: *mut u8,
    /// Camera 0, put where the walking camera is.
    debug: *mut u8,
    input: Input,
    last: Instant,
    /// Frames in a row camera 0 wasn't the one drawn.
    away: u32,
    /// The capsule's body and its physics system, if it was put on the no-vehicle layer; and the second shape
    /// bumping into the truck.
    body: Option<(*mut u8, *mut u8)>,
    bump: Option<*mut u8>,
    /// The third shape, bumping into the traffic (`TRAFFIC_LAYER`).
    traffic: Option<*mut u8>,
    /// The game's person on the player's feet (third person), and whether the camera is behind them.
    person: Option<person::Body>,
    third_person: bool,
    /// How far out the third-person camera is, of the way to where it would be (0..1): in at once where a thing is
    /// in the way, back out slowly (`CAMERA_BACK`).
    camera_reach: f64,
    /// Climbing in from behind the person: the camera reached their eyes, the body to be hidden.
    hide_body: bool,
    /// Where the body faces (radians, as the walking camera keeps its heading), and where the player was last
    /// frame (to turn the body where they walk).
    body_yaw: f32,
    last_at: Option<[f64; 3]>,
    /// Where the player looks: heading (radians, 0..2π, as `yaw_of` reads one) and pitch (radians, up positive).
    look_yaw: f32,
    look_pitch: f32,
    /// The steps' clock in first person, where no animation is drawn (seconds of the one that would be: `Gait`),
    /// and the animation the last frame's steps were looked for in and how far it had played (`person::Body::steps`);
    /// and whether their speed is kept to a person's (the capsule's velocity virtuals as known).
    step_clock: f32,
    last_steps: Option<(u8, f32)>,
    /// Walking (not standing) as the person is shown, and how long the player has gone otherwise (seconds).
    moving: bool,
    flip_for: f32,
    /// Whether the player was running the last frame they went on (`ran`: kept stopping).
    ran: bool,
    /// Space pressed, the jump waiting for its animation's take-off (`person::Body::taking_off`).
    jump_pending: bool,
    /// When the physics' jump left the ground (for the log), and how far down the eyes are crouching (metres).
    left_at: Option<Instant>,
    crouch_depth: f32,
    /// Crouching in first person, how far the body is moved back and to its left (metres): the crouched head goes
    /// ahead of the feet (and walking crouched, to the right), so the body goes the other way, the head under the
    /// eyes (owner 2026-11-06: walking crouched, the camera was inside the body, the arm across it).
    crouch_shift: [f32; 2],
    /// The steps of the animation walking (and whether running) last frame, and how much the eyes bob now (0..1):
    /// `bob`.
    last_gait: Option<(person::Gait, bool)>,
    bob: f32,
    paces: bool,
    /// The jump (the game has none, `INVESTIGACAO.md` §7.5): frames in a row the capsule hasn't moved up or down
    /// (on the ground), whether it's in the air after a jump, and whether the jump key was down last frame.
    grounded: u32,
    airborne: bool,
    /// How long the person has been carried off faster than anyone walks (seconds: run over, `RUN_OVER`), and
    /// whether this walk began after one (it lands by the truck at once, no getting out).
    pushed_for: f32,
    skip_descend: bool,
    /// In the air, whether it has been seen coming down yet: at the top of the jump the capsule is still for a
    /// frame or two, which isn't landing (owner's log 2026-11-06: "landed" 0.32 s after leaving, the top, and the
    /// animation sent to its feet down while still up: "all wrong").
    falling: bool,
    /// In a jump, the walking camera's height as it left the ground: the third-person camera stays there while in the
    /// air, so the person is seen going up (owner's video, 2026-11-06: the camera went up with her, so on screen she
    /// didn't; her legs drawn up in the air looked like a crouch on the ground, then the landing's, "two animations").
    jump_ground: Option<f64>,
    jump_was_down: bool,
    /// Climbing back into the cab (first person, the camera only), after the walk key at the door.
    climb: Option<climb::Climb>,
    /// Getting out of the cab (first person, the camera only), before walking: `climb::Climb::out`.
    descend: Option<climb::Climb>,
    /// The flashlight in the player's hand, while it's on.
    flashlight: Option<person::Person>,
    /// Filling the tank on foot (`refuel`): the pump's nozzle in the hand at the tank, where it stands, and the view
    /// as it was before (third person or not), to go back to.
    nozzle: Option<person::Person>,
    nozzle_at: Option<Placement>,
    nozzle_was_third: Option<bool>,
}

// SAFETY: game objects, used only on its main thread.
unsafe impl Send for Walking {}

struct State {
    game: Option<Option<Game>>,
    walking: Option<Walking>,
    /// F8 pressed with another camera on screen: the game's camera 0 key was given it, this many frames ago;
    /// walking starts once camera 0 is drawn.
    switching: Option<u32>,
    /// A camera key given to the game, to be let go of at the next frame.
    release: Option<CameraKey>,
    /// Since when F8 is held, while walking.
    held_since: Option<Instant>,
    /// The cargo's papers and coupling by hand (`freight`), once the game has run a frame with them on.
    freight: Option<freight::Freight>,
    /// Refuelling by hand (`refuel`), once the game has run a frame with it on.
    refuel: Option<refuel::Refuel>,
}

// SAFETY: game objects (the clerk's person), used only on its main thread.
unsafe impl Send for freight::Freight {}

static STATE: Mutex<State> = Mutex::new(State {
    game: None,
    walking: None,
    switching: None,
    release: None,
    held_since: None,
    freight: None,
    refuel: None,
});
/// On foot, held where they stand (signing the cargo's papers, coupling the trailer): the walking keys as let go.
static HELD_STILL: AtomicBool = AtomicBool::new(false);
/// On foot, down on a knee whatever the crouch key (coupling the trailer at the fifth wheel).
static KNEELING: AtomicBool = AtomicBool::new(false);
/// On foot, working with the hands at something low in front (coupling and uncoupling the trailer by hand): the
/// person's own `work` animation, where they have one (`person::Body`), else the crouch.
static WORKING: AtomicBool = AtomicBool::new(false);
static WAS_DOWN: AtomicBool = AtomicBool::new(false);
static WAS_SLOWER: AtomicBool = AtomicBool::new(false);
static WAS_FASTER: AtomicBool = AtomicBool::new(false);
static WAS_VIEW: AtomicBool = AtomicBool::new(false);
static WAS_HINTS: AtomicBool = AtomicBool::new(false);
/// Whether the game was paused (its menu open) at the last frame: the phone is put away as it opens.
static WAS_PAUSED: AtomicBool = AtomicBool::new(false);
static WAS_FLASHLIGHT: AtomicBool = AtomicBool::new(false);
/// The key changing first and third person while walking (the plan's V; the game's diff lock, kept from it while
/// walking), as a virtual key and a scan code.
const VIEW_KEY: (u16, u32) = (b'V' as u16, 0x2f);
/// Run over: carried faster than this (m/s; running is 2.6) for this long (seconds), on the ground.
const CARRIED_OVER: f64 = 5.0;
const CARRIED_FOR: f32 = 0.2;
/// Or carried faster than this (m/s) with no movement key held, for this long (seconds): a slow car pushing.
const DRIFTED_OVER: f64 = 1.5;
const DRIFTED_FOR: f32 = 0.5;
/// The person run over, waiting for the screen to be black to be put back by the truck.
static RUN_OVER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The third-person camera kept out of things (owner 2026-11-19: "when the camera comes near a thing, zoom in on the
/// person"): a ray through the game's physics from the chest (`pivot`) to where the camera would be (`full`), and the
/// camera that far along it, less `CAMERA_GAP`, at least `CAMERA_NEAREST` from the chest. In at once (never a frame
/// inside a thing), back out at `CAMERA_BACK`; starting near the chest, so getting out beside a thing it's never in
/// it. No rays (none in this game, or off after a close): out as before, at that pace.
fn pulled_in(walking: &mut Walking, pivot: [f64; 3], full: [f64; 3], dt: f32) -> [f64; 3] {
    let d = [full[0] - pivot[0], full[1] - pivot[1], full[2] - pivot[2]];
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if length < 0.1 {
        return full;
    }
    crate::ray::camera_ask(pivot, full);
    let nearest = (CAMERA_NEAREST / length).min(1.0);
    let allowed = match crate::ray::camera_answer() {
        Some(Some(hit)) => ((f64::from(hit) - CAMERA_GAP) / length).clamp(nearest, 1.0),
        _ => 1.0,
    };
    let reach = walking.camera_reach.max(nearest);
    walking.camera_reach = if allowed < reach { allowed } else { (reach + CAMERA_BACK * f64::from(dt) / length).min(allowed) };
    let r = walking.camera_reach;
    [pivot[0] + d[0] * r, pivot[1] + d[1] * r, pivot[2] + d[2] * r]
}

/// How far before a thing the camera stays (metres), how near the chest it may come, and how fast it goes back out
/// (metres a second).
const CAMERA_GAP: f64 = 0.3;
const CAMERA_NEAREST: f64 = 0.6;
const CAMERA_BACK: f64 = 3.0;

/// L on foot: the flashlight on or off (kept from the game too: its L is the truck's lights).
const FLASHLIGHT_KEY: (u16, u32) = (b'L' as u16, 0x26);
/// Whether the flashlight is on offer (L, the key bar). Off since 2026-11-05 (owner: "leave the flashlight for
/// now"): no light of the game's lit on a mover of the plugin's, a `light_source` nor a `flare_blink` (`INVESTIGACAO.md`
/// §7.11). Off, L goes to the game as in the cab.
const FLASHLIGHT_ON: bool = false;
/// The flashlight's mover (WestPlay's mod: `westplay_setup::flashlight::MOVER`), and where it's held from the
/// eyes (metres: to the right, down, forward). Even the game's own `mover.light_flare` showed no flare in the hand
/// (owner's test, 2026-11-05): the people's spawning (`person::show_thing`) seems not to make a model's hookups.
const FLASHLIGHT: &str = "mover.wp_torch";
const FLASHLIGHT_HELD: [f64; 3] = [0.22, -0.35, 0.15];
/// Whether the player walks now (any thread: the keyboard hooks ask).
static WALKING: AtomicBool = AtomicBool::new(false);
/// The mouse's speed while walking (percent of the game's): the app's setting (the companion's Personagem page);
/// `-` and `=` while walking take it down and up, sent to the app to keep. Set here (0: none) until the app's
/// is the same.
static SENSITIVITY_HERE: AtomicU32 = AtomicU32::new(0);
const SENSITIVITY_STEP: u32 = 10;
/// Frames to wait for camera 0 after giving the game its key.
const SWITCH_FRAMES: u32 = 90;

/// A key of the game's camera controls (the player's controls.sii: `camdbg` on 0, `cam1` on 1), as a virtual key
/// and a scan code: given to the game's window as the player's key presses are, so it changes cameras itself.
#[derive(Clone, Copy)]
struct CameraKey(u16, u32);
const CAMERA_0: CameraKey = CameraKey(b'0' as u16, 0x0b);
const CAMERA_1: CameraKey = CameraKey(b'1' as u16, 0x02);

impl CameraKey {
    fn send(self, down: bool) {
        let Some(window) = crate::d3d::game_window() else {
            log!("character: the game's window isn't known yet: press {} yourself", self.0 as u8 as char);
            return;
        };
        let (message, flags) = if down { (WM_KEYDOWN, 1) } else { (WM_KEYUP, 1 | 0xc000_0000) };
        let lparam = flags | (self.1 as isize) << 16;
        // SAFETY: a key message posted to the game's own window, as Windows posts the player's.
        let _ = unsafe { PostMessageW(Some(window), message, WPARAM(usize::from(self.0)), LPARAM(lparam)) };
    }
}

fn sensitivity() -> u32 {
    let (least, most, default) = CHARACTER_SENSITIVITY;
    let app = crate::bridge::get().map_or(0, |b| b.host.character_sensitivity.load(Ordering::Acquire));
    let here = SENSITIVITY_HERE.load(Ordering::Acquire);
    if here != 0 && here == app {
        SENSITIVITY_HERE.store(0, Ordering::Release);
    }
    match (here, app) {
        (0, 0) => default,
        (0, app) => app.clamp(least, most),
        (here, _) => here,
    }
}

/// `-` or `=` while walking: the mouse slower or faster, sent to the app to keep.
fn change_sensitivity(faster: bool) {
    let (least, most, _) = CHARACTER_SENSITIVITY;
    let now = sensitivity();
    let next = if faster { now + SENSITIVITY_STEP } else { now.saturating_sub(SENSITIVITY_STEP) }.clamp(least, most);
    SENSITIVITY_HERE.store(next, Ordering::Release);
    if let Some(bridge) = crate::bridge::get() {
        bridge.plugin.character_sensitivity.store(next, Ordering::Release);
    }
    log!("character: mouse at {next}% while walking");
}

/// The key getting out of the truck and back in, as the app's Personagem page has it (F8 if none).
fn walk_key() -> VIRTUAL_KEY {
    let chosen = crate::bridge::get().map_or(0, |b| b.host.character_key.load(Ordering::Acquire));
    if (1..=0xfe).contains(&chosen) { VIRTUAL_KEY(chosen as u16) } else { VK_F8 }
}

/// Who walks, as the app's Personagem page has it: a place in `westplay_shared::CHARACTER_PEOPLE`.
fn chosen_person() -> usize {
    let chosen = crate::bridge::get().map_or(0, |b| b.host.character_person.load(Ordering::Acquire));
    chosen.checked_sub(1).map_or(westplay_shared::CHARACTER_PERSON, |place| place as usize)
}

/// Whether the Modo Personagem is on (the app running, and its setting on).
fn enabled() -> bool {
    crate::bridge::host_alive()
        && crate::bridge::get().is_some_and(|b| b.host.character_enabled.load(Ordering::Acquire) == 1)
}

/// Whether the game is kept from `key` (a virtual key; the generic Shift and Ctrl stand for both sides): the
/// walking keys, while walking.
pub fn takes(key: u32) -> bool {
    // The cargo's papers' keys, in the cab too (`freight`).
    if freight::takes(key) || refuel::takes(key) {
        return true;
    }
    if !WALKING.load(Ordering::Acquire) {
        return false;
    }
    // Each side too: the game's window messages and polls come as the left key (`keybinds::sided`), and the
    // player's controls shift gears with them (gearup on the left Shift, geardown on the left Ctrl: owner
    // 2026-11-04, "it's changing the truck's gear").
    let generic =
        [VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_CONTROL, VK_LCONTROL, VK_RCONTROL].iter().any(|k| u32::from(k.0) == key);
    generic
        || CAMERA_KEYS.contains(&key)
        || CONTROL_KEYS
            .iter()
            .chain([&VIEW_KEY, &FLASHLIGHT_KEY].into_iter().filter(|&&k| FLASHLIGHT_ON || k != FLASHLIGHT_KEY))
            .any(|&control| u32::from(virtual_key(control)) == key)
}

/// The game's camera keys (`cam1`..`cam8`, `camcycle`, `camdbg`: 1 to 9 and 0), kept from it while walking: another
/// camera would end walking (owner 2026-11-04: "it went back to the truck"). As virtual keys and scan codes.
const CAMERA_KEYS: std::ops::RangeInclusive<u32> = 0x30..=0x39;
const CAMERA_SCANCODES: std::ops::RangeInclusive<u32> = 0x02..=0x0b;

/// A walking key's virtual key on the player's keyboard layout: the game binds keys by where they are (W, A, S, D
/// are Z, Q, S, D on an AZERTY keyboard), so it's asked from the key's scan code.
fn virtual_key((vk, scancode): (u16, u32)) -> u16 {
    // SAFETY: a plain query, no pointers.
    match unsafe { MapVirtualKeyW(scancode, MAPVK_VSC_TO_VK) } {
        0 => vk,
        mapped => mapped as u16,
    }
}

/// The same for DirectInput's keyboard (a scan code).
pub fn takes_scancode(scancode: u32) -> bool {
    freight::takes_scancode(scancode)
        || refuel::takes_scancode(scancode)
        || WALKING.load(Ordering::Acquire)
            && (CAMERA_SCANCODES.contains(&scancode)
                || CONTROL_KEYS
                    .iter()
                    .chain([&VIEW_KEY, &FLASHLIGHT_KEY].into_iter().filter(|&&k| FLASHLIGHT_ON || k != FLASHLIGHT_KEY))
                    .any(|&(_, code)| code == scancode)
                || RIGHT_MODIFIERS.contains(&scancode))
}

/// DirectInput's right Shift and right Ctrl: kept from the game while walking too, as their left ones are.
const RIGHT_MODIFIERS: [u32; 2] = [0x36, 0x9d];

/// Whether `key` is down, as it really is (the keyboard hooks leave the plugin's own calls alone).
fn really_down(key: VIRTUAL_KEY) -> bool {
    let mut down = false;
    // SAFETY: a plain query, no pointers.
    crate::hooks::guarded(|| down = unsafe { GetAsyncKeyState(i32::from(key.0)) } as u16 & 0x8000 != 0);
    down
}

/// Once a frame, at its end, on the game's main thread: the keys, and the walking camera's step.
pub fn on_frame_end() {
    person::on_frame();
    let shown = crate::d3d::game_shown();
    let f8_down = really_down(walk_key()) && shown;
    let pressed = crate::input::newly_down(&WAS_DOWN, f8_down);
    let slower = crate::input::newly_down(&WAS_SLOWER, really_down(VK_OEM_MINUS)) && shown;
    let faster = crate::input::newly_down(&WAS_FASTER, really_down(VK_OEM_PLUS)) && shown;
    let view_pressed = crate::input::newly_down(&WAS_VIEW, really_down(VIRTUAL_KEY(virtual_key(VIEW_KEY)))) && shown;
    let flashlight_pressed =
        crate::input::newly_down(&WAS_FLASHLIGHT, really_down(VIRTUAL_KEY(virtual_key(FLASHLIGHT_KEY))))
            && shown
            && FLASHLIGHT_ON;
    let hints_key = crate::bridge::get().map_or(0, |b| b.host.character_hints_key.load(Ordering::Acquire));
    let hints_down = hints_key != 0 && really_down(VIRTUAL_KEY(hints_key as u16)) && shown;
    if crate::input::newly_down(&WAS_HINTS, hints_down) {
        crate::hints::toggle();
    }
    // The game's menu (Esc) or pause: the phone put away as it opens (owner 2026-11-07: it stayed over the menu,
    // stuck there). Once, as it opens: the phone taken out again over the menu stays.
    let paused = crate::telemetry::paused();
    if paused && !WAS_PAUSED.swap(true, Ordering::AcqRel) {
        crate::bridge::put_phone_away("the game's menu");
    } else if !paused {
        WAS_PAUSED.store(false, Ordering::Release);
    }
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let state = &mut *state;
    // The cargo's papers and coupling by hand: in the cab and on foot.
    // SAFETY: on the game's main thread, at the end of a frame.
    unsafe { freight::frame(state) };
    // Refuelling by hand: the pump's key in the cab, the tank on foot.
    // SAFETY: on the game's main thread, at the end of a frame.
    unsafe { refuel::frame(state) };
    publish_hints(state);
    if let Some(bridge) = crate::bridge::get() {
        bridge.plugin.on_foot.store(u32::from(state.walking.is_some()), Ordering::Release);
    }
    if let Some(key) = state.release.take() {
        key.send(false);
    }
    if let Some(walking) = state.walking.as_mut()
        && walking.climb.is_some()
    {
        // Climbing in: the walk key again sits down at once; the truck moving ends it there too.
        let done = if pressed {
            Some("the walk key while climbing in")
        } else if crate::telemetry::truck().speed.abs() > MOVING {
            Some("the truck moved")
        } else if crate::telemetry::paused() {
            None
        } else {
            // SAFETY: on the game's main thread, with camera 0 `start` found.
            let frame = unsafe { climb_frame(walking, false) };
            // The camera in the person's eyes: the body hidden now (`climb::Climb::from_behind`).
            if std::mem::take(&mut walking.hide_body)
                && let Some(Some(game)) = state.game.as_ref()
            {
                // SAFETY: on the game's main thread, `walking` as `start` made it.
                unsafe { set_third_person(game, walking, false, false) };
            }
            match frame {
                Ok(true) => Some("climbed in"),
                Ok(false) => None,
                Err(why) => Some(why),
            }
        };
        if let Some(why) = done {
            // Sat down, the door pulled shut (`steps::door`).
            steps::door(steps::Door::Shut, 0.0);
            stop(state, why, true);
        }
        return;
    }
    if let Some(walking) = state.walking.as_mut()
        && walking.descend.is_some()
    {
        // Getting out: the walk key again lands at once on the ground (not back in: it was the getting out).
        if crate::telemetry::paused() && !pressed {
            return;
        }
        // SAFETY: on the game's main thread, with camera 0 `start` found.
        let done = pressed || walking.skip_descend || !matches!(unsafe { climb_frame(walking, true) }, Ok(false));
        if done {
            // Walking on from where the way ended, looking as it did.
            if let Some((_, yaw, pitch)) = walking.descend.take().and_then(|out| out.end()) {
                walking.look_yaw = yaw;
                walking.look_pitch = pitch;
            }
            walking.last = Instant::now();
            log!("character: got out");
            // On the ground, the door shut behind them, a moment after (turning to it).
            steps::door(steps::Door::Shut, DOOR_SHUT_AFTER);
            // On the ground: the body under the eyes (first person, headless), as the walk begins.
            if let Some(Some(game)) = state.game.as_ref() {
                // In the view last chosen with V (owner 2026-11-06: "it remembers": got out in first person, changed
                // to third, back in; the next time out, third once on the ground). The way out is always first person.
                let third = THIRD_PERSON_LAST.load(Ordering::Relaxed);
                // SAFETY: on the game's main thread, with the walking camera `start` made.
                unsafe { set_third_person(game, walking, third, true) };
                // In third person, the person with their back to the door (looking out, as the way out ended) and
                // the camera beside them, along the truck toward its front: behind them it was in the truck's side,
                // the door filling the view (owner 2026-11-06, two pictures: "the truck shows, not the person").
                if third {
                    let turn = f64::from(crate::telemetry::truck().heading) * std::f64::consts::TAU;
                    walking.body_yaw = walking.look_yaw;
                    walking.look_yaw = heading_of(-turn.sin(), -turn.cos());
                    walking.look_pitch = START_PITCH;
                }
            }
        }
        return;
    }
    // Run over: once the screen is black, the walk begun again by the truck's door, on the ground at once.
    if RUN_OVER.load(Ordering::Acquire) && (crate::fade::covered() || state.walking.is_none()) {
        RUN_OVER.store(false, Ordering::Release);
        if state.walking.is_some() {
            stop(state, "run over", false);
            match try_start(state) {
                Ok(()) => {
                    if let Some(walking) = state.walking.as_mut() {
                        walking.skip_descend = true;
                    }
                }
                Err(why) => log!("character: not put back by the truck: {why}"),
            }
        }
        return;
    }
    if state.walking.is_some() {
        if slower || faster {
            change_sensitivity(faster);
        }
        // Back in the cab by the driver's door (F8 there), as one would climb in; F8 held a while gets back from
        // anywhere (lost, or stuck somewhere).
        let truck = crate::telemetry::truck();
        let far = state.walking.as_ref().and_then(|walking| from_door(walking, &truck)).filter(|&d| d > DOOR_REACH);
        // (Held from a press while walking: not the one that started walking.)
        if pressed {
            state.held_since = Some(Instant::now());
        }
        if !f8_down {
            state.held_since = None;
        } else if state.held_since.is_some_and(|since| since.elapsed() >= HOLD_TO_LEAVE) {
            state.held_since = None;
            stop(state, "F8 held", true);
            return;
        }
        if flashlight_pressed
            && let (Some(Some(game)), Some(walking)) = (&state.game, &mut state.walking)
            && walking.climb.is_none()
        {
            // SAFETY: on the game's main thread, with the walking camera `start` made and holds.
            unsafe { switch_flashlight(game, walking) };
        }
        if view_pressed
            && let (Some(Some(game)), Some(walking)) = (&state.game, &mut state.walking)
            && walking.nozzle_was_third.is_none()
        {
            let third = !walking.third_person;
            THIRD_PERSON_LAST.store(third, Ordering::Relaxed);
            // SAFETY: on the game's main thread, with the walking camera `start` made and holds.
            unsafe { set_third_person(game, walking, third, true) };
        }
        if pressed && let Some(far) = far {
            log!("character: {far:.1} m from the driver's door: walk up to it to get in (or hold F8)");
            tell(state, notice::Why::FarFromDoor);
        } else if pressed {
            // SAFETY: on the game's main thread, with the objects `start` made.
            if !unsafe { begin_climb(state, &truck) } {
                stop(state, "F8 at the door", true);
            }
        } else if crate::telemetry::truck().speed.abs() > MOVING {
            stop(state, "the truck moved", true);
        } else if !crate::telemetry::paused() {
            // SAFETY: on the game's main thread, with the objects `start` made and checked.
            if let Err(why) = unsafe { step(state) } {
                stop(state, why, false);
            }
        }
    } else if let Some(frames) = state.switching {
        // Waiting for camera 0 (F8 again gives up); not while the virtual mirrors are turned off (that wait is short,
        // and F8 pressed again as it seemed nothing happened gave it up: owner 2026-11-14, pressed three or four times).
        if pressed && WAITING_MIRRORS.load(Ordering::Relaxed) {
            log!("character: F8 again while the virtual mirrors are turned off: still walking once they are");
        } else if pressed {
            state.switching = None;
            log!("character: not walking: F8 again");
        } else if frames >= 2 {
            match try_start(state) {
                Ok(()) => {
                    state.switching = None;
                    WAITING_MIRRORS.store(false, Ordering::Relaxed);
                }
                // The mirrors off, camera 0 next, its wait afresh (it had used the mirrors' and gave up: owner
                // 2026-11-14, F8 pressed twice to get out).
                Err(NOT_ON_SCREEN) if WAITING_MIRRORS.swap(false, Ordering::Relaxed) => {
                    CAMERA_0.send(true);
                    state.release = Some(CAMERA_0);
                    state.switching = Some(0);
                }
                Err(NOT_ON_SCREEN) if frames < SWITCH_FRAMES => state.switching = Some(frames + 1),
                Err(MIRRORS_ON) if frames < MIRRORS_FRAMES => state.switching = Some(frames + 1),
                Err(why) => {
                    state.switching = None;
                    WAITING_MIRRORS.store(false, Ordering::Relaxed);
                    not_walking(state, why);
                }
            }
        } else {
            state.switching = Some(frames + 1);
        }
    } else if pressed {
        match try_start(state) {
            Ok(()) => {}
            // Camera 0 first, as the player would press it.
            Err(NOT_ON_SCREEN) => {
                CAMERA_0.send(true);
                state.release = Some(CAMERA_0);
                state.switching = Some(0);
            }
            Err(MIRRORS_ON) => {
                log!("character: walking once the virtual mirrors are off");
                WAITING_MIRRORS.store(true, Ordering::Relaxed);
                state.switching = Some(0);
            }
            Err(why) => not_walking(state, why),
        }
    }
}

/// The game's language, two letters (its `config.cfg`): the notices' and the key hints'.
pub fn language() -> &'static str {
    notice::language()
}

/// What the key hints bar shows now (`hints`): getting out (the truck stopped on its parking brake, the mode on,
/// single player) and the screen's key in the cab; getting back in by the driver's door on foot; nothing while
/// climbing in; the phone's key always.
fn publish_hints(state: &State) {
    use crate::hints::{Action, Hint, key_name};
    let Some(bridge) = crate::bridge::get() else { return };
    let host = &bridge.host;
    let mut hints = Vec::new();
    let walk_vk = u32::from(walk_key().0);
    let walk = key_name(walk_vk);
    let truck = crate::telemetry::truck();
    let screen_key = match host.interaction_key.load(Ordering::Acquire) {
        0 => westplay_shared::DEFAULT_INTERACTION_KEY,
        key => key,
    };
    match &state.walking {
        Some(walking) if walking.climb.is_some() || walking.descend.is_some() => {
            crate::hints::set(hints);
            return;
        }
        Some(walking) => {
            if from_door(walking, &truck).is_some_and(|d| d <= DOOR_REACH) {
                hints.push(Hint::single(walk, walk_vk, Action::GetIn));
            }
            // By the clerk, at the fifth wheel (`freight`), or at a pump (`refuel`): E interacts.
            if let Some(action) = freight::offer(state).or_else(|| refuel::offer(state)) {
                let interact_vk = u32::from(virtual_key((b'E' as u16, 0x12)));
                hints.push(Hint::single(key_name(interact_vk), interact_vk, action));
            }
            if FLASHLIGHT_ON {
                let action = if walking.flashlight.is_some() { Action::FlashlightOff } else { Action::Flashlight };
                let fl_vk = u32::from(virtual_key(FLASHLIGHT_KEY));
                hints.push(Hint::single(key_name(fl_vk), fl_vk, action));
            }
            // The camera: first or third person.
            let view_vk = u32::from(virtual_key(VIEW_KEY));
            hints.push(Hint::single(key_name(view_vk), view_vk, Action::Camera));
            // Crouching (Ctrl).
            let crouch_vk = u32::from(virtual_key(CONTROL_KEYS[CROUCH_CONTROL]));
            hints.push(Hint::single(key_name(crouch_vk), crouch_vk, Action::Crouch));
        }
        None => {
            let can_get_out = enabled()
                && truck.speed.abs() <= STOPPED
                && truck.parking_brake
                && !truck.multiplayer
                && !crate::truckersmp::loaded();
            if can_get_out {
                hints.push(Hint::single(walk, walk_vk, Action::GetOut));
            }
            if host.screen_enabled.load(Ordering::Acquire) == 1 {
                hints.push(Hint::single(key_name(screen_key), screen_key, Action::Screen));
            }
        }
    }
    // The keybinds' own (the app's Controls): the phone, its Personagem app with the mode on, hiding this HUD.
    let keybind = |keybind: westplay_shared::Keybind, action: Action| -> Option<Hint> {
        let value = host.keybinds[keybind as usize].load(Ordering::Acquire);
        westplay_shared::keybind_of(value).map(|(key, with_screen_key)| {
            if with_screen_key {
                Hint::combo(format!("{} + {}", key_name(screen_key), key_name(key)), screen_key, key, action)
            } else {
                Hint::single(key_name(key), key, action)
            }
        })
    };
    if let Some(hint) = keybind(westplay_shared::Keybind::Phone, Action::Phone) {
        hints.push(hint);
    }
    if enabled()
        && let Some(hint) = keybind(westplay_shared::Keybind::Character, Action::Character)
    {
        hints.push(hint);
    }
    if let Some(hint) = keybind(westplay_shared::Keybind::Hints, Action::HideBar) {
        hints.push(hint);
    }
    crate::hints::set(hints);
}

/// When the plugin is unloaded (on the game's main thread): walking stops.
pub fn shutdown() {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    freight::shutdown(&mut state);
    refuel::shutdown(&mut state);
    if state.walking.is_some() {
        stop(&mut state, "the plugin is unloaded", false);
    }
}

/// Why walking didn't start when only camera 0 is missing on screen (F8 then gives the game its key).
const NOT_ON_SCREEN: &str = "camera 0 isn't on screen";

/// Why walking didn't start, where a notice tells the player (`notice_for`).
const APP_CLOSED: &str = "the WestPlay app isn't open (the Modo Personagem needs it)";
const TRUCKERSMP: &str = "TruckersMP: the test is single player only";
const MULTIPLAYER: &str = "a multiplayer session (convoy): the test is single player only";
const NOT_STOPPED: &str = "the truck must be stopped";
const NO_PARKING_BRAKE: &str = "the truck must be on its parking brake";
/// Why walking waits: the game's virtual mirrors being turned off first (owner 2026-11-14: getting out as they were
/// being turned off, one stayed on, shown in the walking mode as the game's test picture; walking, the game takes no
/// keys, F2 included). Tried again each frame, a while at most.
const MIRRORS_ON: &str = "the game's virtual mirrors are being turned off first";
const MIRRORS_FRAMES: u32 = 600;
/// Whether walking is waiting for the virtual mirrors to be off.
static WAITING_MIRRORS: AtomicBool = AtomicBool::new(false);

/// The notice for why walking didn't start, if the player is to be told (the mode off says nothing: the key
/// may be the player's for something else).
fn notice_for(why: &str) -> Option<notice::Why> {
    match why {
        APP_CLOSED => Some(notice::Why::AppClosed),
        TRUCKERSMP | MULTIPLAYER => Some(notice::Why::Multiplayer),
        NOT_STOPPED => Some(notice::Why::Moving),
        NO_PARKING_BRAKE => Some(notice::Why::ParkingBrake),
        _ => None,
    }
}

/// Why walking didn't start: in the log, and in the game's corner where the player is to be told.
fn not_walking(state: &State, why: &str) {
    log!("character: not walking: {why}");
    if let Some(notice) = notice_for(why) {
        tell(state, notice);
    }
}

/// Tells the player `why` in the game's corner (the game's notices found), and in the log.
fn tell(state: &State, why: notice::Why) {
    let language = notice::language();
    if let Some(Some(game)) = &state.game
        && let Some(notices) = game.notices.as_ref()
    {
        log!("character: notice {why:?} ({language})");
        // SAFETY: on the game's main thread, at the end of a frame (`on_frame_end`).
        unsafe { notices.show(why.text(language), why.icon()) };
    }
}

fn try_start(state: &mut State) -> Result<(), &'static str> {
    // The game's code first (on the game's main thread, the end of a frame): its notices tell why not.
    // SAFETY: the code is the game's, read as it lies.
    state.game.get_or_insert_with(|| unsafe { locate() });
    if !crate::bridge::host_alive() {
        return Err(APP_CLOSED);
    }
    if !enabled() {
        return Err("the Modo Personagem is off (the app's Personagem page)");
    }
    if crate::truckersmp::loaded() || under_truckersmp() {
        return Err(TRUCKERSMP);
    }
    let truck = crate::telemetry::truck();
    if truck.multiplayer {
        return Err(MULTIPLAYER);
    }
    if crate::telemetry::paused() {
        return Err("the game is paused");
    }
    if truck.speed.abs() > STOPPED || !truck.parking_brake {
        // What the SDK says, to tell which (owner 2026-11-04: in ATS, F8 did nothing).
        log!(
            "character: the truck at {:.2} m/s, parking brake {}",
            truck.speed,
            if truck.parking_brake { "on" } else { "off" }
        );
        return Err(if truck.speed.abs() > STOPPED { NOT_STOPPED } else { NO_PARKING_BRAKE });
    }
    if crate::virtual_toggle::mirrors_on() {
        crate::virtual_toggle::turn_off_first();
        return Err(MIRRORS_ON);
    }
    let Some(Some(game)) = state.game.as_ref() else { return Err("the game's code isn't the version WestPlay knows") };
    // SAFETY: as above: on the game's main thread, with what `locate` found.
    let walking = unsafe { start(game, &truck) }?;
    state.walking = Some(walking);
    WALKING.store(true, Ordering::Release);
    log!("character: mouse at {}% while walking (- and = change it)", sensitivity());
    Ok(())
}

/// Outside the driver's door, at a person's eye height over the truck's ground (world metres): the head's side of
/// the cab (left in most trucks, right in right-hand-drive ones), a little ahead of the head (where the step is).
fn door(truck: &crate::telemetry::Truck) -> [f64; 3] {
    let [hx, _, hz] = truck.head;
    let side = if hx > 0.0 { 1.0 } else { -1.0 };
    in_truck(truck, [f64::from(hx + side * DOOR_OUT), EYE_HEIGHT, f64::from(hz - DOOR_AHEAD)])
}

/// Where camera 0 is left once back in the cab: out of the driver's side, `PARKED_OUT` from the head, at a
/// person's eye height (world metres), so it can't be taken for the cab's camera (`stop`).
fn parked_camera(truck: &crate::telemetry::Truck) -> [f64; 3] {
    let [hx, _, hz] = truck.head;
    let side = if hx > 0.0 { 1.0 } else { -1.0 };
    in_truck(truck, [f64::from(hx) + side * PARKED_OUT, EYE_HEIGHT, f64::from(hz)])
}
const PARKED_OUT: f64 = 3.0;

/// A place in the truck's own space (metres: x to its right, y up, z backwards) in the world.
fn in_truck(truck: &crate::telemetry::Truck, [lx, ly, lz]: [f64; 3]) -> [f64; 3] {
    let turn = f64::from(truck.heading) * std::f64::consts::TAU;
    let [x, y, z] = truck.position;
    [x + lx * turn.cos() + lz * turn.sin(), y + ly, z - lx * turn.sin() + lz * turn.cos()]
}

/// How far the player stands from the driver's door, along the ground (metres), if the walking camera is readable.
fn from_door(walking: &Walking, truck: &crate::telemetry::Truck) -> Option<f64> {
    // SAFETY: the walking camera is held (one reference); read guarded.
    let at = unsafe { read::<Placement>(walking.camera as usize + PLACEMENT) }?.world();
    let door = door(truck);
    Some((at[0] - door[0]).hypot(at[2] - door[2]))
}

/// Makes the walking camera at the driver's door and gives it the input.
///
/// # Safety
/// On the game's main thread, with `game` as `locate` found it.
unsafe fn start(game: &Game, truck: &crate::telemetry::Truck) -> Result<Walking, &'static str> {
    // SAFETY: each address is read guarded (`read`); the game's functions are called as its own code calls
    // them (the configuration scene's sequence), on its main thread.
    unsafe {
        let core = read::<usize>(game.core).filter(|&c| c != 0).ok_or("no core object")?;
        let debug = debug_camera(&game.image, core)?;
        // Camera 0 must be the one on screen: it's what the game draws from.
        let drawn = crate::camera::position().ok_or("the drawn camera isn't known")?;
        let at = read::<Placement>(debug + PLACEMENT).ok_or("camera 0 unreadable")?.world();
        let off = (0..3).map(|i| (at[i] - f64::from(drawn[i])).abs()).fold(0.0, f64::max);
        if off > 1.0 {
            return Err(NOT_ON_SCREEN);
        }
        let world = read::<usize>(core + game.world).filter(|&w| w != 0).ok_or("no road physics world")?;

        let mode = read::<u32>(game.allocator_mode).ok_or("the factory's mode unreadable")?;
        if mode == 4 {
            return Err("the factory's mode is 4 (the scene gives up there too)");
        }
        let allocator = read::<usize>(game.allocator_table + mode as usize * 8)
            .filter(|&a| a != 0)
            .ok_or("no allocator for the factory")?;
        let camera = (game.factory)(allocator as *mut u8);
        if camera.is_null() {
            return Err("the game's factory made no walking camera");
        }
        if class_name(&game.image, camera as usize).as_deref() != Some("walking_camera") {
            // Not what the scene gets: left alone (a leak, no worse).
            return Err("the factory's object isn't a walking_camera");
        }
        let vtable = read::<usize>(camera as usize).ok_or("no vtable")?;
        let activate = read::<usize>(vtable + ACTIVATE_SLOT * 8).ok_or("no activate")?;
        let input = input_of(&game.image, activate).ok_or("the walking camera's activate isn't as known")?;
        let saved = Input {
            context_was: read::<u64>(input.0 + input.1).ok_or("input unreadable")?,
            flag_was: read::<u8>(input.0 + input.2).ok_or("input unreadable")?,
            input: input.0,
            context: input.1,
            flag: input.2,
            apply: input.3,
        };

        // Held as the scene's smart pointer holds it, through the game's own assignment.
        let mut held: *mut u8 = std::ptr::null_mut();
        (game.assign)(&mut held, camera);
        (camera.add(WALKING_MODE) as *mut u32).write(0);

        let door = door(truck);
        let placement = Placement::at(door, truck.heading);
        log!("character: walking camera {camera:p} at {door:.2?} (head {:?}), world {world:#x}", truck.head);
        (game.start)(camera, &placement, world as *mut u8);
        log!("character: started");
        std::mem::transmute::<usize, Activate>(activate)(camera);
        let body = match game.physics.as_ref() {
            Some(physics) => match no_vehicle_layer(physics, camera) {
                Ok(body) => {
                    // Thinner, so it passes where a person would (before the bumping shape, made as it is).
                    if let Err(why) = thin_capsule(physics, camera, body.0, body.1) {
                        log!("character: the capsule as the game's (not thinner): {why}");
                    }
                    Some(body)
                }
                Err(why) => {
                    log!("character: the capsule stays on the game's layer (it can damage the truck): {why}");
                    None
                }
            },
            None => {
                log!("character: the capsule stays on the game's layer (it can damage the truck): not found");
                None
            }
        };
        // The look starts where camera 0 looks (its angles: heading, pitch), the pitch a little down.
        let angles = read::<[f32; 2]>(debug + CAMERA_ANGLES).unwrap_or_default();
        let pitch = (angles[1] + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        let look = (angles[0].rem_euclid(std::f32::consts::TAU), pitch.clamp(-PITCH_DOWN, PITCH_UP).min(START_PITCH));
        LOOK_DX.store(0, Ordering::Release);
        LOOK_DY.store(0, Ordering::Release);
        let mut walking = Walking {
            camera: held,
            debug: debug as *mut u8,
            input: saved,
            last: Instant::now(),
            away: 0,
            body,
            bump: None,
            traffic: None,
            person: None,
            third_person: false,
            hide_body: false,
            body_yaw: 0.0,
            last_at: None,
            look_yaw: look.0,
            look_pitch: look.1,
            step_clock: 0.0,
            last_steps: None,
            moving: false,
            flip_for: 0.0,
            ran: false,
            jump_pending: false,
            left_at: None,
            crouch_depth: 0.0,
            crouch_shift: [0.0; 2],
            last_gait: None,
            bob: 0.0,
            paces: paces(&game.image, held),
            grounded: 0,
            airborne: false,
            falling: false,
            jump_ground: None,
            jump_was_down: true,
            meter: meter::Meter::default(),
            climb: None,
            descend: Some(climb::Climb::out(truck)),
            flashlight: None,
            nozzle: None,
            nozzle_at: None,
            nozzle_was_third: None,
            camera_reach: 0.0,
            pushed_for: 0.0,
            skip_descend: false,
        };
        // Bumping into the truck without damaging it, from the start (owner 2026-11-04: on when getting out, off
        // when getting back in, where `stop` lets the shape go).
        if let Some(physics) = game.physics.as_ref()
            && let Err(why) = bump_on(physics, &mut walking)
        {
            log!("character: walking through the truck (no damage): {why}");
        }
        // First person at first, as the driver gets out with their own eyes (owner 2026-11-05, after testing his
        // own character: "it doesn't make sense to get out in third person"); V changes, the key bar says so.
        set_third_person(game, &mut walking, false, false);
        // No phone getting out: the person has no animation holding it there.
        crate::bridge::put_phone_away("getting out of the truck");
        // The driver's door opens as they get up off the seat (`steps::door`).
        steps::door(steps::Door::Open, 0.0);
        log!("character: walking (F8 again to stop, V first/third person)");
        Ok(walking)
    }
}

/// Puts the walking camera's capsule on the layer that meets no vehicle (so it never damages one); its body and
/// physics system.
///
/// # Safety
/// On the game's main thread, `camera` a walking camera its start made, `physics` as `locate` found it.
unsafe fn no_vehicle_layer(physics: &Physics, camera: *mut u8) -> Result<(*mut u8, *mut u8), &'static str> {
    // SAFETY: reads guarded; the game's own set_layer, called as its start calls it.
    unsafe {
        let body = read::<usize>(camera as usize + physics.body).filter(|&b| b != 0).ok_or("no capsule body")?;
        let world = read::<usize>(camera as usize + physics.world).filter(|&w| w != 0).ok_or("no capsule world")?;
        let system = read::<usize>(world + physics.system).filter(|&s| s != 0).ok_or("no physics system")?;
        let field = read::<u32>(body + physics.layer_field).ok_or("the body's layer unreadable")?;
        if (field >> 6) & 0x3f != 0x12 {
            return Err("the capsule isn't on layer 18 (not as known)");
        }
        (physics.set_layer)(body as *mut u8, BODY_LAYER);
        log!("character: the capsule on layer {BODY_LAYER} (no vehicle: no damage)");
        Ok((body as *mut u8, system as *mut u8))
    }
}

/// The capsule's radius walking (metres; the game's is 0.6, 1.2 m wide: owner 2026-11-19, "he catches on a lot,
/// between the truck and a thing he gets stuck; make the cylinder thinner"): about a person's shoulders.
const THIN_RADIUS: f32 = 0.3;

/// The game's capsule swapped for a thinner one (`THIN_RADIUS`), as tall: its straight part longer by what the
/// radius loses at each end, so the feet and the head stay where they were (`feet_under` reads the same 1.05 m
/// from the middle to the bottom). The physics is PhysX's under the game's shapes: the sizes the shape keeps are
/// a copy, so a new shape is made (the game's own `create_shape`, in the same body, as `bump_on` makes its own),
/// the game's let go (as the camera's destructor does), and the new one put where the camera keeps its shape, for
/// the destructor to let go later.
///
/// # Safety
/// On the game's main thread, `camera` a walking camera its start made, `body`/`system` as `no_vehicle_layer`
/// returned them.
unsafe fn thin_capsule(physics: &Physics, camera: *mut u8, body: *mut u8, system: *mut u8) -> Result<(), &'static str> {
    // SAFETY: reads guarded; the game's own functions, called as its start and the camera's destructor call them.
    unsafe {
        let slot = camera.add(physics.shape).cast::<usize>();
        let old = slot.read_unaligned();
        if old == 0 {
            return Err("no capsule shape");
        }
        let straight = read::<f32>(old + 0x1c).ok_or("capsule unreadable")?;
        let radius = read::<f32>(old + 0x18).ok_or("capsule unreadable")?;
        if !(0.05..3.0).contains(&straight) || !(0.05..3.0).contains(&radius) {
            return Err("the capsule's size isn't as known");
        }
        if radius <= THIN_RADIUS {
            return Ok(());
        }
        let new_straight = straight + 2.0 * (radius - THIN_RADIUS);
        let kind = read::<i32>(system as usize + physics.kind).filter(|k| (0..8).contains(k)).ok_or("no kind")?;
        let margin = read::<f32>(physics.margins + kind as usize * 4).ok_or("no margin")?;
        let turn = f32::from_bits(SHAPE_TURN);
        let local = Placement { rotation: [turn, turn, 0.0, 0.0], ..Placement::default() };
        let thin = (physics.create_shape)(system, body, new_straight, THIN_RADIUS, margin, &local);
        if thin.is_null() {
            return Err("the game made no shape");
        }
        let shape_vtable = read::<usize>(thin as usize).ok_or("no shape vtable")?;
        let set: SetShapeLayer = std::mem::transmute(
            read::<usize>(shape_vtable + physics.shape_layer_slot).ok_or("no shape layer setter")?,
        );
        set(thin, BODY_LAYER);
        let system_vtable = read::<usize>(system as usize).ok_or("no system vtable")?;
        let destroy: DestroyShape = std::mem::transmute(
            read::<usize>(system_vtable + physics.destroy_shape_slot).ok_or("no shape destroyer")?,
        );
        destroy(system, old as *mut u8);
        slot.write_unaligned(thin as usize);
        log!(
            "character: the capsule thinner: radius {radius:.2} -> {THIN_RADIUS:.2} m, straight part {straight:.2} -> {new_straight:.2} m"
        );
        Ok(())
    }
}

/// A second shape, like the capsule's, in its body, on the layer that bumps into the truck without damaging it
/// (kept until walking ends, then let go as the game lets go of the capsule's own: `stop`).
///
/// # Safety
/// On the game's main thread, `walking` as `start` made it.
unsafe fn bump_on(physics: &Physics, walking: &mut Walking) -> Result<(), &'static str> {
    let Some((body, system)) = walking.body else { return Err("the capsule isn't on WestPlay's layer") };
    // SAFETY: reads guarded; the game's own functions, called as its start calls them (the shape made as the
    // capsule's is, in the same body); the layer field written as the game's attach writes it.
    unsafe {
        let shape = match walking.bump {
            Some(shape) => shape,
            None => {
                let capsule = read::<usize>(walking.camera as usize + physics.shape)
                    .filter(|&s| s != 0)
                    .ok_or("no capsule shape")?;
                let a = read::<f32>(capsule + 0x1c).ok_or("capsule unreadable")?;
                let b = read::<f32>(capsule + 0x18).ok_or("capsule unreadable")?;
                if !(0.05..3.0).contains(&a) || !(0.05..3.0).contains(&b) {
                    return Err("the capsule's size isn't as known");
                }
                let kind =
                    read::<i32>(system as usize + physics.kind).filter(|k| (0..8).contains(k)).ok_or("no kind")?;
                let margin = read::<f32>(physics.margins + kind as usize * 4).ok_or("no margin")?;
                let turn = f32::from_bits(SHAPE_TURN);
                let local = Placement { rotation: [turn, turn, 0.0, 0.0], ..Placement::default() };
                let shape = (physics.create_shape)(system, body, a, b, margin, &local);
                if shape.is_null() {
                    return Err("the game made no shape");
                }
                // And the one bumping into the traffic, as it.
                let traffic = (physics.create_shape)(system, body, a, b, margin, &local);
                if !traffic.is_null() {
                    walking.traffic = Some(traffic);
                }
                shape
            }
        };
        walking.bump = Some(shape);
        let vtable = read::<usize>(shape as usize).ok_or("no shape vtable")?;
        let set: SetShapeLayer =
            std::mem::transmute(read::<usize>(vtable + physics.shape_layer_slot).ok_or("no shape layer setter")?);
        set(shape, BUMP_LAYER);
        if let Some(traffic) = walking.traffic
            && let Some(vtable) = read::<usize>(traffic as usize)
            && let Some(setter) = read::<usize>(vtable + physics.shape_layer_slot)
        {
            std::mem::transmute::<usize, SetShapeLayer>(setter)(traffic, TRAFFIC_LAYER);
            log!("character: bumping into the traffic (a shape on layer {TRAFFIC_LAYER})");
        }
        let field = body.add(physics.layer_field).cast::<u32>();
        field.write_unaligned((field.read_unaligned() & !0xfc0) | MIXED_LAYERS << 6);
        log!("character: bumping on (a shape on layer {BUMP_LAYER}: hits the truck, no damage)");
        Ok(())
    }
}

/// A frame of walking: the walking camera's own update, then camera 0 put where it is.
///
/// # Safety
/// On the game's main thread, `state.walking` as `start` made it.
unsafe fn step(state: &mut State) -> Result<(), &'static str> {
    let Some(walking) = state.walking.as_mut() else { return Ok(()) };
    // SAFETY: the walking camera is held (one reference) and its vtable is the game's; camera 0 is the game's
    // own, read guarded before it's written.
    unsafe {
        let now = Instant::now();
        let dt = now.duration_since(walking.last).as_secs_f32().clamp(0.0, 0.1);
        walking.last = now;
        let vtable = read::<usize>(walking.camera as usize).ok_or("the walking camera is gone")?;
        let update: Update = std::mem::transmute(read::<usize>(vtable + UPDATE_SLOT * 8).ok_or("no update")?);
        let before = read::<Placement>(walking.debug as usize + PLACEMENT).ok_or("camera 0 is gone")?;
        // The look: the mouse is the plugin's while walking (`input` gives it here, none to the game), so the
        // view's turn and the third-person camera's place are worked out together, in the same frame (owner
        // 2026-11-05: turning the camera "shook": camera 0 turned by the mouse after the plugin had placed it).
        let rate = LOOK_RATE * sensitivity() as f32 / 100.0;
        let (dx, dy) = (LOOK_DX.swap(0, Ordering::AcqRel), LOOK_DY.swap(0, Ordering::AcqRel));
        walking.look_yaw = (walking.look_yaw - dx as f32 * rate).rem_euclid(std::f32::consts::TAU);
        walking.look_pitch = (walking.look_pitch - dy as f32 * rate).clamp(-PITCH_DOWN, PITCH_UP);
        let look = look_rotation(walking.look_yaw, walking.look_pitch);
        // The walking camera turned that way, so its steps go where the player looks.
        (walking.camera.add(YAW) as *mut f32).write_unaligned(walking.look_yaw);
        // Its controls from the walking keys (kept from the game, which leaves them up).
        for (i, &key) in CONTROL_KEYS.iter().enumerate() {
            let Some(control) = read::<usize>(walking.camera as usize + CONTROLS + i * 0x10).filter(|&c| c != 0) else {
                continue;
            };
            if read::<u32>(control + CONTROL_STATE).is_some() {
                // Held still (signing, coupling: `freight`): every walking key as let go.
                let held = !HELD_STILL.load(Ordering::Acquire) && really_down(VIRTUAL_KEY(virtual_key(key)));
                let value = if held { DOWN } else { 0 };
                ((control + CONTROL_STATE) as *mut u32).write(value);
            }
        }
        update(walking.camera, dt);
        // At a person's pace (the walking camera's are 4 m/s, 8 running: the walking person's feet would slide).
        let running = really_down(VIRTUAL_KEY(virtual_key(CONTROL_KEYS[RUN])));
        // No running with the phone out (no animation for it): it's put away as the person sets off running.
        if running && walking.moving && walking.paces {
            crate::bridge::put_phone_away("running");
        }
        // Crouching (Ctrl) is the plugin's: the walking camera's own crouch did nothing (owner 2026-11-05, "the
        // crouch doesn't work"): the eyes down (`crouch_depth`), slower, the person's crouch if it has one.
        let crouch_held =
            really_down(VIRTUAL_KEY(virtual_key(CONTROL_KEYS[CROUCH_CONTROL]))) || KNEELING.load(Ordering::Acquire);
        // As far down as the person's head goes: on the knee, or walking crouched (higher), a part of their eyes'
        // height (measured in Blender on the woman's: 1.42 m standing, 0.71 on the knee, 1.11 walking crouched).
        let eyes = person::eye_height(CHARACTER_PEOPLE.get(chosen_person()).copied().unwrap_or_default()) as f32;
        let walks_crouched = walking.person.as_ref().is_some_and(person::Body::walks_crouched);
        let target = match (crouch_held, walks_crouched) {
            (false, _) => 0.0,
            (true, false) => eyes * CROUCH_DROP,
            (true, true) => eyes * CROUCH_WALK_DROP,
        };
        let step = CROUCH_EASE * dt;
        walking.crouch_depth += (target - walking.crouch_depth).clamp(-step, step);
        let shift = match (crouch_held, walks_crouched) {
            // Running, the body leans: its head ahead of the feet as well (owner 2026-11-06: running, "inside the body").
            (false, _) if running && walking.moving => [eyes * RUN_HEAD_AHEAD, 0.0],
            (false, _) => [0.0, 0.0],
            (true, false) => [eyes * KNEE_HEAD_AHEAD, 0.0],
            (true, true) => [eyes * CROUCH_WALK_HEAD.0, eyes * CROUCH_WALK_HEAD.1],
        };
        for (now, to) in walking.crouch_shift.iter_mut().zip(shift) {
            *now += (to - *now).clamp(-CROUCH_SHIFT_EASE * dt, CROUCH_SHIFT_EASE * dt);
        }
        if walking.paces
            && let Some((body, _)) = walking.body
        {
            limit_speed(
                body,
                if crouch_held {
                    CROUCH_SPEED
                } else if running {
                    RUN_SPEED
                } else {
                    WALK_SPEED
                },
            );
        }
        // The walking animation played faster running, its steps keeping up with the ground (the game has no
        // running person: `INVESTIGACAO.md` §7.2); with the walking camera's own pace, as it always was. A person
        // with a running animation of its own (WestPlay's) runs with that one, as it is.
        let own_run = match walking.person.as_ref() {
            Some(body) => body.runs(),
            None => person::gaits(CHARACTER_PEOPLE.get(chosen_person()).copied().unwrap_or_default()).1.is_some(),
        };
        let pace = if running && walking.paces && !own_run { RUN_SPEED / WALK_SPEED } else { 1.0 };
        let jump_down = really_down(VIRTUAL_KEY(virtual_key(CONTROL_KEYS[JUMP])));
        if walking.paces
            && let Some((body, _)) = walking.body
            && let Some(mut velocity) = velocity_of(body)
        {
            walking.grounded = if velocity[1].abs() < STILL_UP_DOWN { walking.grounded + 1 } else { 0 };
            if walking.airborne && velocity[1] < -COMING_DOWN {
                walking.falling = true;
            }
            // Why Space didn't jump, in the log (owner 2026-11-05: "running, Space does nothing").
            // On the ground: still a few frames, or on the move not rising or falling fast (running, the capsule goes
            // up and down over the ground: 0.2 to 1.8 m/s, owner's log 2026-11-05, and Space never jumped).
            let on_ground = !walking.airborne
                && !walking.jump_pending
                && (walking.grounded >= GROUNDED_FRAMES || walking.moving && velocity[1].abs() < MOVING_UP_DOWN);
            if jump_down && !walking.jump_was_down && !on_ground {
                log!(
                    "character: no jump: not on the ground yet ({} of {GROUNDED_FRAMES} frames still, up/down {:.2} m/s, \
                     running {running}, in the air {})",
                    walking.grounded,
                    velocity[1],
                    walking.airborne
                );
            }
            let own_jump = walking.person.as_ref().is_some_and(|shown| shown.jumps(walking.moving));
            // Standing still, WestPlay's own people don't jump (`Body::stays_down`: the standard, owner 2026-11-06).
            let stays_down = walking.person.as_ref().is_some_and(|shown| shown.stays_down(walking.moving));
            if jump_down && !walking.jump_was_down && on_ground && stays_down {
                log!("character: no jump standing still (only on the move)");
            }
            if jump_down && !walking.jump_was_down && on_ground && !stays_down {
                log!("character: jump{}", if own_jump { " (its animation first)" } else { "" });
                // Its animation, if the person has one (the woman): the physics' jump as its feet leave the ground
                // (at once on the move, after the crouch standing).
                if own_jump && let Some(shown) = walking.person.as_mut() {
                    shown.jump(walking.moving);
                }
                walking.jump_pending = true;
            }
            // Off the ground: as its jump's animation comes to it, or at once without one (the man, no body).
            let leaves = walking.jump_pending
                && walking.person.as_ref().is_none_or(|shown| !shown.jump_playing() || shown.taking_off());
            if leaves {
                velocity[1] = JUMP_SPEED;
                set_velocity(body, &velocity);
                walking.grounded = 0;
                walking.airborne = true;
                walking.falling = false;
                walking.jump_pending = false;
                walking.left_at = Some(Instant::now());
                walking.meter.jumped();
            } else if walking.airborne
                && ((walking.falling && walking.grounded >= 2)
                    || walking.left_at.is_none_or(|left| left.elapsed().as_secs_f32() > LONGEST_IN_AIR))
            {
                // Landed: a step's sound (and how long it was in the air, for the log: its animation's is ~0.6 s).
                walking.airborne = false;
                if let Some(left) = walking.left_at.take() {
                    log!("character: landed after {:.2} s in the air", left.elapsed().as_secs_f32());
                }
                steps::play(running);
            }
        }
        // Space seen at all, for the log (with Shift held, running, it didn't jump: owner 2026-11-05).
        if jump_down && !walking.jump_was_down {
            log!("character: Space (running {running}, walking {}, paces {})", walking.moving, walking.paces);
        }
        walking.jump_was_down = jump_down;
        let mut placement = read::<Placement>(walking.camera as usize + PLACEMENT).ok_or("no placement")?;
        // In a jump, how high the walking camera (where the body is put from) goes, for the log (`meter`).
        let up = walking.body.filter(|_| walking.paces).and_then(|(body, _)| velocity_of(body)).map(|v| v[1]);
        walking.meter.height(dt, placement.world()[1], up);
        // Where it is, turned where the player looks.
        placement.rotation = look;
        // Camera 0 still drawn? (What was put there last frame, moved by at most its own step.)
        if let Some(drawn) = crate::camera::position() {
            let at = before.world();
            let off = (0..3).map(|i| (at[i] - f64::from(drawn[i])).abs()).fold(0.0, f64::max);
            walking.away = if off > 5.0 { walking.away + 1 } else { 0 };
            if walking.away > 30 {
                return Err("camera 0 isn't on screen any more");
            }
        }
        // In third person, camera 0 behind the player's head and the game's person on their feet.
        let under = feet_under(walking, state.game.as_ref().and_then(|g| g.as_ref()).and_then(|g| g.physics.as_ref()));
        // In first person, the eyes moving with the steps (`bob`: owner 2026-11-05, "movement walking, more real").
        // In a jump, the third-person camera at the height it left from (`jump_ground`), lower if the player falls
        // lower (off a step); back with them as they land.
        let [ex, ey, ez] = placement.world();
        let camera_y = if walking.airborne {
            ey.min(*walking.jump_ground.get_or_insert(ey))
        } else {
            walking.jump_ground = None;
            ey
        };
        // At the person's own eye height (the walking camera's eyes are `under` over the feet).
        let first_person_eyes = {
            let eyes = person::eye_height(CHARACTER_PEOPLE.get(chosen_person()).copied().unwrap_or_default());
            let down = under - eyes + f64::from(walking.crouch_depth);
            Placement { rotation: placement.rotation, ..Placement::at([ex, ey - down, ez], 0.0) }
        };
        let drawn_from = if walking.third_person {
            let held = Placement { rotation: placement.rotation, ..Placement::at([ex, camera_y, ez], 0.0) };
            let behind = behind(&held, under);
            let [x, y, z] = pulled_in(walking, [ex, camera_y - under + PIVOT_OVER_FEET, ez], behind.world(), dt);
            Placement {
                rotation: behind.rotation,
                ..Placement::at([x, y - f64::from(walking.crouch_depth) * 0.5, z], 0.0)
            }
        } else {
            if walking.camera_reach != 0.0 {
                walking.camera_reach = 0.0;
                crate::ray::camera_stop();
            }
            bob(walking, &first_person_eyes, dt)
        };
        (walking.debug.add(PLACEMENT) as *mut Placement).write_unaligned(drawn_from);
        // And camera 0's own angles (it works its turn out of them each frame: heading, then pitch, both
        // radians 0..2π), so it keeps the plugin's.
        (walking.debug.add(CAMERA_ANGLES) as *mut [f32; 2])
            .write_unaligned([walking.look_yaw, walking.look_pitch.rem_euclid(std::f32::consts::TAU)]);
        // Where the player went this frame (along the ground): the body turns that way, walks, and steps.
        let at = placement.world();
        let went = walking.last_at.map_or((0.0, 0.0), |before| (at[0] - before[0], at[2] - before[2]));
        walking.last_at = Some(at);
        let moved = went.0.hypot(went.1);
        let went_on = dt > 0.0 && moved / f64::from(dt) > TURNS_ABOVE;
        // Walking or standing, once it's been so a moment (`MOVING_AFTER`, `STILL_AFTER`): frame by frame it
        // flicked between them as the speed crossed `TURNS_ABOVE` stopping (and on frames the physics didn't
        // step), the person shown switching each time (owner 2026-11-05: "he blinks when I stop").
        if went_on == walking.moving {
            walking.flip_for = 0.0;
        } else {
            walking.flip_for += dt;
            if walking.flip_for >= if went_on { MOVING_AFTER } else { STILL_AFTER } {
                walking.moving = went_on;
                walking.flip_for = 0.0;
            }
        }
        let moving = walking.moving;
        // The capsule's speed frame by frame as a move key goes down or up, for the log (`meter`).
        let keys =
            (0..4).fold(0u8, |keys, i| keys | u8::from(really_down(VIRTUAL_KEY(virtual_key(CONTROL_KEYS[i])))) << i);
        let physics = walking
            .body
            .filter(|_| walking.paces)
            .and_then(|(body, _)| velocity_of(body))
            .map(|velocity| velocity[0].hypot(velocity[2]));
        walking.meter.motion(keys, dt, if dt > 0.0 { (moved / f64::from(dt)) as f32 } else { 0.0 }, physics, moving);
        // Run over (owner 2026-11-19: "in front of a car, it carries me along till I'm out of its way"): carried
        // faster than anyone runs, on the ground, a moment; the screen fades and the person is put back by the
        // truck (`RUN_OVER`, done where the walk can be begun again).
        // Also a slow car (owner 2026-11-19: "it goes on carrying me"): moving with no movement key held.
        let speed = if dt > 0.0 { moved / f64::from(dt) } else { 0.0 };
        let fast = speed > CARRIED_OVER;
        let drifting = keys == 0 && speed > DRIFTED_OVER;
        let carried = (fast || drifting) && !walking.airborne;
        walking.pushed_for = if carried { walking.pushed_for + dt } else { (walking.pushed_for - dt).max(0.0) };
        let long = if fast { CARRIED_FOR } else { DRIFTED_FOR };
        if walking.pushed_for >= long && !RUN_OVER.swap(true, Ordering::AcqRel) {
            log!("character: run over (carried at {:.1} m/s): back by the truck", moved / f64::from(dt));
            crate::fade::start();
        }
        // Stopping (gone no more, not yet standing), the last gait: Shift let go with the walk key, running went
        // to walking for that moment before standing, two switches (the owner's test, 2026-11-05).
        if went_on {
            walking.ran = running;
        }
        let running = if went_on { running } else { walking.ran };
        if went_on {
            // The body turns only as the player walks (owner 2026-11-05: "the camera turns 360° around the
            // character without moving it; I move it only with WASD"): toward where it went.
            walking.body_yaw = turn_toward(walking.body_yaw, heading_of(went.0, went.1), TURN_SPEED * dt);
        }
        // A step's sound as a foot of the animation drawn comes down (owner 2026-11-05: "the sound isn't in step
        // with the feet"); in first person, of the one that would be, its clock going only while moving, the first
        // step soon after setting off.
        let steps = match walking.person.as_ref().and_then(|body| body.steps(running)) {
            Some(steps) => steps,
            None => {
                let name = CHARACTER_PEOPLE.get(chosen_person()).copied().unwrap_or_default();
                let (walk, run) = person::gaits(name);
                let (which, gait) = match (running, run) {
                    (true, Some(run)) => (3, run),
                    _ => (2, walk),
                };
                if moving {
                    walking.step_clock += dt * pace;
                } else {
                    walking.step_clock = gait.steps[0] - FIRST_STEP_AFTER;
                }
                (which, walking.step_clock, gait)
            }
        };
        let (which, clock, gait) = steps;
        if moving
            && !walking.airborne
            && walking.last_steps.is_some_and(|(was, before)| was == which && gait.stepped(before, clock))
        {
            steps::play(running);
        }
        walking.last_steps = Some((which, clock));
        walking.last_gait = Some((gait, running));
        // In first person the headless body turns with the view and stands a little behind the eyes, so looking
        // down shows the chest, the arms and the legs, not the inside of the neck.
        let feet = if walking.third_person {
            on_feet(&placement, walking.body_yaw, under)
        } else {
            walking.body_yaw = walking.look_yaw;
            let [ahead, right] = walking.crouch_shift.map(f64::from);
            let back = ahead_of(walking.look_yaw, -FIRST_PERSON_BACK - ahead);
            // A heading of θ looks along (-sin θ, -cos θ); its right is (cos θ, -sin θ).
            let turn = f64::from(walking.look_yaw);
            let left = [-turn.cos() * right, turn.sin() * right];
            let [x, y, z] = placement.world();
            on_feet(&Placement::at([x + back[0] + left[0], y, z + back[1] + left[1]], 0.0), walking.body_yaw, under)
        };
        if let (Some(Some(game)), Some(shown)) = (&state.game, walking.person.as_mut())
            && let Some(people) = game.people.as_ref()
        {
            // What placing the people costs, for the log (`meter`).
            let started = std::time::Instant::now();
            person::place(
                people,
                shown,
                &feet,
                dt * pace,
                moving,
                running,
                walking.airborne,
                crouch_held,
                crate::bridge::phone_out(),
            );
            walking.meter.placed(started.elapsed(), shown.people());
        }
        if let (Some(Some(game)), Some(flashlight)) = (&state.game, walking.flashlight.as_mut())
            && let Some(people) = game.people.as_ref()
        {
            person::place_thing(people, flashlight, &in_hand(&placement, walking.look_yaw, walking.look_pitch));
        }
        nozzle_frame(state.game.as_ref().and_then(Option::as_ref), walking, &first_person_eyes);
    }
    Ok(())
}

/// Filling the tank on foot (`refuel::nozzle`; owner 2026-11-12: "only the arm holding the nozzle, in first
/// person"): as it begins, first person without the body, the view turned to the tank and down, and the pump's
/// nozzle (WestPlay's mod: `westplay_setup::nozzle`) in the hand by it, staying there as the player looks about; as
/// it ends, the nozzle gone and the view as it was.
///
/// # Safety
/// On the game's main thread, `walking` as `start` made it.
unsafe fn nozzle_frame(game: Option<&Game>, walking: &mut Walking, eyes: &Placement) {
    let people = game.and_then(|game| game.people.as_ref());
    // SAFETY: on the game's main thread, with what `person::locate` found.
    unsafe {
        match (refuel::nozzle(), walking.nozzle_was_third) {
            (Some((yaw, ahead)), None) => {
                walking.nozzle_was_third = Some(walking.third_person);
                if let Some(game) = game {
                    set_third_person(game, walking, false, false);
                }
                walking.look_yaw = yaw;
                walking.look_pitch = -NOZZLE_LOOK_DOWN;
                // The person's own body holding the nozzle (owner 2026-11-12: "a real arm, not a blocky one"), where
                // the first-person body stands: a little behind the eyes, facing the tank. Without it in the mod (a
                // person exported before), the plain nozzle with its own arm.
                let walk = CHARACTER_PEOPLE.get(chosen_person()).copied().unwrap_or_default();
                let holding = walk.strip_suffix("_walk").map(|prefix| format!("mover.{prefix}f_fuel"));
                let back = ahead_of(yaw, -FIRST_PERSON_BACK);
                let [x, y, z] = eyes.world();
                let feet = on_feet(&Placement::at([x + back[0], y, z + back[1]], 0.0), yaw, person::eye_height(walk));
                let shown = holding.as_deref().zip(people).and_then(|(name, people)| {
                    person::show_thing(people, &feet, name)
                        .inspect_err(|why| log!("character: no body holding the nozzle ({name}): {why}"))
                        .ok()
                });
                if let Some(body) = shown {
                    walking.nozzle = Some(body);
                    walking.nozzle_at = Some(feet);
                    log!("character: the body holding the nozzle ({})", holding.as_deref().unwrap_or_default());
                } else {
                    let at = nozzle_held(eyes, yaw, ahead);
                    walking.nozzle_at = Some(at);
                    match people.map(|people| person::show_thing(people, &at, NOZZLE)) {
                        Some(Ok(nozzle)) => {
                            walking.nozzle = Some(nozzle);
                            log!("character: the nozzle in the hand ({ahead:.2} m ahead)");
                        }
                        Some(Err(why)) => log!("character: no nozzle ({NOZZLE}, WestPlay's mod): {why}"),
                        None => {}
                    }
                }
            }
            (Some(_), Some(_)) => {
                if let (Some(people), Some(nozzle), Some(at)) = (people, walking.nozzle.as_mut(), walking.nozzle_at) {
                    person::place_thing(people, nozzle, &at);
                }
            }
            (None, Some(was_third)) => {
                walking.nozzle_was_third = None;
                walking.nozzle_at = None;
                if let (Some(people), Some(nozzle)) = (people, walking.nozzle.take()) {
                    person::hide_thing(people, nozzle);
                }
                if let Some(game) = game {
                    set_third_person(game, walking, was_third, true);
                }
                log!("character: the nozzle put back");
            }
            (None, None) => {}
        }
    }
}

/// The pump's nozzle's mover, how far down the view looks filling (radians), and where the nozzle's grip is held
/// from the eyes: to the right, down (metres); ahead, the tank's say (`refuel::nozzle`). Its spout points a little
/// down, into the tank.
const NOZZLE: &str = "mover.wp_nozzle";
const NOZZLE_LOOK_DOWN: f32 = 0.55;
const NOZZLE_HELD: [f64; 2] = [0.12, -0.42];
const NOZZLE_PITCH: f32 = -0.12;

/// The nozzle by the tank: `ahead` metres ahead of the eyes toward it (heading `yaw`), in the right hand.
fn nozzle_held(eyes: &Placement, yaw: f32, ahead: f64) -> Placement {
    let [x, y, z] = eyes.world();
    let ([right, down], turn) = (NOZZLE_HELD, f64::from(yaw));
    // A heading of θ looks along (-sin θ, -cos θ); its right is (cos θ, -sin θ).
    let (forward, side) = ([-turn.sin(), -turn.cos()], [turn.cos(), -turn.sin()]);
    let at = [x + side[0] * right + forward[0] * ahead, y + down, z + side[1] * right + forward[1] * ahead];
    Placement { rotation: look_rotation(yaw, NOZZLE_PITCH), ..Placement::at(at, 0.0) }
}

/// A person's pace on foot, walking and running (m/s). The game has no running person (`INVESTIGACAO.md` §7.2):
/// running is the walking animation played `RUN_SPEED / WALK_SPEED` times as fast, at a brisk pace rather than a
/// sprint (owner 2026-11-05: at 3.6 m/s with the animation as it was, the feet slid: "the walk sped up"); WestPlay's
/// own person has a running one, so running is a bit faster since (3.2, was 2.6: owner 2026-11-05, "too held back
/// running"). The steps are heard where the animation's feet come down (`person::Gait`).
const WALK_SPEED: f32 = 1.6;
const RUN_SPEED: f32 = 3.2;
/// In first person, how soon after setting off the first step is heard (seconds).
const FIRST_STEP_AFTER: f32 = 0.15;
/// The view last chosen with V on foot (third person or first), for the next time out of the truck; first person
/// until V is first pressed (while the game runs).
static THIRD_PERSON_LAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Out on the ground, how long before the door is heard shut behind (seconds).
const DOOR_SHUT_AFTER: f32 = 0.35;
/// The run control's place among the walking camera's (`CONTROL_KEYS`).
const RUN: usize = 4;
/// The crouch control's place (`CONTROL_KEYS`: left Ctrl), how far down the eyes go on the knee (a part of their
/// height: the woman's head 0.71 m down of 1.42), how fast they go down and up (metres a second).
const CROUCH_CONTROL: usize = 6;
const CROUCH_DROP: f32 = 0.5;
/// Walking crouched, how far down the eyes go (a part of their height), and the speed: the "Crouched Walking"'s feet
/// go at 1.24 m/s, and the body goes about 0.57 m/s faster than the limit set (owner's measurements, 2026-11-06:
/// 1.6 set, 2.15 gone; 3.2, 3.78), so 0.7 (it was 0.9, the crouch's pose sliding along).
const CROUCH_WALK_DROP: f32 = 0.22;
const CROUCH_EASE: f32 = 1.2;
/// How far ahead of the feet the head goes crouching (a part of the eyes' height, measured in Blender 2026-11-06):
/// on the knee 0.31 m of the woman's 1.56 and 0.39 of the man's 1.64; walking crouched ahead 0.30 / 0.34 and to
/// the right 0.21 / 0.23. How fast the body moves for it (metres a second: the knee's 0.67 s down).
const KNEE_HEAD_AHEAD: f32 = 0.22;
const CROUCH_WALK_HEAD: (f32, f32) = (0.2, 0.14);
/// Running, measured the same way: the head 0.15 m ahead of the woman's feet (eyes 1.56), 0.18 of the tattooed
/// man's and 0.2 of the cowboy's (1.64).
const RUN_HEAD_AHEAD: f32 = 0.115;
const CROUCH_SHIFT_EASE: f32 = 0.8;
const CROUCH_SPEED: f32 = 0.7;
/// On the move, how fast the capsule may go up or down and still jump (m/s).
const MOVING_UP_DOWN: f32 = 2.0;
/// The jump's: Space. The jump starts at this speed up (m/s: about 0.45 m high under the game's gravity), only
/// after the capsule has stayed this many frames without going up or down faster than `STILL_UP_DOWN` (m/s).
const JUMP: usize = 5;
const JUMP_SPEED: f32 = 3.0;
const GROUNDED_FRAMES: u32 = 3;
/// In the air, how fast down the capsule goes before it counts as coming down (m/s), and the longest it is taken
/// to be in the air at all (seconds: off a ledge it may be longer, but never stuck "in the air").
const COMING_DOWN: f32 = 0.5;
const LONGEST_IN_AIR: f32 = 3.0;
const STILL_UP_DOWN: f32 = 0.15;
/// The capsule body's virtuals reading and setting its velocity (x, y, z m/s): the walking camera's own update
/// reads it and sets it back with its speed kept (`call [rax+138h]`, `call [rax+240h]`).
const GET_VELOCITY: usize = 0x138;
const SET_VELOCITY: usize = 0x240;

type GetVelocity = unsafe extern "system" fn(*mut u8, *mut [f32; 4]) -> *mut [f32; 4];
type SetVelocity = unsafe extern "system" fn(*mut u8, *const [f32; 4]);

/// Whether the walking camera's update uses the capsule's velocity as known (both virtuals called in it).
fn paces(image: &Image, camera: *mut u8) -> bool {
    // SAFETY: the camera's vtable read guarded; its update within the game's image.
    let update = unsafe { read::<usize>(camera as usize).and_then(|vt| read::<usize>(vt + UPDATE_SLOT * 8)) };
    let Some(code) = update.and_then(|u| image.code_at(u, 0x300)) else { return false };
    // `call [rax+slot]`: FF 90 and the slot (four bytes).
    let calls = |slot: usize| {
        let b = (slot as u32).to_le_bytes();
        code.windows(6).any(|w| w == [0xff, 0x90, b[0], b[1], b[2], b[3]])
    };
    let known = calls(GET_VELOCITY) && calls(SET_VELOCITY);
    if !known {
        log!("character: the capsule's velocity isn't as known: walking at the walking camera's own pace");
    }
    known
}

/// The capsule's velocity along the ground kept under `most` (m/s); its fall left as it is.
///
/// # Safety
/// On the game's main thread, `body` the walking camera's capsule (its virtuals checked by `paces`).
unsafe fn limit_speed(body: *mut u8, most: f32) {
    // SAFETY: as `velocity_of` and `set_velocity` (the caller's contract).
    unsafe {
        let Some(mut velocity) = velocity_of(body) else { return };
        let along = velocity[0].hypot(velocity[2]);
        if along > most && along.is_finite() {
            let keep = most / along;
            velocity[0] *= keep;
            velocity[2] *= keep;
            set_velocity(body, &velocity);
        }
    }
}

/// The capsule's velocity (x, y, z m/s; the fourth unused).
///
/// # Safety
/// On the game's main thread, `body` the walking camera's capsule (its virtuals checked by `paces`).
unsafe fn velocity_of(body: *mut u8) -> Option<[f32; 4]> {
    // SAFETY: the body's own virtual (the game's), called as the walking camera's update calls it.
    unsafe {
        let get = read::<usize>(read::<usize>(body as usize)? + GET_VELOCITY)?;
        let mut velocity = [0.0f32; 4];
        std::mem::transmute::<usize, GetVelocity>(get)(body, &mut velocity);
        velocity.iter().all(|v| v.is_finite()).then_some(velocity)
    }
}

/// The capsule's velocity set (x, y, z m/s).
///
/// # Safety
/// As `velocity_of`.
unsafe fn set_velocity(body: *mut u8, velocity: &[f32; 4]) {
    // SAFETY: the body's own virtual (the game's), called as the walking camera's update calls it.
    unsafe {
        let Some(set) = read::<usize>(body as usize).and_then(|vt| read::<usize>(vt + SET_VELOCITY)) else { return };
        std::mem::transmute::<usize, SetVelocity>(set)(body, velocity);
    }
}

/// How far under the player's eyes their feet are (metres), if the walking camera's own sizes can't be read.
/// What it is (both games, the walking camera's start and its "put the camera on the body", `0x16cc000` /
/// `0x16cce70` in ETS2 1.61, `0x16fd4f0` / `0x16fe3e0` in ATS 1.62): the eyes are `EYES_OVER_BODY` + the
/// camera's `+0x660` − its crouch (`+0x664`, 0..1) over the capsule's middle, and the capsule's bottom is its
/// radius (the shape's `+0x18`, 0.6) and half its straight part (`+0x1c`, 0.9 long) under it — 1.8 m standing.
/// Owner 2026-11-05: at 1.7 m the person "floated"; at 2.25 (both sizes whole) it was sunk to the knees.
const FEET_UNDER_EYES: f64 = 1.8;
const EYES_OVER_BODY: f64 = 0.75;
const EYES_EXTRA: usize = 0x660;
const CROUCH: usize = 0x664;
const SHAPE_SIZES: usize = 0x18;
/// The third-person camera turns about the chest (`PIVOT_OVER_FEET` over the feet), `BEHIND` it and `LIFT` over
/// it: at about head height, the whole body in the picture with some ground (owner 2026-11-05: 3 m was "too
/// close to the back", 4.5 m over the eyes "too high", 3.8 m aimed at the eyes cut the feet off; tuned with the
/// feet 1.7 m under the eyes and the chest 0.45 m under them).
const PIVOT_OVER_FEET: f64 = 1.25;

/// How far under the walking camera (the eyes) the capsule's bottom is now (metres): crouching lowers the eyes, not
/// the body.
///
/// # Safety
/// `walking.camera` a walking camera its start made; `physics` as `locate` found it.
unsafe fn feet_under(walking: &Walking, physics: Option<&Physics>) -> f64 {
    let camera = walking.camera as usize;
    // SAFETY: reads guarded, within the walking camera and its capsule shape.
    let measured = unsafe {
        (|| {
            let extra = read::<f32>(camera + EYES_EXTRA).filter(|e| e.is_finite() && e.abs() < 1.0)?;
            let crouch = read::<f32>(camera + CROUCH).filter(|c| c.is_finite() && c.abs() <= 1.0)?;
            let shape = read::<usize>(camera + physics?.shape).filter(|&s| s != 0)?;
            let [a, b] = read::<[f32; 2]>(shape + SHAPE_SIZES)?;
            let size = |s: f32| (s.is_finite() && (0.05..2.0).contains(&s)).then_some(f64::from(s));
            Some(EYES_OVER_BODY + f64::from(extra) - f64::from(crouch.abs()) + size(a)? + size(b)? / 2.0)
        })()
    };
    measured.filter(|u| (0.5..3.5).contains(u)).unwrap_or(FEET_UNDER_EYES)
}
const BEHIND: f64 = 4.8;
const LIFT: f64 = 0.5;

/// The look's speed (radians per mouse count, at 100% sensitivity), how far it pitches down and up (radians: up
/// less, the camera behind would go under the ground), and the pitch it starts at (a little down).
const LOOK_RATE: f32 = 0.004;
const PITCH_DOWN: f32 = 1.2;
const PITCH_UP: f32 = 0.3;
const START_PITCH: f32 = -0.17;

/// The mouse's movement while walking, kept from the game (counts, x and y), until the next frame's look.
static LOOK_DX: AtomicI32 = AtomicI32::new(0);
static LOOK_DY: AtomicI32 = AtomicI32::new(0);

/// Where a camera keeps its angles: heading and pitch (radians, 0..2π), then roll (`core_camera` + 0x10, read
/// in the game: camera 0's heading there matched its rotation's).
const CAMERA_ANGLES: usize = 0x10;

/// Whether the player is out of the cab on foot, from the moment the walk key is pressed (getting out included).
pub fn walking() -> bool {
    WALKING.load(Ordering::Acquire)
}

/// Whether the mouse is the plugin's now (while walking): `input` gives its movement here, none to the game.
pub fn takes_mouse() -> bool {
    // Not with the game paused (its menu, its map: they move its own pointer; owner 2026-11-05: "the mouse
    // doesn't answer in the menu any more").
    WALKING.load(Ordering::Acquire) && !crate::telemetry::paused()
}

/// The mouse moved (counts, on `axis`: 0 x, 1 y) while it's the plugin's.
pub fn look(axis: usize, counts: i32) {
    let total = if axis == 0 { &LOOK_DX } else { &LOOK_DY };
    total.fetch_add(counts, Ordering::AcqRel);
}

/// A view turned by `yaw` about the vertical, then pitched by `pitch` about its side (radians), as the game's
/// cameras make their rotation (w, x, y, z).
fn look_rotation(yaw: f32, pitch: f32) -> [f32; 4] {
    let (t, p) = (yaw / 2.0, pitch / 2.0);
    [t.cos() * p.cos(), t.cos() * p.sin(), t.sin() * p.cos(), -t.sin() * p.sin()]
}
/// The body turns toward where the player walks above this speed (m/s), at this speed (radians a second).
const TURNS_ABOVE: f64 = 0.3;
/// How long the player has to have gone (or stood) before the person shown walks (or stands), seconds. Standing
/// after 0.05 s (3 frames at 60), not the 0.15 it was: the capsule stops in one frame, and for 0.15 s the walk went
/// on stepping with the body still; walking, its speed never fell to nothing (owner's measurements, 2026-11-06:
/// `docs/personagem/TRANSICOES.md` 7.8). If the person blinks stopping again, this is the number to put back.
const MOVING_AFTER: f32 = 0.08;
const STILL_AFTER: f32 = 0.05;
const TURN_SPEED: f32 = 10.0;

/// The game's person on the player's feet (`under` the eyes), facing `yaw` (radians, as `yaw_of` reads a
/// heading): `eyes` is where the walking camera is.
fn on_feet(eyes: &Placement, yaw: f32, under: f64) -> Placement {
    let [x, y, z] = eyes.world();
    Placement::at([x, y - under, z], yaw / std::f32::consts::TAU)
}

/// In first person, how far behind the eyes the headless body stands (metres along the view): the camera just in
/// front of where the neck was, the chest, the arms and the legs below it.
const FIRST_PERSON_BACK: f64 = 0.12;
/// The eyes' bob with the steps (metres): up and down twice a cycle, lowest as a foot comes down, and side to side
/// once (over the foot down); walking and running. It comes in and goes out over a moment (`BOB_EASE`, a second).
const BOB_WALK: (f64, f64) = (0.018, 0.012);
const BOB_RUN: (f64, f64) = (0.03, 0.016);
const BOB_EASE: f32 = 8.0;

/// `dist` metres ahead along the ground of a heading `yaw` (x, z: a heading of θ looks along (-sin θ, -cos θ)).
fn ahead_of(yaw: f32, dist: f64) -> [f64; 2] {
    let yaw = f64::from(yaw);
    [-yaw.sin() * dist, -yaw.cos() * dist]
}

/// The first-person eyes at `eyes`, bobbing with the steps of the animation that walks (`Walking::last_gait`,
/// `last_steps`), as much as the player is walking (eased in and out).
fn bob(walking: &mut Walking, eyes: &Placement, dt: f32) -> Placement {
    let target = if walking.moving && !walking.airborne { 1.0 } else { 0.0 };
    walking.bob += (target - walking.bob) * (1.0 - (-BOB_EASE * dt).exp());
    let (Some((gait, running)), Some((_, clock))) = (walking.last_gait, walking.last_steps) else { return *eyes };
    if walking.bob < 1e-3 {
        return *eyes;
    }
    let (up, side) = if running { BOB_RUN } else { BOB_WALK };
    let tau = std::f64::consts::TAU;
    let (t, cycle) = (f64::from(clock - gait.steps[0]), f64::from(gait.cycle));
    let weight = f64::from(walking.bob);
    let dy = -up * (tau * t / (cycle / 2.0)).cos() * weight;
    let dx = side * (tau * t / cycle).sin() * weight;
    let yaw = f64::from(walking.look_yaw);
    let [x, y, z] = eyes.world();
    Placement { rotation: eyes.rotation, ..Placement::at([x + dx * yaw.cos(), y + dy, z - dx * yaw.sin()], 0.0) }
}

/// The heading (radians, 0..2π) of a step along the ground: a heading of θ looks along (-sin θ, -cos θ).
fn heading_of(dx: f64, dz: f64) -> f32 {
    ((-dx).atan2(-dz) as f32).rem_euclid(std::f32::consts::TAU)
}

/// `from` turned toward `to` by at most `step` (radians), the short way round.
fn turn_toward(from: f32, to: f32, step: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let off = (to - from + PI).rem_euclid(TAU) - PI;
    (from + off.clamp(-step, step)).rem_euclid(TAU)
}

/// Camera 0 behind the player's chest, looking where they look (the mouse turns it about them).
fn behind(eyes: &Placement, under: f64) -> Placement {
    // Where the rotation (w, x, y, z) looks: the camera's -z turned by it.
    let [w, x, y, z] = eyes.rotation.map(f64::from);
    let forward = [-2.0 * (x * z + w * y), 2.0 * (w * x - y * z), -(1.0 - 2.0 * (x * x + y * y))];
    let [ex, ey, ez] = eyes.world();
    let pivot = ey - under + PIVOT_OVER_FEET;
    let at = [ex - BEHIND * forward[0], pivot + LIFT - BEHIND * forward[1], ez - BEHIND * forward[2]];
    Placement { rotation: eyes.rotation, ..Placement::at(at, 0.0) }
}

/// L on foot: the flashlight on (the game's light on WestPlay's mover, in the hand) or off.
///
/// # Safety
/// On the game's main thread, `walking` as `start` made it.
unsafe fn switch_flashlight(game: &Game, walking: &mut Walking) {
    let Some(people) = game.people.as_ref() else {
        log!("character: no flashlight (the game's people code wasn't found)");
        return;
    };
    // SAFETY: on the game's main thread, with what `person::locate` found; the camera read guarded.
    unsafe {
        if let Some(flashlight) = walking.flashlight.take() {
            person::hide_thing(people, flashlight);
            log!("character: flashlight off");
            return;
        }
        let Some(eyes) = read::<Placement>(walking.camera as usize + PLACEMENT) else { return };
        match person::show_thing(people, &in_hand(&eyes, walking.look_yaw, walking.look_pitch), FLASHLIGHT) {
            Ok(flashlight) => {
                walking.flashlight = Some(flashlight);
                log!("character: flashlight on");
            }
            Err(why) => log!("character: no flashlight ({FLASHLIGHT}, WestPlay's mod): {why}"),
        }
    }
}

/// The flashlight in the right hand, under the eyes, pointed where the player looks (`yaw`, `pitch`).
fn in_hand(eyes: &Placement, yaw: f32, pitch: f32) -> Placement {
    let [x, y, z] = eyes.world();
    let (turn, [right, down, forward]) = (f64::from(yaw), FLASHLIGHT_HELD);
    // A heading of θ looks along (-sin θ, -cos θ); its right is (cos θ, -sin θ).
    let (ahead, side) = ([-turn.sin(), -turn.cos()], [turn.cos(), -turn.sin()]);
    let at = [x + side[0] * right + ahead[0] * forward, y + down, z + side[1] * right + ahead[1] * forward];
    Placement { rotation: look_rotation(yaw, pitch), ..Placement::at(at, 0.0) }
}

/// V while walking: third person (the person on the player's feet, the camera behind) or first (the same person
/// without the head, the camera where it was: owner 2026-11-05, "see the arms, the legs, not a floating camera").
/// With `body` false, no body either way (getting out, climbing in: the camera goes its own way).
///
/// # Safety
/// On the game's main thread, `walking` as `start` made it.
unsafe fn set_third_person(game: &Game, walking: &mut Walking, third: bool, body: bool) {
    walking.third_person = third;
    let Some(people) = game.people.as_ref() else {
        log!("character: third person {third}, no body (the game's people code wasn't found)");
        return;
    };
    // SAFETY: on the game's main thread, with what `person::locate` found; the cameras read guarded.
    unsafe {
        if let Some(shown) = walking.person.take() {
            person::hide(people, shown);
        }
        if body {
            let Some(mut eyes) = read::<Placement>(walking.camera as usize + PLACEMENT) else { return };
            if let Some(view) = read::<Placement>(walking.debug as usize + PLACEMENT) {
                eyes.rotation = view.rotation;
            }
            // Facing where the player looks at first; after, where they walk (in first person, where they look).
            walking.body_yaw = yaw_of(eyes.rotation);
            walking.last_at = None;
            let under = feet_under(walking, game.physics.as_ref());
            match person::show(people, &on_feet(&eyes, walking.body_yaw, under), chosen_person(), !third) {
                Ok(mut shown) => {
                    let (owner, model, instance) = shown.parts(people);
                    log!(
                        "character: the person: owner {owner:#x}, model {model:#x} ({}), instance {instance:#x}",
                        class_name(&game.image, model).unwrap_or_else(|| "?".into())
                    );
                    log!("character: feet {under:.2} m under the eyes");
                    walking.person = Some(shown);
                }
                Err(why) => log!("character: no body: {why}"),
            }
        }
    }
    log!("character: {} person", if third { "third" } else { "first" });
}

/// The walk key at the driver's door: first person (no body), and the camera climbing in from where it is
/// (false if camera 0 can't be read: straight into the cab, as before).
///
/// # Safety
/// On the game's main thread, `state.walking` as `start` made it.
unsafe fn begin_climb(state: &mut State, truck: &crate::telemetry::Truck) -> bool {
    let (Some(Some(game)), Some(walking)) = (&state.game, &mut state.walking) else { return false };
    // SAFETY: as the caller says; camera 0 read guarded.
    unsafe {
        let Some(from) = read::<Placement>(walking.debug as usize + PLACEMENT) else { return false };
        // In third person, the camera into the person's eyes first, the body hidden only there (owner 2026-11-14:
        // "first the camera comes close to him, then he disappears"); else as before.
        let eyes = walking.third_person.then(|| read::<Placement>(walking.camera as usize + PLACEMENT)).flatten();
        if eyes.is_none() {
            set_third_person(game, walking, false, false);
        }
        if walking.flashlight.is_some() {
            switch_flashlight(game, walking);
        }
        if let (Some(people), Some(nozzle)) = (game.people.as_ref(), walking.nozzle.take()) {
            person::hide_thing(people, nozzle);
        }
        walking.nozzle_was_third = None;
        walking.climb = Some(match eyes {
            Some(eyes) => climb::Climb::from_behind(from.world(), eyes.world(), walking.look_yaw, walking.look_pitch, truck),
            None => climb::Climb::new(from.world(), walking.look_yaw, walking.look_pitch, truck),
        });
        crate::bridge::put_phone_away("climbing into the truck");
    }
    walking.last = Instant::now();
    log!("character: climbing in (first person)");
    true
}

/// A frame of the climb in (or, `out`, of getting out): camera 0 where it has got to; true once at its end.
///
/// # Safety
/// On the game's main thread, `walking` as `start` made it.
unsafe fn climb_frame(walking: &mut Walking, out: bool) -> Result<bool, &'static str> {
    let now = Instant::now();
    let dt = now.duration_since(walking.last).as_secs_f32().clamp(0.0, 0.1);
    walking.last = now;
    // The mouse's moves while climbing are dropped (the look is the climb's).
    LOOK_DX.store(0, Ordering::Release);
    LOOK_DY.store(0, Ordering::Release);
    let way = if out { walking.descend.as_mut() } else { walking.climb.as_mut() };
    let Some(climb) = way else { return Ok(true) };
    let (view, landed) = climb.go(dt);
    if landed {
        steps::play(false);
    }
    // Climbing in, at the door on the ground: it opens (the way's second place, `climb::path`).
    if !out && climb.passed(climb.door_key(), dt) {
        steps::door(steps::Door::Open, 0.0);
    }
    // From behind the person, the camera now in their eyes: the body goes (`climbing`'s caller hides it).
    if !out && climb.reached_eyes(dt) {
        walking.hide_body = true;
    }
    let Some((at, yaw, pitch)) = view else { return Ok(true) };
    // SAFETY: camera 0 is the game's own, read guarded before it's written, as `step` writes it.
    unsafe {
        read::<Placement>(walking.debug as usize + PLACEMENT).ok_or("camera 0 is gone")?;
        let placement = Placement { rotation: look_rotation(yaw, pitch), ..Placement::at(at, 0.0) };
        (walking.debug.add(PLACEMENT) as *mut Placement).write_unaligned(placement);
        (walking.debug.add(CAMERA_ANGLES) as *mut [f32; 2])
            .write_unaligned([yaw, pitch.rem_euclid(std::f32::consts::TAU)]);
    }
    Ok(false)
}

/// Gives the input back as it was and lets the walking camera go; back to the cab (the game's camera 1 key)
/// if `to_cab`.
fn stop(state: &mut State, why: &str, to_cab: bool) {
    let Some(walking) = state.walking.take() else { return };
    WALKING.store(false, Ordering::Release);
    // The press that got back in isn't held into the next walk (it ended that one at once: owner 2026-11-04, F8
    // "had to be pressed twice").
    state.held_since = None;
    let Some(Some(game)) = state.game.as_ref() else { return };
    // SAFETY: on the game's main thread; the input fields are the ones the camera's activate wrote, put back
    // as they were (and applied as it applies them); the camera released through the game's assignment.
    unsafe {
        let i = &walking.input;
        (i.input as *mut u8).add(i.context).cast::<u64>().write_unaligned(i.context_was);
        let flag = (i.input as *mut u8).add(i.flag);
        if flag.read() != i.flag_was {
            flag.write(i.flag_was);
            (i.apply)(i.input as *mut u8);
        }
        if let (Some(shown), Some(people)) = (walking.person, game.people.as_ref()) {
            person::hide(people, shown);
        }
        if let (Some(flashlight), Some(people)) = (walking.flashlight, game.people.as_ref()) {
            person::hide_thing(people, flashlight);
        }
        if let (Some(nozzle), Some(people)) = (walking.nozzle, game.people.as_ref()) {
            person::hide_thing(people, nozzle);
        }
        // No more rays for the camera.
        crate::ray::camera_stop();
        // The bumping shape let go as the camera's destructor lets go of the capsule's own, before the body goes.
        if let (Some((_, system)), Some(physics)) = (walking.body, game.physics.as_ref())
            && let Some(vtable) = read::<usize>(system as usize)
            && let Some(destroy) = read::<usize>(vtable + physics.destroy_shape_slot)
        {
            for shape in [walking.bump, walking.traffic].into_iter().flatten() {
                std::mem::transmute::<usize, DestroyShape>(destroy)(system, shape);
            }
        }
        let mut held = walking.camera;
        (game.assign)(&mut held, std::ptr::null_mut());
        // Camera 0 parked outside the cab, off the driver's head: climbing in left it there, where the cab's
        // camera is, and the next getting out took it for on screen (it is "on screen" only within a metre of the
        // drawn camera), walked with it and the cab's camera stayed drawn (owner 2026-11-05: "sometimes he gets
        // out but the camera stays in the cab"). Now that one presses its key first, as it should.
        if to_cab && read::<Placement>(walking.debug as usize + PLACEMENT).is_some() {
            let parked = Placement::at(parked_camera(&crate::telemetry::truck()), 0.0);
            (walking.debug.add(PLACEMENT) as *mut Placement).write_unaligned(parked);
        }
    }
    if to_cab {
        CAMERA_1.send(true);
        state.release = Some(CAMERA_1);
    }
    log!("character: stopped walking ({why})");
}

/// The game's camera 0 (`debug_camera`) from its camera manager, found in the core object by class name.
///
/// # Safety
/// `core` is the game's core object.
unsafe fn debug_camera(image: &Image, core: usize) -> Result<usize, &'static str> {
    // SAFETY: every read guarded.
    unsafe {
        let manager = (0..CORE_SIZE)
            .step_by(8)
            .filter_map(|off| read::<usize>(core + off))
            .find(|&p| class_name(image, p).as_deref() == Some("camera_manager"))
            .ok_or("no camera manager")?;
        let list = read::<usize>(manager + MANAGER_LIST).ok_or("no camera list")?;
        let count = read::<usize>(manager + MANAGER_LIST + 8).filter(|&c| c <= 64).ok_or("no camera count")?;
        (0..count)
            .filter_map(|i| read::<usize>(list + i * 8))
            .find(|&c| class_name(image, c).as_deref() == Some("debug_camera"))
            .ok_or("no camera 0 (debug_camera) in the camera manager")
    }
}

/// An object's class name, from its `get_class` (`lea rax, [descriptor]; ret`) and the descriptor's name.
///
/// # Safety
/// None beyond the reads being guarded; `object` may be anything.
unsafe fn class_name(image: &Image, object: usize) -> Option<String> {
    // SAFETY: every read guarded, or within the image (`code_at`).
    unsafe {
        if object < 0x10000 {
            return None;
        }
        let vtable = read::<usize>(object).filter(|&v| image.in_constants(v))?;
        let get_class = read::<usize>(vtable + CLASS_SLOT * 8).filter(|&f| image.in_code(f))?;
        let b = image.code_at(get_class, 8)?;
        if b[..3] != [0x48, 0x8d, 0x05] || b[7] != 0xc3 {
            return None;
        }
        let descriptor = (get_class + 7).wrapping_add_signed(i32::from_le_bytes(b[3..7].try_into().ok()?) as isize);
        let record = read::<usize>(descriptor).filter(|&r| image.contains(r))?;
        let name = read::<usize>(record).filter(|&n| image.in_constants(n))?;
        let bytes = image.code_at(name, 48)?;
        let end = bytes.iter().position(|&c| c == 0)?;
        std::str::from_utf8(&bytes[..end]).ok().map(str::to_owned)
    }
}

/// The input object and the two fields the walking camera's activate writes, and the function applying them.
///
/// # Safety
/// `activate` is the walking camera's activate (its vtable's).
unsafe fn input_of(image: &Image, activate: usize) -> Option<(usize, usize, usize, Apply)> {
    let b = image.code_at(activate, 51)?;
    if !fits(b, &pattern(ACTIVATE)) {
        return None;
    }
    let disp = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize;
    let global = rel(image, activate + 3)?;
    let context = disp(17);
    let flag = disp(30);
    if disp(41) != flag {
        return None;
    }
    let apply = rel(image, activate + 46).filter(|&f| image.in_code(f))?;
    // SAFETY: a global of the game's executable, read guarded.
    let input = unsafe { read::<usize>(global) }.filter(|&i| i != 0)?;
    // SAFETY: the game's code, at the address its own jump goes to.
    Some((input, context, flag, unsafe { std::mem::transmute::<usize, Apply>(apply) }))
}

/// Finds everything in the game's code; logs what wasn't.
///
/// # Safety
/// In the game's process.
unsafe fn locate() -> Option<Game> {
    let found = (|| {
        // SAFETY: no name: the game's own executable, mapped for the process's life.
        let base = unsafe { GetModuleHandleW(None) }.ok()?.0 as usize;
        // SAFETY: `base` is the game's executable, as mapped.
        let image = unsafe { Image::of(base) }?;
        let site = image.unique_masked(&pattern(START_SITE)).or_else(|| missing("the walking camera's start"))?;
        let start = rel(&image, site + 18)?;
        let assign = rel(&image, site + 38)?;
        let factory_site = image
            .unique_masked_in(&pattern(FACTORY_SITE), site.saturating_sub(0x100), site)
            .or_else(|| missing("the walking camera's factory"))?;
        let factory = rel(&image, factory_site + 36)?;
        let allocator_mode = rel(&image, factory_site + 2)?;
        let allocator_table = rel(&image, factory_site + 18)?;
        let world_site = image.unique_masked(&pattern(WORLD_SITE)).or_else(|| missing("the road physics world"))?;
        let u32_at = |at: usize| image.code_at(at, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
        let system = u32_at(world_site + 3)?;
        let create_world = rel(&image, world_site + 16)?;
        let world = u32_at(world_site + 23)?;
        if u32_at(world_site + 47)? != world {
            return missing("the road physics world (offsets disagree)");
        }
        let mut cores = image
            .all_masked(&pattern(CORE_SITE))
            .into_iter()
            .filter(|&at| u32_at(at + 10) == Some(system) && rel(&image, at + 18) == Some(create_world));
        let core_site = cores.next().or_else(|| missing("the core object"))?;
        if cores.next().is_some() {
            return missing("the core object (more than one)");
        }
        let core = rel(&image, core_site + 3)?;
        if ![start, assign, factory].iter().all(|&f| image.in_code(f))
            || ![allocator_mode, allocator_table, core].iter().all(|&g| image.contains(g))
        {
            return missing("the walking camera's pieces (outside the game's image)");
        }
        let people = person::locate(&image, base);
        let notices = notice::locate(&image, core);
        let physics = physics_of(&image, start);
        if physics.is_none() {
            log!("character: not found: the capsule's collision layers (walking keeps the game's)");
        }
        log!(
            "character: found (+{:#x} factory, +{:#x} start, +{:#x} assign, core +{:#x}, world at +{world:#x})",
            factory - base,
            start - base,
            assign - base,
            core - base
        );
        // SAFETY: each address is the game's code where its own calls go, of the type its call sites give.
        Some(unsafe {
            Game {
                factory: std::mem::transmute::<usize, Factory>(factory),
                allocator_mode,
                allocator_table,
                start: std::mem::transmute::<usize, Start>(start),
                assign: std::mem::transmute::<usize, Assign>(assign),
                core,
                world,
                physics,
                people,
                notices,
                image,
            }
        })
    })();
    if found.is_none() {
        log!("character: the game's walking camera code isn't the version WestPlay knows; the test is off");
    }
    found
}

/// The capsule's physics pieces, from the walking camera's start (`start`) and the game's set_layer and
/// destructor, each checked against the others (the same offsets wherever two of them say one).
fn physics_of(image: &Image, start: usize) -> Option<Physics> {
    let u32_at = |at: usize| image.code_at(at, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let within = |text: &str, from: usize, len: usize| image.unique_masked_in(&pattern(text), from, from + len);
    let system_site = within(SYSTEM_SITE, start, 0x40)?;
    let (world, system) = (u32_at(system_site + 3)?, u32_at(system_site + 10)?);
    let layer_site = within(LAYER_SITE, start, 0x300)?;
    let body = u32_at(layer_site + 3)?;
    let set_layer = rel(image, layer_site + 13).filter(|&f| image.in_code(f))?;
    if !fits(image.code_at(set_layer, 25)?, &pattern(SET_LAYER)) {
        return None;
    }
    let layer_field = u32_at(set_layer + 17)?;
    let shape_layer_slot = u32_at(within(SET_SHAPE_LAYER, set_layer, 0x70)? + 10)?;
    let shape_site = within(SHAPE_SITE, start, 0x300)?;
    let create_shape = rel(image, shape_site + 1).filter(|&f| image.in_code(f))?;
    let shape = u32_at(shape_site + 31)?;
    let margin_site = within(MARGIN_SITE, start, 0x300)?;
    let kind = usize::from(image.code_at(margin_site + 3, 1)?[0]);
    let margins = rel(image, margin_site + 7).filter(|&m| image.contains(m))?;
    let release = image.unique_masked(&pattern(RELEASE_SITE))?;
    if u32_at(release + 3)? != world || u32_at(release + 10)? != shape || u32_at(release + 22)? != system {
        return None;
    }
    let destroy_shape_slot = u32_at(release + 39)?;
    if [layer_field, shape_layer_slot, destroy_shape_slot].iter().any(|&o| o > 0x1000) || shape != body + 8 {
        return None;
    }
    // SAFETY: each is the game's code where its own calls go, with the arguments its call sites give.
    Some(unsafe {
        Physics {
            body,
            shape,
            world,
            system,
            set_layer: std::mem::transmute::<usize, SetLayer>(set_layer),
            layer_field,
            shape_layer_slot,
            create_shape: std::mem::transmute::<usize, CreateShape>(create_shape),
            kind,
            margins,
            destroy_shape_slot,
        }
    })
}

fn missing<T>(what: &str) -> Option<T> {
    log!("character: not found: {what}");
    None
}

/// Whether TruckersMP's client is in the game (its core module loaded).
fn under_truckersmp() -> bool {
    // SAFETY: plain queries of the process's modules by name.
    unsafe { GetModuleHandleW(w!("core_ets2mp.dll")).is_ok() || GetModuleHandleW(w!("core_atsmp.dll")).is_ok() }
}

/// Where a RIP-relative displacement at `at` (four bytes, its instruction ending right after) points.
fn rel(image: &Image, at: usize) -> Option<usize> {
    let b = image.code_at(at, 4)?;
    Some((at + 4).wrapping_add_signed(i32::from_le_bytes(b.try_into().ok()?) as isize))
}

/// A pattern written as hex bytes, `??` for any.
fn pattern(text: &str) -> Vec<Option<u8>> {
    text.split_whitespace().map(|t| if t == "??" { None } else { u8::from_str_radix(t, 16).ok() }).collect()
}

fn fits(bytes: &[u8], pattern: &[Option<u8>]) -> bool {
    bytes.len() >= pattern.len() && bytes.iter().zip(pattern).all(|(b, p)| p.is_none_or(|p| *b == p))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests turning walking on, one at a time (they share it).
    static WALKING_TESTS: Mutex<()> = Mutex::new(());

    #[test]
    fn a_placement_is_a_sector_and_a_place_in_it() {
        // Camera 0 as the game kept it (ETS2 1.61, 2026-11-04).
        let p = Placement { local: [-171.205, 49.924, 185.7], sector: [-225, 327], rotation: [1.0, 0.0, 0.0, 0.0] };
        let w = p.world();
        assert!((w[0] + 115371.205).abs() < 0.01 && (w[2] - 167609.7).abs() < 0.01);
        let back = Placement::at(w, 0.0);
        assert_eq!(back.sector, [-225, 327]);
        assert!((back.local[0] - p.local[0]).abs() < 0.01);
    }

    #[test]
    fn a_heading_turns_about_the_vertical() {
        // A quarter turn: (cos 45°, 0, sin 45°, 0).
        let r = Placement::at([0.0; 3], 0.25).rotation;
        assert!((r[0] - r[2]).abs() < 1e-6 && r[1] == 0.0 && r[3] == 0.0);
    }

    #[test]
    fn the_door_is_on_the_drivers_side() {
        let truck = |head_x: f32, heading: f32| crate::telemetry::Truck {
            position: [100.0, 10.0, 200.0],
            heading,
            speed: 0.0,
            pitch: 0.0,
            roll: 0.0,
            parking_brake: true,
            multiplayer: false,
            head: [head_x, 2.5, -1.8],
        };
        // Facing north (-z): left-hand drive, the door to the west; right-hand drive, to the east.
        let left = door(&truck(-0.6, 0.0));
        assert!((left[0] - 98.4).abs() < 1e-6 && (left[2] - 197.95).abs() < 1e-6 && (left[1] - 11.8).abs() < 1e-6);
        assert!((door(&truck(0.6, 0.0))[0] - 101.6).abs() < 1e-6);
        // Turned a quarter (facing west): the left-hand door to the south.
        let turned = door(&truck(-0.6, 0.25));
        assert!((turned[2] - 201.6).abs() < 1e-6 && (turned[0] - 97.95).abs() < 1e-6);
    }

    #[test]
    fn the_heading_is_read_back_from_a_rotation() {
        // The walking camera as the game turned it: heading 4.694 rad, rotation (0.701, 0, -0.713, 0).
        assert!((yaw_of([0.701, 0.0, -0.713, 0.0]) - 4.694).abs() < 0.01);
        // Any heading made into a rotation reads back.
        for turns in [0.1f32, 0.3, 0.6, 0.9] {
            let r = Placement::at([0.0; 3], turns).rotation;
            assert!((yaw_of(r) - turns * std::f32::consts::TAU).abs() < 1e-4);
        }
        // Looking down too (a turn about the vertical, then about the side): still the same heading.
        let (t, p) = (1.0f32 / 2.0, -0.4f32 / 2.0);
        let r = [t.cos() * p.cos(), t.cos() * p.sin(), t.sin() * p.cos(), -t.sin() * p.sin()];
        assert!((yaw_of(r) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn patterns_read_as_written() {
        assert_eq!(pattern("48 ?? C3"), vec![Some(0x48), None, Some(0xc3)]);
        assert!(fits(&[0x48, 0x11, 0xc3], &pattern("48 ?? C3")));
        assert_eq!(pattern(START_SITE).len(), 42);
        assert_eq!(pattern(FACTORY_SITE).len(), 43);
        assert_eq!(pattern(WORLD_SITE).len(), 51);
        assert_eq!(pattern(ACTIVATE).len(), 51);
        // The offsets `physics_of` reads are inside them.
        assert_eq!(pattern(LAYER_SITE).len(), 17);
        assert_eq!(pattern(SHAPE_SITE).len(), 35);
        assert_eq!(pattern(SYSTEM_SITE).len(), 14);
        assert_eq!(pattern(MARGIN_SITE).len(), 11);
        assert_eq!(pattern(SET_LAYER).len(), 25);
        assert_eq!(pattern(SET_SHAPE_LAYER).len(), 14);
        assert_eq!(pattern(RELEASE_SITE).len(), 43);
    }

    #[test]
    fn shift_and_ctrl_are_kept_from_the_game_on_either_side_while_walking() {
        let _one = WALKING_TESTS.lock();
        WALKING.store(true, Ordering::Release);
        let keys = [VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_CONTROL, VK_LCONTROL, VK_RCONTROL];
        let taken = keys.iter().all(|k| takes(u32::from(k.0)));
        let scancodes = [0x2a, 0x36, 0x1d, 0x9d].iter().all(|&code| takes_scancode(code));
        WALKING.store(false, Ordering::Release);
        assert!(taken && scancodes);
        assert!(!takes(u32::from(VK_LSHIFT.0)) && !takes_scancode(0x2a));
    }

    /// Back in the cab, camera 0 is left where getting out again can't take it for the cab's camera: more than the
    /// metre `start` allows (on its largest axis) from the driver's head, whichever way the truck faces.
    #[test]
    fn ahead_is_where_a_heading_looks() {
        let [x, z] = ahead_of(0.0, 1.0);
        assert!(x.abs() < 1e-9 && (z + 1.0).abs() < 1e-9);
        let [x, z] = ahead_of(std::f32::consts::FRAC_PI_2, 2.0);
        assert!((x + 2.0).abs() < 1e-6 && z.abs() < 1e-6);
        // The headless body behind the eyes: opposite the view.
        let (back, turned) =
            (ahead_of(1.0, -FIRST_PERSON_BACK), ahead_of(1.0 + std::f32::consts::PI, FIRST_PERSON_BACK));
        assert!((back[0] - turned[0]).abs() < 1e-6 && (back[1] - turned[1]).abs() < 1e-6);
    }

    #[test]
    fn camera_0_is_parked_off_the_drivers_head() {
        for side in [-0.6f32, 0.6] {
            for turn in 0..16 {
                let truck = crate::telemetry::Truck {
                    position: [100.0, 10.0, 200.0],
                    heading: turn as f32 / 16.0,
                    speed: 0.0,
                    pitch: 0.0,
                    roll: 0.0,
                    parking_brake: true,
                    multiplayer: false,
                    head: [side, 2.3, -1.7],
                };
                let head = in_truck(&truck, truck.head.map(f64::from));
                let parked = parked_camera(&truck);
                let off = (0..3).map(|i| (parked[i] - head[i]).abs()).fold(0.0, f64::max);
                assert!(off > 1.5, "{off} at heading {turn}/16, side {side}");
            }
        }
    }

    #[test]
    fn the_third_person_camera_is_behind_and_over_the_eyes() {
        // Looking north (-z, no turn): the camera south of the eyes (+z), over them, looking the same way.
        let eyes = Placement::at([100.0, 10.0, 200.0], 0.0);
        let cam = behind(&eyes, FEET_UNDER_EYES).world();
        let pivot = 10.0 - FEET_UNDER_EYES + PIVOT_OVER_FEET;
        assert!((cam[2] - (200.0 + BEHIND)).abs() < 1e-3 && (cam[1] - (pivot + LIFT)).abs() < 1e-3);
        assert!((cam[0] - 100.0).abs() < 1e-3 && behind(&eyes, FEET_UNDER_EYES).rotation == eyes.rotation);
        // A quarter turn (looking west, -x, as the walking camera's heading reads it): the camera to the east.
        let turned = Placement::at([100.0, 10.0, 200.0], 0.25);
        assert!((yaw_of(turned.rotation) - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!((behind(&turned, FEET_UNDER_EYES).world()[0] - (100.0 + BEHIND)).abs() < 1e-3);
        // The person on the feet, turned the same way.
        let feet = on_feet(&turned, yaw_of(turned.rotation), FEET_UNDER_EYES);
        assert!((feet.world()[1] - (10.0 - FEET_UNDER_EYES)).abs() < 1e-3);
        assert!((yaw_of(feet.rotation) - yaw_of(turned.rotation)).abs() < 1e-4);
    }

    #[test]
    fn the_body_turns_where_the_player_walks_the_short_way() {
        use std::f32::consts::{FRAC_PI_2, PI};
        // Walking north (-z) is heading 0; west (-x) a quarter turn; south half a turn.
        assert!(heading_of(0.0, -1.0).abs() < 1e-5);
        assert!((heading_of(-1.0, 0.0) - FRAC_PI_2).abs() < 1e-5);
        assert!((heading_of(0.0, 1.0) - PI).abs() < 1e-5);
        // From 350° to 10°: through 0°, not back round.
        let (from, to) = (350f32.to_radians(), 10f32.to_radians());
        let turned = turn_toward(from, to, 5f32.to_radians());
        assert!((turned - 355f32.to_radians()).abs() < 1e-4);
        assert!((turn_toward(from, to, 1.0) - to).abs() < 1e-4);
    }

    #[test]
    fn the_look_is_made_as_the_games_cameras_make_theirs() {
        // A heading and a pitch read back as made (`yaw_of`, and the pitch from where the view looks).
        for (yaw, pitch) in [(0.0f32, 0.0f32), (1.0, -0.3), (4.0, 0.3), (6.0, -1.1)] {
            let r = look_rotation(yaw, pitch);
            assert!((yaw_of(r) - yaw).abs() < 1e-4);
            let [w, x, y, z] = r;
            assert!(((2.0 * (w * x - y * z)).asin() - pitch).abs() < 1e-4);
        }
        // Looking up, the camera behind goes lower than the eyes, but not under the feet.
        let eyes = Placement { rotation: look_rotation(0.0, PITCH_UP), ..Placement::at([0.0, 10.0, 0.0], 0.0) };
        let cam = behind(&eyes, FEET_UNDER_EYES).world();
        assert!(cam[1] < 10.0 && cam[1] > 10.0 - FEET_UNDER_EYES);
    }

    #[test]
    fn v_is_kept_from_the_game_while_walking() {
        let _one = WALKING_TESTS.lock();
        WALKING.store(true, Ordering::Release);
        let (key, code) = (takes(u32::from(b'V')), takes_scancode(0x2f));
        WALKING.store(false, Ordering::Release);
        assert!(key && code && !takes(u32::from(b'V')));
    }

    #[test]
    fn the_capsules_layers_are_the_right_ones() {
        // The road world's collision table (ETS2 1.61, read in the game): who each layer meets.
        let meets = |row: u32, layer: u32| row >> layer & 1 == 1;
        let (row_9, row_3) =
            (0b1111_1111_1000_0111_1111_1010_0000_0110u32, 0b0000_0000_0001_0000_0000_0001_0100_0000u32);
        // Layer 9: the ground and fixed world (1, 2), no vehicle (4 traffic, 6/7 the truck, 8 the trailer).
        assert!(meets(row_9, 1) && meets(row_9, 2));
        assert!([4, 6, 7, 8].iter().all(|&v| !meets(row_9, v)));
        // Layer 3: the truck's chassis and the trailer, nothing else of the world.
        assert!(meets(row_3, 6) && meets(row_3, 8) && !meets(row_3, 1));
        assert_eq!((BODY_LAYER, BUMP_LAYER), (9, 3));
    }
}
