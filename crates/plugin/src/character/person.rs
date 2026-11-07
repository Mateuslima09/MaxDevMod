//! Modo Personagem, etapa A's write test (docs/personagem/INVESTIGACAO.md §7.8): one of the game's own people
//! (a mover: its model, its look, its animation) put where the plugin wants, as the game puts a bus passenger.
//!
//! The game makes its bus passengers on the spot: an owner object (0x110 bytes, made by its constructor) holding
//! a scene node (`+0x90`: local placement at `+0x20`, parent at `+0x40`), a "placed" flag (`+0x88`), the
//! mover's model (`+0xf8`), its look (`+0x100`) and the mover instance (`+0x108`). Its spawn makes the instance
//! from a `mover_model`/`mover_model_group` unit and hangs it under the owner's node; moving the node (the
//! game's set_world) moves the person. Its release lets the instance go. The plugin does the same with an owner
//! of its own, made by the game's constructor (so its virtuals are the game's), and a model the game finds by
//! name, as its prefabs' `mover.%s` does.
//!
//! Every piece is found by the shape of the game's code (each once, the same in ETS2 1.61 and ATS 1.62); nothing
//! found, nothing done (the log says what).

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use windows::Win32::System::Threading::GetCurrentThreadId;

use super::{Placement, fits, pattern, read, rel};
use crate::log;
use crate::navigation::Image;

/// The passenger owner's constructor: `mov [rsp+8], rbx; push rdi; sub rsp, 20h; lea rax, [vtable]; xor edi,
/// edi; mov [rcx], rax; mov rbx, rcx; mov [rcx+8], edi; lea rax, [..]; mov dword [rcx+18h], 2e35d5ffh`.
const OWNER_CTOR: &str = "48 89 5C 24 08 57 48 83 EC 20 48 8D 05 ?? ?? ?? ?? 33 FF 48 89 01 48 8B D9 89 79 08 48 8D 05 \
                          ?? ?? ?? ?? C7 41 18 FF D5 35 2E";
/// The passenger's spawn, its first lines: `... sub rsp, 0d0h; cmp byte [rcx+placed], 0; mov rsi, r8; mov rdi,
/// rdx; mov rbx, rcx`.
const SPAWN: &str =
    "48 89 5C 24 08 48 89 74 24 10 57 48 81 EC D0 00 00 00 80 B9 ?? ?? ?? ?? 00 49 8B F0 48 8B FA 48 8B D9";
/// In it, hanging the instance under the node: `mov rcx, [rbx+instance]; lea rdx, [rbx+node]; call attach`; and,
/// when the instance fails, `mov rcx, rbx; call release; mov edx, [rdi+0ch]`.
const SPAWN_ATTACH: &str = "48 8B 8B ?? ?? ?? ?? 48 8D 93 ?? ?? ?? ?? E8 ?? ?? ?? ??";
const SPAWN_RELEASE: &str = "48 8B CB E8 ?? ?? ?? ?? 8B 57 0C";
/// In attach: `call set_parent; lea rdx, [rsp+20h]; mov rcx, rdi; call set_world`.
const ATTACH_WORLD: &str = "E8 ?? ?? ?? ?? 48 8D 54 24 20 48 8B CF E8 ?? ?? ?? ??";
/// The prefabs finding their mover by name: `lea rdx, ["mover.%s"]; lea rcx, [rbp+..]; call format; lea rcx,
/// [rbp+..]; call find_unit; mov rdx, [rbx+..]`.
const FIND_UNIT: &str = "48 8D 15 ?? ?? ?? ?? 48 8D 4D ?? E8 ?? ?? ?? ?? 48 8D 4D ?? E8 ?? ?? ?? ?? 48 8B 93";

/// A mover instance's update, as its own first lines read (`push rbp; push rsi; lea rbp, [rsp-58h]; sub rsp,
/// 158h; mov rax, [rcx+model]; mov rsi, rcx; movaps [rsp+100h], xmm8; movaps xmm8, xmm1`): (instance, seconds).
/// Every owner of movers calls it each frame (the map's, the prefabs', the bus passengers'); it shows the
/// instance (`+0x158` bit 8) once its model is ready, and plays its animation.
const UPDATE: &str = "40 55 56 48 8D 6C 24 A8 48 81 EC 58 01 00 00 48 8B 81 ?? ?? ?? ?? 48 8B F1 44 0F 29 84 24 00 01 00 \
                      00 44 0F 28 C1";
/// The game putting its bus passengers into a frame's drawing: `mov rcx, [rsi+220h]; test rcx, rcx; je ..; mov
/// rdx, rdi; call draw; add rsi, 228h` (each passenger's mover instance, drawn with the frame's draw context), in
/// the function gathering the player's world things to draw (a virtual: hooked, to draw ours there too).
const DRAW_SITE: &str = "48 8B 8E 20 02 00 00 48 85 C9 74 08 48 8B D7 E8 ?? ?? ?? ?? 48 81 C6 28 02 00 00";
/// That function's first lines: `mov [rsp+18h], rbx; push rdi; sub rsp, 50h; mov rax, [rcx+0f0h]; mov rdi, rdx`.
const GATHER: &str = "48 89 5C 24 18 57 48 83 EC 50 48 8B 81 F0 00 00 00 48 8B FA";

/// The player's person: one, always the same, chosen on the companion's Personagem page among WestPlay's own
/// (`westplay_shared::CHARACTER_PEOPLE`, in WestPlay's mod: `westplay_setup::own_character`), each standing with
/// their own idle animation. Without their files, the game's man `m_vanilla_ice_01` (`CHARACTER_GAME_PERSON`), with
/// crossed arms standing (`m_idle01` in ATS, `idle_event01` in ETS2, then the same man angry: read from
/// /def/world/mover*.sii of ETS2 1.61 and ATS 1.62).
/// (WestPlay's own people stand as `<prefix>_idle`: `Profile`.)
const STANDING: [(&str, &[&str]); 1] = [("m_walk_06", &["m_idle01", "idle_event01", "m_angry01"])];

/// The mover of a person's headless body (WestPlay's mod: `westplay_setup::own_character`, `<prefix>f_`): the same
/// animation, `wt_walk` → `wtf_walk`.
fn headless(name: &str) -> String {
    name.replacen('_', "f_", 1)
}

/// A `mover_model`'s unit name, as the game looks one up (`mover.<name>`, NUL-ended).
fn unit_name(name: &str) -> Vec<u8> {
    format!("mover.{name}\0").into_bytes()
}
/// Where a mover instance keeps its animated model (the update's and the draw's first read).
const MODEL_INSTANCE: usize = 0xa8;
/// Room for the owner: the game's is 0x110 bytes (its destructor's size), checked against this.
const OWNER_ROOM: usize = 0x200;

type Ctor = unsafe extern "system" fn(*mut u8) -> *mut u8;
type Dtor = unsafe extern "system" fn(*mut u8, u32);
type Spawn = unsafe extern "system" fn(*mut u8, *mut u8, *const u64);
type Release = unsafe extern "system" fn(*mut u8);
type SetWorld = unsafe extern "system" fn(*mut u8, *const Placement);
type FindUnit = unsafe extern "system" fn(*const *const u8) -> *mut u8;
type Update = unsafe extern "system" fn(*mut u8, f32);
type Draw = unsafe extern "system" fn(*mut u8, *mut u8);
type Gather = unsafe extern "system" fn(*mut u8, *mut u8) -> usize;

/// The person's mover instance to draw (0: none), the game's draw, and the thread the plugin's frame runs on: the
/// hooked gathering draws the person only there (where `show` and `hide` run, so never while it's let go).
static DRAWN: AtomicUsize = AtomicUsize::new(0);
/// Whether the twins of `probe::Twins` were made this session.
static TWINS_MADE: AtomicBool = AtomicBool::new(false);
/// A thing of the player's drawn too (the flashlight's mover: its light is drawn with it), 0: none.
static DRAWN_TOO: AtomicUsize = AtomicUsize::new(0);
/// The person shown before a switch, drawn too for a few frames after it (`place`), 0: none; and how many.
static DRAWN_BEFORE: AtomicUsize = AtomicUsize::new(0);
const OVERLAP: u8 = 2;
static DRAW: AtomicUsize = AtomicUsize::new(0);
static GATHER_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static FRAME_THREAD: AtomicU32 = AtomicU32::new(0);
static OTHER_THREAD_SEEN: AtomicBool = AtomicBool::new(false);

/// At each frame's end (the plugin's): the thread the person may be drawn on.
pub(super) fn on_frame() {
    // SAFETY: a plain query.
    FRAME_THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Release);
}

