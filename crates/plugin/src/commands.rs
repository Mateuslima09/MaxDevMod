//! The truck's commands from the phone (owner 2026-11-02: "open the window, lower the truck, the suspension"): an
//! input device of the SDK's input API (`scs_input_init`), of the semantical kind, whose inputs are the game's own
//! controls by name (`westplay_shared::TRUCK_COMMANDS`): the game takes each as that control held, with nothing
//! for the player to bind. The app says which are held (`HostBlock::truck_commands`); each frame, every one that
//! changed since is given to the game as an event.
//!
//! The player's own phone as a steering wheel (owner 2026-11-19, the host's wheel page): three analog inputs after
//! the commands ([`AXES`]), the game's own `steering`, `aforward` and `abackward` (its profiles' mixes take
//! `semantical.steering` and the pedals with no binding). Given only while the phone wheel talks
//! (`HostBlock::wheel_active`, its values changing): otherwise each is let back to 0 once and left alone, so the
//! player's keyboard and wheel drive as ever.

use std::ffi::{c_char, c_void};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use westplay_shared::TRUCK_COMMANDS;

use crate::scs::{SCS_RESULT_OK, ScsResult, SdkInitParamsV100, make_version};
use crate::{bridge, hooks, log};

pub const SCS_INPUT_VERSION_1_00: u32 = make_version(1, 0);
const SCS_RESULT_NOT_FOUND: ScsResult = -4;
const SCS_INPUT_DEVICE_TYPE_SEMANTICAL: u32 = 2;
const SCS_VALUE_TYPE_BOOL: u32 = 1;
const SCS_VALUE_TYPE_FLOAT: u32 = 5;
const FIRST_IN_FRAME: u32 = 0x1;
const FIRST_AFTER_ACTIVATION: u32 = 0x2;

/// The phone wheel's analog inputs, after the commands on the device: the steering (-1 left to 1 right as the
/// phone sends it; the game's `steering` mix takes it away from the keys', which turn the other way round), the
/// throttle and the brake (0 to 1).
const AXES: &[&str] = &["steering", "aforward", "abackward"];
/// How hard the phone wheel steers the game's wheel toward the phone's angle while the game's steering is relative
/// (its keyboard's and gamepad's profiles, `c_relatsteer 1`: the steering it's given is how fast the wheel turns,
/// not where it is; owner's log 2026-11-19: the phone at -0.04 and the game's wheel creeping from -0.12 to -0.20,
/// then left at full lock): the turning asked per unit of the angle still to go.
const RELATIVE_GAIN: f32 = 8.0;
/// Close enough: the game's wheel left where it is.
const RELATIVE_NEAR: f32 = 0.004;

/// The phone wheel counts as gone when its values haven't changed for this long (the app's own timeout is
/// shorter: this is for an app that went without saying).
const WHEEL_QUIET: Duration = Duration::from_millis(400);

/// `scs_input_device_input_t` (24 bytes on x64).
#[repr(C)]
struct DeviceInput {
    name: *const c_char,
    display_name: *const c_char,
    value_type: u32,
    _padding: u32,
}

type ActiveCallback = unsafe extern "system" fn(active: u8, context: *mut c_void);
type EventCallback = unsafe extern "system" fn(event: *mut InputEvent, flags: u32, context: *mut c_void) -> ScsResult;

/// `scs_input_device_t` (56 bytes on x64).
#[repr(C)]
struct Device {
    name: *const c_char,
    display_name: *const c_char,
    kind: u32,
    input_count: u32,
    inputs: *const DeviceInput,
    callback_context: *mut c_void,
    input_active_callback: Option<ActiveCallback>,
    input_event_callback: Option<EventCallback>,
}

/// `scs_input_event_t` (28 bytes): the input's index, and its value (a bool's in the first byte).
#[repr(C)]
pub struct InputEvent {
    input_index: u32,
    value: [u8; 24],
}

type RegisterDevice = unsafe extern "system" fn(device: *const Device) -> ScsResult;

