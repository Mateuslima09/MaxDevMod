//! The plugin's mapping of the shared bridge (see `westplay-shared`).

use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use westplay_shared::{Bridge, BridgeMapping};

use crate::log;

static MAPPING: OnceLock<Option<BridgeMapping>> = OnceLock::new();

/// The bridge, mapped on first use. `None` if the shared memory could not be created.
/// `None` too while the app that opened it is of another version ([`BridgeMapping::matches`]): nothing is read
/// from it or written to it then.
pub fn get() -> Option<&'static Bridge> {
    let mapping = MAPPING
        .get_or_init(|| match BridgeMapping::open() {
            Ok(mapping) => Some(mapping),
            Err(e) => {
                log!("bridge: shared memory unavailable: {e}");
                None
            }
        })
        .as_ref()?;
    if !mapping.matches() {
        static TOLD: std::sync::Once = std::sync::Once::new();
        TOLD.call_once(|| {
            let theirs = mapping.bridge().version.load(Ordering::Relaxed);
            log!(
                "bridge: the app's version {theirs} isn't this plugin's {}: not used",
                westplay_shared::BRIDGE_VERSION
            );
        });
        return None;
    }
    Some(mapping.bridge())
}

/// The app counts as gone when its heartbeat has not changed for this long.
const HOST_TIMEOUT: Duration = Duration::from_secs(2);

/// The last heartbeat value seen from the app, and when it changed.
static HOST_HEARTBEAT: Mutex<Option<(u64, Instant)>> = Mutex::new(None);

/// Whether the app is running: its heartbeat changed within the last [`HOST_TIMEOUT`].
pub fn host_alive() -> bool {
    let Some(bridge) = get() else { return false };
    let heartbeat = bridge.host.heartbeat.load(Ordering::Acquire);
    let mut last = HOST_HEARTBEAT.lock().unwrap_or_else(|e| e.into_inner());
    match *last {
        Some((seen, _)) if seen == heartbeat => {}
        // The first value seen proves nothing: it may be left over from an app that has quit.
        None => *last = Some((heartbeat, Instant::now() - HOST_TIMEOUT)),
        Some(_) => *last = Some((heartbeat, Instant::now())),
    }
    last.is_some_and(|(_, changed)| changed.elapsed() <= HOST_TIMEOUT)
}

/// Whether WestPlay is on the truck's screen: the app is running, the player hasn't switched it off, and the
/// truck is one WestPlay supports.
pub fn screen_on() -> bool {
    host_alive()
        && crate::screens::on_supported_truck()
        && get().is_some_and(|bridge| bridge.host.screen_enabled.load(Ordering::Acquire) == 1)
}

/// Whether the phone is out (the app running): used with the mouse, typed into, as the screen is, in any truck
/// (owner 2026-11-02: in a truck WestPlay's screen isn't in, the phone should still work, the music with it).
pub fn phone_out() -> bool {
    host_alive() && get().is_some_and(|bridge| bridge.host.phone_shown.load(Ordering::Acquire) == 1)
}

/// Whether there's WestPlay to use: on the truck's screen, or the phone out.
pub fn usable() -> bool {
    screen_on() || phone_out()
}

/// When the phone was last asked away (`put_phone_away`, milliseconds since the plugin's start), so it isn't
/// asked twice before the app has put it away (each ask toggles it).
static PHONE_ASKED_AWAY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

/// Puts the phone away if it's out (owner 2026-11-06: running, getting out of the truck or back in, the person has
/// no animation holding it): as a press of the phone's keybind, which the app answers by putting it away.
pub fn put_phone_away(why: &str) {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let now = START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64;
    let asked = PHONE_ASKED_AWAY.load(Ordering::Acquire);
    if !phone_out() || (asked != u64::MAX && now.saturating_sub(asked) < 600) {
        return;
    }
    let Some(bridge) = get() else { return };
    PHONE_ASKED_AWAY.store(now, Ordering::Release);
    bridge.plugin.keybind_presses[westplay_shared::Keybind::Phone as usize].fetch_add(1, Ordering::AcqRel);
    crate::log!("phone put away: {why}");
}