/// In place of the game's gathering of the player's world things to draw: the game's, then the person.
///
/// # Safety
/// Called by the game in place of its own function, with its arguments.
unsafe extern "system" fn gather(this: *mut u8, context: *mut u8) -> usize {
    // SAFETY: the trampoline stored before the hook was switched on: the original function.
    let result =
        unsafe { std::mem::transmute::<usize, Gather>(GATHER_ORIGINAL.load(Ordering::Acquire))(this, context) };
    let instance = DRAWN.load(Ordering::Acquire);
    let too = DRAWN_TOO.load(Ordering::Acquire);
    if instance == 0 && too == 0 {
        return result;
    }
    // SAFETY: a plain query.
    if unsafe { GetCurrentThreadId() } != FRAME_THREAD.load(Ordering::Acquire) {
        if !OTHER_THREAD_SEEN.swap(true, Ordering::AcqRel) {
            log!("character: the game draws on another thread than the plugin's frame: the person isn't drawn");
        }
        return result;
    }
    crate::hooks::guarded(|| {
        // The passes the game draws its passengers in (`[context+18h] & 0fh`, as its own loop checks).
        // SAFETY: the context the game passed, read guarded; the game's draw, called as its own loop calls it,
        // with an instance `show` made (let go only on this thread, after `DRAWN` is cleared).
        unsafe {
            if read::<u32>(context as usize + 0x18).is_some_and(|passes| passes & 0xf != 0) {
                let draw = std::mem::transmute::<usize, Draw>(DRAW.load(Ordering::Acquire));
                let before = DRAWN_BEFORE.load(Ordering::Acquire);
                for drawn in [before, instance, too].into_iter().filter(|&i| i != 0) {
                    draw(drawn as *mut u8, context);
                }
            }
        }
    });
    result
}

/// What was found in the game's code.
pub(super) struct Game {
    ctor: Ctor,
    vtable: usize,
    spawn: Spawn,
    release: Release,
    set_world: SetWorld,
    find_unit: FindUnit,
    /// Offsets in the owner: the "placed" flag, the node, the look, the instance.
    placed: usize,
    node: usize,
    look: usize,
    instance: usize,
    update: Update,
}

// SAFETY: addresses in the game's executable, used only on its main thread.
unsafe impl Send for Game {}

/// A person of the game's, shown.
pub(super) struct Person {
    /// The owner, the plugin's memory (16-byte aligned), made by the game's constructor.
    owner: Box<[u128; OWNER_ROOM / 16]>,
    /// How far its animation has played (seconds, all it was updated by): where its steps are (`Gait`).
    clock: f32,
    /// Where the game keeps its animation time, looked for in its first updates (`probe`: the log only).
    probe: Option<Box<super::probe::Probe>>,
}

// SAFETY: used only on the game's main thread.
unsafe impl Send for Person {}

/// An animation's steps: its cycle (seconds) and when in it each foot comes down (seconds from its start), so
/// the step's sound is heard as the foot is seen to land (owner 2026-11-05: "the sound isn't in step").
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Gait {
    pub cycle: f32,
    pub steps: [f32; 2],
}

impl Gait {
    /// Whether a foot came down between the animation at `before` and at `now` (seconds of it).
    pub(super) fn stepped(&self, before: f32, now: f32) -> bool {
        now > before
            && self.steps.iter().any(|&at| ((now - at) / self.cycle).floor() > ((before - at) / self.cycle).floor())
    }
}

/// The game's walking people (`walk/walk_01.pma`): a step each 0.75 m at 1.6 m/s, as their sound always was.
const GAME_WALK: Gait = Gait { cycle: 0.9375, steps: [0.0, 0.46875] };
/// The steps of the person walking as `walking` (a `CHARACTER_PEOPLE` name), and running if it has a running
/// animation of its own (the game's people have none: they run with the walking one played faster).
pub(super) fn gaits(walking: &str) -> (Gait, Option<Gait>) {
    profile(walking).map_or((GAME_WALK, None), |person| (person.walk, person.run))
}

/// How high the eyes of the person walking as `walking` are over their feet (metres), where the first-person camera
/// goes (the walking camera's eyes are 1.8 m up).
pub(super) fn eye_height(walking: &str) -> f64 {
    profile(walking).map_or(1.62, |person| person.eyes)
}

/// A jump's animation, played once straight through, the physics' jump starting at its take-off (seconds of it):
/// its length, where it starts, where the feet leave the ground (the physics lifting the body then), and from when
/// on walking cuts it (the feet down). Its time in the air is about the physics' (0.6 s at 3 m/s). It was held at
/// its top while in the air and sent to its landing as the physics landed: frozen poses, "climbing something" and
/// "the cylinder goes up and down, then the animation" (owner's tests, 2026-11-05).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Once {
    pub length: f32,
    pub from: f32,
    pub takeoff: f32,
    pub land: f32,
    pub cut_after: f32,
}

/// How long before its feet come down a jump's animation waits while the physics is still in the air (seconds).
const BEFORE_LANDING: f32 = 1.0 / 30.0;
/// How far short of its end a jump's animation stops (seconds): its end is its loop's start, its first frame.
const END_MARGIN: f32 = 1.0 / 120.0;

/// A crouch's animation (seconds of it): its length, where it's at the bottom (held while the crouch key is), and
/// where it's stood up again (done).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Crouch {
    pub length: f32,
    pub bottom: f32,
    pub up: f32,
}

/// One second of an animation at 30 frames a second, Mixamo's (its frame `f`, counted from 1, is at `frame(f)`).
const fn frame(f: f32) -> f32 {
    (f - 1.0) / 30.0
}

/// Stopping's animation, played as walking or running stops (walking again cuts it): where it starts and its
/// length (seconds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Stop {
    pub from: f32,
    pub length: f32,
}

/// The phone's animations: standing with it (`<prefix>_text`, from its start as the phone opens, looping back to
/// `back`, not taking the phone up again, over its `length`) and walking with it (`<prefix>_txwalk`, looping).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Phone {
    pub back: f32,
    pub length: f32,
}

/// Walking into standing (owner 2026-11-06, "no sudden change to standing": `docs/personagem/TRANSICOES.md`): the
/// engine blends no animations, so one was made in Blender (`tools/personagem/transicao_parar.py`,
/// `<prefix>_settle`): [`PIECES`] pieces of `piece` seconds each, a piece from the walk's pose at that piece of its
/// cycle, held, into standing's first frame. As walking stops, the piece nearest to where the walk was plays (at once:
/// no step more), then standing from its first frame. The first try had the walk go on into standing, after
/// waiting for its next quarter: "two, three steps, as if sliding", with the body already still (owner). Its pieces'
/// length (from its first frame to its last, standing's), and standing's (seconds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Settle {
    pub piece: f32,
    pub idle: f32,
}

/// How many pieces of the walk's cycle have a way into standing ([`Settle`]).
pub(super) const PIECES: usize = 8;

/// Walking into standing as it goes ([`Settle`]): `piece`'s way playing, `at` seconds into it (`started` once its
/// clock is there).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Settling {
    piece: usize,
    at: f32,
    started: bool,
}

/// Where walking stops, its clock at `clock` (seconds) on a cycle of `cycle`: the piece of it nearest.
fn nearest_piece(clock: f32, cycle: f32) -> usize {
    ((clock.rem_euclid(cycle) / cycle * PIECES as f32).round() as usize) % PIECES
}

impl Settle {
    /// Where in the animation `piece`'s way starts (seconds: each piece has its frames, and a frame more between).
    fn start(&self, piece: usize) -> f32 {
        piece as f32 * (self.piece + 1.0 / 30.0)
    }

    /// The whole animation's length (seconds), as the game loops it.
    fn length(&self) -> f32 {
        self.start(PIECES) - 1.0 / 30.0
    }
}

/// One of WestPlay's own people, all the plugin knows of them (`docs/personagem/INCLUIR_PERSONAGEM.md`, step 8):
/// the first part of their movers' names (`mover.<prefix>_walk`, `_idle`, `_run`, `_jump`, `_jumprun`, `_crouch`,
/// `_stop`, `_text`, `_txwalk`: `westplay_setup::own_character`), how high their eyes are (metres: the Head bone's
/// height plus 45% of the way to its top, in Blender), their steps walking and running (each foot down where its
/// heel meets the ground: the toes' height over the frames), and the timings of the animations they have (none,
/// the person does without: standing still for a stop, walking on for the phone). Each must have walking; a mover
/// that isn't in the mod (its file not there) is done without too.
pub(super) struct Profile {
    pub prefix: &'static str,
    pub eyes: f64,
    pub walk: Gait,
    pub run: Option<Gait>,
    pub jump: Option<Once>,
    pub jumprun: Option<Once>,
    pub crouch: Option<Crouch>,
    pub stop: Option<Stop>,
    pub phone: Option<Phone>,
    pub settle: Option<Settle>,
    pub crouch_walk: Option<CrouchWalk>,
}

/// Walking crouched (`<prefix>_cwalk`, looping), the way into it from the knee (`_cstart`) and out of it into the
/// knee (`_csettle`, in pieces as [`Settle`]): its cycle, the way in's last frame (seconds: the walk goes on from the
/// frame after), and the way out's pieces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CrouchWalk {
    pub cycle: f32,
    pub start: f32,
    pub settle: Settle,
}