/// `scs_input_init_params_v100_t` (40 bytes on x64).
#[repr(C)]
pub struct InputInitParamsV100 {
    common: SdkInitParamsV100,
    register_device: Option<RegisterDevice>,
}

const _: () = assert!(size_of::<DeviceInput>() == 24);
const _: () = assert!(size_of::<Device>() == 56);
const _: () = assert!(size_of::<InputEvent>() == 28);
const _: () = assert!(size_of::<InputInitParamsV100>() == 40);

/// The plugin's own commands: the game's `attach` (coupling and uncoupling the trailer), given by
/// `character::freight` as the player does it by hand at the truck's side, and its `activate` (filling the tank at
/// a pump), given by `character::refuel` as the player fills it at the truck's tank. `attach` is one of the
/// phone's commands too (`TRUCK_COMMANDS`): the same input of the device (a name given twice, and the game
/// refuses the whole device: owner's log 2026-11-11, "registered (-2)", nothing coupled, no tank filled).
/// `activate` comes after the phone's on the device, never the phone's to give.
const OWN_COMMANDS: &[&str] = &["activate"];
pub const ATTACH: u64 = 1;
pub const ACTIVATE: u64 = 2;
/// The game's `ignitionoff` (the phone's too, its input shared as `attach`'s): the engine off for loading, the papers
/// collected (`character::freight`: the game loads only with it off).
pub const ENGINE_OFF: u64 = 4;

/// Whether the game took the device: without it nothing of the plugin's reaches the game's controls, and the
/// player's own keys are left to them (`character::freight`, `character::refuel`).
static REGISTERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn registered() -> bool {
    REGISTERED.load(Ordering::Acquire)
}

/// What the device is told is held: the phone's commands (`phones`), with the plugin's own (`own`: `ATTACH`,
/// `ACTIVATE`) each on its input.
fn wanted(phones: u64, own: u64) -> u64 {
    let attach = TRUCK_COMMANDS.iter().position(|name| *name == "attach");
    let mut wanted = phones & ((1u64 << TRUCK_COMMANDS.len()) - 1);
    if own & ATTACH != 0
        && let Some(attach) = attach
    {
        wanted |= 1 << attach;
    }
    if own & ENGINE_OFF != 0
        && let Some(off) = TRUCK_COMMANDS.iter().position(|name| *name == "ignitionoff")
    {
        wanted |= 1 << off;
    }
    if own & ACTIVATE != 0 {
        wanted |= 1 << TRUCK_COMMANDS.len();
    }
    wanted
}

/// Which of the plugin's own commands are held now (`ATTACH`, `ACTIVATE`).
static OWN_HELD: AtomicU64 = AtomicU64::new(0);

/// Holds or lets go one of the plugin's own commands.
pub fn hold(command: u64, held: bool) {
    if held {
        OWN_HELD.fetch_or(command, Ordering::AcqRel);
    } else {
        OWN_HELD.fetch_and(!command, Ordering::AcqRel);
    }
}

/// Which commands the game was last told are held.
static REPORTED: AtomicU64 = AtomicU64::new(0);

