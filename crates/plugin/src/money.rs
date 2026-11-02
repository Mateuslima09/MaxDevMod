//! The money the game moves as the player plays, for the phone's Nupay (owner 2026-11-03: "what came in and what
//! went out of the account"): the SDK's gameplay events (a job delivered or cancelled, a fine, a toll, a ferry or a
//! train) as [`MoneyEvent`]s in the bridge, the latest [`MONEY_EVENTS`] of them; the app keeps them.

use std::ffi::CStr;

use westplay_shared::{FixedStr, MONEY_EVENTS, MoneyEvent, MoneyEvents, MoneyKind};

use crate::scs::ConfigurationEvent;
use crate::{bridge, log};

/// A gameplay event (`scs_telemetry_gameplay_event_t`: an id and its attributes, as a configuration's).
///
/// # Safety
/// The event's, on the game's main thread, alive through the call (the SDK's contract).
pub unsafe fn on_gameplay(event: &ConfigurationEvent) {
    // SAFETY: the SDK's strings and attributes, alive through the call (the caller's contract).
    unsafe {
        if event.id.is_null() {
            return;
        }
        let id = CStr::from_ptr(event.id).to_string_lossy().into_owned();
        let mut amount: i64 = 0;
        let mut offence = String::new();
        let mut from = String::new();
        let mut to = String::new();
        for attribute in event.attributes() {
            let value = &attribute.value;
            match CStr::from_ptr(attribute.name).to_bytes() {
                // Signed 64-bit values: their bits as the SDK's u64.
                b"revenue" | b"cancel.penalty" | b"fine.amount" | b"pay.amount" => amount = value.u64() as i64,
                b"fine.offence" => offence = value.string().unwrap_or_default(),
                b"source.name" => from = value.string().unwrap_or_default(),
                b"target.name" => to = value.string().unwrap_or_default(),
                _ => {}
            }
        }
        let job = || bridge::get().and_then(|bridge| bridge.plugin.job.read());
        let (kind, signed, detail) = match id.as_str() {
            "job.delivered" | "car_job.delivered" => {
                let job = job().filter(|job| job.active);
                let detail = job.map(|job| format!("{} → {}", job.cargo.as_str(), job.destination_city.as_str()));
                (MoneyKind::Delivered, amount, detail.unwrap_or_default())
            }
            "job.cancelled" | "car_job.cancelled" => {
                let detail = job().filter(|job| job.active).map(|job| job.cargo.as_str().to_owned());
                (MoneyKind::Cancelled, -amount, detail.unwrap_or_default())
            }
            "player.fined" => (MoneyKind::Fined, -amount, offence),
            "player.tollgate.paid" => (MoneyKind::Toll, -amount, String::new()),
            "player.use.ferry" => (MoneyKind::Ferry, -amount, format!("{from} → {to}")),
            "player.use.train" => (MoneyKind::Train, -amount, format!("{from} → {to}")),
            _ => return,
        };
        log!("money: {id} {signed:+} {detail}");
        let Some(bridge) = bridge::get() else { return };
        let game_time = bridge.plugin.telemetry.read().map_or(0, |telemetry| telemetry.game_time);
        let mut events: MoneyEvents = bridge.plugin.money.read().unwrap_or_default();
        events.events[(events.count % MONEY_EVENTS as u64) as usize] =
            MoneyEvent { kind: kind as u32, game_time, amount: signed, detail: FixedStr::new(&detail) };
        events.count += 1;
        bridge.plugin.money.write(&events);
    }
}
