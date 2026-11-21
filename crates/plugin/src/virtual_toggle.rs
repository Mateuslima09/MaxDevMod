//! The game's virtual mirrors on only while reversing (owner 2026-11-14): with them on the game draws its mirrors
//! even zoomed in on the dashboard's screen, so our reversing camera (made from a mirror) stays; hidden
//! (`hide_virtual`) they can't be seen, but they cost frames, so off again going forward.
//!
//! The game turns them on and off with a key (F2, its default; `g_mirrors` alone doesn't: it's only remembered).
//! So the key is pressed as the player would ([`press`]: a scan code through `SendInput`, which the game's
//! DirectInput reads), in the cab, till the mirrors counted are the ones wanted: both in reverse, none out of it,
//! whoever turned them on (hidden, they're of no use then). How many are on is counted from their pictures drawn into the
//! final picture (`hide_virtual::virtual_drawn`), so a press of the player's own is never undone the wrong way.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_MENU,
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, SendInput, VIRTUAL_KEY,
};

use crate::log;

/// Whether the plugin turns the virtual mirrors on for reversing.
const AUTO: bool = true;
/// The key that turns the game's virtual mirrors on and off: F2's scan code.
const TOGGLE_SCANCODE: u16 = 0x3c;
/// Frames to wait after a press before looking again (the game shows them a few frames later).
/// Frames to wait after a press before counting again or pressing (the game shows or takes them a moment later:
/// counted too soon, a press too many went past the step wanted, owner's log 2026-11-14).
/// 150, not 90 (owner 2026-11-14): counted at 90 the step was still the old one now and then, and F2 was pressed
/// again (both on took three presses), each a moment the mirrors could be seen.
const SETTLE: u64 = 150;
/// The cab's camera counts as in use if the dashboard was drawn this few frames ago.
const CAB_LATELY: u64 = 30;

/// The frame of the last press (0: none).
static PRESSED_AT: AtomicU64 = AtomicU64::new(0);

/// F2 down and, a moment later (the game polls its keys), up: on a thread of its own.
fn press() {
    let _ = std::thread::Builder::new().name("westplay-mirrors-key".into()).spawn(|| {
        let key = |up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: TOGGLE_SCANCODE,
                    dwFlags: if up { KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP } else { KEYEVENTF_SCANCODE },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        // SAFETY: one keyboard input of ours, its size given.
        unsafe { SendInput(&[key(false)], std::mem::size_of::<INPUT>() as i32) };
        std::thread::sleep(std::time::Duration::from_millis(80));
        // SAFETY: as above.
        unsafe { SendInput(&[key(true)], std::mem::size_of::<INPUT>() as i32) };
    });
}

/// Presses made this time (turning them on, or off), at most [`MOST_PRESSES`]: never a key held down forever.
static PRESSES: AtomicU64 = AtomicU64::new(0);
/// Six: the F2 cycle differs truck to truck (a Scania's off, one, both, one; a MAN's had one shown three presses
/// running, owner 2026-11-14), so it's pressed till what's counted is what's wanted, any cycle of four within.
const MOST_PRESSES: u64 = 6;
/// Whether turning them off has begun (its presses counted afresh).
static LEAVING: AtomicBool = AtomicBool::new(true);

/// How many virtual mirrors were shown when last counted (in the cab: out of it they can't be, in the walking mode
/// the game fills them with a test picture), and the frame now.
static COUNTED: AtomicU64 = AtomicU64::new(0);
static NOW: AtomicU64 = AtomicU64::new(0);