/// Walking crouched as it goes: into it from the knee (`at` seconds into the way in), walking, or out of it into the
/// knee (the piece nearest to where the walk was, `at` seconds into the whole of the way out).
#[derive(Clone, Copy, Debug, PartialEq)]
enum CrouchMove {
    Starting { at: f32, started: bool },
    Walking,
    Settling { piece: usize, at: f32, started: bool },
}

// The animations WestPlay's people have (Mixamo's, 30 frames a second), their timings: one person's may be
// another's (`tools/personagem/reaproveitar_animacoes.py`), the same frames on another skeleton.

/// The first man's walking ("Walking", 32 frames) and running ("Running", 22), each foot down where its heel meets
/// the ground (measured in Blender: the toes' height over the frames).
const MAN_WALK: Gait = Gait { cycle: 31.0 / 30.0, steps: [0.31, 0.84] };
const MAN_RUN: Gait = Gait { cycle: 21.0 / 30.0, steps: [0.25, 0.59] };
/// The woman's ("Walking", 30 frames; "Running", 22).
const WOMAN_WALK: Gait = Gait { cycle: 29.0 / 30.0, steps: [0.29, 0.77] };
const WOMAN_RUN: Gait = Gait { cycle: 21.0 / 30.0, steps: [0.22, 0.57] };
// No jump standing still (owner 2026-11-06, the standard: "no standing jump, it's not coming out right"): after
// Mixamo's "Jump" and "Jumping", cut and turned many ways (`docs/personagem/TRANSICOES.md` 7.10–7.14), none looked as
// the owner wanted. WestPlay's people jump only on the move (`LEAPING`); standing, Space does nothing.
/// On the move, walking or running (owner 2026-11-05: "on the move it doesn't need the push … land and go on
/// walking"): "Jump" running (31 frames, no push: off the ground from frame 1, the feet down at 21), cut as one
/// lands still moving, else played to its end.
const LEAPING: Once = Once { length: frame(31.0), from: 0.0, takeoff: 0.0, land: frame(21.0), cut_after: frame(21.0) };
/// Crouching (owner 2026-11-06, "smooth, nothing strange"; `tools/personagem/agachar_completo.py`): Mixamo's
/// "Standing To Crouched", down to one knee by frame 20 and held there while Ctrl is, then the same backwards, up by
/// 40, both ends standing's first frame (the "Air Squat Bent Arms" before it had the arms out in front, "open").
const SQUATTING: Crouch = Crouch { length: frame(40.0), bottom: frame(20.0), up: frame(40.0) };
/// Walking crouched: Mixamo's "Crouched Walking" (32 frames, its feet at 1.24 m/s), from the knee into it in 10
/// frames (`cstart`, the legs already stepping, the walk going on from its 11th), into the knee from the nearest
/// eighth of its cycle in 10 (`csettle`, as walking into standing: `Settle`).
const CROUCH_WALKING: CrouchWalk =
    CrouchWalk { cycle: 31.0 / 30.0, start: frame(10.0), settle: Settle { piece: frame(10.0), idle: 0.0 } };
/// "Female Stop Walking" (47 frames, held in place in Blender): only its end, from frame 31, the last foot just
/// down (its three steps before, 1.1 s, would walk on the spot: "slides on the spot", 2026-11-05).
const STOPPING: Stop = Stop { from: frame(31.0), length: frame(47.0) };
/// "Texting" (557 frames: the phone up to the chest by frame 31, then typing) and "Texting And Walking", the
/// phone in the right hand (the person's phone models).
const TEXTING: Phone = Phone { back: frame(31.0), length: frame(557.0) };
/// Walking into standing: pieces of 10 frames (from the first to the last: 0.3 s), into the first man's standing
/// ("Standing W_Briefcase Idle", 419 frames) or the woman's ("Idle", 500).
const MAN_SETTLE: Settle = Settle { piece: frame(10.0), idle: frame(419.0) };
const WOMAN_SETTLE: Settle = Settle { piece: frame(10.0), idle: frame(500.0) };

/// WestPlay's own people, in the order the player chooses among them (`westplay_shared::CHARACTER_PEOPLE`).
pub(super) const PEOPLE: [Profile; 3] = [
    // The tattooed man (`wt_`, owner 2026-11-06, in place of the first man, `wp_`): the first man's walking,
    // running and standing (a man's), the woman's other animations (`reaproveitar_animacoes.py`).
    Profile {
        prefix: "wt",
        eyes: 1.64,
        walk: MAN_WALK,
        run: Some(MAN_RUN),
        jump: None,
        jumprun: Some(LEAPING),
        crouch: Some(SQUATTING),
        stop: Some(STOPPING),
        phone: Some(TEXTING),
        settle: Some(MAN_SETTLE),
        crouch_walk: Some(CROUCH_WALKING),
    },
    // The woman (`wm_`, owner 2026-11-05).
    Profile {
        prefix: "wm",
        eyes: 1.56,
        walk: WOMAN_WALK,
        run: Some(WOMAN_RUN),
        jump: None,
        jumprun: Some(LEAPING),
        crouch: Some(SQUATTING),
        stop: Some(STOPPING),
        phone: Some(TEXTING),
        settle: Some(WOMAN_SETTLE),
        crouch_walk: Some(CROUCH_WALKING),
    },
    // The cowboy (`wc_`, owner 2026-11-06): his own Mixamo rig ("3 Chain Fingers": the ring fingers turn as the
    // index, `anelar_como_indicador.py`), at the man's height, with all the tattooed man's animations
    // (`reaproveitar_animacoes.py`): his timings.
    Profile {
        prefix: "wc",
        eyes: 1.64,
        walk: MAN_WALK,
        run: Some(MAN_RUN),
        jump: None,
        jumprun: Some(LEAPING),
        crouch: Some(SQUATTING),
        stop: Some(STOPPING),
        phone: Some(TEXTING),
        settle: Some(MAN_SETTLE),
        crouch_walk: Some(CROUCH_WALKING),
    },
];

/// The profile of the person walking as `walking` (`<prefix>_walk`), if they're one of WestPlay's own.
pub(super) fn profile(walking: &str) -> Option<&'static Profile> {
    PEOPLE.iter().find(|person| walking.strip_suffix("_walk") == Some(person.prefix))
}

/// The player's body: the person standing, the same person walking (and running, jumping, if it has animations
/// for them), all kept placed and updated, the one that fits drawn (switching at once: all ready).
pub(super) struct Body {
    standing: Option<Person>,
    walking: Option<Person>,
    running: Option<Person>,
    jumping: Option<(Person, Once)>,
    leaping: Option<(Person, Once)>,
    /// How far into the jump it is (seconds of its animation) while it plays, whether it has started, and whether
    /// it's the jump on the move (`leaping`).
    jump: Option<(f32, bool, bool)>,
    crouching: Option<(Person, Crouch)>,
    /// The stop (its start and length), how far into it while it plays and whether started, and whether the person
    /// was walking last frame (a stop plays as it stops).
    stopping: Option<(Person, f32, f32)>,
    /// The phone's people (standing, its loop start and length; walking), and how far into standing with it.
    texting: Option<(Person, f32, f32)>,
    txwalking: Option<Person>,
    text: Option<(f32, bool)>,
    stop: Option<(f32, bool)>,
    was_moving: bool,
    /// How far into the crouch it is (seconds of its animation) while it plays, and whether it has started.
    crouch: Option<(f32, bool)>,
    /// Walking crouched ([`CrouchWalk`]): its people (walking, the way in, the way out: all, or none), its timings,
    /// and where in it it is.
    crouch_walking: Option<(Person, Person, Person, CrouchWalk)>,
    crouch_move: Option<CrouchMove>,
    /// Walking into standing ([`Settle`]): its person and timings, and where in it it is.
    settling: Option<(Person, Settle)>,
    /// Two running people made alike, never drawn, for the log (`probe::Twins`), once a session.
    twins: Option<(Person, Person, super::probe::Twins)>,
    settle: Option<Settling>,
    gaits: (Gait, Option<Gait>),
    /// Whether it's one of WestPlay's own people (`PEOPLE`), not one of the game's.
    own: bool,
    /// Which one was shown last (0 standing, 1 walking, 2 running, 3 jumping, 4 leaping, 5 crouching, 6 stopping, 7 with the phone, 8 walking with it), for the log.
    shown: Option<usize>,
    /// Frames the one shown before the last switch is still drawn (`DRAWN_BEFORE`).
    overlap: u8,
}

impl Body {
    /// For the log: the standing person's owner, model unit and mover instance.
    pub(super) fn parts(&mut self, game: &Game) -> (usize, usize, usize) {
        self.standing.as_mut().or(self.walking.as_mut()).map_or((0, 0, 0), |person| person.parts(game))
    }

    /// Whether it runs with a running animation of its own (else the walking one, played faster).
    pub(super) fn runs(&self) -> bool {
        self.running.is_some()
    }

