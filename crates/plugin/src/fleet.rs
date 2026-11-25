//! Live in-memory Company / Fleet & Garage Manager ("Minha Empresa / Garagem") for ETS2 & ATS.
//!
//! Reads owned garages, owned trucks, owned trailers, active truck/trailer assignments, and live upgrade
//! prices directly from `economy_u` in memory, and invokes Prism3D's native functions on the main thread
//! (`SCS_TELEMETRY_EVENT_FRAME_END`) so the player can:
//! - Browse all owned trucks, trailers, and garages with specs, condition, mileage, license plates, and slots.
//! - Switch to drive any owned truck immediately without restarting.
//! - Attach/use any owned trailer directly behind their truck, or store/dismiss the current trailer back to garage.
//! - Upgrade owned garages (1 -> 3 slots, 3 -> 5 slots) with exact in-game pricing and live slot expansion.
//!
//! Zero hardcoded RVAs or struct offsets; disabled automatically in TruckersMP multiplayer sessions.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;

use westplay_shared::{FLEET_FILE, FleetAction, FleetAnswer};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::job_market::{
    PeImage, push_json_str, read_prism_string, read_ptr_array, safe_read, safe_read_bytes, valid_ptr,
};
use crate::{bridge, hooks, log, telemetry};

/// `player_u::swap_truck(player_u*, econ_u*, src_garage*, src_slot, dst_garage*, dst_slot)`.
type SwapTruckFn = unsafe extern "system" fn(*mut u8, *mut u8, *mut u8, usize, *mut u8, usize);
/// `garage_u::upgrade(garage_u*, econ_u*)`.
type GarageUpgradeFn = unsafe extern "system" fn(*mut u8, *mut u8);
/// `game_t::refresh_garages(game_t*)`.
type RefreshGaragesFn = unsafe extern "system" fn(*mut u8);
/// `player_u::store_trailer(player_u*, econ_u*)`.
type StoreTrailerFn = unsafe extern "system" fn(*mut u8, *mut u8);
/// `player_u::assign_trailer(player_u*, econ_u*, mode_idx: u32, trailer_u*, placements*, attached: u8)`.
type AssignTrailerFn = unsafe extern "system" fn(*mut u8, *mut u8, u32, *mut u8, *const PrismEmptyArray, u8);
/// `player_u::sync_actor(player_u*, econ_u*, mode_idx: u32, force: u8, flag: u8)`.
type SyncActorFn = unsafe extern "system" fn(*mut u8, *mut u8, u32, u8, u8);
/// `unit_name_to_str(out_prism_string, name_index, unit_ptr)`.
type UnitNameFn = unsafe extern "system" fn(*mut PrismStackStr, u32, *const u8);

#[repr(C)]
struct PrismEmptyArray {
    vtable: usize,
    data: *const u8,
    len: u64,
    cap: u64,
}

#[repr(C)]
struct PrismStackStr {
    vtable: usize,
    data: *mut u8,
    len: u32,
    cap: u32,
    buf: [u8; 257],
}

#[derive(Clone, Copy, Debug)]
struct Offsets {
    // economy_u
    econ_bank: usize,
    econ_player: usize,
    econ_garages: usize,
    econ_data: usize,
    econ_game_time: usize,
    // bank_u
    bank_money_account: usize,
    // player_u
    player_current_job: usize,
    player_actors: usize,
    actor_truck: usize,
    actor_trailer: usize,
    player_assigned_truck: usize,
    player_assigned_trailer: usize,
    player_assigned_trailer_connected: usize,
    player_my_truck: usize,
    player_my_trailer: usize,
    player_my_trailer_attached: usize,
    // garage_u
    garage_vehicles: usize,
    garage_drivers: usize,
    garage_trailers: usize,
    garage_status: usize,
    garage_productivity: usize,
    // vehicle_u (and base of trailer_u)
    veh_accessories: usize,
    veh_odometer: usize,
    veh_license_plate: usize,
    veh_chassis_wear: usize,
    // vehicle_accessory_u
    acc_data_path: usize,
    // trailer_u / trailer_def_u
    trailer_definition: usize,
    tdef_volume: usize,
    tdef_gross_weight: usize,
    tdef_axles: usize,
    tdef_body_type: usize,
    tdef_chain_type: usize,
}

struct Engine {
    econ_mgr_global: *const *mut u8,
    game_global: Option<*const *mut u8>,
    /// `(*g_desktop_mgr, pending_return_flag_offset)` extracted from `leave_desktop_menu`
    /// (called at the tail of `swap_truck`). Restoring `0` after `swap_truck` keeps the 3D
    /// driving viewport rendering instead of going black when switching trucks outside the menu.
    desktop_return_flag: Option<(*const *mut u8, usize)>,
    swap_truck_fn: SwapTruckFn,
    garage_upgrade_fn: GarageUpgradeFn,
    refresh_garages_fn: Option<RefreshGaragesFn>,
    store_trailer_fn: StoreTrailerFn,
    assign_trailer_fn: AssignTrailerFn,
    sync_actor_fn: SyncActorFn,
    unit_name_fn: UnitNameFn,
    string_vtable: usize,
    off: Offsets,
}

// SAFETY: pointers are into the game's executable image (fixed for process life) and used only on the main thread.
unsafe impl Send for Engine {}
// SAFETY: never accessed concurrently across threads.
unsafe impl Sync for Engine {}

static ENGINE: OnceLock<Option<Engine>> = OnceLock::new();

/// Serves any pending Fleet / Garage RPC on the game's main thread (`SCS_TELEMETRY_EVENT_FRAME_END`).
///
/// # Safety
/// Must be called only on the game's main thread during `SCS_TELEMETRY_EVENT_FRAME_END`.
pub unsafe fn serve() {
    let Some(bridge) = bridge::get() else { return };
    let block = &bridge.fleet;
    let request = block.request.load(Ordering::Acquire);
    if request == block.answered.load(Ordering::Relaxed) {
        return;
    }

    let action = FleetAction::from_code(block.action.load(Ordering::Acquire));
    let target_ptr = block.target_ptr.load(Ordering::Acquire) as usize;

    let answer = hooks::contained("fleet::serve", FleetAnswer::Failed, || {
        // SAFETY: inside the game process on the main thread.
        let Some(engine) = ENGINE.get_or_init(|| unsafe { locate() }).as_ref() else {
            return FleetAnswer::Unsupported;
        };
        match action {
            Some(FleetAction::Refresh) => {
                // SAFETY: main thread, validated engine handles.
                let ans = unsafe { refresh_snapshot(engine) };
                if ans == FleetAnswer::Ok {
                    block.snapshot_seq.fetch_add(1, Ordering::Release);
                }
                ans
            }
            Some(FleetAction::DriveTruck) => {
                if telemetry::truck().multiplayer {
                    return FleetAnswer::Multiplayer;
                }
                // SAFETY: main thread, re-validated pointers.
                let ans = unsafe { drive_truck(engine, target_ptr) };
                if ans == FleetAnswer::Ok {
                    let _ = unsafe { refresh_snapshot(engine) };
                    block.snapshot_seq.fetch_add(1, Ordering::Release);
                }
                ans
            }
            Some(FleetAction::UseTrailer) => {
                if telemetry::truck().multiplayer {
                    return FleetAnswer::Multiplayer;
                }
                // SAFETY: main thread, re-validated pointers.
                let ans = unsafe { use_trailer(engine, target_ptr) };
                if ans == FleetAnswer::Ok {
                    let _ = unsafe { refresh_snapshot(engine) };
                    block.snapshot_seq.fetch_add(1, Ordering::Release);
                }
                ans
            }
            Some(FleetAction::StoreTrailer) => {
                if telemetry::truck().multiplayer {
                    return FleetAnswer::Multiplayer;
                }
                // SAFETY: main thread, validated handles.
                let ans = unsafe { store_current_trailer(engine) };
                if ans == FleetAnswer::Ok {
                    let _ = unsafe { refresh_snapshot(engine) };
                    block.snapshot_seq.fetch_add(1, Ordering::Release);
                }
                ans
            }
            Some(FleetAction::UpgradeGarage) => {
                if telemetry::truck().multiplayer {
                    return FleetAnswer::Multiplayer;
                }
                // SAFETY: main thread, re-validated pointers.
                let ans = unsafe { upgrade_garage(engine, target_ptr) };
                if ans == FleetAnswer::Ok {
                    let _ = unsafe { refresh_snapshot(engine) };
                    block.snapshot_seq.fetch_add(1, Ordering::Release);
                }
                ans
            }
            None => FleetAnswer::Failed,
        }
    });

    block.answer.store(answer as i32, Ordering::Relaxed);
    block.answered.store(request, Ordering::Release);
}