/// At each present: in reverse in the cab, both virtual mirrors on (owner 2026-11-14, "with only one on the screen
/// bugs"), and in the cab all the time while one is chosen to be seen (owner 2026-11-18); else none. No cycle assumed: pressed, given time, counted, pressed again till it's so (a
/// cycle assumed, a MAN's went round and round, owner 2026-11-14). Only in the cab, where they can be counted.
pub fn on_present(frame: u64) {
    NOW.store(frame, Ordering::Relaxed);
    if !AUTO {
        return;
    }
    let Some(bridge) = crate::bridge::get() else { return };
    let plugin = &bridge.plugin;
    let reversing = plugin.telemetry.read().is_some_and(|t| t.electric_enabled && (t.displayed_gear < 0 || t.gear < 0));
    let in_cab = frame.saturating_sub(plugin.cab_frame.load(Ordering::Relaxed)) <= CAB_LATELY && !crate::character::walking();
    // Paused (the game's menu): nothing pressed, left as they are till the game goes on.
    if crate::telemetry::paused() {
        return;
    }
    // Getting out (the walk key pressed with them on): off first, whatever the gear, a while.
    if REQUEST_OFF.swap(false, Ordering::Relaxed) {
        OFF_UNTIL.store(frame + 600, Ordering::Relaxed);
    }
    let getting_out = frame < OFF_UNTIL.load(Ordering::Relaxed) && !crate::character::walking();
    // Both on all the time in the cab while one is chosen to be seen (owner 2026-11-18: "by default both on, the right
    // one shown, the left one hidden"; fewer presses of F2 than on and off at every reverse); none chosen to be seen,
    // only while reversing, as before (they cost frames).
    let choice = bridge.host.virtual_mirrors.load(Ordering::Acquire);
    let always = choice != westplay_shared::VIRTUAL_MIRRORS_NONE;
    let wanted = (reversing || always) && in_cab && !getting_out;
    // Settled: the count changed since the press (a moment for it to hold), or a while gone by (F2 pressed again
    // only then: waiting the whole while each press, getting out took seconds, owner 2026-11-14).
    let pressed = PRESSED_AT.load(Ordering::Relaxed);
    let counted = crate::hide_virtual::virtual_count().min(2);
    let since = frame.saturating_sub(pressed);
    let settled = pressed == 0 || since >= SETTLE || (since >= 20 && counted != COUNT_AT_PRESS.load(Ordering::Relaxed));
    SETTLED.store(settled, Ordering::Relaxed);
    if !settled || !in_cab {
        return;
    }
    COUNTED.store(counted, Ordering::Relaxed);
    let target = if wanted { 2 } else { 0 };
    if LEAVING.swap(!wanted, Ordering::Relaxed) == wanted {
        // Turning them on, or off: its presses counted afresh.
        PRESSES.store(0, Ordering::Relaxed);
    }
    if counted == target {
        // There: the presses counted afresh (on all the time, the most presses are for each time it's put right, not
        // for the whole time in the cab).
        PRESSES.store(0, Ordering::Relaxed);
        return;
    }
    // Not with Alt held (the phone or the screen used with the mouse): the game takes Alt+F2, not F2 (owner
    // 2026-11-18: six presses and nothing changed). Pressed once it's let go.
    // SAFETY: plain queries of the keyboard's state.
    let alt = unsafe { (GetAsyncKeyState(i32::from(VK_MENU.0)) as u16 & 0x8000) != 0 };
    if alt {
        return;
    }
    let presses = PRESSES.fetch_add(1, Ordering::Relaxed) + 1;
    if presses > MOST_PRESSES {
        return;
    }
    log!(
        "retrovisor: F2 pressed ({}; {counted} virtual mirrors on, {target} wanted, press {presses})",
        if !wanted { "off, out of reverse" } else if always { "both on, one or both shown" } else { "both on for reversing" }
    );
    PRESSED_AT.store(frame, Ordering::Relaxed);
    COUNT_AT_PRESS.store(counted, Ordering::Relaxed);
    SETTLED.store(false, Ordering::Relaxed);
    press();
}

/// What was counted when F2 was last pressed, and whether that press has settled.
static COUNT_AT_PRESS: AtomicU64 = AtomicU64::new(0);
static SETTLED: AtomicBool = AtomicBool::new(true);

/// Asked to turn them off before walking (`turn_off_first`), and till which frame that holds.
static REQUEST_OFF: AtomicBool = AtomicBool::new(false);
static OFF_UNTIL: AtomicU64 = AtomicU64::new(0);

/// Whether any virtual mirror may be on: counted so, or F2 pressed lately (the game shows it a moment later:
/// walking let start before then, one stayed on, owner 2026-11-14).
pub fn mirrors_on() -> bool {
    AUTO && (COUNTED.load(Ordering::Relaxed) != 0 || !SETTLED.load(Ordering::Relaxed))
}

/// Whether F2 is done for reversing: no press waiting to settle, both virtual mirrors on (or no pressing at all).
pub fn done_for_reversing() -> bool {
    !AUTO || (SETTLED.load(Ordering::Relaxed) && COUNTED.load(Ordering::Relaxed) >= 2)
}

/// The walk key pressed with virtual mirrors on: turned off first (walking, the game takes no keys).
pub fn turn_off_first() {
    REQUEST_OFF.store(true, Ordering::Relaxed);
}