    /// How many people it has made (each moved and updated every frame, shown or not), for the log.
    pub(super) fn people(&self) -> usize {
        [
            self.standing.is_some(),
            self.walking.is_some(),
            self.running.is_some(),
            self.jumping.is_some(),
            self.leaping.is_some(),
            self.crouching.is_some(),
            self.stopping.is_some(),
            self.texting.is_some(),
            self.txwalking.is_some(),
            self.settling.is_some(),
        ]
        .into_iter()
        .filter(|&made| made)
        .count()
    }

    /// Whether it has a jump of its own (standing, or `moving`).
    pub(super) fn jumps(&self, moving: bool) -> bool {
        self.jumping.is_some() || (moving && self.leaping.is_some())
    }

    /// Whether it's walking crouched now (or going into it from the knee): the eyes higher than on the knee.
    pub(super) fn walks_crouched(&self) -> bool {
        matches!(self.crouch_move, Some(CrouchMove::Starting { .. } | CrouchMove::Walking))
    }

    /// Whether Space does nothing now: one of WestPlay's own people standing still, who has no jump standing (the
    /// standard: they jump only on the move, owner 2026-11-06). The game's people jump as ever, with no animation.
    pub(super) fn stays_down(&self, moving: bool) -> bool {
        self.own && !moving && self.jumping.is_none()
    }

    /// Whether a jump's animation is playing.
    pub(super) fn jump_playing(&self) -> bool {
        self.jump.is_some()
    }

    /// Whether the jump playing has come to its feet leaving the ground (the physics' jump goes then).
    pub(super) fn taking_off(&self) -> bool {
        let Some((at, _, leap)) = self.jump else { return false };
        let once = if leap { self.leaping.as_ref() } else { self.jumping.as_ref() };
        once.is_some_and(|(_, once)| at >= once.takeoff)
    }

    /// A jump begins (Space): its animation from its start, if it has one; `moving`, the jump on the move.
    pub(super) fn jump(&mut self, moving: bool) {
        let leap = moving && self.leaping.is_some();
        if let Some((_, once)) = if leap { &self.leaping } else { &self.jumping } {
            self.jump = Some((once.from, false, leap));
        }
    }

    /// The animation drawn moving (`running` or walking): which (1 running, 0 walking), how far it has played and
    /// its steps.
    pub(super) fn steps(&self, running: bool) -> Option<(u8, f32, Gait)> {
        match (running, &self.running, self.gaits.1) {
            (true, Some(run), Some(gait)) => Some((1, run.clock, gait)),
            _ => self.walking.as_ref().map(|walk| (0, walk.clock, self.gaits.0)),
        }
    }
}

impl Person {
    fn instance(&mut self, game: &Game) -> usize {
        // SAFETY: a read guarded, within the owner's room.
        unsafe { read::<usize>(self.at() as usize + game.instance) }.unwrap_or(0)
    }

    fn at(&mut self) -> *mut u8 {
        self.owner.as_mut_ptr().cast()
    }

    /// For the log: the owner, its model unit and its mover instance (addresses).
    pub(super) fn parts(&mut self, game: &Game) -> (usize, usize, usize) {
        let owner = self.at() as usize;
        // SAFETY: reads guarded, within the owner's room.
        let read_at = |off: usize| unsafe { read::<usize>(owner + off) }.unwrap_or(0);
        (owner, read_at(0xf8), read_at(game.instance))
    }
}

/// Finds the pieces; logs what wasn't found.
pub(super) fn locate(image: &Image, base: usize) -> Option<Game> {
    let found = (|| {
        let u32_at = |at: usize| image.code_at(at, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
        let within = |text: &str, from: usize, len: usize| image.unique_masked_in(&pattern(text), from, from + len);
        let ctor = image.unique_masked(&pattern(OWNER_CTOR))?;
        let vtable = rel(image, ctor + 13).filter(|&v| image.in_constants(v))?;
        // The owner's virtuals as known: the model's getter (`mov rax, [rcx+0f8h]; ret`), and the destructor's
        // size fitting the room.
        // SAFETY: the vtable lies in the game's constants (checked), read guarded.
        let slot = |i: usize| unsafe { read::<usize>(vtable + i * 8) }.filter(|&f| image.in_code(f));
        if image.code_at(slot(5)?, 8)? != [0x48, 0x8b, 0x81, 0xf8, 0x00, 0x00, 0x00, 0xc3] {
            return None;
        }
        let dtor = image.code_at(slot(0)?, 0x30)?;
        let size = dtor.windows(5).find(|w| w[0] == 0xba).map(|w| u32::from_le_bytes(w[1..5].try_into().unwrap()));
        if !size.is_some_and(|s| (0x100..=OWNER_ROOM as u32).contains(&s)) {
            return None;
        }
        let spawn = image.unique_masked(&pattern(SPAWN))?;
        let placed = u32_at(spawn + 20)?;
        let attach_site = within(SPAWN_ATTACH, spawn, 0x160)?;
        let (instance, node) = (u32_at(attach_site + 3)?, u32_at(attach_site + 10)?);
        let attach = rel(image, attach_site + 15).filter(|&f| image.in_code(f))?;
        let release = rel(image, within(SPAWN_RELEASE, spawn, 0x160)? + 4).filter(|&f| image.in_code(f))?;
        let set_world = rel(image, within(ATTACH_WORLD, attach, 0x60)? + 14).filter(|&f| image.in_code(f))?;
        let format = image.constant(b"mover.%s\0")?;
        let mut finds = image
            .all_masked(&pattern(FIND_UNIT))
            .into_iter()
            .filter(|&at| rel(image, at + 3) == Some(format))
            .filter_map(|at| rel(image, at + 21));
        let find_unit = finds.next().filter(|&f| image.in_code(f))?;
        if finds.next().is_some() || ![placed, node, instance].iter().all(|&o| o < 0x200) {
            return None;
        }
        let update = image.unique_masked(&pattern(UPDATE))?;
        if u32_at(update + 18)? != MODEL_INSTANCE {
            return None;
        }
        let draw_site = image.unique_masked(&pattern(DRAW_SITE))?;
        let draw = rel(image, draw_site + 16).filter(|&f| image.in_code(f))?;
        let gather_fn = image.function_start(draw_site)?;
        if !fits(image.code_at(gather_fn, 20)?, &pattern(GATHER)) {
            return None;
        }
        DRAW.store(draw, Ordering::Release);
        if GATHER_ORIGINAL.load(Ordering::Acquire) == 0 {
            // SAFETY: the game's own function (its start, checked), hooked with a detour of its signature that
            // calls on through the trampoline.
            let hooked = unsafe {
                crate::hooks::hook_now(gather_fn, gather as *mut c_void, &GATHER_ORIGINAL, "the world's drawing")
            };
            if let Err(why) = hooked {
                log!("character: {why}");
                return None;
            }
        }
        // The look follows the model, as the owner's own getters keep them (`+0xf8`, `+0x100`).
        let look = 0x100;
        log!(
            "character: found the game's people (owner +{:#x}, spawn +{:#x}, set_world +{:#x})",
            ctor - base,
            spawn - base,
            set_world - base
        );
        // SAFETY: each is the game's code where its own calls go, of the type its call sites give.
        Some(unsafe {
            Game {
                ctor: std::mem::transmute::<usize, Ctor>(ctor),
                vtable,
                spawn: std::mem::transmute::<usize, Spawn>(spawn),
                release: std::mem::transmute::<usize, Release>(release),
                set_world: std::mem::transmute::<usize, SetWorld>(set_world),
                find_unit: std::mem::transmute::<usize, FindUnit>(find_unit),
                placed,
                node,
                look,
                instance,
                update: std::mem::transmute::<usize, Update>(update),
            }
        })
    })();
    if found.is_none() {
        log!("character: the game's people code isn't the version WestPlay knows (no person shown)");
    }
    found
}

/// Makes the chosen person at `placement` (`person`: a place in `westplay_shared::CHARACTER_PEOPLE`); the game's
/// own (`CHARACTER_GAME_PERSON`) if the chosen one isn't in this game (their files not on the PC). In `first`
/// person, the chosen one's headless body (`headless`), or none (the game's person has its head).
///
/// # Safety
/// On the game's main thread, `game` as `locate` found it.
pub(super) unsafe fn show(
    game: &Game,
    placement: &Placement,
    person: usize,
    first: bool,
) -> Result<Body, &'static str> {
    use westplay_shared::{CHARACTER_GAME_PERSON, CHARACTER_PEOPLE, CHARACTER_PERSON};
    // SAFETY: as `show_one` (the caller's contract).
    unsafe {
        let chosen = CHARACTER_PEOPLE.get(person).copied().unwrap_or(CHARACTER_PEOPLE[CHARACTER_PERSON]);
        let body = show_as(game, placement, chosen, first);
        if body.is_err() && first {
            log!("character: no headless body of {chosen} (its files aren't on the PC): none in first person");
            return body;
        }
        if body.is_err() {
            log!("character: {chosen} isn't in this game (its files aren't on the PC): the game's own person");
            return show_as(game, placement, CHARACTER_GAME_PERSON, false);
        }
        body
    }
}