/// Registers the device, at the game's `scs_input_init`.
///
/// # Safety
/// The game's call: `params` for `version`, alive through it, on its main thread.
pub unsafe fn init(version: u32, params: *const InputInitParamsV100) -> ScsResult {
    if version != SCS_INPUT_VERSION_1_00 {
        return crate::scs::SCS_RESULT_UNSUPPORTED;
    }
    // SAFETY: the game's params for this version (the caller's contract); the device and its inputs are leaked,
    // alive for the program's life, as the game may keep them.
    unsafe {
        let Some(register) = params.as_ref().and_then(|params| params.register_device) else {
            return crate::scs::SCS_RESULT_UNSUPPORTED;
        };
        let names: Vec<(&'static std::ffi::CStr, u32)> = TRUCK_COMMANDS
            .iter()
            .chain(OWN_COMMANDS)
            .map(|name| (*name, SCS_VALUE_TYPE_BOOL))
            .chain(AXES.iter().map(|name| (*name, SCS_VALUE_TYPE_FLOAT)))
            .map(|(name, kind)| {
                (&*Box::leak(std::ffi::CString::new(name).unwrap_or_default().into_boxed_c_str()), kind)
            })
            .collect();
        let inputs: &'static [DeviceInput] = Box::leak(
            names
                .iter()
                .map(|(name, kind)| DeviceInput {
                    name: name.as_ptr(),
                    display_name: name.as_ptr(),
                    value_type: *kind,
                    _padding: 0,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let device: &'static Device = Box::leak(Box::new(Device {
            name: c"westplay_commands".as_ptr(),
            display_name: c"MaxDevMod".as_ptr(),
            kind: SCS_INPUT_DEVICE_TYPE_SEMANTICAL,
            input_count: inputs.len() as u32,
            inputs: inputs.as_ptr(),
            callback_context: std::ptr::null_mut(),
            input_active_callback: None,
            input_event_callback: Some(next_event),
        }));
        let result = register(device);
        REGISTERED.store(result == SCS_RESULT_OK, Ordering::Release);
        log!("commands: input device registered ({result}), {} inputs ({} analog)", inputs.len(), AXES.len());
        result
    }
}

/// The next command that changed since the game was last told, as an event; `not_found` once none has.
///
/// # Safety
/// The game's call, on its main thread, `event` its own to fill.
unsafe extern "system" fn next_event(event: *mut InputEvent, flags: u32, _context: *mut c_void) -> ScsResult {
    hooks::contained("commands", SCS_RESULT_NOT_FOUND, || {
        // Active anew: the game takes every command as let go.
        if flags & FIRST_AFTER_ACTIVATION != 0 {
            REPORTED.store(0, Ordering::Relaxed);
            for axis in &AXES_REPORTED {
                axis.store(0, Ordering::Relaxed);
            }
        }
        let phones = bridge::get()
            .filter(|_| bridge::host_alive())
            .map_or(0, |bridge| bridge.host.truck_commands.load(Ordering::Acquire));
        let wanted = wanted(phones, OWN_HELD.load(Ordering::Acquire));
        let reported = REPORTED.load(Ordering::Relaxed);
        let changed = wanted ^ reported;
        if changed == 0 {
            // SAFETY: the game's event to fill (the caller's contract).
            return next_axis(unsafe { event.as_mut() }, flags);
        }
        let index = changed.trailing_zeros();
        let held = wanted & (1 << index) != 0;
        // SAFETY: the game's event to fill (the caller's contract).
        let Some(event) = (unsafe { event.as_mut() }) else { return SCS_RESULT_NOT_FOUND };
        event.input_index = index;
        event.value = [0; 24];
        event.value[0] = u8::from(held);
        REPORTED.store(reported ^ (1 << index), Ordering::Relaxed);
        let name = TRUCK_COMMANDS.iter().chain(OWN_COMMANDS).nth(index as usize).copied().unwrap_or_default();
        log!("commands: {name} {}", if held { "held" } else { "let go" });
        SCS_RESULT_OK
    })
}

/// The axes' values the game was last told (`f32` bits), and which were told this frame (one event each a frame
/// at most, so a phone sending fast never keeps the game asking).
static AXES_REPORTED: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];
static AXES_TOLD: AtomicU32 = AtomicU32::new(0);
/// The game frame (`PluginBlock::heartbeat`) `AXES_TOLD` is of.
static AXES_FRAME: AtomicU64 = AtomicU64::new(u64::MAX);
/// The phone wheel's last `wheel_seq` seen and when it changed, and whether it was taken as on (for the log).
static WHEEL_SEEN: Mutex<(u64, Option<Instant>, bool)> = Mutex::new((0, None, false));
/// When the values given were last logged (every 5 s while the phone drives: what the game was given).
static WHEEL_LOGGED: Mutex<Option<Instant>> = Mutex::new(None);