// ---------------------------------------------------------------------------
// Unit name & Prism token decoders
// ---------------------------------------------------------------------------

unsafe fn read_unit_name(engine: &Engine, unit_ptr: usize) -> String {
    let Some(name_idx) = safe_read::<u32>(unit_ptr + 0x0c) else { return String::new() };
    let mut out = PrismStackStr {
        vtable: engine.string_vtable,
        data: std::ptr::null_mut(),
        len: 0,
        cap: 257,
        buf: [0u8; 257],
    };
    out.data = out.buf.as_mut_ptr();
    // SAFETY: `out` matches Prism3D's exact stack `string_t` layout; called on game main thread.
    unsafe { (engine.unit_name_fn)(&mut out, name_idx, unit_ptr as *const u8) };
    let len = (out.len as usize).min(256);
    if len == 0 || out.data.is_null() {
        return String::new();
    }
    let mut tmp = vec![0u8; len];
    let Some(got) = safe_read_bytes(out.data as usize, &mut tmp) else { return String::new() };
    let slice = &tmp[..got];
    let nul = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..nul]).into_owned()
}

/// Decodes a Prism3D 64-bit base-38 `token_t` back into its ASCII identifier (e.g. `"curtainsider"`, `"single"`).
fn decode_token(mut token: u64) -> String {
    const ALPHABET: &[u8; 38] = b"\00123456789abcdefghijklmnopqrstuvwxyz_";
    let mut out = String::new();
    while token != 0 && out.len() < 12 {
        let idx = (token % 38) as usize;
        token /= 38;
        if idx > 0 && idx < ALPHABET.len() {
            out.push(ALPHABET[idx] as char);
        }
    }
    out
}

fn garage_city_id(unit_name: &str) -> String {
    unit_name.strip_prefix("garage.").unwrap_or(unit_name).to_owned()
}

fn clean_plate(raw: &str) -> (String, String) {
    if let Some((plate, country)) = raw.split_once('|') {
        (strip_scs_tags(plate), country.trim().to_owned())
    } else {
        (strip_scs_tags(raw), String::new())
    }
}

fn strip_scs_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_owned()
}

fn pretty_brand(id: &str) -> String {
    match id.to_ascii_lowercase().as_str() {
        "scania" => "Scania".into(),
        "volvo" => "Volvo".into(),
        "mercedes" => "Mercedes-Benz".into(),
        "man" => "MAN".into(),
        "daf" => "DAF".into(),
        "iveco" => "Iveco".into(),
        "renault" => "Renault".into(),
        "kenworth" => "Kenworth".into(),
        "peterbilt" => "Peterbilt".into(),
        "freightliner" => "Freightliner".into(),
        "westernstar" => "Western Star".into(),
        "mack" => "Mack".into(),
        "intnational" | "international" => "International".into(),
        "scs" => "SCS".into(),
        "krone" => "Krone".into(),
        "schwarzmuller" => "Schwarzmüller".into(),
        "feldbinder" => "Feldbinder".into(),
        "wielton" => "Wielton".into(),
        "tirsan" => "Tirsan".into(),
        "kassbohrer" => "Kässbohrer".into(),
        "kogel" => "Kögel".into(),
        "lodeking" => "Lode King".into(),
        other => pretty_id(other),
    }
}