/// The person walking as `walking` (a `mover_model`'s name) and, if the game has one, standing as the same model;
/// `first`, their headless body's movers (`headless`).
///
/// # Safety
/// As `show`.
unsafe fn show_as(game: &Game, placement: &Placement, walking: &str, first: bool) -> Result<Body, &'static str> {
    let mover = |name: &str| unit_name(&if first { headless(name) } else { name.to_owned() });
    // SAFETY: as `show_one` (the caller's contract).
    unsafe {
        let own = profile(walking);
        // One of WestPlay's own people's animations, by its name (`walk` → `wm_walk`).
        let named = |animation: &str| own.map(|person| format!("{}_{animation}", person.prefix));
        let standing_names: Vec<Vec<u8>> = match named("idle") {
            Some(idle) => vec![mover(&idle)],
            None => STANDING
                .iter()
                .find(|(walk, _)| *walk == walking)
                .map_or(&[][..], |(_, names)| names)
                .iter()
                .map(|name| mover(name))
                .collect(),
        };
        let mut standing = if standing_names.is_empty() {
            None
        } else {
            show_one(game, placement, &standing_names)
                .inspect_err(|why| log!("character: no standing person (walking only): {why}"))
                .ok()
        };
        let gaits = gaits(walking);
        let running_name = own.and_then(|person| person.run.and(named("run")));
        let jumping_name = own.and_then(|person| Some((named("jump")?, person.jump?)));
        let leaping_name = own.and_then(|person| Some((named("jumprun")?, person.jumprun?)));
        let crouching_name = own.and_then(|person| Some((named("crouch")?, person.crouch?)));
        let phoning = own.and_then(|person| Some((named("text")?, named("txwalk")?, person.phone?)));
        let stopping_name = own.and_then(|person| Some((named("stop")?, person.stop?)));
        let walking = show_one(game, placement, &[mover(walking)])
            .inspect_err(|why| log!("character: no walking person ({walking}): {why}"))
            .ok();
        let Some(walking) = walking else {
            // Walking is the chosen one: without it, this one isn't there.
            if let Some(standing) = standing.take() {
                hide_one(game, standing);
            }
            return Err("the chosen person isn't in this game");
        };
        let running = running_name.and_then(|name| {
            show_one(game, placement, &[mover(&name)])
                .inspect_err(|why| log!("character: no running person ({name}, the walking one faster): {why}"))
                .ok()
        });
        let once_person = |(name, once): (String, Once)| {
            show_one(game, placement, &[mover(&name)])
                .inspect_err(|why| log!("character: no jumping person ({name}): {why}"))
                .ok()
                .map(|person| (person, once))
        };
        let jumping = jumping_name.and_then(once_person);
        let leaping = leaping_name.and_then(once_person);
        let phone_person = |name: &str| {
            show_one(game, placement, &[mover(name)])
                .inspect_err(|why| log!("character: no person with the phone ({name}): {why}"))
                .ok()
        };
        let texting =
            phoning.as_ref().and_then(|(text, _, phone)| phone_person(text).map(|p| (p, phone.back, phone.length)));
        let txwalking = phoning.as_ref().and_then(|(_, walk, _)| phone_person(walk));
        let stopping = stopping_name.and_then(|(name, stop)| {
            show_one(game, placement, &[mover(&name)])
                .inspect_err(|why| log!("character: no stopping person ({name}): {why}"))
                .ok()
                .map(|person| (person, stop.from, stop.length))
        });
        let crouching = crouching_name.and_then(|(name, crouch)| {
            show_one(game, placement, &[mover(&name)])
                .inspect_err(|why| log!("character: no crouching person ({name}): {why}"))
                .ok()
                .map(|person| (person, crouch))
        });
        let settling = own.and_then(|person| Some((named("settle")?, person.settle?))).and_then(|(name, settle)| {
            show_one(game, placement, &[mover(&name)])
                .inspect_err(|why| log!("character: no way into standing ({name}): {why}"))
                .ok()
                .map(|person| (person, settle))
        });
        // Walking crouched: its three people, all or none.
        let crouch_walking = own.and_then(|person| person.crouch_walk).and_then(|timing| {
            let mut made = Vec::new();
            for animation in ["cwalk", "cstart", "csettle"] {
                let name = named(animation)?;
                match show_one(game, placement, &[mover(&name)]) {
                    Ok(person) => made.push(person),
                    Err(why) => {
                        log!("character: no walking crouched ({name}): {why}");
                        for person in made {
                            hide_one(game, person);
                        }
                        return None;
                    }
                }
            }
            let settler = made.pop()?;
            let starter = made.pop()?;
            let walker = made.pop()?;
            Some((walker, starter, settler, timing))
        });
        // Twins of the running person, once a session, for the log (`probe::Twins`).
        let twins = if TWINS_MADE.swap(true, Ordering::AcqRel) {
            None
        } else {
            named("run").and_then(|name| {
                let a = show_one(game, placement, &[mover(&name)]).ok()?;
                let Ok(b) = show_one(game, placement, &[mover(&name)]) else {
                    hide_one(game, a);
                    return None;
                };
                let (mut a, mut b) = (a, b);
                a.probe = None;
                b.probe = None;
                match super::probe::Twins::new(a.instance(game), b.instance(game)) {
                    Some(twins) => Some((a, b, twins)),
                    None => {
                        hide_one(game, a);
                        hide_one(game, b);
                        None
                    }
                }
            })
        };
        let mut body = Body {
            standing,
            walking: Some(walking),
            running,
            jumping,
            leaping,
            jump: None,
            crouching,
            crouch: None,
            settling,
            crouch_walking,
            crouch_move: None,
            twins,
            settle: None,
            stopping,
            stop: None,
            texting,
            txwalking,
            text: None,
            was_moving: false,
            gaits,
            own: own.is_some(),
            shown: None,
            overlap: 0,
        };
        let drawn = match body.standing.as_mut() {
            Some(standing) => standing.instance(game),
            None => body.walking.as_mut().map_or(0, |walking| walking.instance(game)),
        };
        DRAWN.store(drawn, Ordering::Release);
        Ok(body)
    }
}

/// Makes one of the game's people at `placement`, the first of `models` found (unit names, NUL-ended).
///
/// # Safety
/// On the game's main thread, `game` as `locate` found it.
unsafe fn show_one(game: &Game, placement: &Placement, models: &[Vec<u8>]) -> Result<Person, &'static str> {
    // SAFETY: the game's own functions, called as its passenger code calls them, on an owner its constructor
    // made in the plugin's memory (room checked against its size).
    unsafe {
        let mut person = Person { owner: Box::new([0; OWNER_ROOM / 16]), clock: 0.0, probe: None };
        let owner = person.at();
        (game.ctor)(owner);
        if read::<usize>(owner as usize) != Some(game.vtable) {
            return Err("the owner isn't as the game makes it");
        }
        (game.set_world)(owner.add(game.node), placement);
        owner.add(game.placed).write(1);
        let model = models.iter().find_map(|name| {
            let text = name.as_ptr();
            let unit = (game.find_unit)(&text);
            (!unit.is_null()).then(|| (unit, String::from_utf8_lossy(&name[..name.len() - 1]).into_owned()))
        });
        let Some((model, name)) = model else {
            drop_owner(game, &mut person);
            return Err("none of the game's people models found");
        };
        (game.spawn)(owner, model, owner.add(game.look).cast());
        if read::<usize>(owner as usize + game.instance).is_none_or(|i| i == 0) {
            drop_owner(game, &mut person);
            return Err("the game made no person (its log says why)");
        }
        log!("character: a person of the game's shown ({name})");
        // Standing and running go on every update: where the game keeps their animation time (the log only).
        if name.ends_with("_idle") || name.ends_with("_run") {
            person.probe = read::<usize>(owner as usize + game.instance)
                .and_then(|instance| super::probe::Probe::new(&name, instance, name.ends_with("_run")))
                .map(Box::new);
        }
        Ok(person)
    }
}