/// The phone wheel's values now (`None`: it isn't talking, so the axes are let go).
fn wheel_values() -> Option<[f32; 3]> {
    let host = bridge::get().filter(|_| bridge::host_alive()).map(|bridge| &bridge.host);
    let seq = host.map_or(0, |host| host.wheel_seq.load(Ordering::Acquire));
    // 1: the game's steering relative (the default profiles'); 2: absolute (a wheel's), the phone's angle as it is.
    let mode = host.map_or(0, |host| host.wheel_active.load(Ordering::Acquire));
    let active = mode == 1 || mode == 2;
    let mut seen = WHEEL_SEEN.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    if seen.1.is_none() {
        // The first value seen proves nothing: it may be left from a phone gone before the game started.
        *seen = (seq, Some(now.checked_sub(WHEEL_QUIET * 2).unwrap_or(now)), false);
    } else if seen.0 != seq {
        seen.0 = seq;
        seen.1 = Some(now);
    }
    let fresh = seen.1.is_some_and(|changed| now.duration_since(changed) <= WHEEL_QUIET);
    let on = active && fresh;
    if on != seen.2 {
        seen.2 = on;
        log!("commands: phone wheel {}", if on { "driving" } else { "let go" });
    }
    drop(seen);
    let host = host.filter(|_| on)?;
    let value = |bits: &AtomicU32| f32::from_bits(bits.load(Ordering::Acquire));
    let target = value(&host.wheel_steering).clamp(-1.0, 1.0);
    // Where the game's wheel is: its effective steering, counter-clockwise (the phone's turned round).
    let game = bridge::get().and_then(|bridge| bridge.plugin.telemetry.read()).map(|t| -t.steering);
    let steering = if mode == 2 { target } else { game.map_or(0.0, |game| toward(target, game)) };
    let values = [steering, value(&host.wheel_throttle).clamp(0.0, 1.0), value(&host.wheel_brake).clamp(0.0, 1.0)];
    // What the phone wants, what the game is given and where its wheel is: every 5 s, and twice a second while the
    // game's wheel is away from the phone's, to tell from a test how well it follows.
    let behind = game.is_some_and(|game| (game - target).abs() > 0.1);
    let mut logged = WHEEL_LOGGED.lock().unwrap_or_else(|e| e.into_inner());
    let every = if behind { Duration::from_millis(500) } else { Duration::from_secs(5) };
    if logged.is_none_or(|at| now.duration_since(at) >= every) {
        *logged = Some(now);
        log!(
            "commands: phone wheel at {target:+.2} {} gives steering {:+.2} (the game's wheel at {}), throttle {:.2}, brake {:.2}",
            if mode == 2 { "absolute" } else { "relative" },
            values[0],
            game.map_or("?".to_owned(), |game| format!("{game:+.2}")),
            values[1],
            values[2]
        );
    }
    Some(values).filter(|values| values.iter().all(|v| v.is_finite()))
}

/// The steering asked of a relative game to turn its wheel (at `game`) toward the phone's (`target`): faster the
/// farther, none once there.
fn toward(target: f32, game: f32) -> f32 {
    let still = target - game;
    if still.abs() < RELATIVE_NEAR { 0.0 } else { (still * RELATIVE_GAIN).clamp(-1.0, 1.0) }
}

/// Brake fade (owner 2026-11-21, host brake_fade.rs): how much of the player's brake the app says is left (1 with
/// no app, or before it said).
fn brake_strength() -> f32 {
    let Some(bridge) = bridge::get().filter(|_| bridge::host_alive()) else { return 1.0 };
    let strength = f32::from_bits(bridge.host.brake_strength.load(Ordering::Acquire));
    if strength.is_finite() && strength > 0.0 { strength.clamp(0.15, 1.0) } else { 1.0 }
}

