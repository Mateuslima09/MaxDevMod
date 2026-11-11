//! The truck's commands from the phone (owner 2026-11-02: "open the window, lower the truck, the suspension"): an
//! input device of the SDK's input API (`scs_input_init`), of the semantical kind, whose inputs are the game's own
//! controls by name (`westplay_shared::TRUCK_COMMANDS`): the game takes each as that control held, with nothing
//! for the player to bind. The app says which are held (`HostBlock::truck_commands`); each frame, every one that
//! changed since is given to the game as an event.

use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicU64, Ordering};

use westplay_shared::TRUCK_COMMANDS;

use crate::scs::{SCS_RESULT_OK, ScsResult, SdkInitParamsV100, make_version};
use crate::{bridge, hooks, log};

pub const SCS_INPUT_VERSION_1_00: u32 = make_version(1, 0);
const SCS_RESULT_NOT_FOUND: ScsResult = -4;
const SCS_INPUT_DEVICE_TYPE_SEMANTICAL: u32 = 2;
const SCS_VALUE_TYPE_BOOL: u32 = 1;
const FIRST_AFTER_ACTIVATION: u32 = 0x2;

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

/// The plugin's own commands, after the phone's on the device: the game's `attach` (coupling the trailer), given
/// by `character::freight` as the player couples it by hand at the fifth wheel. Never the phone's to give.
const OWN_COMMANDS: &[&str] = &["attach"];
pub const ATTACH: u64 = 1;

/// Which of the plugin's own commands are held now (`ATTACH`).
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
        let names: Vec<&'static std::ffi::CStr> = TRUCK_COMMANDS
            .iter()
            .chain(OWN_COMMANDS)
            .map(|name| &*Box::leak(std::ffi::CString::new(*name).unwrap_or_default().into_boxed_c_str()))
            .collect();
        let inputs: &'static [DeviceInput] = Box::leak(
            names
                .iter()
                .map(|name| DeviceInput {
                    name: name.as_ptr(),
                    display_name: name.as_ptr(),
                    value_type: SCS_VALUE_TYPE_BOOL,
                    _padding: 0,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let device: &'static Device = Box::leak(Box::new(Device {
            name: c"westplay_commands".as_ptr(),
            display_name: c"WestPlay".as_ptr(),
            kind: SCS_INPUT_DEVICE_TYPE_SEMANTICAL,
            input_count: inputs.len() as u32,
            inputs: inputs.as_ptr(),
            callback_context: std::ptr::null_mut(),
            input_active_callback: None,
            input_event_callback: Some(next_event),
        }));
        let result = register(device);
        log!("commands: input device registered ({result}), {} commands", inputs.len());
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
        }
        let all = (1u64 << TRUCK_COMMANDS.len()) - 1;
        let phones = bridge::get()
            .filter(|_| bridge::host_alive())
            .map_or(0, |bridge| bridge.host.truck_commands.load(Ordering::Acquire))
            & all;
        let own = OWN_HELD.load(Ordering::Acquire) & ((1u64 << OWN_COMMANDS.len()) - 1);
        let wanted = phones | (own << TRUCK_COMMANDS.len());
        let reported = REPORTED.load(Ordering::Relaxed);
        let changed = wanted ^ reported;
        if changed == 0 {
            return SCS_RESULT_NOT_FOUND;
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