/// Puts the person at `placement`: standing still, the standing one if there is one, else the walking one with
/// its animation stopped (updated by no time); moving, the walking one, or the running one `running` if it has
/// one.
///
/// # Safety
/// On the game's main thread, `person` as `show` made it.
pub(super) unsafe fn place(
    game: &Game,
    body: &mut Body,
    placement: &Placement,
    seconds: f32,
    moving: bool,
    running: bool,
    airborne: bool,
    crouch_held: bool,
    phone: bool,
) {
    // The jump, played once (`Once`): from its start (the jumping one's clock brought there the first frame: its
    // animation loops over its length), the physics' jump going as its feet leave the ground (`Body::taking_off`);
    // in the air for longer than it is, it waits just before its feet come down (the legs down, as one falling),
    // and lands as the physics does (sent on to its feet down if the physics lands first); cut by walking once the
    // feet are down, done at its end. Not jumping, its clock stays.
    let mut jump_by = 0.0;
    let mut jumping = None;
    // Standing's clock sent to its first frame this frame (`stand_by`): as a jump that ends in its pose ends, or the
    // way into standing (`Settle`).
    let mut stand_by = None;
    let jumper = |leap: bool| if leap { body.leaping.as_ref() } else { body.jumping.as_ref() };
    if let Some((at, going, leap)) = body.jump
        && let Some((jumper, once)) = jumper(leap)
    {
        // Its end is its loop's start: never sent past it (the owner, 2026-11-06: at the jump's end "as if she
        // reset the animation, she opens her arms": its first frame, drawn as it switched to standing).
        let last = once.length - END_MARGIN;
        if at >= last || (moving && at > once.cut_after) {
            body.jump = None;
            // Ended standing (not cut walking): standing from its first frame, the pose the jump ends in
            // (`tools/personagem/emendar_no_parado.py`).
            if at >= last
                && !moving
                && let (Some(standing), Some((_, settle))) = (body.standing.as_ref(), body.settling.as_ref())
            {
                stand_by = Some((-standing.clock).rem_euclid(settle.idle));
            }
        } else {
            jumping = Some(leap);
            let up = at >= once.takeoff;
            let next = if !going {
                at
            } else if up && airborne {
                (at + seconds).min((once.land - BEFORE_LANDING).max(at))
            } else if up && at < once.land {
                once.land
            } else {
                at + seconds
            }
            .min(last);
            jump_by = if going { next - at } else { (at - jumper.clock).rem_euclid(once.length) };
            body.jump = Some((next, true, leap));
        }
    }
    // The crouch, while the crouch key is held (`Crouch`): down to its bottom and held there, up once let go, done
    // when stood; let go or held again halfway, turned round there. Not while jumping.
    let mut crouch_by = 0.0;
    let mut crouched = false;
    // Ctrl let go walking crouched: up at once, walking (standing up on the spot would slide along).
    if !crouch_held && moving && body.crouch_move.is_some() {
        body.crouch = None;
        body.crouch_move = None;
    }
    if let Some((croucher, crouch)) = body.crouching.as_ref() {
        if body.crouch.is_none() && crouch_held && jumping.is_none() {
            body.crouch = Some((0.0, false));
        }
        if let Some((was, going)) = body.crouch {
            // Turned round where it is, the way up being the way down backwards (`SQUATTING`): let go on the way
            // down, up from the same pose; held again on the way up, down from it (owner 2026-11-06: a quick press
            // "bugged the animation", down all the way before up, and pressed again standing up, nothing).
            let turned = going && (crouch_held && was > crouch.bottom || !crouch_held && was < crouch.bottom);
            let at = if turned { crouch.length - was } else { was };
            let next = if !going {
                at
            } else if crouch_held && at <= crouch.bottom {
                (at + seconds).min(crouch.bottom)
            } else {
                at + seconds
            };
            if going && next >= crouch.up {
                body.crouch = None;
                // Up: standing from its first frame, the pose the crouch ends in.
                if let (Some(standing), Some((_, settle))) = (body.standing.as_ref(), body.settling.as_ref()) {
                    stand_by = Some((-standing.clock).rem_euclid(settle.idle));
                }
            } else {
                crouched = true;
                crouch_by = if going {
                    (next - was).rem_euclid(crouch.length)
                } else {
                    (at - croucher.clock).rem_euclid(crouch.length)
                };
                body.crouch = Some((next, true));
            }
        }
    }
    // Walking crouched (`CrouchWalk`), down on the knee with Ctrl held: moving, from the knee into the walk, then
    // walking; still again, from the nearest eighth of its cycle into the knee. `crouch_role`: which of them shows.
    let (mut cwalk_by, mut cstart_by, mut csettle_by) = (0.0, 0.0, 0.0);
    let mut crouch_role = None;
    if let (Some((walker, starter, settler, timing)), Some((_, crouch))) =
        (body.crouch_walking.as_ref(), body.crouching.as_ref())
    {
        let down = crouched && crouch_held && body.crouch.is_some_and(|(at, _)| at >= crouch.bottom);
        let settle = timing.settle;
        body.crouch_move = match (down, body.crouch_move) {
            (false, _) => None,
            (true, None) if moving => {
                // Its first frame (its loop is its length: its last frame is at `start`).
                cstart_by = (-starter.clock).rem_euclid(timing.start);
                crouch_role = Some(10);
                Some(CrouchMove::Starting { at: 0.0, started: true })
            }
            (true, None) => None,
            (true, Some(CrouchMove::Starting { at, .. })) if at + seconds >= timing.start => {
                // Into the walk at the frame after the way in's last.
                cwalk_by = (timing.start + 1.0 / 30.0 - walker.clock).rem_euclid(timing.cycle);
                crouch_role = Some(11);
                Some(CrouchMove::Walking)
            }
            (true, Some(CrouchMove::Starting { at, .. })) => {
                cstart_by = seconds;
                crouch_role = Some(10);
                Some(CrouchMove::Starting { at: at + seconds, started: true })
            }
            (true, Some(CrouchMove::Walking)) if !moving => {
                let piece = nearest_piece(walker.clock, timing.cycle);
                let at = settle.start(piece);
                csettle_by = (at - settler.clock).rem_euclid(settle.length());
                crouch_role = Some(12);
                Some(CrouchMove::Settling { piece, at, started: true })
            }
            (true, Some(CrouchMove::Walking)) => {
                cwalk_by = seconds;
                crouch_role = Some(11);
                Some(CrouchMove::Walking)
            }
            (true, Some(CrouchMove::Settling { .. })) if moving => {
                cwalk_by = seconds;
                crouch_role = Some(11);
                Some(CrouchMove::Walking)
            }
            (true, Some(CrouchMove::Settling { piece, at, .. }))
                if at + seconds >= settle.start(piece) + settle.piece =>
            {
                // On the knee: the crouch's own person, held at its bottom (its pose).
                None
            }
            (true, Some(CrouchMove::Settling { piece, at, started })) => {
                csettle_by = seconds;
                crouch_role = Some(12);
                Some(CrouchMove::Settling { piece, at: at + seconds, started })
            }
        };
    }
    // Walking into standing (`Settle`): walking (not running) stops, the phone away, not jumping nor crouching: the
    // piece nearest to where the walk was plays at once, then standing from its first frame (`stand_by`: standing's
    // clock sent there). Moving again cuts it, the walk on from where it was.
    let mut settle_by = 0.0;
    let mut settling_now = false;
    if let (Some((settler, settle)), Some(walk)) = (body.settling.as_ref(), body.walking.as_ref()) {
        let held = phone || jumping.is_some() || crouch_held;
        if body.was_moving && !moving && !running && !held && body.settle.is_none() {
            let piece = nearest_piece(walk.clock, body.gaits.0.cycle);
            body.settle = Some(Settling { piece, at: settle.start(piece), started: false });
        }
        match body.settle {
            Some(_) if moving || held => body.settle = None,
            Some(Settling { piece, at, started }) => {
                if started && at + seconds >= settle.start(piece) + settle.piece {
                    body.settle = None;
                    if let Some(standing) = body.standing.as_ref() {
                        stand_by = Some((-standing.clock).rem_euclid(settle.idle));
                    }
                } else {
                    settle_by = if started { seconds } else { (at - settler.clock).rem_euclid(settle.length()) };
                    body.settle = Some(Settling { piece, at: if started { at + seconds } else { at }, started: true });
                    settling_now = true;
                }
            }
            None => {}
        }
    }
    // The stop, as walking or running stops (`STOPPING`): from its start to its end, cut by walking again, a jump or
    // a crouch.
    let mut stop_by = 0.0;
    let mut stopped = false;
    if let Some((stopper, from, length)) = body.stopping.as_ref() {
        if body.was_moving && !moving && jumping.is_none() && !crouched && body.settle.is_none() && stand_by.is_none() {
            body.stop = Some((*from, false));
        }
        if let Some((at, going)) = body.stop {
            if moving || jumping.is_some() || crouched || (going && at + seconds >= *length) {
                body.stop = None;
            } else {
                stopped = true;
                stop_by = if going { seconds } else { (at - stopper.clock).rem_euclid(*length) };
                body.stop = Some((if going { at + seconds } else { at }, true));
            }
        }
    }
    body.was_moving = moving;
    // The phone out (`phone`), not jumping nor crouching: standing with it (taking it up first), or walking with it
    // (not running).
    let mut text_by = 0.0;
    let with_phone = phone && jumping.is_none() && !crouched;
    let walking_with_phone = with_phone && moving && !running && body.txwalking.is_some();
    let mut texting = false;
    if let Some((texter, back, length)) = body.texting.as_ref() {
        if with_phone && !moving {
            let (at, going) = body.text.unwrap_or((0.0, false));
            let mut next = if going { at + seconds } else { at };
            if next >= *length {
                next = next - length + back;
            }
            text_by = if going { (next - at).rem_euclid(*length) } else { (at - texter.clock).rem_euclid(*length) };
            body.text = Some((next, true));
            texting = true;
        } else if !with_phone {
            body.text = None;
        }
    }
    // Which one is in use: jumping (standing or on the move), crouching, running, walking, or standing (walking if
    // there's no standing one, its animation stopped).
    let role: usize = if let Some(leap) = jumping {
        if leap { 4 } else { 3 }
    } else if crouched {
        crouch_role.unwrap_or(5)
    } else if texting {
        7
    } else if walking_with_phone {
        8
    } else if settling_now {
        9
    } else if stopped {
        6
    } else if moving && running && body.running.is_some() {
        2
    } else if (moving || body.standing.is_none()) && body.walking.is_some() {
        1
    } else {
        0
    };
    // SAFETY: as `place_one` (the caller's contract).
    unsafe {
        let standing = body.standing.as_mut().map(|standing| {
            place_one(game, standing, placement, stand_by.unwrap_or(seconds));
            standing.instance(game)
        });
        let still = if standing.is_none() && !moving { 0.0 } else { seconds };
        let walking = body.walking.as_mut().map(|walking| {
            place_one(game, walking, placement, still);
            walking.instance(game)
        });
        let run = body.running.as_mut().map(|run| {
            place_one(game, run, placement, seconds);
            run.instance(game)
        });
        // Only the jump playing goes on; the other stays where it is.
        let jump = body.jumping.as_mut().map(|(jumper, _)| {
            place_one(game, jumper, placement, if jumping == Some(false) { jump_by } else { 0.0 });
            jumper.instance(game)
        });
        let leap = body.leaping.as_mut().map(|(leaper, _)| {
            place_one(game, leaper, placement, if jumping == Some(true) { jump_by } else { 0.0 });
            leaper.instance(game)
        });
        let crouch = body.crouching.as_mut().map(|(croucher, _)| {
            place_one(game, croucher, placement, crouch_by);
            croucher.instance(game)
        });
        let stop = body.stopping.as_mut().map(|(stopper, ..)| {
            place_one(game, stopper, placement, stop_by);
            stopper.instance(game)
        });
        let text = body.texting.as_mut().map(|(texter, ..)| {
            place_one(game, texter, placement, text_by);
            texter.instance(game)
        });
        let txwalk = body.txwalking.as_mut().map(|walker| {
            place_one(game, walker, placement, if walking_with_phone { seconds } else { 0.0 });
            walker.instance(game)
        });
        // The ways into standing: only the one playing goes on.
        let settle = body.settling.as_mut().map(|(settler, _)| {
            place_one(game, settler, placement, settle_by);
            settler.instance(game)
        });
        // Walking crouched: its three people, each going on only as it plays.
        let (mut cwalk, mut cstart, mut csettle) = (None, None, None);
        if let Some((walker, starter, settler, _)) = body.crouch_walking.as_mut() {
            place_one(game, walker, placement, cwalk_by);
            place_one(game, starter, placement, cstart_by);
            place_one(game, settler, placement, csettle_by);
            (cwalk, cstart, csettle) =
                (Some(walker.instance(game)), Some(starter.instance(game)), Some(settler.instance(game)));
        }
        // The twins (never drawn, the log only: `probe::Twins`), let go once done.
        let mut twins_done = false;
        if let Some((a, b, twins)) = body.twins.as_mut() {
            match twins.next() {
                super::probe::TwinStep::Same => {
                    place_one(game, a, placement, seconds);
                    place_one(game, b, placement, seconds);
                    twins.after();
                }
                super::probe::TwinStep::Big => {
                    for _ in 0..60 {
                        place_one(game, a, placement, super::probe::BIG / 60.0);
                    }
                    place_whole(game, b, placement, super::probe::BIG);
                    twins.after();
                }
                super::probe::TwinStep::Done => twins_done = true,
            }
        }
        if twins_done && let Some((a, b, _)) = body.twins.take() {
            hide_one(game, a);
            hide_one(game, b);
        }
        let all = [standing, walking, run, jump, leap, crouch, stop, text, txwalk, settle, cstart, cwalk, csettle];
        DRAWN.store(all[role].unwrap_or(0), Ordering::Release);
        // Each switch of the person shown, in the log. One a stop (the owner's test, 2026-11-05): the blink is the
        // switch itself, the one switched to drawn wrong its first frame. So the one switched from is drawn too for
        // `OVERLAP` frames, over it.
        if body.shown != Some(role) {
            if let Some(was) = body.shown {
                DRAWN_BEFORE.store(all[was].unwrap_or(0), Ordering::Release);
                body.overlap = OVERLAP;
            }
            body.shown = Some(role);
            log!(
                "character: shown {}",
                [
                    "standing",
                    "walking",
                    "running",
                    "jumping",
                    "leaping",
                    "crouching",
                    "stopping",
                    "with the phone",
                    "walking with the phone",
                    "settling",
                    "starting to walk crouched",
                    "walking crouched",
                    "stopping crouched"
                ][role]
            );
        } else if body.overlap > 0 {
            body.overlap -= 1;
        } else {
            DRAWN_BEFORE.store(0, Ordering::Release);
        }
    }
}