/// What the game's `abackward` is given to take the faded part of the player's brake away: a negative value, which
/// the game adds to the pedal (owner's test 2026-11-21: -0.4 with the pedal down gave 0.60). The game's
/// `truck.input.brake` already has what was given added, so the player's own pedal is it less that; with no pedal
/// (the sum held at 0) nothing is given, so the next frame reads the pedal as it is. -0.85 at most.
fn fade_axis() -> f32 {
    let strength = brake_strength();
    if strength >= 0.999 {
        return 0.0;
    }
    let Some(t) = bridge::get().and_then(|bridge| bridge.plugin.telemetry.read()) else { return 0.0 };
    if t.paused || t.input_brake <= 0.001 {
        return 0.0;
    }
    let told = f32::from_bits(AXES_REPORTED[2].load(Ordering::Relaxed));
    let pedal = (t.input_brake - told.min(0.0)).clamp(0.0, 1.0);
    (-(1.0 - strength) * pedal).clamp(-0.85, 0.0)
}

/// The brake fade's sounds (owner 2026-11-21), checked every 100 ms: with the brake at its least, the dashboard's
/// warning over and over (stopping as soon as the brake comes back a little) and, with the pedal down and the truck
/// moving, iron grinding on iron. Their volumes from the app (`HostBlock::brake_sound`; 0 off).
fn brake_sounds() {
    struct Sounds {
        checked: Option<Instant>,
        warning_until: Option<Instant>,
        squeal_until: Option<Instant>,
    }
    static SOUNDS: Mutex<Sounds> = Mutex::new(Sounds { checked: None, warning_until: None, squeal_until: None });
    let mut sounds = SOUNDS.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    if sounds.checked.is_some_and(|at| now.duration_since(at) < Duration::from_millis(100)) {
        return;
    }
    sounds.checked = Some(now);
    let Some(bridge) = bridge::get().filter(|_| bridge::host_alive()) else { return };
    let Some(t) = bridge.plugin.telemetry.read().filter(|t| !t.paused) else { return };
    let said = bridge.host.brake_sound.load(Ordering::Acquire);
    let (squeal, warning) =
        if said & 0x1_0000 != 0 { ((said & 0xff) as f32 / 100.0, ((said >> 8) & 0xff) as f32 / 100.0) } else { (0.7, 0.7) };
    let least = brake_strength() <= 0.155;
    let due = |until: Option<Instant>| until.is_none_or(|until| now >= until);
    if least && warning > 0.0 && due(sounds.warning_until) {
        let lasts = crate::character::brake_sound(crate::character::BrakeSound::Warning, warning);
        sounds.warning_until = Some(now + lasts + Duration::from_millis(300));
        log!("brake fade: the warning");
    }
    if !least {
        sounds.warning_until = None;
    }
    let pedal = (t.input_brake - f32::from_bits(AXES_REPORTED[2].load(Ordering::Relaxed)).min(0.0)).clamp(0.0, 1.0);
    if least && squeal > 0.0 && pedal > 0.05 && t.speed.abs() > 1.0 && due(sounds.squeal_until) {
        // Overlapping a little, so it doesn't stop between two.
        let lasts = crate::character::brake_sound(crate::character::BrakeSound::Squeal, squeal * (0.5 + 0.5 * pedal));
        sounds.squeal_until = Some(now + lasts.saturating_sub(Duration::from_millis(150)));
    }
}

/// Brake fade: the game's brake temperature, the pedal before and after the game, the speed and the brake left,
/// every second while the truck moves or brakes (the owner's tests; to be let down to only while fading).
fn brake_log() {
    static LOGGED: Mutex<Option<Instant>> = Mutex::new(None);
    let mut logged = LOGGED.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    if logged.is_some_and(|at| now.duration_since(at) < Duration::from_secs(1)) {
        return;
    }
    *logged = Some(now);
    drop(logged);
    let Some(t) = bridge::get().and_then(|bridge| bridge.plugin.telemetry.read()) else { return };
    if t.speed.abs() < 0.5 && t.input_brake < 0.01 && t.brake < 0.01 {
        return;
    }
    log!(
        "brake fade: temp {:.1} C, pedal {:.2}, effective {:.2}, speed {:.0} km/h, strength {:.2}, told {:+.2}",
        t.brake_temperature,
        t.input_brake,
        t.brake,
        t.speed * 3.6,
        brake_strength(),
        f32::from_bits(AXES_REPORTED[2].load(Ordering::Relaxed))
    );
}