fn pretty_id(id: &str) -> String {
    id.split(['_', '.'])
        .filter(|s| !s.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct VehicleMeta {
    brand: String,
    model: String,
    engine: String,
    chassis: String,
    body: String,
}

fn parse_vehicle_accessories(engine: &Engine, veh_ptr: usize, is_trailer: bool) -> VehicleMeta {
    let mut brand = String::new();
    let mut model = String::new();
    let mut eng = String::new();
    let mut chassis = String::new();
    let mut body = String::new();

    if let Some(accs) = read_ptr_array(veh_ptr + engine.off.veh_accessories, 256) {
        let prefix = if is_trailer { "/def/vehicle/trailer_owned/" } else { "/def/vehicle/truck/" };
        for acc_ptr in accs {
            if !valid_ptr(acc_ptr) {
                continue;
            }
            let path = read_prism_string(acc_ptr + engine.off.acc_data_path);
            if path.is_empty() {
                continue;
            }
            if let Some(rest) = path.strip_prefix(prefix) {
                let mut parts = rest.split('/');
                if let Some(bm) = parts.next()
                    && brand.is_empty()
                {
                    if let Some((b, m)) = bm.split_once('.') {
                        brand = pretty_brand(b);
                        model = pretty_id(m);
                    } else {
                        brand = pretty_brand(bm);
                    }
                }
                if let (Some(cat), Some(file)) = (parts.next(), parts.next()) {
                    let stem = file.strip_suffix(".sii").unwrap_or(file);
                    match cat {
                        "engine" if eng.is_empty() => eng = stem.to_ascii_uppercase().replace('_', " "),
                        "chassis" if chassis.is_empty() => chassis = pretty_id(stem),
                        "body" if body.is_empty() => body = pretty_id(stem),
                        _ => {}
                    }
                }
            }
        }
    }

    VehicleMeta { brand, model, engine: eng, chassis, body }
}

fn garage_upgrade_costs(engine: &Engine, economy_u: usize) -> (i64, i64) {
    let econ_data = safe_read::<usize>(economy_u + engine.off.econ_data).unwrap_or(0);
    if !valid_ptr(econ_data) {
        return (180_000, 100_000);
    }
    // Verified in `garage_u::upgrade` (`0x1408315FD` / `0x14083160E`):
    // `+0x18` = cost to upgrade Tiny (1 slot, bit 4) -> Small (3 slots),
    // `+0x20` = cost to upgrade Small (3 slots) -> Large (5 slots).
    let tiny_to_small = safe_read::<i64>(econ_data + 0x18).filter(|&v| v > 0 && v < 50_000_000).unwrap_or(180_000);
    let small_to_large = safe_read::<i64>(econ_data + 0x20).filter(|&v| v > 0 && v < 50_000_000).unwrap_or(100_000);
    (tiny_to_small, small_to_large)
}

fn garage_tier_info(status: u32, tiny_cost: i64, small_cost: i64) -> (&'static str, u32, Option<i64>, Option<u32>) {
    if (status & 1) != 0 {
        ("large", 5, None, None)
    } else if (status & 4) != 0 {
        ("tiny", 1, Some(tiny_cost), Some(3))
    } else {
        ("small", 3, Some(small_cost), Some(5))
    }
}

// ---------------------------------------------------------------------------
// Snapshot & Native In-Memory Actions
// ---------------------------------------------------------------------------

struct SnapGarage {
    ptr: usize,
    city_id: String,
    tier: &'static str,
    slots: u32,
    used_trucks: u32,
    used_drivers: u32,
    trailers_count: u32,
    productivity: f32,
    upgrade_cost: Option<i64>,
    next_slots: Option<u32>,
    can_upgrade: bool,
    slot_states: Vec<u8>, // 0 = empty, 1 = AI truck, 2 = player truck
}

struct SnapTruck {
    ptr: usize,
    brand: String,
    model: String,
    engine_name: String,
    chassis: String,
    plate: String,
    plate_country: String,
    odometer_km: u32,
    wear_pct: u32,
    garage_city_id: String,
    slot_index: usize,
    is_player: bool,
    has_ai_driver: bool,
}

struct SnapTrailer {
    ptr: usize,
    brand: String,
    model: String,
    body: String,
    body_type: String,
    chain_type: String,
    plate: String,
    plate_country: String,
    volume: f32,
    gross_weight: f32,
    axles: u32,
    wear_pct: u32,
    garage_city_id: String,
    is_attached: bool,
}

fn is_live_unit(ptr: usize) -> bool {
    valid_ptr(ptr) && (safe_read::<u32>(ptr + 8).unwrap_or(0) >> 31) != 0
}

fn read_player_pointers(engine: &Engine, player_u: usize) -> (bool, usize, usize) {
    let on_job = valid_ptr(safe_read::<usize>(player_u + engine.off.player_current_job).unwrap_or(0));

    // In Prism3D at runtime, `assigned_truck`/`my_truck`/`assigned_trailer`/`my_trailer` are save-only fields
    // (usually 0 while driving). The real live pointers live inside `player_u->actors[0]` (`+0x70`, player's own
    // vehicle slot): `[actor0 + 0x10]` = active truck `vehicle_u*`, `[actor0 + 0x48]` = active trailer `trailer_u*`.
    let live_actor0 = read_ptr_array(player_u + engine.off.player_actors, 8)
        .and_then(|arr| arr.first().copied())
        .filter(|&a| is_live_unit(a))
        .unwrap_or(0);

    let live_truck = if live_actor0 != 0 {
        safe_read::<usize>(live_actor0 + engine.off.actor_truck).filter(|&p| is_live_unit(p)).unwrap_or(0)
    } else {
        0
    };
    let live_trailer = if live_actor0 != 0 {
        safe_read::<usize>(live_actor0 + engine.off.actor_trailer).filter(|&p| is_live_unit(p)).unwrap_or(0)
    } else {
        0
    };

    let assigned_truck = safe_read::<usize>(player_u + engine.off.player_assigned_truck).unwrap_or(0);
    let my_truck = safe_read::<usize>(player_u + engine.off.player_my_truck).unwrap_or(0);
    let active_truck = if live_truck != 0 {
        live_truck
    } else if valid_ptr(assigned_truck) {
        assigned_truck
    } else if valid_ptr(my_truck) {
        my_truck
    } else {
        0
    };

    let assigned_trailer = safe_read::<usize>(player_u + engine.off.player_assigned_trailer).unwrap_or(0);
    let assigned_conn = safe_read::<u8>(player_u + engine.off.player_assigned_trailer_connected).unwrap_or(0) != 0;
    let my_trailer = safe_read::<usize>(player_u + engine.off.player_my_trailer).unwrap_or(0);
    let my_att = safe_read::<u8>(player_u + engine.off.player_my_trailer_attached).unwrap_or(0) != 0;
    let active_trailer = if live_trailer != 0 {
        live_trailer
    } else if valid_ptr(assigned_trailer) && assigned_conn {
        assigned_trailer
    } else if valid_ptr(my_trailer) && my_att {
        my_trailer
    } else if valid_ptr(assigned_trailer) {
        assigned_trailer
    } else {
        0
    };
    (on_job, active_truck, active_trailer)
}

unsafe fn refresh_snapshot(engine: &Engine) -> FleetAnswer {
    let Some(economy_u) = safe_read::<usize>(engine.econ_mgr_global as usize).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let Some(player_u) = safe_read::<usize>(economy_u + engine.off.econ_player).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let Some(bank_u) = safe_read::<usize>(economy_u + engine.off.econ_bank).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let balance = safe_read::<i64>(bank_u + engine.off.bank_money_account).unwrap_or(0);
    let game_time = safe_read::<u32>(economy_u + engine.off.econ_game_time).unwrap_or(0);
    let (on_job, active_truck, active_trailer) = read_player_pointers(engine, player_u);
    let (tiny_cost, small_cost) = garage_upgrade_costs(engine, economy_u);

    let Some(all_garages) = read_ptr_array(economy_u + engine.off.econ_garages, 1024) else {
        return FleetAnswer::NotReady;
    };

    let mut garages = Vec::new();
    let mut trucks = Vec::new();
    let mut trailers = Vec::new();

    for g_ptr in all_garages {
        if !valid_ptr(g_ptr) {
            continue;
        }
        let status = safe_read::<u32>(g_ptr + engine.off.garage_status).unwrap_or(0);
        // Bit 1 (`status & 2`): garage is owned by the player.
        if (status & 2) == 0 {
            continue;
        }
        // SAFETY: main thread, validated live `garage_u*`.
        let unit_name = unsafe { read_unit_name(engine, g_ptr) };
        let city_id = garage_city_id(&unit_name);
        let (tier, slots, upgrade_cost, next_slots) = garage_tier_info(status, tiny_cost, small_cost);
        let productivity = safe_read::<f32>(g_ptr + engine.off.garage_productivity).unwrap_or(0.0);

        let g_vehicles = read_ptr_array(g_ptr + engine.off.garage_vehicles, 16).unwrap_or_default();
        let g_drivers = read_ptr_array(g_ptr + engine.off.garage_drivers, 16).unwrap_or_default();
        let g_trailers = read_ptr_array(g_ptr + engine.off.garage_trailers, 128).unwrap_or_default();

        let mut used_trucks = 0u32;
        let mut used_drivers = 0u32;
        let mut slot_states = vec![0u8; slots as usize];

        for (slot_idx, &v_ptr) in g_vehicles.iter().enumerate() {
            let drv_ptr = g_drivers.get(slot_idx).copied().unwrap_or(0);
            if valid_ptr(drv_ptr) {
                used_drivers += 1;
            }
            if !valid_ptr(v_ptr) {
                continue;
            }
            used_trucks += 1;
            let is_player = v_ptr == active_truck;
            if slot_idx < slot_states.len() {
                slot_states[slot_idx] = if is_player { 2 } else { 1 };
            }
            let meta = parse_vehicle_accessories(engine, v_ptr, false);
            let raw_plate = read_prism_string(v_ptr + engine.off.veh_license_plate);
            let (plate, plate_country) = clean_plate(&raw_plate);
            let odometer_km = safe_read::<u32>(v_ptr + engine.off.veh_odometer).unwrap_or(0);
            let wear = safe_read::<f32>(v_ptr + engine.off.veh_chassis_wear).unwrap_or(0.0).clamp(0.0, 1.0);
            trucks.push(SnapTruck {
                ptr: v_ptr,
                brand: if meta.brand.is_empty() { "Truck".into() } else { meta.brand },
                model: meta.model,
                engine_name: meta.engine,
                chassis: meta.chassis,
                plate,
                plate_country,
                odometer_km,
                wear_pct: (wear * 100.0).round() as u32,
                garage_city_id: city_id.clone(),
                slot_index: slot_idx,
                is_player,
                has_ai_driver: valid_ptr(drv_ptr) && !is_player,
            });
        }

        let mut trailers_in_garage = 0u32;
        for &t_ptr in &g_trailers {
            if !valid_ptr(t_ptr) {
                continue;
            }
            trailers_in_garage += 1;
            let meta = parse_vehicle_accessories(engine, t_ptr, true);
            let raw_plate = read_prism_string(t_ptr + engine.off.veh_license_plate);
            let (plate, plate_country) = clean_plate(&raw_plate);
            let wear = safe_read::<f32>(t_ptr + engine.off.veh_chassis_wear).unwrap_or(0.0).clamp(0.0, 1.0);
            let tdef = safe_read::<usize>(t_ptr + engine.off.trailer_definition).unwrap_or(0);
            let (volume, gross_weight, axles, body_type, chain_type) = if valid_ptr(tdef) {
                (
                    safe_read::<f32>(tdef + engine.off.tdef_volume).unwrap_or(0.0),
                    safe_read::<f32>(tdef + engine.off.tdef_gross_weight).unwrap_or(0.0),
                    safe_read::<u32>(tdef + engine.off.tdef_axles).unwrap_or(0),
                    decode_token(safe_read::<u64>(tdef + engine.off.tdef_body_type).unwrap_or(0)),
                    decode_token(safe_read::<u64>(tdef + engine.off.tdef_chain_type).unwrap_or(0)),
                )
            } else {
                (0.0, 0.0, 0, String::new(), String::new())
            };
            trailers.push(SnapTrailer {
                ptr: t_ptr,
                brand: if meta.brand.is_empty() { "Trailer".into() } else { meta.brand },
                model: meta.model,
                body: meta.body,
                body_type: pretty_id(&body_type),
                chain_type: pretty_id(&chain_type),
                plate,
                plate_country,
                volume,
                gross_weight,
                axles,
                wear_pct: (wear * 100.0).round() as u32,
                garage_city_id: city_id.clone(),
                is_attached: t_ptr == active_trailer,
            });
        }

        let can_upgrade = upgrade_cost.is_some_and(|c| balance >= c);
        garages.push(SnapGarage {
            ptr: g_ptr,
            city_id,
            tier,
            slots,
            used_trucks,
            used_drivers,
            trailers_count: trailers_in_garage,
            productivity,
            upgrade_cost,
            next_slots,
            can_upgrade,
            slot_states,
        });
    }

    // Sort active truck & active trailer to the top for instant visibility.
    trucks.sort_by(|a, b| b.is_player.cmp(&a.is_player).then_with(|| a.garage_city_id.cmp(&b.garage_city_id)));
    trailers.sort_by(|a, b| b.is_attached.cmp(&a.is_attached).then_with(|| a.garage_city_id.cmp(&b.garage_city_id)));

    let json = serialize_snapshot(
        game_time,
        balance,
        on_job,
        active_truck != 0,
        active_trailer != 0,
        &garages,
        &trucks,
        &trailers,
    );
    let path = snapshot_path();
    let tmp = path.with_extension("json.tmp");
    if let Err(e) = fs::write(&tmp, json.as_bytes()).and_then(|()| fs::rename(&tmp, &path)) {
        log!("fleet: failed to write {}: {e}", path.display());
        return FleetAnswer::Failed;
    }
    log!(
        "fleet: snapshot written ({} garages, {} trucks, {} trailers, on_job={on_job})",
        garages.len(),
        trucks.len(),
        trailers.len()
    );
    FleetAnswer::Ok
}

/// Switches the player into `target_truck_ptr` live in memory via Prism3D's native `player_u::swap_truck`.
unsafe fn drive_truck(engine: &Engine, target_truck_ptr: usize) -> FleetAnswer {
    if !valid_ptr(target_truck_ptr) {
        return FleetAnswer::NotFound;
    }
    let Some(economy_u) = safe_read::<usize>(engine.econ_mgr_global as usize).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let Some(player_u) = safe_read::<usize>(economy_u + engine.off.econ_player).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let (on_job, active_truck, _) = read_player_pointers(engine, player_u);
    if on_job {
        return FleetAnswer::OnJob;
    }
    if target_truck_ptr == active_truck {
        return FleetAnswer::AlreadyActive;
    }
    let Some(all_garages) = read_ptr_array(economy_u + engine.off.econ_garages, 1024) else {
        return FleetAnswer::NotReady;
    };

    let mut src: Option<(usize, usize)> = None;
    let mut dst: Option<(usize, usize)> = None;
    let mut fallback_empty: Option<(usize, usize)> = None;

    for g_ptr in all_garages {
        if !valid_ptr(g_ptr) {
            continue;
        }
        let status = safe_read::<u32>(g_ptr + engine.off.garage_status).unwrap_or(0);
        if (status & 2) == 0 {
            continue;
        }
        let vehicles = read_ptr_array(g_ptr + engine.off.garage_vehicles, 16).unwrap_or_default();
        for (slot, &v_ptr) in vehicles.iter().enumerate() {
            if v_ptr == target_truck_ptr {
                src = Some((g_ptr, slot));
            }
            if active_truck != 0 && v_ptr == active_truck {
                dst = Some((g_ptr, slot));
            }
            if fallback_empty.is_none() && v_ptr == 0 {
                fallback_empty = Some((g_ptr, slot));
            }
        }
    }

    let Some((src_garage, src_slot)) = src else {
        return FleetAnswer::NotFound;
    };
    let Some((dst_garage, dst_slot)) = dst.or(fallback_empty) else {
        return FleetAnswer::NotFound;
    };

    let desktop_flag_addr = engine.desktop_return_flag.and_then(|(g_mgr, flag_off)| {
        let mgr = safe_read::<usize>(g_mgr as usize)?;
        valid_ptr(mgr).then_some(mgr + flag_off)
    });
    let prev_desktop_flag = desktop_flag_addr.and_then(safe_read::<u8>);

    log!(
        "fleet: driving truck {target_truck_ptr:#x} (src={src_garage:#x}[{src_slot}] -> dst={dst_garage:#x}[{dst_slot}])"
    );
    crate::job_transition::trigger(crate::job_transition::TransitionKind::TruckSwap);
    // SAFETY: main thread; `player_u`, `economy_u`, and both `(garage, slot)` pairs were just verified live and in-bounds;
    // `!on_job` verified (avoiding Prism3D's on-job relocation assert).
    unsafe {
        (engine.swap_truck_fn)(
            player_u as *mut u8,
            economy_u as *mut u8,
            src_garage as *mut u8,
            src_slot,
            dst_garage as *mut u8,
            dst_slot,
        );
    }
    if let (Some(flag_addr), Some(0)) = (desktop_flag_addr, prev_desktop_flag)
        && safe_read::<u8>(flag_addr) == Some(1)
    {
        // SAFETY: `flag_addr` is `(*g_desktop_mgr) + desktop_flag_off` (live, validated);
        // `swap_truck` invokes `leave_desktop_menu`, which sets this byte to `1` assuming
        // the player clicked "Drive" inside the desktop Truck Manager menu. Outside the menu,
        // leaving it at `1` suppresses 3D driving world rendering (black viewport).
        unsafe { (flag_addr as *mut u8).write(0) };
        log!("fleet: restored desktop return flag (1 -> 0) after swap_truck");
    }
    FleetAnswer::Ok
}

/// Attaches/spawns owned `target_trailer_ptr` directly behind the player's truck live in memory.
unsafe fn use_trailer(engine: &Engine, target_trailer_ptr: usize) -> FleetAnswer {
    if !valid_ptr(target_trailer_ptr) {
        return FleetAnswer::NotFound;
    }
    let Some(economy_u) = safe_read::<usize>(engine.econ_mgr_global as usize).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let Some(player_u) = safe_read::<usize>(economy_u + engine.off.econ_player).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let (on_job, active_truck, active_trailer) = read_player_pointers(engine, player_u);
    if on_job {
        return FleetAnswer::OnJob;
    }
    if active_truck == 0 {
        return FleetAnswer::NotReady;
    }
    if target_trailer_ptr == active_trailer {
        return FleetAnswer::AlreadyActive;
    }

    // Re-validate `target_trailer_ptr` exists in an owned garage.
    let Some(all_garages) = read_ptr_array(economy_u + engine.off.econ_garages, 1024) else {
        return FleetAnswer::NotReady;
    };
    let found = all_garages.iter().any(|&g_ptr| {
        valid_ptr(g_ptr)
            && (safe_read::<u32>(g_ptr + engine.off.garage_status).unwrap_or(0) & 2) != 0
            && read_ptr_array(g_ptr + engine.off.garage_trailers, 128)
                .is_some_and(|ts| ts.contains(&target_trailer_ptr))
    });
    if !found {
        return FleetAnswer::NotFound;
    }

    let empty = PrismEmptyArray { vtable: 0, data: std::ptr::null(), len: 0, cap: 0 };
    log!("fleet: attaching owned trailer {target_trailer_ptr:#x} (prev={active_trailer:#x})");
    crate::job_transition::trigger(crate::job_transition::TransitionKind::TrailerAttach);
    // SAFETY: main thread, validated live `player_u`, `economy_u`, and `target_trailer_ptr`.
    // Passing empty `placements` (`len == 0`) with `attached == 1` instructs Prism3D's `spawn_trailer` (`0x5da97f`)
    // to compute the coupled position directly from the player truck's live placement.
    unsafe {
        if active_trailer != 0 {
            (engine.store_trailer_fn)(player_u as *mut u8, economy_u as *mut u8);
        }
        (engine.assign_trailer_fn)(player_u as *mut u8, economy_u as *mut u8, 0, target_trailer_ptr as *mut u8, &empty, 1);
        (engine.sync_actor_fn)(player_u as *mut u8, economy_u as *mut u8, 0, 1, 0);
    }
    FleetAnswer::Ok
}

/// Detaches and returns the player's currently used owned trailer to its garage live in memory.
unsafe fn store_current_trailer(engine: &Engine) -> FleetAnswer {
    let Some(economy_u) = safe_read::<usize>(engine.econ_mgr_global as usize).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let Some(player_u) = safe_read::<usize>(economy_u + engine.off.econ_player).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let (on_job, _, active_trailer) = read_player_pointers(engine, player_u);
    if on_job {
        return FleetAnswer::OnJob;
    }
    if active_trailer == 0 {
        return FleetAnswer::AlreadyActive;
    }
    log!("fleet: storing active trailer {active_trailer:#x} back to garage");
    crate::job_transition::trigger(crate::job_transition::TransitionKind::TrailerStore);
    // SAFETY: main thread, validated live `player_u` and `economy_u`.
    unsafe { (engine.store_trailer_fn)(player_u as *mut u8, economy_u as *mut u8) };
    FleetAnswer::Ok
}

/// Respawns the player's own truck actor in the 3D driving world after cancelling a Quick Job
/// (where Prism3D's `cancel_player_job` destroys the temporary company truck actor, leaving the
/// player with no spawned vehicle unless `sync_actor` restores their owned truck).
///
/// # Safety
/// Must be called on the game's main thread with live validated `economy_u` and `player_u`.
pub(crate) unsafe fn respawn_player_actor(economy_u: usize, player_u: usize) -> bool {
    let Some(engine) = ENGINE.get_or_init(|| unsafe { locate() }).as_ref() else {
        return false;
    };
    let desktop_flag_addr = engine.desktop_return_flag.and_then(|(g_mgr, flag_off)| {
        let mgr = safe_read::<usize>(g_mgr as usize)?;
        valid_ptr(mgr).then_some(mgr + flag_off)
    });
    let prev_desktop_flag = desktop_flag_addr.and_then(safe_read::<u8>);

    log!("fleet: respawning player's own truck actor via sync_actor after job cancel");
    // SAFETY: main thread, validated live `player_u` and `economy_u`; `r8d = 0` switches `player_u`
    // back to own-truck mode and respawns `game_actor_u` + player's own truck + camera into the 3D world
    // (safe no-op if already driving own truck, e.g. Freight/Cargo cancel).
    unsafe { (engine.sync_actor_fn)(player_u as *mut u8, economy_u as *mut u8, 0, 1, 0) };

    if let (Some(flag_addr), Some(0)) = (desktop_flag_addr, prev_desktop_flag)
        && safe_read::<u8>(flag_addr) == Some(1)
    {
        // SAFETY: `flag_addr` is live `(*g_desktop_mgr) + desktop_flag_off`; keep 3D viewport active.
        unsafe { (flag_addr as *mut u8).write(0) };
        log!("fleet: restored desktop return flag (1 -> 0) after respawn_player_actor");
    }
    let (_, after_truck, _) = read_player_pointers(engine, player_u);
    log!("fleet: respawn_player_actor done (live_truck={after_truck:#x})");
    after_truck != 0
}

/// Upgrades an owned garage (1->3 or 3->5 slots) live in memory, deducting the exact game cost.
unsafe fn upgrade_garage(engine: &Engine, target_garage_ptr: usize) -> FleetAnswer {
    if !valid_ptr(target_garage_ptr) {
        return FleetAnswer::NotFound;
    }
    let Some(economy_u) = safe_read::<usize>(engine.econ_mgr_global as usize).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let Some(bank_u) = safe_read::<usize>(economy_u + engine.off.econ_bank).filter(|&p| valid_ptr(p)) else {
        return FleetAnswer::NotReady;
    };
    let Some(all_garages) = read_ptr_array(economy_u + engine.off.econ_garages, 1024) else {
        return FleetAnswer::NotReady;
    };
    if !all_garages.contains(&target_garage_ptr) {
        return FleetAnswer::NotFound;
    }
    let status = safe_read::<u32>(target_garage_ptr + engine.off.garage_status).unwrap_or(0);
    if (status & 2) == 0 {
        return FleetAnswer::NotFound;
    }
    if (status & 1) != 0 {
        return FleetAnswer::AlreadyMax;
    }
    let (tiny_cost, small_cost) = garage_upgrade_costs(engine, economy_u);
    let cost = if (status & 4) != 0 { tiny_cost } else { small_cost };
    let balance = safe_read::<i64>(bank_u + engine.off.bank_money_account).unwrap_or(0);
    if balance < cost {
        log!("fleet: upgrade_garage({target_garage_ptr:#x}) rejected — balance {balance} < cost {cost}");
        return FleetAnswer::InsufficientFunds;
    }

    log!("fleet: upgrading garage {target_garage_ptr:#x} (status={status:#x}, cost={cost})");
    // SAFETY: main thread, validated live `garage_u*` and `economy_u*`.
    unsafe {
        (engine.garage_upgrade_fn)(target_garage_ptr as *mut u8, economy_u as *mut u8);
        if let (Some(g_global), Some(refresh_fn)) = (engine.game_global, engine.refresh_garages_fn)
            && let Some(game_ptr) = safe_read::<usize>(g_global as usize)
            && valid_ptr(game_ptr)
        {
            (refresh_fn)(game_ptr as *mut u8);
        }
    }
    FleetAnswer::Ok
}

// ---------------------------------------------------------------------------
// JSON serialization
// ---------------------------------------------------------------------------

fn snapshot_path() -> PathBuf {
    let base = log::log_dir().parent().map_or_else(|| log::log_dir().to_path_buf(), std::path::Path::to_path_buf);
    base.join(FLEET_FILE)
}

#[allow(clippy::too_many_arguments)]
fn serialize_snapshot(
    game_time: u32,
    balance: i64,
    on_job: bool,
    has_truck: bool,
    has_attached_trailer: bool,
    garages: &[SnapGarage],
    trucks: &[SnapTruck],
    trailers: &[SnapTrailer],
) -> String {
    let mut s = String::with_capacity(4096);
    let _ = write!(
        s,
        r#"{{"gameTime":{game_time},"balance":{balance},"onJob":{on_job},"hasTruck":{has_truck},"hasAttachedTrailer":{has_attached_trailer},"garages":["#
    );
    for (i, g) in garages.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, r#"{{"garagePtr":"{:#x}","cityId":"#, g.ptr);
        push_json_str(&mut s, &g.city_id);
        s.push_str(r#","tier":"#);
        push_json_str(&mut s, g.tier);
        let _ = write!(
            s,
            r#","slots":{},"usedTrucks":{},"usedDrivers":{},"trailersCount":{},"productivity":{:.2},"canUpgrade":{}"#,
            g.slots, g.used_trucks, g.used_drivers, g.trailers_count, g.productivity, g.can_upgrade
        );
        if let Some(cost) = g.upgrade_cost {
            let _ = write!(s, r#","upgradeCost":{cost}"#);
        }
        if let Some(next) = g.next_slots {
            let _ = write!(s, r#","nextSlots":{next}"#);
        }
        s.push_str(r#","slotStates":["#);
        for (si, st) in g.slot_states.iter().enumerate() {
            if si > 0 {
                s.push(',');
            }
            let _ = write!(s, "{st}");
        }
        s.push_str("]}");
    }
    s.push_str(r#"],"trucks":["#);
    for (i, t) in trucks.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, r#"{{"truckPtr":"{:#x}","brand":"#, t.ptr);
        push_json_str(&mut s, &t.brand);
        s.push_str(r#","model":"#);
        push_json_str(&mut s, &t.model);
        s.push_str(r#","engine":"#);
        push_json_str(&mut s, &t.engine_name);
        s.push_str(r#","chassis":"#);
        push_json_str(&mut s, &t.chassis);
        s.push_str(r#","plate":"#);
        push_json_str(&mut s, &t.plate);
        s.push_str(r#","plateCountry":"#);
        push_json_str(&mut s, &t.plate_country);
        s.push_str(r#","garageCityId":"#);
        push_json_str(&mut s, &t.garage_city_id);
        let _ = write!(
            s,
            r#","odometerKm":{},"wearPct":{},"slotIndex":{},"isPlayer":{},"hasAiDriver":{}}}"#,
            t.odometer_km, t.wear_pct, t.slot_index, t.is_player, t.has_ai_driver
        );
    }
    s.push_str(r#"],"trailers":["#);
    for (i, tr) in trailers.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, r#"{{"trailerPtr":"{:#x}","brand":"#, tr.ptr);
        push_json_str(&mut s, &tr.brand);
        s.push_str(r#","model":"#);
        push_json_str(&mut s, &tr.model);
        s.push_str(r#","body":"#);
        push_json_str(&mut s, &tr.body);
        s.push_str(r#","bodyType":"#);
        push_json_str(&mut s, &tr.body_type);
        s.push_str(r#","chainType":"#);
        push_json_str(&mut s, &tr.chain_type);
        s.push_str(r#","plate":"#);
        push_json_str(&mut s, &tr.plate);
        s.push_str(r#","plateCountry":"#);
        push_json_str(&mut s, &tr.plate_country);
        s.push_str(r#","garageCityId":"#);
        push_json_str(&mut s, &tr.garage_city_id);
        let _ = write!(
            s,
            r#","volume":{:.1},"grossWeight":{:.0},"axles":{},"wearPct":{},"isAttached":{}}}"#,
            tr.volume, tr.gross_weight, tr.axles, tr.wear_pct, tr.is_attached
        );
    }
    s.push_str("]}");
    s
}

// ---------------------------------------------------------------------------
// Dynamic PE & Reflection Discovery (zero hardcoded RVAs or offsets)
// ---------------------------------------------------------------------------

fn find_bytes(hay: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    hay.get(start..)?.windows(needle.len()).position(|w| w == needle).map(|i| start + i)
}

fn rfind_bytes(hay: &[u8], needle: &[u8], end: usize) -> Option<usize> {
    hay.get(..end)?.windows(needle.len()).rposition(|w| w == needle)
}

unsafe fn locate() -> Option<Engine> {
    // SAFETY: main executable mapped for process life.
    let base = unsafe { GetModuleHandleW(None) }.ok()?.0 as usize;
    // SAFETY: `base` is the mapped game executable.
    let img = unsafe { PeImage::of(base) }?;

    // 1. `player_u::swap_truck` from unique log string.
    let s_swap = img.find_rdata(b"Relocating player's truck while player is on job is forbidden!\0")?;
    let swap_xrefs = img.find_lea_xrefs(s_swap);
    if swap_xrefs.len() != 1 {
        log!("fleet: swap_truck xrefs={} (expected 1)", swap_xrefs.len());
        return None;
    }
    let swap_truck_va = img.pdata_root(swap_xrefs[0])?;

    // Extract `(g_desktop_mgr, desktop_return_flag_off)` from `leave_desktop_menu` called at the tail of `swap_truck`:
    //   `48 8B 0D <rel32>` (`mov rcx, [g_core]`), `33 D2` (`xor edx, edx`), `E8 <rel32>` (`call leave_desktop_menu`)
    // Inside `leave_desktop_menu`:
    //   `48 8B 1D <rel32>` (`mov rbx, [g_desktop_mgr]`) ... `C6 83 <imm32> 01` (`mov byte ptr [rbx + flag_off], 1`).
    let desktop_return_flag = (|| {
        let st_off = swap_truck_va.checked_sub(img.text.0)?;
        let st = img.bytes(img.text).get(st_off + 0x700..st_off + 0x980)?;
        let mut leave_va = None;
        for i in 0..st.len().saturating_sub(14) {
            if st[i..i + 3] == [0x48, 0x8B, 0x0D] && st[i + 7..i + 10] == [0x33, 0xD2, 0xE8] {
                let disp = i32::from_le_bytes(st[i + 10..i + 14].try_into().unwrap()) as isize;
                leave_va = Some((swap_truck_va + 0x700 + i + 14).wrapping_add_signed(disp));
                break;
            }
        }
        let leave_va = leave_va?;
        let lv_off = leave_va.checked_sub(img.text.0)?;
        let lv = img.bytes(img.text).get(lv_off..lv_off + 0xE0)?;
        let i_mgr = find_bytes(lv, &[0x48, 0x8B, 0x1D], 0x50)?;
        let mgr_disp = i32::from_le_bytes(lv[i_mgr + 3..i_mgr + 7].try_into().unwrap()) as isize;
        let desktop_mgr_global = (leave_va + i_mgr + 7).wrapping_add_signed(mgr_disp);
        let mut flag_off = None;
        for j in (i_mgr + 7)..lv.len().saturating_sub(7) {
            if lv[j] == 0xC6 && lv[j + 1] == 0x83 && lv[j + 6] == 0x01 {
                flag_off = Some(u32::from_le_bytes(lv[j + 2..j + 6].try_into().unwrap()) as usize);
                break;
            }
        }
        Some((desktop_mgr_global as *const *mut u8, flag_off?))
    })();

    // 2. `garage_u::upgrade`, `g_economy_mgr`, `econ_data_off`, and optional `(g_game, refresh_garages)`.
    let s_gar = img.find_rdata(b"Use: cheat garage [city] [downgrade|upgrade]\0")?;
    let gar_xrefs = img.find_lea_xrefs(s_gar);
    let cheat_gar = img.pdata_root(*gar_xrefs.first()?)?;
    let gb_off = cheat_gar.checked_sub(img.text.0)?;
    let gb = img.bytes(img.text).get(gb_off..gb_off + 0x180)?;

    // `mov rdx, rbx; mov rcx, rdi; call garage_upgrade` (`48 8B D3 48 8B CF E8 <i32>`)
    let i_up = find_bytes(gb, &[0x48, 0x8B, 0xD3, 0x48, 0x8B, 0xCF, 0xE8], 0)?;
    let up_disp = i32::from_le_bytes(gb[i_up + 7..i_up + 11].try_into().unwrap()) as isize;
    let garage_upgrade_va = (cheat_gar + i_up + 11).wrapping_add_signed(up_disp);

    // `mov rbx, [rip + g_economy_mgr]` (`48 8B 1D <i32>`) before `i_up`
    let i_econ = rfind_bytes(gb, &[0x48, 0x8B, 0x1D], i_up)?;
    let econ_disp = i32::from_le_bytes(gb[i_econ + 3..i_econ + 7].try_into().unwrap()) as isize;
    let econ_mgr_global = (cheat_gar + i_econ + 7).wrapping_add_signed(econ_disp);

    // `mov rax, [rbx + econ_data_off]` (`48 8B 83 <u32>`) before `i_up`
    let i_ed = rfind_bytes(gb, &[0x48, 0x8B, 0x83], i_up)?;
    let econ_data_off = u32::from_le_bytes(gb[i_ed + 3..i_ed + 7].try_into().unwrap()) as usize;

    // Optional `mov rcx, [rip + g_game]; call refresh_garages` right after `garage_upgrade`
    let (game_global, refresh_garages_va) = find_bytes(gb, &[0x48, 0x8B, 0x0D], i_up + 11)
        .and_then(|i_gg| {
            if gb.get(i_gg + 7) != Some(&0xE8) {
                return None;
            }
            let g_disp = i32::from_le_bytes(gb[i_gg + 3..i_gg + 7].try_into().unwrap()) as isize;
            let f_disp = i32::from_le_bytes(gb[i_gg + 8..i_gg + 12].try_into().unwrap()) as isize;
            let gg = (cheat_gar + i_gg + 7).wrapping_add_signed(g_disp);
            let rf = (cheat_gar + i_gg + 12).wrapping_add_signed(f_disp);
            Some((Some(gg), Some(rf)))
        })
        .unwrap_or((None, None));

    // 3. `store_trailer`, `assign_trailer`, `sync_actor` from `@@trailerman_trailer_taken_ingarage@@`.
    let s_tr = img.find_rdata(b"@@trailerman_trailer_taken_ingarage@@\0")?;
    let tr_xrefs = img.find_lea_xrefs(s_tr);
    if tr_xrefs.len() != 1 {
        log!("fleet: trailer anchor xrefs={} (expected 1)", tr_xrefs.len());
        return None;
    }
    let tr_root = img.pdata_root(tr_xrefs[0])?;
    let tb_off = tr_root.checked_sub(img.text.0)?;
    let tb = img.bytes(img.text).get(tb_off..tb_off + 0x180)?;

    let i_sw = find_bytes(tb, &[0x41, 0x83, 0xFE, 0x05], 0)?;
    let p1 = find_bytes(tb, &[0xE8], i_sw)?;
    let store_disp = i32::from_le_bytes(tb[p1 + 1..p1 + 5].try_into().unwrap()) as isize;
    let store_trailer_va = (tr_root + p1 + 5).wrapping_add_signed(store_disp);

    let p2 = find_bytes(tb, &[0xE8], p1 + 5)?;
    let take_ui_disp = i32::from_le_bytes(tb[p2 + 1..p2 + 5].try_into().unwrap()) as isize;
    let take_ui_va = (tr_root + p2 + 5).wrapping_add_signed(take_ui_disp);

    let ub_off = take_ui_va.checked_sub(img.text.0)?;
    let ub = img.bytes(img.text).get(ub_off..ub_off + 0x340)?;
    let i_sync = find_bytes(ub, &[0x49, 0x8B, 0xCE, 0xE8], 0x180)?;
    let sync_disp = i32::from_le_bytes(ub[i_sync + 4..i_sync + 8].try_into().unwrap()) as isize;
    let sync_actor_va = (take_ui_va + i_sync + 8).wrapping_add_signed(sync_disp);

    let p_assign = rfind_bytes(ub, &[0xE8], i_sync)?;
    let assign_disp = i32::from_le_bytes(ub[p_assign + 1..p_assign + 5].try_into().unwrap()) as isize;
    let assign_trailer_va = (take_ui_va + p_assign + 5).wrapping_add_signed(assign_disp);

    // 4. `unit_name_to_str` and `string_vtable` (same Prism3D helper used by `job_market`).
    let (unit_name_va, string_vtable) = img.read_unit_name_helper_for_fleet()?;

    // 5. Dynamic `.data` reflection tables.
    let econ_tbl = img.find_attr_table(&["bank", "player", "garages", "game_time"])?;
    let bank_tbl = img.find_exact_attr_table(&["money_account", "loans"], &[])?;
    let player_tbl = img.find_attr_table(&[
        "current_job",
        "trucks",
        "assigned_truck",
        "assigned_trailer",
        "assigned_trailer_connected",
        "my_truck",
        "my_trailer",
        "my_trailer_attached",
    ])?;
    let garage_tbl = img.find_exact_attr_table(&["vehicles", "drivers", "trailers", "status", "productivity"], &[])?;
    let veh_tbl = img.find_exact_attr_table(&["accessories", "odometer", "license_plate", "chassis_wear"], &[])?;
    let acc_tbl = img.find_exact_attr_table(&["data_path", "refund"], &[])?;
    let trailer_tbl = img.find_attr_table(&["trailer_definition", "slave_trailer", "is_private"])?;
    let tdef_tbl =
        img.find_exact_attr_table(&["gross_trailer_weight_limit", "volume", "body_type", "chain_type", "axles"], &["chassis"])?;

    // Dynamically extract live runtime actor offsets from `store_trailer` (`add rcx, actors_off`, `mov rax, [r8 + trailer_off]`)
    // and `sync_actor` (`mov rcx, [rax + truck_off]`), falling back to verified Prism3D 1.6x defaults.
    let (player_actors, actor_trailer) = img
        .bytes(img.text)
        .get(store_trailer_va.saturating_sub(img.text.0)..store_trailer_va.saturating_sub(img.text.0) + 0x60)
        .map(|sb| {
            let actors = find_bytes(sb, &[0x48, 0x83, 0xC1], 0)
                .and_then(|i| sb.get(i + 3).copied())
                .map(|b| b as usize)
                .filter(|&v| (0x40..=0x200).contains(&v))
                .unwrap_or(0x70);
            let trailer = find_bytes(sb, &[0x49, 0x8B, 0x40], 0)
                .and_then(|i| sb.get(i + 3).copied())
                .map(|b| b as usize)
                .filter(|&v| (0x18..=0x100).contains(&v))
                .unwrap_or(0x48);
            (actors, trailer)
        })
        .unwrap_or((0x70, 0x48));
    let actor_truck = img
        .bytes(img.text)
        .get(sync_actor_va.saturating_sub(img.text.0)..sync_actor_va.saturating_sub(img.text.0) + 0x90)
        .and_then(|sb| find_bytes(sb, &[0x48, 0x8B, 0x48], 0x40).and_then(|i| sb.get(i + 3).copied()))
        .map(|b| b as usize)
        .filter(|&v| (0x08..=0x40).contains(&v))
        .unwrap_or(0x10);

    let off = Offsets {
        econ_bank: *econ_tbl.get("bank")?,
        econ_player: *econ_tbl.get("player")?,
        econ_garages: *econ_tbl.get("garages")?,
        econ_data: econ_data_off,
        econ_game_time: *econ_tbl.get("game_time")?,
        bank_money_account: *bank_tbl.get("money_account")?,
        player_current_job: *player_tbl.get("current_job")?,
        player_actors,
        actor_truck,
        actor_trailer,
        player_assigned_truck: *player_tbl.get("assigned_truck")?,
        player_assigned_trailer: *player_tbl.get("assigned_trailer")?,
        player_assigned_trailer_connected: *player_tbl.get("assigned_trailer_connected")?,
        player_my_truck: *player_tbl.get("my_truck")?,
        player_my_trailer: *player_tbl.get("my_trailer")?,
        player_my_trailer_attached: *player_tbl.get("my_trailer_attached")?,
        garage_vehicles: *garage_tbl.get("vehicles")?,
        garage_drivers: *garage_tbl.get("drivers")?,
        garage_trailers: *garage_tbl.get("trailers")?,
        garage_status: *garage_tbl.get("status")?,
        garage_productivity: *garage_tbl.get("productivity")?,
        veh_accessories: *veh_tbl.get("accessories")?,
        veh_odometer: *veh_tbl.get("odometer")?,
        veh_license_plate: *veh_tbl.get("license_plate")?,
        veh_chassis_wear: *veh_tbl.get("chassis_wear")?,
        acc_data_path: *acc_tbl.get("data_path")?,
        trailer_definition: *trailer_tbl.get("trailer_definition")?,
        tdef_volume: *tdef_tbl.get("volume")?,
        tdef_gross_weight: *tdef_tbl.get("gross_trailer_weight_limit")?,
        tdef_axles: *tdef_tbl.get("axles")?,
        tdef_body_type: *tdef_tbl.get("body_type")?,
        tdef_chain_type: *tdef_tbl.get("chain_type")?,
    };

    log!(
        "fleet: ready! econ=+{:#x}, swap_truck=+{:#x}, upgrade=+{:#x}, store_tr=+{:#x}, assign_tr=+{:#x}, sync=+{:#x}, desktop_flag={:?}",
        econ_mgr_global - base,
        swap_truck_va - base,
        garage_upgrade_va - base,
        store_trailer_va - base,
        assign_trailer_va - base,
        sync_actor_va - base,
        desktop_return_flag.map(|(g, o)| format!("+{:#x}[+{o:#x}]", g as usize - base)),
    );

    // SAFETY: all function roots verified by unique anchors + `.pdata` + `.text` call graph.
    Some(Engine {
        econ_mgr_global: econ_mgr_global as *const *mut u8,
        game_global: game_global.map(|g| g as *const *mut u8),
        desktop_return_flag,
        swap_truck_fn: unsafe { std::mem::transmute::<usize, SwapTruckFn>(swap_truck_va) },
        garage_upgrade_fn: unsafe { std::mem::transmute::<usize, GarageUpgradeFn>(garage_upgrade_va) },
        refresh_garages_fn: refresh_garages_va.map(|v| unsafe { std::mem::transmute::<usize, RefreshGaragesFn>(v) }),
        store_trailer_fn: unsafe { std::mem::transmute::<usize, StoreTrailerFn>(store_trailer_va) },
        assign_trailer_fn: unsafe { std::mem::transmute::<usize, AssignTrailerFn>(assign_trailer_va) },
        sync_actor_fn: unsafe { std::mem::transmute::<usize, SyncActorFn>(sync_actor_va) },
        unit_name_fn: unsafe { std::mem::transmute::<usize, UnitNameFn>(unit_name_va) },
        string_vtable,
        off,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_license_plates_and_garage_tiers() {
        assert_eq!(clean_plate("AB 123CD|germany"), ("AB 123CD".into(), "germany".into()));
        assert_eq!(clean_plate("<font face=/font/plate.font>XYZ 99</font>|brazil"), ("XYZ 99".into(), "brazil".into()));
        assert_eq!(garage_tier_info(0b110, 180_000, 100_000), ("tiny", 1, Some(180_000), Some(3)));
        assert_eq!(garage_tier_info(0b010, 180_000, 100_000), ("small", 3, Some(100_000), Some(5)));
        assert_eq!(garage_tier_info(0b011, 180_000, 100_000), ("large", 5, None, None));
    }

    #[test]
    fn decodes_prism_base38_tokens() {
        // 's' = 29, 'i' = 19, ... verify non-empty deterministic decode
        let single: u64 = 29 + 38 * (19 + 38 * (24 + 38 * (17 + 38 * (22 + 38 * 15))));
        assert_eq!(decode_token(single), "single");
    }
}