/// Puts one person at `placement` and updates it.
///
/// # Safety
/// On the game's main thread, `person` as `show_one` made it.
unsafe fn place_one(game: &Game, person: &mut Person, placement: &Placement, seconds: f32) {
    // SAFETY: as `place_whole`.
    unsafe { place_in_steps(game, person, placement, seconds, STEP) }
}

/// The most an animation is updated by at once (seconds): a longer update (an animation sent on to a frame of its
/// own: a jump's start, a way into standing, standing's first frame) goes in steps of it, as many as it takes, in
/// the same frame. Sent whole, the game took only part of it (owner 2026-11-06: the standing jump's animation
/// started from its first frame, not its 15th, the body up "stiff" before she bent; a step more stopping;
/// `docs/personagem/TRANSICOES.md` 7.12). A frame's own update (1/60, 1/30 s) goes whole, as ever.
const STEP: f32 = 1.0 / 30.0;

/// [`place_one`] with no steps: the update whole, however long (the twins' test of it, `probe::Twins`).
///
/// # Safety
/// As `place_one`.
unsafe fn place_whole(game: &Game, person: &mut Person, placement: &Placement, seconds: f32) {
    // SAFETY: as `place_in_steps` (the caller's contract).
    unsafe { place_in_steps(game, person, placement, seconds, f32::INFINITY) }
}

/// # Safety
/// As `place_one`.
unsafe fn place_in_steps(game: &Game, person: &mut Person, placement: &Placement, seconds: f32, step: f32) {
    // SAFETY: the owner's node, moved by the game's own set_world (its children, the person, follow); then the
    // instance's own update, as every owner of movers calls it each frame.
    unsafe {
        (game.set_world)(person.at().add(game.node), placement);
        if let Some(instance) = read::<usize>(person.at() as usize + game.instance).filter(|&i| i != 0) {
            let mut left = seconds;
            while left > step {
                (game.update)(instance as *mut u8, step);
                left -= step;
            }
            (game.update)(instance as *mut u8, left);
            person.clock += seconds;
            // Where the game keeps its time, and whether it takes a long update whole (the log only: `probe`).
            if let Some(probe) = person.probe.as_mut() {
                match probe.sample(seconds) {
                    super::probe::Next::More => {}
                    super::probe::Next::Big => {
                        (game.update)(instance as *mut u8, super::probe::BIG);
                        person.clock += super::probe::BIG;
                        probe.after_big();
                        person.probe = None;
                    }
                    super::probe::Next::Done => person.probe = None,
                }
            }
        }
    }
}

/// Makes a thing of the player's (a `mover_model` by its unit name, `mover.…`) at `placement`, drawn with the
/// person: the flashlight.
///
/// # Safety
/// As `show`.
pub(super) unsafe fn show_thing(game: &Game, placement: &Placement, name: &str) -> Result<Person, &'static str> {
    // SAFETY: as `show_one` (the caller's contract).
    unsafe {
        let mut thing = show_one(game, placement, &[format!("{name}\0").into_bytes()])?;
        DRAWN_TOO.store(thing.instance(game), Ordering::Release);
        Ok(thing)
    }
}

/// Puts the thing at `placement` (its own update, no time).
///
/// # Safety
/// As `place`, `thing` as `show_thing` made it.
pub(super) unsafe fn place_thing(game: &Game, thing: &mut Person, placement: &Placement) {
    // SAFETY: as `place_one` (the caller's contract).
    unsafe {
        place_one(game, thing, placement, 0.0);
        DRAWN_TOO.store(thing.instance(game), Ordering::Release);
    }
}

/// Lets the thing go.
///
/// # Safety
/// As `hide`, `thing` as `show_thing` made it.
pub(super) unsafe fn hide_thing(game: &Game, thing: Person) {
    DRAWN_TOO.store(0, Ordering::Release);
    // SAFETY: as `hide_one` (the caller's contract).
    unsafe { hide_one(game, thing) };
}