/// Whether an axis told `reported` should be told `wanted`: a change worth it, or back to exactly 0.
fn axis_changed(reported: f32, wanted: f32) -> bool {
    (wanted - reported).abs() > 0.0005 || (wanted == 0.0 && reported != 0.0)
}

/// The next axis of the phone wheel that changed, as an event (not told yet this frame); `not_found` once none.
fn next_axis(event: Option<&mut InputEvent>, flags: u32) -> ScsResult {
    let frame = bridge::get().map_or(0, |bridge| bridge.plugin.heartbeat.load(Ordering::Relaxed));
    if AXES_FRAME.swap(frame, Ordering::Relaxed) != frame || flags & FIRST_IN_FRAME != 0 {
        AXES_TOLD.store(0, Ordering::Relaxed);
    }
    brake_log();
    brake_sounds();
    let wanted = match wheel_values() {
        // The phone wheel's brake, faded as the player's own is.
        Some([steering, throttle, brake]) => [steering, throttle, brake * brake_strength()],
        None => [0.0, 0.0, fade_axis()],
    };
    let told = AXES_TOLD.load(Ordering::Relaxed);
    let Some(axis) = (0..AXES.len()).find(|&axis| {
        told & (1 << axis) == 0
            && axis_changed(f32::from_bits(AXES_REPORTED[axis].load(Ordering::Relaxed)), wanted[axis])
    }) else {
        return SCS_RESULT_NOT_FOUND;
    };
    let Some(event) = event else { return SCS_RESULT_NOT_FOUND };
    event.input_index = (TRUCK_COMMANDS.len() + OWN_COMMANDS.len() + axis) as u32;
    event.value = [0; 24];
    event.value[..4].copy_from_slice(&wanted[axis].to_le_bytes());
    AXES_REPORTED[axis].store(wanted[axis].to_bits(), Ordering::Relaxed);
    AXES_TOLD.store(told | (1 << axis), Ordering::Relaxed);
    SCS_RESULT_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name given twice, and the game refuses the whole device.
    #[test]
    fn every_input_of_the_device_has_a_name_of_its_own() {
        let mut names: Vec<&str> = TRUCK_COMMANDS.iter().chain(OWN_COMMANDS).chain(AXES).copied().collect();
        let all = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all);
    }

    /// The plugin's own `attach` is the phone's input; its `activate`, the one after the phone's.
    #[test]
    fn the_plugins_own_commands_are_held_on_their_inputs() {
        let attach = TRUCK_COMMANDS.iter().position(|name| *name == "attach").unwrap();
        assert_eq!(wanted(0, ATTACH), 1 << attach);
        assert_eq!(wanted(0, ACTIVATE), 1 << TRUCK_COMMANDS.len());
        assert_eq!(wanted(0b101, 0), 0b101);
        assert_eq!(wanted(u64::MAX, 0), (1 << TRUCK_COMMANDS.len()) - 1, "only the phone's own");
    }

    /// A relative game's wheel is turned toward the phone's, faster the farther, and left once there.
    #[test]
    fn a_relative_games_wheel_is_turned_toward_the_phones() {
        assert_eq!(toward(0.5, 0.5), 0.0);
        assert_eq!(toward(0.5, 0.498), 0.0);
        assert!(toward(0.5, 0.4) > 0.5);
        assert_eq!(toward(1.0, -1.0), 1.0);
        assert_eq!(toward(-0.04, 1.0), -1.0, "the wheel left at full lock is brought back");
        assert!((toward(0.0, 0.05) + 0.4).abs() < 1e-6);
    }

    /// An axis is told only of a change worth it, and always when let back to 0.
    #[test]
    fn the_wheels_axes_are_told_of_changes_and_of_being_let_go() {
        assert!(!axis_changed(0.0, 0.0));
        assert!(!axis_changed(0.5, 0.5002));
        assert!(axis_changed(0.5, 0.51));
        assert!(axis_changed(0.0001, 0.0));
        assert!(axis_changed(-1.0, 1.0));
    }
}