/// Lets the person go.
///
/// # Safety
/// On the game's main thread, `person` as `show` made it.
pub(super) unsafe fn hide(game: &Game, body: Body) {
    DRAWN.store(0, Ordering::Release);
    DRAWN_BEFORE.store(0, Ordering::Release);
    // SAFETY: as `hide_one` (the caller's contract).
    unsafe {
        if let Some(standing) = body.standing {
            hide_one(game, standing);
        }
        if let Some(walking) = body.walking {
            hide_one(game, walking);
        }
        if let Some(running) = body.running {
            hide_one(game, running);
        }
        if let Some((jumping, _)) = body.jumping {
            hide_one(game, jumping);
        }
        if let Some((leaping, _)) = body.leaping {
            hide_one(game, leaping);
        }
        if let Some((crouching, _)) = body.crouching {
            hide_one(game, crouching);
        }
        if let Some((stopping, ..)) = body.stopping {
            hide_one(game, stopping);
        }
        if let Some((texting, ..)) = body.texting {
            hide_one(game, texting);
        }
        if let Some(txwalking) = body.txwalking {
            hide_one(game, txwalking);
        }
        if let Some((a, b, _)) = body.twins {
            hide_one(game, a);
            hide_one(game, b);
        }
        if let Some((walker, starter, settler, _)) = body.crouch_walking {
            hide_one(game, walker);
            hide_one(game, starter);
            hide_one(game, settler);
        }
        if let Some((settling, _)) = body.settling {
            hide_one(game, settling);
        }
    }
}

/// Lets one person go.
///
/// # Safety
/// On the game's main thread, `person` as `show_one` made it.
unsafe fn hide_one(game: &Game, mut person: Person) {
    // SAFETY: the passenger's own release (the instance and the model let go), then the owner's own destructor,
    // not freeing (the memory is the plugin's).
    unsafe {
        // Nothing of the body drawn once one of its people is let go (`place` puts the others back).
        DRAWN.store(0, Ordering::Release);
        DRAWN_BEFORE.store(0, Ordering::Release);
        (game.release)(person.at());
        drop_owner(game, &mut person);
    }
    log!("character: the person hidden");
}

/// The owner's destructor, told not to free it (flag 0).
///
/// # Safety
/// `person`'s owner made by the game's constructor.
unsafe fn drop_owner(game: &Game, person: &mut Person) {
    // SAFETY: the owner's own vtable (the game's), slot 0.
    unsafe {
        if let Some(dtor) = read::<usize>(game.vtable) {
            std::mem::transmute::<usize, Dtor>(dtor)(person.at(), 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_read_as_written() {
        assert_eq!(pattern(OWNER_CTOR).len(), 42);
        assert_eq!(pattern(SPAWN).len(), 34);
        assert_eq!(pattern(SPAWN_ATTACH).len(), 19);
        assert_eq!(pattern(SPAWN_RELEASE).len(), 11);
        assert_eq!(pattern(ATTACH_WORLD).len(), 18);
        assert_eq!(pattern(FIND_UNIT).len(), 28);
        assert_eq!(pattern(UPDATE).len(), 38);
        assert_eq!(pattern(DRAW_SITE).len(), 27);
        assert_eq!(pattern(GATHER).len(), 20);
        assert_eq!(unit_name("m_walk_06"), b"mover.m_walk_06\0");
        // Each standing one belongs to a person the player can choose, or to the game's own one walking without them.
        let ours_or_the_games = |walk: &&str| {
            westplay_shared::CHARACTER_PEOPLE.contains(walk) || *walk == westplay_shared::CHARACTER_GAME_PERSON
        };
        assert!(STANDING.iter().all(|(walk, _)| ours_or_the_games(walk)));
        // Each person the player can choose has a profile, in the same order, and each profile is one of them.
        assert_eq!(westplay_shared::CHARACTER_PEOPLE.len(), PEOPLE.len());
        for (walk, person) in westplay_shared::CHARACTER_PEOPLE.iter().zip(&PEOPLE) {
            assert_eq!(*walk, format!("{}_walk", person.prefix));
            assert!(profile(walk).is_some_and(|found| found.prefix == person.prefix));
            // Eyes under the walking camera's (1.8 m); steps inside their cycle.
            assert!((1.3..1.8).contains(&person.eyes), "{}", person.prefix);
            for gait in [Some(person.walk), person.run].into_iter().flatten() {
                assert!(gait.cycle > 0.3 && gait.steps.iter().all(|&at| (0.0..gait.cycle).contains(&at)));
            }
            // A jump starts, leaves the ground, lands and ends in that order (on the move it may start at its
            // first frame, and be cut as it lands).
            for once in [person.jump, person.jumprun].into_iter().flatten() {
                assert!(
                    0.0 <= once.from && once.from <= once.takeoff && once.takeoff < once.land - BEFORE_LANDING,
                    "{once:?}"
                );
                assert!(once.land <= once.cut_after && once.cut_after <= once.length, "{once:?}");
            }
            if let Some(c) = person.crouch {
                assert!(0.0 < c.bottom && c.bottom < c.up && c.up <= c.length, "{c:?}");
            }
            if let Some(stop) = person.stop {
                assert!(0.0 <= stop.from && stop.from < stop.length);
            }
            if let Some(phone) = person.phone {
                assert!(0.0 < phone.back && phone.back < phone.length);
            }
        }
        assert!(profile("m_walk_06").is_none() && profile("walk").is_none());
        // The woman's timings as they were tuned in the game (frames counted from 1 at 30 a second).
        let woman = profile("wm_walk").unwrap();
        assert!(woman.jump.is_none() && woman.jumprun.is_some());
        assert_eq!(woman.stop.unwrap(), Stop { from: 30.0 / 30.0, length: 46.0 / 30.0 });
        assert!(fits(&[0x48, 0x8b], &pattern("48 ??")));
    }

    #[test]
    fn the_headless_body_is_the_same_person_and_animation() {
        assert_eq!(headless("wt_walk"), "wtf_walk");
        assert_eq!(headless("wm_txwalk"), "wmf_txwalk");
        assert_eq!(unit_name(&headless("wm_idle")), b"mover.wmf_idle\0");
        // Their eyes under the walking camera's (1.8 m), the woman's lower than the man's.
        assert!(eye_height("wm_walk") < eye_height("wt_walk") && eye_height("wt_walk") < 1.8);
    }

    #[test]
    fn walking_stops_from_the_nearest_piece_of_its_cycle() {
        let cycle = 1.0;
        // The nearest of the eight, before or after; past the last, the first again.
        assert_eq!(nearest_piece(0.0, cycle), 0);
        assert_eq!(nearest_piece(0.06, cycle), 0);
        assert_eq!(nearest_piece(0.07, cycle), 1);
        assert_eq!(nearest_piece(0.5, cycle), 4);
        assert_eq!(nearest_piece(0.95, cycle), 0);
        assert_eq!(nearest_piece(12.26, cycle), 2);
        for step in 0..400 {
            let clock = step as f32 * 0.013;
            let piece = nearest_piece(clock, cycle);
            let off = (clock.rem_euclid(cycle) - piece as f32 / PIECES as f32).rem_euclid(cycle);
            assert!(piece < PIECES && off.min(cycle - off) <= 1.0 / 16.0 + 1e-5, "{step}");
        }
        // The pieces in the animation as Blender made them (10 frames each, back to back: 80 frames).
        for person in &PEOPLE {
            let settle = person.settle.unwrap();
            assert!((settle.start(1) - 10.0 / 30.0).abs() < 1e-6);
            assert!((settle.length() - 79.0 / 30.0).abs() < 1e-6);
            assert!(settle.start(PIECES - 1) + settle.piece <= settle.length() + 1e-6);
            assert!(settle.length() < settle.idle);
        }
    }

    #[test]
    fn a_step_is_heard_as_each_foot_comes_down() {
        const OWN_WALK: Gait = PEOPLE[0].walk;
        let own_run = PEOPLE[0].run.unwrap();
        // Two steps a cycle, each once, however the frames fall.
        for frame in [1.0 / 30.0, 1.0 / 60.0, 1.0 / 144.0, 0.1] {
            let mut clock = 0.0f32;
            let mut heard = 0;
            while clock < 10.0 * OWN_WALK.cycle {
                if OWN_WALK.stepped(clock, clock + frame) {
                    heard += 1;
                }
                clock += frame;
            }
            assert_eq!(heard, 20, "frames of {frame} s");
        }
        // At the foot's time, not before.
        assert!(!OWN_WALK.stepped(0.0, 0.30));
        assert!(OWN_WALK.stepped(0.30, 0.32));
        assert!(OWN_WALK.stepped(OWN_WALK.cycle + 0.83, OWN_WALK.cycle + 0.85));
        // A clock not going on, no step.
        assert!(!own_run.stepped(0.5, 0.5));
        // Running with its own animation: a step about every 0.35 s (it was every 0.29 s, the sound too fast).
        let gap = own_run.steps[1] - own_run.steps[0];
        assert!((0.3..0.4).contains(&gap) && (0.3..0.4).contains(&(own_run.cycle - gap)));
        assert_eq!(gaits("wt_walk"), (OWN_WALK, Some(own_run)));
        assert_eq!(gaits("m_walk_06"), (GAME_WALK, None));
        assert_eq!(gaits("wm_walk"), (PEOPLE[1].walk, PEOPLE[1].run));
    }
}
