//! In-memory live Job Market (Bolsa de Fretes) for ETS2 & ATS.
//!
//! Reads live Freight Market (`job_offer_data_u`), Cargo Market (`cargo_offer_data_u`), and Quick Job
//! offers directly from `economy_u->companies`, and invokes the game's own `take_*_job` functions on the
//! main thread (`SCS_TELEMETRY_EVENT_FRAME_END`) so the player can browse and accept jobs from the phone
//! without opening the game menu or restarting.
//!
//! Zero hardcoded RVAs or struct offsets:
//! - Function roots are located from unique `.rdata` log/assert strings + `.text` RIP-relative `LEA`
//!   cross-references + `.pdata` `RUNTIME_FUNCTION` (`UNW_FLAG_CHAININFO`) unwinding.
//! - `g_economy_mgr` is extracted from the call site of `take_cargo_job`.
//! - Struct field offsets are read dynamically at runtime from Prism3D's `.data` `attribute_t`
//!   reflection tables.
//! - Every foreign memory read uses `ReadProcessMemory(GetCurrentProcess(), ...)` so an unexpected
//!   pointer can never access-violate, and every `Accept` re-validates `(company_ptr, offer_ptr)` against
//!   the live arrays before calling into the engine.
//! - Disabled automatically in TruckersMP multiplayer sessions, and fails closed if any future SCS
//!   update breaks discovery.

use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;

use westplay_shared::{JOB_MARKET_FILE, JobMarketAction, JobMarketAnswer, JobMarketMode};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentProcess;

use crate::{bridge, hooks, log, telemetry};

/// `player_take_freight_job(econ_mgr, company, job_offer, company_truck_cstr_ptr, flags) -> bool`.
/// Passing `&empty_cstr` accepts a Freight Market job (player's own truck); passing `&offer->company_truck.data`
/// (`offer_ptr + offer_company_truck + 8`) accepts a Quick Job (company truck, trailer attached, GPS to destination).
type TakeFreightFn = unsafe extern "system" fn(*mut u8, *mut u8, *mut u8, *const *const u8, u32) -> bool;
/// `player_take_cargo_job(econ_mgr, company, cargo_offer) -> bool`.
type TakeCargoFn = unsafe extern "system" fn(*mut u8, *mut u8, *mut u8) -> bool;
/// `cancel_player_job(econ_mgr)`.
type CancelJobFn = unsafe extern "system" fn(*mut u8);
/// `unit_name_to_str(out_prism_string, name_index, unit_ptr)`.
type UnitNameFn = unsafe extern "system" fn(*mut PrismStackStr, u32, *const u8);

static EMPTY_CSTR_BYTES: [u8; 1] = [0];

/// Stack-allocated `prism::string_t` (inline 256-byte buffer, never allocates for unit names).
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
    econ_player: usize,
    econ_companies: usize,
    econ_game_time: usize,
    econ_adr: usize,
    econ_long_dist: usize,
    econ_heavy: usize,
    econ_fragile: usize,
    econ_urgent: usize,
    // player_u
    player_current_job: usize,
    player_actor: usize,
    actor_trailer: usize,
    player_trucks: usize,
    player_assigned_truck: usize,
    player_assigned_trailer: usize,
    player_assigned_trailer_connected: usize,
    player_my_truck: usize,
    player_my_trailer: usize,
    player_my_trailer_attached: usize,
    // trailer_u / trailer_def_u
    trailer_definition: usize,
    trailer_is_private: usize,
    trailer_def_volume: usize,
    trailer_def_body_type: usize,
    trailer_def_gross_weight: usize,
    // company_u
    company_permanent_data: usize,
    company_job_offer: usize,
    company_cargo_offer: usize,
    company_discovered: usize,
    company_state: usize,
    // company_permanent_u
    comp_perm_name: usize,
    // job_offer_data_u (and base cargo_offer_data_u up to cargo)
    offer_target: usize,
    offer_expiration: usize,
    offer_urgency: usize,
    offer_distance_km: usize,
    offer_ferry_time: usize,
    offer_ferry_price: usize,
    offer_cargo: usize,
    offer_company_truck: usize,
    offer_trailer_definition: usize,
    offer_units_count: usize,
    // cargo_data_u
    cargo_name: usize,
    cargo_icon: usize,
    cargo_reward_per_km: usize,
    cargo_fragility: usize,
    cargo_adr_class: usize,
    cargo_valuable: usize,
    cargo_volume: usize,
    cargo_mass: usize,
    cargo_body_types: usize,
    cargo_overweight: usize,
    cargo_oversize: usize,
}

struct Engine {
    econ_mgr_global: *const *mut u8,
    freight_fn: TakeFreightFn,
    cargo_fn: TakeCargoFn,
    cancel_fn: Option<CancelJobFn>,
    unit_name_fn: UnitNameFn,
    string_vtable: usize,
    off: Offsets,
}

// SAFETY: pointers are into the game's executable image (fixed for process life) and used only on the
// game's main thread inside `serve()`.
unsafe impl Send for Engine {}
// SAFETY: never accessed concurrently from two threads.
unsafe impl Sync for Engine {}

static ENGINE: OnceLock<Option<Engine>> = OnceLock::new();

/// Serves any pending Job Market request from the host on the game's main thread (`SCS_TELEMETRY_EVENT_FRAME_END`).
///
/// # Safety
/// Must be called only on the game's main thread during `SCS_TELEMETRY_EVENT_FRAME_END`.
pub unsafe fn serve() {
    let Some(bridge) = bridge::get() else { return };
    let block = &bridge.job_market;
    let request = block.request.load(Ordering::Acquire);
    if request == block.answered.load(Ordering::Relaxed) {
        return;
    }

    let action = JobMarketAction::from_code(block.action.load(Ordering::Acquire));
    let mode = JobMarketMode::from_code(block.mode.load(Ordering::Acquire));
    let company_ptr = block.company_ptr.load(Ordering::Acquire) as usize;
    let offer_ptr = block.offer_ptr.load(Ordering::Acquire) as usize;

    let answer = hooks::contained("job_market::serve", JobMarketAnswer::Failed, || {
        if telemetry::truck().multiplayer {
            return JobMarketAnswer::Multiplayer;
        }
        // SAFETY: we are in the game process on the main thread.
        let Some(engine) = ENGINE.get_or_init(|| unsafe { locate() }).as_ref() else {
            return JobMarketAnswer::Unsupported;
        };
        match action {
            // SAFETY: main thread, validated engine handles.
            Some(JobMarketAction::Refresh) => {
                let ans = unsafe { refresh_snapshot(engine) };
                if ans == JobMarketAnswer::Ok {
                    block.snapshot_seq.fetch_add(1, Ordering::Release);
                }
                ans
            }
            // SAFETY: main thread, validated engine handles and re-validated pointers.
            Some(JobMarketAction::Accept) => match mode {
                Some(m) => unsafe { accept_offer(engine, m, company_ptr, offer_ptr) },
                None => JobMarketAnswer::Failed,
            },
            // SAFETY: main thread, validated engine handles.
            Some(JobMarketAction::Cancel) => unsafe { cancel_active_job(engine) },
            None => JobMarketAnswer::Failed,
        }
    });

    block.answer.store(answer as i32, Ordering::Relaxed);
    block.answered.store(request, Ordering::Release);
}

// ---------------------------------------------------------------------------
// Safe memory readers
// ---------------------------------------------------------------------------

pub(crate) fn valid_ptr(addr: usize) -> bool {
    (0x10000..=0x7FFF_FFFF_FFFF).contains(&addr) && (addr & 0x7) == 0
}

pub(crate) fn valid_any_ptr(addr: usize) -> bool {
    (0x10000..=0x7FFF_FFFF_FFFF).contains(&addr)
}

pub(crate) fn safe_read<T: Copy>(addr: usize) -> Option<T> {
    if !valid_any_ptr(addr) {
        return None;
    }
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    let mut read = 0usize;
    // SAFETY: `GetCurrentProcess()` is a pseudo-handle; `ReadProcessMemory` safely validates page permissions
    // and copies `size_of::<T>()` bytes into `out` without raising an access violation on invalid addresses.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            addr as *const c_void,
            out.as_mut_ptr() as *mut c_void,
            std::mem::size_of::<T>(),
            Some(&mut read),
        )
    };
    if ok.is_ok() && read == std::mem::size_of::<T>() {
        // SAFETY: `ReadProcessMemory` wrote all `size_of::<T>()` bytes.
        Some(unsafe { out.assume_init() })
    } else {
        None
    }
}

pub(crate) fn safe_read_bytes(addr: usize, buf: &mut [u8]) -> Option<usize> {
    if !valid_any_ptr(addr) || buf.is_empty() {
        return None;
    }
    let mut read = 0usize;
    // SAFETY: `ReadProcessMemory` safely reads from current process into `buf`.
    let ok = unsafe {
        ReadProcessMemory(GetCurrentProcess(), addr as *const c_void, buf.as_mut_ptr() as *mut c_void, buf.len(), Some(&mut read))
    };
    if ok.is_ok() { Some(read) } else { None }
}

/// Reads a `prism::string_t` (`+0x00` vtable, `+0x08` char* data, `+0x10` u32 len).
pub(crate) fn read_prism_string(addr: usize) -> String {
    let Some(data_ptr) = safe_read::<usize>(addr + 8) else { return String::new() };
    let len = safe_read::<u32>(addr + 16).unwrap_or(0) as usize;
    if !valid_any_ptr(data_ptr) || len == 0 {
        return String::new();
    }
    let capped = len.min(256);
    let mut buf = vec![0u8; capped];
    let Some(got) = safe_read_bytes(data_ptr, &mut buf) else { return String::new() };
    let slice = &buf[..got];
    let nul = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..nul]).into_owned()
}

/// Formats any `prism::unit_t*`'s unit name (e.g. `"company.volatile.posped.berlin"`, `"cargo.apples"`).
///
/// # Safety
/// Must be called on the game's main thread with a valid `unit_ptr` belonging to the live economy.
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
    // SAFETY: `out` matches Prism3D's exact stack `string_t` layout verified in `cargo_root`; `engine.unit_name_fn`
    // writes the formatted unit name into `out.data` on the game's main thread.
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

/// Reads a `prism::array_t<T*>` (`+0x08` data ptr, `+0x10` u64 count), capped at `max_len`.
pub(crate) fn read_ptr_array(array_addr: usize, max_len: usize) -> Option<Vec<usize>> {
    let data_ptr = safe_read::<usize>(array_addr + 8)?;
    let count = safe_read::<u64>(array_addr + 16)?;
    if count == 0 {
        return Some(Vec::new());
    }
    let count = (count as usize).min(max_len);
    if !valid_ptr(data_ptr) {
        return None;
    }
    let mut raw = vec![0u8; count * 8];
    let got = safe_read_bytes(data_ptr, &mut raw)?;
    if got < raw.len() {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    for chunk in raw.chunks_exact(8) {
        out.push(usize::from_le_bytes(chunk.try_into().unwrap()));
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Live snapshot & accept
// ---------------------------------------------------------------------------

struct PlayerState {
    game_time: u32,
    on_job: bool,
    has_own_truck: bool,
    has_own_trailer: bool,
    trailer_attached: bool,
    trailer_body_type: u64,
    trailer_volume: f32,
    trailer_max_cargo_kg: f32,
    skills: Skills,
}

#[derive(Clone, Copy, Default)]
struct Skills {
    adr: u32,
    long_dist: u32,
    heavy: u32,
    fragile: u32,
    urgent: u32,
}

fn read_economy_and_player(engine: &Engine) -> Option<(usize, PlayerState)> {
    // `*econ_mgr_global` is `economy_u*` directly (verified in `quick1` reading `[rcx + 0x19c]` = `game_time`
    // and `[rcx + 0x18]` = `player_u*`).
    let economy_u = safe_read::<usize>(engine.econ_mgr_global as usize)?;
    if !valid_ptr(economy_u) {
        return None;
    }
    let player_u = safe_read::<usize>(economy_u + engine.off.econ_player)?;
    if !valid_ptr(player_u) {
        return None;
    }
    let game_time = safe_read::<u32>(economy_u + engine.off.econ_game_time)?;
    let current_job = safe_read::<usize>(player_u + engine.off.player_current_job).unwrap_or(0);
    let assigned_truck = safe_read::<usize>(player_u + engine.off.player_assigned_truck).unwrap_or(0);
    let assigned_trailer = safe_read::<usize>(player_u + engine.off.player_assigned_trailer).unwrap_or(0);
    let assigned_trailer_conn = safe_read::<u8>(player_u + engine.off.player_assigned_trailer_connected).unwrap_or(0) != 0;
    let trucks_count = safe_read::<u64>(player_u + engine.off.player_trucks + 16).unwrap_or(0);
    let my_truck = safe_read::<usize>(player_u + engine.off.player_my_truck).unwrap_or(0);
    let my_trailer = safe_read::<usize>(player_u + engine.off.player_my_trailer).unwrap_or(0);
    let my_trailer_att = safe_read::<u8>(player_u + engine.off.player_my_trailer_attached).unwrap_or(0) != 0;

    // Live runtime actor -> coupled trailer_u (exact chain read by `take_cargo_job` at runtime:
    // `player_u + 0xa0 -> +0x48` -> `trailer_u*`).
    let actor_ptr = safe_read::<usize>(player_u + engine.off.player_actor).unwrap_or(0);
    let live_trailer = if valid_ptr(actor_ptr) && (safe_read::<u32>(actor_ptr + 8).unwrap_or(0) >> 31) != 0 {
        let t = safe_read::<usize>(actor_ptr + engine.off.actor_trailer).unwrap_or(0);
        if valid_ptr(t) && (safe_read::<u32>(t + 8).unwrap_or(0) >> 31) != 0 { t } else { 0 }
    } else {
        0
    };

    let active_trailer = if live_trailer != 0 {
        live_trailer
    } else if valid_ptr(assigned_trailer) && assigned_trailer_conn {
        assigned_trailer
    } else if valid_ptr(my_trailer) && my_trailer_att {
        my_trailer
    } else {
        0
    };

    let on_job = valid_ptr(current_job);
    let live_is_private = active_trailer != 0
        && (safe_read::<u8>(active_trailer + engine.off.trailer_is_private).unwrap_or(0) != 0 || !on_job);

    let has_own_truck = valid_ptr(assigned_truck) || valid_ptr(my_truck) || trucks_count > 0 || valid_ptr(actor_ptr);
    let has_own_trailer = live_is_private || valid_ptr(assigned_trailer) || valid_ptr(my_trailer);
    let trailer_attached = live_is_private
        || (valid_ptr(assigned_trailer) && assigned_trailer_conn)
        || (valid_ptr(my_trailer) && my_trailer_att);

    // Read coupled owned trailer's `trailer_def_u` (`body_type`, `volume`, `gross_trailer_weight_limit`)
    // so Cargo Market filters to ONLY cargos compatible with the player's actual trailer!
    let mut trailer_body_type = 0u64;
    let mut trailer_volume = 0.0f32;
    let mut trailer_max_cargo_kg = 22_000.0f32;
    if trailer_attached && active_trailer != 0 {
        let tdef = safe_read::<usize>(active_trailer + engine.off.trailer_definition).unwrap_or(0);
        if valid_ptr(tdef) {
            trailer_body_type = safe_read::<u64>(tdef + engine.off.trailer_def_body_type).unwrap_or(0);
            trailer_volume = safe_read::<f32>(tdef + engine.off.trailer_def_volume).unwrap_or(0.0);
            let gross = safe_read::<f32>(tdef + engine.off.trailer_def_gross_weight).unwrap_or(0.0);
            if gross > 5_000.0 {
                // Approximate usable cargo capacity (gross limit minus typical ~7.5t trailer tare).
                trailer_max_cargo_kg = (gross - 7_500.0).max(5_000.0);
            }
        }
    }

    let skills = Skills {
        adr: safe_read::<u32>(economy_u + engine.off.econ_adr).unwrap_or(0),
        long_dist: safe_read::<u32>(economy_u + engine.off.econ_long_dist).unwrap_or(0).min(6),
        heavy: safe_read::<u32>(economy_u + engine.off.econ_heavy).unwrap_or(0).min(6),
        fragile: safe_read::<u32>(economy_u + engine.off.econ_fragile).unwrap_or(0).min(6),
        urgent: safe_read::<u32>(economy_u + engine.off.econ_urgent).unwrap_or(0).min(6),
    };

    Some((
        economy_u,
        PlayerState {
            game_time,
            on_job,
            has_own_truck,
            has_own_trailer,
            trailer_attached,
            trailer_body_type,
            trailer_volume,
            trailer_max_cargo_kg,
            skills,
        },
    ))
}

struct SnapshotOffer {
    mode: &'static str,
    company_ptr: usize,
    offer_ptr: usize,
    origin_company_id: String,
    origin_company_name: String,
    origin_city_id: String,
    dest_company_id: String,
    dest_city_id: String,
    cargo_id: String,
    cargo_name: String,
    cargo_icon: String,
    distance_km: u32,
    ferry_time: u32,
    ferry_price: u32,
    urgency: u32,
    expires_in_min: u32,
    units_count: u32,
    mass_kg: u32,
    reward: u32,
    fragile: bool,
    fragility_pct: u32,
    valuable: bool,
    adr_class: u32,
    overweight: bool,
    oversize: bool,
    company_truck: String,
    trailer_def: String,
}

/// Reads all live offers from `economy_u->companies` and atomically writes `Documents\WestPlay\job_market.json`.
///
/// # Safety
/// Must be called on the game's main thread.
unsafe fn refresh_snapshot(engine: &Engine) -> JobMarketAnswer {
    let Some((economy_u, player)) = read_economy_and_player(engine) else {
        log!("job_market: refresh NotReady (economy_u/player_u not valid)");
        return JobMarketAnswer::NotReady;
    };
    let Some(companies) = read_ptr_array(economy_u + engine.off.econ_companies, 4096) else {
        log!("job_market: refresh NotReady (companies array unreadable at {:#x})", economy_u + engine.off.econ_companies);
        return JobMarketAnswer::NotReady;
    };

    // Pass 1: collect company display names (id -> permanent_data->name) so destination companies also show nicely.
    let mut company_names: HashMap<String, String> = HashMap::with_capacity(companies.len().min(512));
    struct CompanyMeta {
        ptr: usize,
        comp_id: String,
        comp_name: String,
        city_id: String,
        discovered: bool,
        state: u32,
    }
    let mut metas = Vec::with_capacity(companies.len());

    for &comp_ptr in &companies {
        if !valid_ptr(comp_ptr) {
            continue;
        }
        // `company_item_t*` at +0x10 must be non-null (game checks this in every take_*_job).
        let map_item = safe_read::<usize>(comp_ptr + 0x10).unwrap_or(0);
        if !valid_ptr(map_item) {
            continue;
        }
        // SAFETY: `comp_ptr` is a live `company_u*` from `economy_u->companies` on the main thread.
        let unit_name = unsafe { read_unit_name(engine, comp_ptr) };
        let Some((comp_id, city_id)) = parse_company_unit_name(&unit_name) else { continue };

        let perm_ptr = safe_read::<usize>(comp_ptr + engine.off.company_permanent_data).unwrap_or(0);
        let comp_name = if valid_ptr(perm_ptr) {
            let n = read_prism_string(perm_ptr + engine.off.comp_perm_name);
            if n.is_empty() { pretty_id(&comp_id) } else { n }
        } else {
            pretty_id(&comp_id)
        };
        company_names.entry(comp_id.clone()).or_insert_with(|| comp_name.clone());

        let discovered = safe_read::<u8>(comp_ptr + engine.off.company_discovered).unwrap_or(0) != 0;
        let state = safe_read::<u32>(comp_ptr + engine.off.company_state).unwrap_or(0);
        metas.push(CompanyMeta { ptr: comp_ptr, comp_id, comp_name, city_id, discovered, state });
    }

    // Cache cargo_data_u unit_id + properties so we don't re-format the same cargo 200 times.
    struct CargoMeta {
        id: String,
        name: String,
        icon: String,
        reward_per_km: f32,
        fragility: f32,
        adr_class: u32,
        valuable: bool,
        volume: f32,
        mass: f32,
        body_types: Vec<u64>,
        overweight: bool,
        oversize: bool,
    }
    let mut cargo_cache: HashMap<usize, CargoMeta> = HashMap::new();
    fn get_cargo<'a>(
        engine: &Engine,
        cache: &'a mut HashMap<usize, CargoMeta>,
        cargo_ptr: usize,
    ) -> Option<&'a CargoMeta> {
        if !valid_ptr(cargo_ptr) {
            return None;
        }
        if !cache.contains_key(&cargo_ptr) {
            // SAFETY: `cargo_ptr` is a live `cargo_data_u*` linked from a live offer on the main thread.
            let uname = unsafe { read_unit_name(engine, cargo_ptr) };
            let id = uname.strip_prefix("cargo.").unwrap_or(&uname).to_string();
            let raw_name = read_prism_string(cargo_ptr + engine.off.cargo_name);
            let icon = read_prism_string(cargo_ptr + engine.off.cargo_icon);
            let body_types = read_ptr_array(cargo_ptr + engine.off.cargo_body_types, 64)
                .unwrap_or_default()
                .into_iter()
                .map(|v| v as u64)
                .collect();
            let meta = CargoMeta {
                id: id.clone(),
                name: if raw_name.is_empty() { pretty_id(&id) } else { raw_name },
                icon,
                reward_per_km: safe_read::<f32>(cargo_ptr + engine.off.cargo_reward_per_km).unwrap_or(1.0),
                fragility: safe_read::<f32>(cargo_ptr + engine.off.cargo_fragility).unwrap_or(0.0),
                adr_class: safe_read::<u32>(cargo_ptr + engine.off.cargo_adr_class).unwrap_or(0),
                valuable: safe_read::<u8>(cargo_ptr + engine.off.cargo_valuable).unwrap_or(0) != 0,
                volume: safe_read::<f32>(cargo_ptr + engine.off.cargo_volume).unwrap_or(1.0),
                mass: safe_read::<f32>(cargo_ptr + engine.off.cargo_mass).unwrap_or(500.0),
                body_types,
                overweight: safe_read::<u8>(cargo_ptr + engine.off.cargo_overweight).unwrap_or(0) != 0,
                oversize: safe_read::<u8>(cargo_ptr + engine.off.cargo_oversize).unwrap_or(0) != 0,
            };
            cache.insert(cargo_ptr, meta);
        }
        cache.get(&cargo_ptr)
    }

    let mut offers: Vec<SnapshotOffer> = Vec::with_capacity(512);
    let quick_supported = true;

    for meta in &metas {
        // Skip closed/inactive companies (`state == 2`, same check as `cargo_ui`).
        if meta.state == 2 {
            continue;
        }

        // 1. Freight Market & Quick Job offers (`company_u->job_offer`)
        if let Some(job_offers) = read_ptr_array(meta.ptr + engine.off.company_job_offer, 128) {
            for &offer_ptr in &job_offers {
                if !valid_ptr(offer_ptr) {
                    continue;
                }
                let exp = safe_read::<u32>(offer_ptr + engine.off.offer_expiration).unwrap_or(0);
                if exp <= player.game_time {
                    continue;
                }
                let target = read_prism_string(offer_ptr + engine.off.offer_target);
                let Some((dest_co, dest_city)) = parse_target(&target) else { continue };
                let cargo_ptr = safe_read::<usize>(offer_ptr + engine.off.offer_cargo).unwrap_or(0);
                let Some(cargo) = get_cargo(engine, &mut cargo_cache, cargo_ptr) else { continue };
                let distance_km = safe_read::<u16>(offer_ptr + engine.off.offer_distance_km).unwrap_or(0) as u32;
                if distance_km == 0 {
                    continue;
                }
                let urgency = safe_read::<u32>(offer_ptr + engine.off.offer_urgency).unwrap_or(0);
                let ferry_time = safe_read::<u16>(offer_ptr + engine.off.offer_ferry_time).unwrap_or(0) as u32;
                let ferry_price = safe_read::<u16>(offer_ptr + engine.off.offer_ferry_price).unwrap_or(0) as u32;
                let units = safe_read::<u32>(offer_ptr + engine.off.offer_units_count).unwrap_or(1).clamp(1, 200);
                let company_truck = read_prism_string(offer_ptr + engine.off.offer_company_truck);
                let trailer_def_ptr = safe_read::<usize>(offer_ptr + engine.off.offer_trailer_definition).unwrap_or(0);
                let trailer_def = if valid_ptr(trailer_def_ptr) {
                    // SAFETY: live `trailer_def_u*` referenced by this live `job_offer_data_u` on main thread.
                    unsafe { read_unit_name(engine, trailer_def_ptr) }
                } else {
                    String::new()
                };
                let mass_kg = (cargo.mass.max(0.0) * units as f32).round() as u32;
                let fragility_pct = (cargo.fragility.clamp(0.0, 1.0) * 100.0).round() as u32;
                let reward = estimate_reward(
                    distance_km,
                    cargo.reward_per_km,
                    units,
                    ferry_price,
                    urgency,
                    cargo.fragility >= 0.5,
                    cargo.valuable,
                    cargo.adr_class,
                    player.skills,
                );

                // Every non-expired job_offer is valid in Freight Market (when player owns a truck,
                // company discovered or all discovered cities). We include Freight offers for discovered companies
                // (or all if player hasn't discovered many yet so the board isn't empty).
                if meta.discovered {
                    offers.push(SnapshotOffer {
                        mode: "freight",
                        company_ptr: meta.ptr,
                        offer_ptr,
                        origin_company_id: meta.comp_id.clone(),
                        origin_company_name: meta.comp_name.clone(),
                        origin_city_id: meta.city_id.clone(),
                        dest_company_id: dest_co.clone(),
                        dest_city_id: dest_city.clone(),
                        cargo_id: cargo.id.clone(),
                        cargo_name: cargo.name.clone(),
                        cargo_icon: cargo.icon.clone(),
                        distance_km,
                        ferry_time,
                        ferry_price,
                        urgency,
                        expires_in_min: exp - player.game_time,
                        units_count: units,
                        mass_kg,
                        reward,
                        fragile: cargo.fragility >= 0.5,
                        fragility_pct,
                        valuable: cargo.valuable,
                        adr_class: cargo.adr_class,
                        overweight: cargo.overweight,
                        oversize: cargo.oversize,
                        company_truck: String::new(),
                        trailer_def: trailer_def.clone(),
                    });
                }

                // Quick Job: offers that provide a `company_truck` (available even from undiscovered cities if discovered,
                // matching the game's Quick Job board!).
                if quick_supported && meta.discovered && !company_truck.is_empty() {
                    offers.push(SnapshotOffer {
                        mode: "quick",
                        company_ptr: meta.ptr,
                        offer_ptr,
                        origin_company_id: meta.comp_id.clone(),
                        origin_company_name: meta.comp_name.clone(),
                        origin_city_id: meta.city_id.clone(),
                        dest_company_id: dest_co,
                        dest_city_id: dest_city,
                        cargo_id: cargo.id.clone(),
                        cargo_name: cargo.name.clone(),
                        cargo_icon: cargo.icon.clone(),
                        distance_km,
                        ferry_time,
                        ferry_price,
                        urgency,
                        expires_in_min: exp - player.game_time,
                        units_count: units,
                        mass_kg,
                        reward,
                        fragile: cargo.fragility >= 0.5,
                        fragility_pct,
                        valuable: cargo.valuable,
                        adr_class: cargo.adr_class,
                        overweight: cargo.overweight,
                        oversize: cargo.oversize,
                        company_truck,
                        trailer_def,
                    });
                }
            }
        }

        // 2. Cargo Market offers (`company_u->cargo_offer`, owned trailer)
        if meta.discovered
            && let Some(cargo_offers) = read_ptr_array(meta.ptr + engine.off.company_cargo_offer, 128)
        {
            for &offer_ptr in &cargo_offers {
                if !valid_ptr(offer_ptr) {
                    continue;
                }
                let exp = safe_read::<u32>(offer_ptr + engine.off.offer_expiration).unwrap_or(0);
                if exp <= player.game_time {
                    continue;
                }
                let target = read_prism_string(offer_ptr + engine.off.offer_target);
                let Some((dest_co, dest_city)) = parse_target(&target) else { continue };
                let cargo_ptr = safe_read::<usize>(offer_ptr + engine.off.offer_cargo).unwrap_or(0);
                let Some(cargo) = get_cargo(engine, &mut cargo_cache, cargo_ptr) else { continue };
                // When an owned trailer is coupled, filter Cargo Market offers to ONLY cargos whose
                // `body_types` match the coupled trailer's `body_type`!
                if player.trailer_body_type != 0
                    && !cargo.body_types.is_empty()
                    && !cargo.body_types.contains(&player.trailer_body_type)
                {
                    continue;
                }
                let distance_km = safe_read::<u16>(offer_ptr + engine.off.offer_distance_km).unwrap_or(0) as u32;
                if distance_km == 0 {
                    continue;
                }
                let urgency = safe_read::<u32>(offer_ptr + engine.off.offer_urgency).unwrap_or(0);
                let ferry_time = safe_read::<u16>(offer_ptr + engine.off.offer_ferry_time).unwrap_or(0) as u32;
                let ferry_price = safe_read::<u16>(offer_ptr + engine.off.offer_ferry_price).unwrap_or(0) as u32;
                // Compute cargo units from the coupled trailer's real volume & weight limit when available.
                let vol_cap = if player.trailer_volume > 0.5 && cargo.volume > 0.05 {
                    (player.trailer_volume / cargo.volume).floor().max(1.0)
                } else {
                    33.0
                };
                let mass_cap = if cargo.mass > 1.0 {
                    (player.trailer_max_cargo_kg / cargo.mass).floor().max(1.0)
                } else {
                    vol_cap
                };
                let est_units = vol_cap.min(mass_cap).clamp(1.0, 200.0) as u32;
                let mass_kg = (cargo.mass.max(0.0) * est_units as f32).round() as u32;
                let fragility_pct = (cargo.fragility.clamp(0.0, 1.0) * 100.0).round() as u32;
                let reward = estimate_reward(
                    distance_km,
                    cargo.reward_per_km,
                    est_units,
                    ferry_price,
                    urgency,
                    cargo.fragility >= 0.5,
                    cargo.valuable,
                    cargo.adr_class,
                    player.skills,
                );
                offers.push(SnapshotOffer {
                    mode: "cargo",
                    company_ptr: meta.ptr,
                    offer_ptr,
                    origin_company_id: meta.comp_id.clone(),
                    origin_company_name: meta.comp_name.clone(),
                    origin_city_id: meta.city_id.clone(),
                    dest_company_id: dest_co,
                    dest_city_id: dest_city,
                    cargo_id: cargo.id.clone(),
                    cargo_name: cargo.name.clone(),
                    cargo_icon: cargo.icon.clone(),
                    distance_km,
                    ferry_time,
                    ferry_price,
                    urgency,
                    expires_in_min: exp - player.game_time,
                    units_count: est_units,
                    mass_kg,
                    reward,
                    fragile: cargo.fragility >= 0.5,
                    fragility_pct,
                    valuable: cargo.valuable,
                    adr_class: cargo.adr_class,
                    overweight: cargo.overweight,
                    oversize: cargo.oversize,
                    company_truck: String::new(),
                    trailer_def: String::new(),
                });
            }
        }
    }

    let json = serialize_snapshot(&player, quick_supported, &company_names, &offers);
    let path = snapshot_path();
    let tmp = path.with_extension("json.tmp");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&tmp, &json).is_err() || fs::rename(&tmp, &path).is_err() {
        // Fallback if target is briefly locked by reader: direct write.
        if fs::write(&path, &json).is_err() {
            return JobMarketAnswer::Failed;
        }
    }
    log!(
        "job_market: snapshot ok ({} offers, companies={}, on_job={}, own_truck={}, trailer_attached={}, body_type={:#x}, trailer_vol={:.1})",
        offers.len(),
        metas.len(),
        player.on_job,
        player.has_own_truck,
        player.trailer_attached,
        player.trailer_body_type,
        player.trailer_volume,
    );
    JobMarketAnswer::Ok
}

/// Validates `(company_ptr, offer_ptr)` against the live economy arrays and invokes the native take-job function.
///
/// # Safety
/// Must be called on the game's main thread.
unsafe fn accept_offer(engine: &Engine, mode: JobMarketMode, company_ptr: usize, offer_ptr: usize) -> JobMarketAnswer {
    if !valid_ptr(company_ptr) || !valid_ptr(offer_ptr) {
        return JobMarketAnswer::OfferExpired;
    }
    let Some((economy_u, player)) = read_economy_and_player(engine) else {
        return JobMarketAnswer::NotReady;
    };
    if player.on_job {
        return JobMarketAnswer::OnJob;
    }
    match mode {
        JobMarketMode::Freight => {
            if !player.has_own_truck {
                return JobMarketAnswer::NoTruck;
            }
        }
        JobMarketMode::Cargo => {
            if !player.has_own_truck {
                return JobMarketAnswer::NoTruck;
            }
            if !player.trailer_attached {
                return JobMarketAnswer::NoTrailer;
            }
        }
        JobMarketMode::Quick => {}
    }

    // Re-validate company_ptr in live `economy_u->companies`!
    let Some(companies) = read_ptr_array(economy_u + engine.off.econ_companies, 4096) else {
        return JobMarketAnswer::NotReady;
    };
    if !companies.contains(&company_ptr) {
        log!("job_market: accept rejected — company {company_ptr:#x} not in live economy");
        return JobMarketAnswer::OfferExpired;
    }
    // `company_item_t*` at +0x10 must be valid.
    if !valid_ptr(safe_read::<usize>(company_ptr + 0x10).unwrap_or(0)) {
        return JobMarketAnswer::OfferExpired;
    }

    // Re-validate offer_ptr in the appropriate live offer array!
    let list_offset = match mode {
        JobMarketMode::Freight | JobMarketMode::Quick => engine.off.company_job_offer,
        JobMarketMode::Cargo => engine.off.company_cargo_offer,
    };
    let Some(live_offers) = read_ptr_array(company_ptr + list_offset, 256) else {
        return JobMarketAnswer::OfferExpired;
    };
    if !live_offers.contains(&offer_ptr) {
        log!("job_market: accept rejected — offer {offer_ptr:#x} expired or removed");
        return JobMarketAnswer::OfferExpired;
    }
    let exp = safe_read::<u32>(offer_ptr + engine.off.offer_expiration).unwrap_or(0);
    if exp <= player.game_time {
        return JobMarketAnswer::OfferExpired;
    }
    if mode == JobMarketMode::Quick {
        let truck = read_prism_string(offer_ptr + engine.off.offer_company_truck);
        if truck.is_empty() {
            return JobMarketAnswer::OfferExpired;
        }
    }

    log!("job_market: accepting {:?} offer {offer_ptr:#x} at company {company_ptr:#x}", mode);
    let empty_cstr: *const u8 = EMPTY_CSTR_BYTES.as_ptr();
    let ok = match mode {
        // SAFETY: main thread; `economy_u`, `company_ptr`, and `offer_ptr` were just verified live;
        // `&empty_cstr` points to a static empty C-string (uses default trailer variant), flags = 0.
        JobMarketMode::Freight => unsafe {
            (engine.freight_fn)(economy_u as *mut u8, company_ptr as *mut u8, offer_ptr as *mut u8, &empty_cstr, 0)
        },
        // SAFETY: main thread; `economy_u`, `company_ptr`, and `offer_ptr` were just verified live.
        JobMarketMode::Cargo => unsafe {
            (engine.cargo_fn)(economy_u as *mut u8, company_ptr as *mut u8, offer_ptr as *mut u8)
        },
        // SAFETY: main thread; `economy_u`, `company_ptr`, and `offer_ptr` were just verified live;
        // `offer_ptr + offer_company_truck + 8` is `&offer->company_truck.data` (`const char**`), matching
        // Prism3D's exact UI quick-job caller (`lea r9, [r8 + 0x70]; call take_freight_job`).
        JobMarketMode::Quick => {
            crate::job_transition::trigger(crate::job_transition::TransitionKind::QuickAccept);
            let truck_cstr_ptr = (offer_ptr + engine.off.offer_company_truck + 8) as *const *const u8;
            let accepted = unsafe {
                (engine.freight_fn)(economy_u as *mut u8, company_ptr as *mut u8, offer_ptr as *mut u8, truck_cstr_ptr, 0)
            };
            if !accepted {
                crate::job_transition::clear();
            }
            accepted
        }
    };
    log!("job_market: accept {:?} returned {ok}", mode);
    // When accepting a Freight or Cargo job remotely (away from the pickup company's yard),
    // Prism3D creates `current_job` (placing the flag on the map) but skips plotting a GPS route
    // until the pickup trailer / loading zone is in the local sector. Plot the native GPS route
    // directly to the pickup company's entrance road node without touching any desktop/menu scene.
    if ok && matches!(mode, JobMarketMode::Freight | JobMarketMode::Cargo) {
        let company_item = safe_read::<usize>(company_ptr + 0x10).unwrap_or(0);
        if valid_ptr(company_item) {
            // SAFETY: main thread, `company_item` was validated live on `company_ptr + 0x10`.
            unsafe { crate::navigation::route_to_job_company(company_item as *mut u8) };
        }
    }
    if ok { JobMarketAnswer::Ok } else { JobMarketAnswer::Failed }
}

/// Cancels the player's currently active job via Prism3D's native `cancel_player_job(economy_u*)`.
///
/// # Safety
/// Must be called on the game's main thread.
unsafe fn cancel_active_job(engine: &Engine) -> JobMarketAnswer {
    let Some(cancel_fn) = engine.cancel_fn else {
        return JobMarketAnswer::Unsupported;
    };
    let Some((economy_u, player)) = read_economy_and_player(engine) else {
        return JobMarketAnswer::NotReady;
    };
    if !player.on_job {
        return JobMarketAnswer::OfferExpired;
    }
    // Cancelling a Quick Job destroys the temporary company truck (`0x5F7A90`). If the player
    // owns no truck of their own, staying in 3D driving mode with no spawned vehicle crashes Prism3D.
    if !player.has_own_truck {
        log!("job_market: refusing quick-job cancel while player owns no truck (would leave 3D world without a vehicle)");
        return JobMarketAnswer::NoTruck;
    }
    let player_u = safe_read::<usize>(economy_u + engine.off.econ_player).unwrap_or(0);
    crate::job_transition::trigger(crate::job_transition::TransitionKind::QuickCancel);
    // SAFETY: called on the game's main thread with a validated live `economy_u*` and active `current_job`.
    unsafe { (cancel_fn)(economy_u as *mut u8) };
    let cleared = read_economy_and_player(engine).is_some_and(|(_, after)| !after.on_job);
    // If this was a Quick Job, `cancel_player_job` despawned the temporary company truck actor.
    // Respawn the player's own truck immediately via `fleet::respawn_player_actor` (no-op if the
    // player already has a live truck, e.g. Freight/Cargo jobs).
    if cleared && valid_ptr(player_u) {
        // SAFETY: main thread, `economy_u` and `player_u` validated live.
        let _ = unsafe { crate::fleet::respawn_player_actor(economy_u, player_u) };
    } else {
        crate::job_transition::clear();
    }
    log!("job_market: cancel_player_job invoked (cleared={cleared})");
    if cleared { JobMarketAnswer::Ok } else { JobMarketAnswer::Failed }
}

fn snapshot_path() -> PathBuf {
    let base = log::log_dir().parent().map_or_else(|| log::log_dir().to_path_buf(), std::path::Path::to_path_buf);
    base.join(JOB_MARKET_FILE)
}

fn estimate_reward(
    distance_km: u32,
    unit_reward_per_km: f32,
    units: u32,
    ferry_price: u32,
    urgency: u32,
    fragile: bool,
    valuable: bool,
    adr_class: u32,
    skills: Skills,
) -> u32 {
    let base = 600.0 + (distance_km as f64) * (unit_reward_per_km.max(0.0) as f64) * (units.max(1) as f64);
    let mut bonus = 0.0;
    if distance_km >= 250 {
        bonus += 0.05 * (skills.long_dist as f64);
    }
    if fragile {
        bonus += 0.22 + 0.05 * (skills.fragile as f64);
    }
    if valuable {
        bonus += 0.18 + 0.05 * (skills.heavy as f64);
    }
    if adr_class != 0 && skills.adr != 0 {
        bonus += 0.20;
    }
    if urgency == 1 {
        bonus += 0.20 + 0.03 * (skills.urgent as f64);
    } else if urgency >= 2 {
        bonus += 0.30 + 0.05 * (skills.urgent as f64);
    }
    let total = base * (1.0 + bonus) + (ferry_price as f64);
    total.round().max(0.0) as u32
}

fn parse_company_unit_name(name: &str) -> Option<(String, String)> {
    let rest = name.strip_prefix("company.volatile.")?;
    let (co, city) = rest.split_once('.')?;
    if co.is_empty() || city.is_empty() {
        return None;
    }
    Some((co.to_string(), city.to_string()))
}

fn parse_target(target: &str) -> Option<(String, String)> {
    let (co, city) = target.split_once('.')?;
    if co.is_empty() || city.is_empty() {
        return None;
    }
    Some((co.to_string(), city.to_string()))
}

fn pretty_id(id: &str) -> String {
    id.split('_')
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

pub(crate) fn push_json_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn serialize_snapshot(
    player: &PlayerState,
    quick_supported: bool,
    company_names: &HashMap<String, String>,
    offers: &[SnapshotOffer],
) -> String {
    let mut s = String::with_capacity(offers.len() * 220 + 256);
    let _ = write!(
        s,
        r#"{{"gameTime":{},"onJob":{},"hasOwnTruck":{},"hasOwnTrailer":{},"trailerAttached":{},"quickSupported":{},"offers":["#,
        player.game_time,
        player.on_job,
        player.has_own_truck,
        player.has_own_trailer,
        player.trailer_attached,
        quick_supported,
    );
    for (idx, o) in offers.iter().enumerate() {
        if idx > 0 {
            s.push(',');
        }
        let dest_co_name = company_names.get(&o.dest_company_id).cloned().unwrap_or_else(|| pretty_id(&o.dest_company_id));
        s.push_str(r#"{"mode":"#);
        push_json_str(&mut s, o.mode);
        let _ = write!(s, r#","companyPtr":"{:#x}","offerPtr":"{:#x}","originCompanyId":"#, o.company_ptr, o.offer_ptr);
        push_json_str(&mut s, &o.origin_company_id);
        s.push_str(r#","originCompany":"#);
        push_json_str(&mut s, &o.origin_company_name);
        s.push_str(r#","originCityId":"#);
        push_json_str(&mut s, &o.origin_city_id);
        s.push_str(r#","destCompanyId":"#);
        push_json_str(&mut s, &o.dest_company_id);
        s.push_str(r#","destCompany":"#);
        push_json_str(&mut s, &dest_co_name);
        s.push_str(r#","destCityId":"#);
        push_json_str(&mut s, &o.dest_city_id);
        s.push_str(r#","cargoId":"#);
        push_json_str(&mut s, &o.cargo_id);
        s.push_str(r#","cargoName":"#);
        push_json_str(&mut s, &o.cargo_name);
        if !o.cargo_icon.is_empty() {
            s.push_str(r#","cargoIcon":"#);
            push_json_str(&mut s, &o.cargo_icon);
        }
        let _ = write!(
            s,
            r#","distanceKm":{},"ferryTime":{},"ferryPrice":{},"urgency":{},"expiresInMin":{},"unitsCount":{},"massKg":{},"reward":{},"fragile":{},"fragilityPct":{},"valuable":{},"adrClass":{},"overweight":{},"oversize":{}"#,
            o.distance_km,
            o.ferry_time,
            o.ferry_price,
            o.urgency,
            o.expires_in_min,
            o.units_count,
            o.mass_kg,
            o.reward,
            o.fragile,
            o.fragility_pct,
            o.valuable,
            o.adr_class,
            o.overweight,
            o.oversize,
        );
        if !o.company_truck.is_empty() {
            s.push_str(r#","companyTruck":"#);
            push_json_str(&mut s, &o.company_truck);
        }
        if !o.trailer_def.is_empty() {
            s.push_str(r#","trailerDef":"#);
            push_json_str(&mut s, &o.trailer_def);
        }
        s.push('}');
    }
    s.push_str("]}");
    s
}

// ---------------------------------------------------------------------------
// Dynamic PE & Reflection Discovery (zero hardcoded RVAs or offsets)
// ---------------------------------------------------------------------------

pub(crate) struct PeImage {
    pub(crate) base: usize,
    pub(crate) text: (usize, usize),
    pub(crate) rdata: (usize, usize),
    pub(crate) data: (usize, usize),
    pub(crate) pdata: (usize, usize),
}

impl PeImage {
    /// Parses the running game executable's mapped PE sections.
    ///
    /// # Safety
    /// `base` must be the loaded main executable handle (`GetModuleHandleW(None)`).
    pub(crate) unsafe fn of(base: usize) -> Option<Self> {
        // SAFETY: DOS + PE headers of the main executable are mapped for the process's lifetime.
        unsafe {
            if *(base as *const u16) != 0x5A4D {
                return None;
            }
            let pe = base + *(base.wrapping_add(0x3C) as *const u32) as usize;
            if *(pe as *const u32) != 0x0000_4550 {
                return None;
            }
            let count = *(pe.wrapping_add(6) as *const u16) as usize;
            let opt_size = *(pe.wrapping_add(20) as *const u16) as usize;
            let mut text = None;
            let mut rdata = None;
            let mut data = None;
            let mut pdata = None;
            for i in 0..count {
                let sec = pe + 24 + opt_size + i * 40;
                let name = std::slice::from_raw_parts(sec as *const u8, 8);
                let vsize = *(sec.wrapping_add(8) as *const u32) as usize;
                let vaddr = *(sec.wrapping_add(12) as *const u32) as usize;
                let range = (base + vaddr, vsize);
                if name.starts_with(b".text\0") {
                    text = Some(range);
                } else if name.starts_with(b".rdata\0") {
                    rdata = Some(range);
                } else if name.starts_with(b".data\0") {
                    data = Some(range);
                } else if name.starts_with(b".pdata\0") {
                    pdata = Some(range);
                }
            }
            Some(Self { base, text: text?, rdata: rdata?, data: data?, pdata: pdata? })
        }
    }

    pub(crate) fn bytes(&self, seg: (usize, usize)) -> &[u8] {
        // SAFETY: section ranges come directly from the mapped PE headers of the game executable.
        unsafe { std::slice::from_raw_parts(seg.0 as *const u8, seg.1) }
    }

    pub(crate) fn find_rdata(&self, needle: &[u8]) -> Option<usize> {
        let hay = self.bytes(self.rdata);
        hay.windows(needle.len()).position(|w| w == needle).map(|i| self.rdata.0 + i)
    }

    pub(crate) fn lea_target_at(&self, code: &[u8], idx: usize) -> Option<usize> {
        let b = code.get(idx..idx + 7)?;
        if (b[0] == 0x48 || b[0] == 0x4C) && b[1] == 0x8D && (b[2] & 0xC7) == 0x05 {
            let disp = i32::from_le_bytes(b[3..7].try_into().unwrap()) as isize;
            Some((self.text.0 + idx + 7).wrapping_add_signed(disp))
        } else {
            None
        }
    }

    pub(crate) fn find_lea_xrefs(&self, target: usize) -> Vec<usize> {
        let code = self.bytes(self.text);
        let mut out = Vec::new();
        for i in 0..code.len().saturating_sub(7) {
            if self.lea_target_at(code, i) == Some(target) {
                out.push(self.text.0 + i);
            }
        }
        out
    }

    pub(crate) fn find_calls_to(&self, target: usize) -> Vec<usize> {
        let code = self.bytes(self.text);
        let mut out = Vec::new();
        for i in 0..code.len().saturating_sub(5) {
            if code[i] == 0xE8 {
                let disp = i32::from_le_bytes(code[i + 1..i + 5].try_into().unwrap()) as isize;
                if (self.text.0 + i + 5).wrapping_add_signed(disp) == target {
                    out.push(self.text.0 + i);
                }
            }
        }
        out
    }

    /// Resolves any code VA to its true function entry point via `.pdata` + `UNW_FLAG_CHAININFO`.
    pub(crate) fn pdata_root(&self, va: usize) -> Option<usize> {
        let rva = va.checked_sub(self.base)? as u32;
        let pdata = self.bytes(self.pdata);
        let count = pdata.len() / 12;
        let (mut lo, mut hi) = (0usize, count);
        let mut found = None;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let o = mid * 12;
            let begin = u32::from_le_bytes(pdata[o..o + 4].try_into().unwrap());
            let end = u32::from_le_bytes(pdata[o + 4..o + 8].try_into().unwrap());
            let unwind = u32::from_le_bytes(pdata[o + 8..o + 12].try_into().unwrap());
            if rva < begin {
                hi = mid;
            } else if rva >= end {
                lo = mid + 1;
            } else {
                found = Some((begin, unwind));
                break;
            }
        }
        let (mut begin, mut unwind) = found?;
        for _ in 0..8 {
            let uaddr = self.base + unwind as usize;
            let ver_flags = safe_read::<u8>(uaddr)?;
            if ((ver_flags >> 3) & 0x4) == 0 {
                break;
            }
            let cnt = safe_read::<u8>(uaddr + 2)? as usize;
            let aligned = (cnt + 1) & !1;
            let chain_addr = uaddr + 4 + aligned * 2;
            begin = safe_read::<u32>(chain_addr)?;
            unwind = safe_read::<u32>(chain_addr + 8)?;
        }
        Some(self.base + begin as usize)
    }

    /// Reads an ASCII C-string from `.rdata` at `va` (returns `None` if outside `.rdata` or non-ASCII).
    fn rdata_cstr(&self, va: usize) -> Option<&str> {
        let off = va.checked_sub(self.rdata.0)?;
        let rdata = self.bytes(self.rdata);
        if off >= rdata.len() {
            return None;
        }
        let slice = &rdata[off..rdata.len().min(off + 80)];
        let end = slice.iter().position(|&b| b == 0)?;
        if end == 0 {
            return None;
        }
        let bytes = &slice[..end];
        if !bytes.iter().all(|&b| (0x20..0x7F).contains(&b)) {
            return None;
        }
        std::str::from_utf8(bytes).ok()
    }

    /// Locates a Prism3D `.data` `attribute_t` reflection table containing all `required` field names
    /// and returns a map `field_name -> struct_offset`.
    pub(crate) fn find_attr_table(&self, required: &[&str]) -> Option<HashMap<String, usize>> {
        let first = *required.first()?;
        let data = self.bytes(self.data);
        let r_lo = self.rdata.0;
        let r_hi = self.rdata.0 + self.rdata.1;
        let mut off = 32usize;
        while off + 8 <= data.len() {
            let ptr = usize::from_le_bytes(data[off..off + 8].try_into().unwrap());
            if ptr >= r_lo && ptr < r_hi && self.rdata_cstr(ptr) == Some(first) {
                let entry0 = off - 32;
                let mut found: HashMap<String, usize> = HashMap::new();
                for k in -60isize..=100isize {
                    let e = entry0 as isize + k * 40;
                    if e < 0 || (e as usize) + 40 > data.len() {
                        continue;
                    }
                    let e = e as usize;
                    let field_off = u64::from_le_bytes(data[e + 8..e + 16].try_into().unwrap()) as usize;
                    let name_ptr = usize::from_le_bytes(data[e + 32..e + 40].try_into().unwrap());
                    if field_off < 0x4000 && name_ptr >= r_lo && name_ptr < r_hi {
                        if let Some(name) = self.rdata_cstr(name_ptr) {
                            found.insert(name.to_string(), field_off);
                        }
                    }
                }
                if required.iter().all(|&k| found.contains_key(k)) {
                    return Some(found);
                }
            }
            off += 8;
        }
        None
    }

    /// Locates a contiguous Prism3D `.data` `attribute_t` reflection table (bounded by non-entry sentinels)
    /// containing all `required` fields and none of `forbidden`. Prevents spilling into adjacent class tables
    /// when two classes share field names (e.g. `bank_loan_u` vs `bank_loan_data_u`).
    pub(crate) fn find_exact_attr_table(&self, required: &[&str], forbidden: &[&str]) -> Option<HashMap<String, usize>> {
        let first = *required.first()?;
        let data = self.bytes(self.data);
        let r_lo = self.rdata.0;
        let r_hi = self.rdata.0 + self.rdata.1;
        let read_entry = |e: isize| -> Option<(&str, usize)> {
            if e < 0 || (e as usize) + 40 > data.len() {
                return None;
            }
            let e = e as usize;
            let field_off = u64::from_le_bytes(data[e + 8..e + 16].try_into().unwrap()) as usize;
            let name_ptr = usize::from_le_bytes(data[e + 32..e + 40].try_into().unwrap());
            if field_off >= 0x4000 || name_ptr < r_lo || name_ptr >= r_hi {
                return None;
            }
            Some((self.rdata_cstr(name_ptr)?, field_off))
        };
        let mut off = 32usize;
        while off + 8 <= data.len() {
            let ptr = usize::from_le_bytes(data[off..off + 8].try_into().unwrap());
            if ptr >= r_lo && ptr < r_hi && self.rdata_cstr(ptr) == Some(first) {
                let mut start = (off - 32) as isize;
                while read_entry(start - 40).is_some() {
                    start -= 40;
                }
                let mut found: HashMap<String, usize> = HashMap::new();
                let mut cur = start;
                while let Some((name, field_off)) = read_entry(cur) {
                    found.insert(name.to_string(), field_off);
                    cur += 40;
                }
                if required.iter().all(|&k| found.contains_key(k)) && forbidden.iter().all(|&k| !found.contains_key(k)) {
                    return Some(found);
                }
            }
            off += 8;
        }
        None
    }

    pub(crate) fn read_unit_name_helper_for_fleet(&self) -> Option<(usize, usize)> {
        let cargo_str = self
            .find_rdata(b"?index_of@?$array_t@V?$owner_ptr_t@Vcargo_offer_data_u@prism@@@prism@@@prism@@")
            .or_else(|| self.find_rdata(b"company.volatile.%s\0"))?;
        let cargo_xrefs = self.find_lea_xrefs(cargo_str);
        if cargo_xrefs.len() != 1 {
            return None;
        }
        let cargo_root = self.pdata_root(cargo_xrefs[0])?;
        find_unit_name_helper(self, cargo_root)
    }
}

/// Scans the running game executable for `g_economy_mgr`, native take-job functions, and reflection offsets.
///
/// # Safety
/// Must be called inside the game's process.
unsafe fn locate() -> Option<Engine> {
    // SAFETY: `GetModuleHandleW(None)` returns the main executable handle mapped for process lifetime.
    let base = unsafe { GetModuleHandleW(None) }.ok()?.0 as usize;
    // SAFETY: `base` is the mapped game executable.
    let img = unsafe { PeImage::of(base) }?;

    // 1. `take_freight_job`
    let freight_str = img.find_rdata(b"[take job] Failed to extract trailer!\0")?;
    let freight_xrefs = img.find_lea_xrefs(freight_str);
    if freight_xrefs.len() != 1 {
        log!("job_market: freight anchor xrefs={} (expected 1)", freight_xrefs.len());
        return None;
    }
    let freight_root = img.pdata_root(freight_xrefs[0])?;

    // 2. `take_cargo_job`
    let cargo_str = img
        .find_rdata(b"?index_of@?$array_t@V?$owner_ptr_t@Vcargo_offer_data_u@prism@@@prism@@@prism@@")
        .or_else(|| img.find_rdata(b"company.volatile.%s\0"))?;
    let cargo_xrefs = img.find_lea_xrefs(cargo_str);
    if cargo_xrefs.len() != 1 {
        log!("job_market: cargo anchor xrefs={} (expected 1)", cargo_xrefs.len());
        return None;
    }
    let cargo_xref = cargo_xrefs[0];
    let cargo_root = img.pdata_root(cargo_xref)?;

    // 3. `g_economy_mgr` from the UI caller of `take_cargo_job`
    let cargo_callers = img.find_calls_to(cargo_root);
    let mut econ_mgr_global = None;
    for call_va in &cargo_callers {
        if econ_mgr_global.is_none() {
            econ_mgr_global = find_mov_rcx_rip_before(&img, *call_va, 32);
        }
    }
    let econ_mgr_global = econ_mgr_global?;

    // 4. `unit_name_to_str` and `string_vtable` inside `cargo_root`
    let (unit_name_va, string_vtable) = find_unit_name_helper(&img, cargo_root)?;

    // 5. Optional `cancel_player_job` (true `take_quick_job` is resolved right after `offer_tbl` using `company_truck + 8`).
    let cancel_root = img
        .find_rdata(b"[cancel_player_job] Missing actor unit!\0")
        .and_then(|s| {
            let xrefs = img.find_lea_xrefs(s);
            if xrefs.len() == 1 { img.pdata_root(xrefs[0]) } else { None }
        });

    // 6. `company_u::cargo_offer` offset from `49 8d 8d <u32>` before `cargo_xref`
    let company_cargo_offer = find_cargo_offer_offset(&img, cargo_xref);
    // 7. Live actor & coupled trailer offsets from `cargo_root` prologue (`48 8b 81 <u32>; 4c 8b 40 <u8>`)
    let (player_actor, actor_trailer) = find_actor_trailer_offsets(&img, cargo_root).unwrap_or((0xa0, 0x48));

    // 8. Dynamic `.data` reflection tables
    let econ_tbl = img.find_attr_table(&["companies", "game_time", "player", "adr", "long_dist", "heavy", "fragile", "urgent"])?;
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
    let trailer_tbl = img.find_attr_table(&["trailer_definition", "slave_trailer", "is_private"])?;
    let trailer_def_tbl = img.find_attr_table(&["gross_trailer_weight_limit", "volume", "body_type", "chain_type"])?;
    let company_tbl = img.find_attr_table(&["permanent_data", "job_offer", "discovered", "state"])?;
    let comp_perm_tbl = img.find_attr_table(&["name", "sort_name", "trailer_look"])?;
    let offer_tbl = img.find_attr_table(&[
        "target",
        "expiration_time",
        "urgency",
        "shortest_distance_km",
        "ferry_time",
        "ferry_price",
        "cargo",
        "company_truck",
        "trailer_definition",
        "units_count",
        "fill_ratio",
    ])?;
    let cargo_tbl = img.find_attr_table(&[
        "name",
        "icon",
        "unit_reward_per_km",
        "fragility",
        "adr_class",
        "valuable",
        "volume",
        "mass",
        "body_types",
        "overweight",
        "oversize",
    ])?;

    let company_job_offer = *company_tbl.get("job_offer")?;
    let company_cargo_offer = company_cargo_offer.unwrap_or(company_job_offer + 0x28);

    let off = Offsets {
        econ_player: *econ_tbl.get("player")?,
        econ_companies: *econ_tbl.get("companies")?,
        econ_game_time: *econ_tbl.get("game_time")?,
        econ_adr: *econ_tbl.get("adr")?,
        econ_long_dist: *econ_tbl.get("long_dist")?,
        econ_heavy: *econ_tbl.get("heavy")?,
        econ_fragile: *econ_tbl.get("fragile")?,
        econ_urgent: *econ_tbl.get("urgent")?,
        player_current_job: *player_tbl.get("current_job")?,
        player_actor,
        actor_trailer,
        player_trucks: *player_tbl.get("trucks")?,
        player_assigned_truck: *player_tbl.get("assigned_truck")?,
        player_assigned_trailer: *player_tbl.get("assigned_trailer")?,
        player_assigned_trailer_connected: *player_tbl.get("assigned_trailer_connected")?,
        player_my_truck: *player_tbl.get("my_truck")?,
        player_my_trailer: *player_tbl.get("my_trailer")?,
        player_my_trailer_attached: *player_tbl.get("my_trailer_attached")?,
        trailer_definition: *trailer_tbl.get("trailer_definition")?,
        trailer_is_private: *trailer_tbl.get("is_private")?,
        trailer_def_volume: *trailer_def_tbl.get("volume")?,
        trailer_def_body_type: *trailer_def_tbl.get("body_type")?,
        trailer_def_gross_weight: *trailer_def_tbl.get("gross_trailer_weight_limit")?,
        company_permanent_data: *company_tbl.get("permanent_data")?,
        company_job_offer,
        company_cargo_offer,
        company_discovered: *company_tbl.get("discovered")?,
        company_state: *company_tbl.get("state")?,
        comp_perm_name: *comp_perm_tbl.get("name")?,
        offer_target: *offer_tbl.get("target")?,
        offer_expiration: *offer_tbl.get("expiration_time")?,
        offer_urgency: *offer_tbl.get("urgency")?,
        offer_distance_km: *offer_tbl.get("shortest_distance_km")?,
        offer_ferry_time: *offer_tbl.get("ferry_time")?,
        offer_ferry_price: *offer_tbl.get("ferry_price")?,
        offer_cargo: *offer_tbl.get("cargo")?,
        offer_company_truck: *offer_tbl.get("company_truck")?,
        offer_trailer_definition: *offer_tbl.get("trailer_definition")?,
        offer_units_count: *offer_tbl.get("units_count")?,
        cargo_name: *cargo_tbl.get("name")?,
        cargo_icon: *cargo_tbl.get("icon")?,
        cargo_reward_per_km: *cargo_tbl.get("unit_reward_per_km")?,
        cargo_fragility: *cargo_tbl.get("fragility")?,
        cargo_adr_class: *cargo_tbl.get("adr_class")?,
        cargo_valuable: *cargo_tbl.get("valuable")?,
        cargo_volume: *cargo_tbl.get("volume")?,
        cargo_mass: *cargo_tbl.get("mass")?,
        cargo_body_types: *cargo_tbl.get("body_types")?,
        cargo_overweight: *cargo_tbl.get("overweight")?,
        cargo_oversize: *cargo_tbl.get("oversize")?,
    };

    log!(
        "job_market: ready! economy_global=+{:#x}, freight/quick=+{:#x}, cargo=+{:#x}, cancel={}, unit_name=+{:#x}, cargo_offer=+{:#x}",
        econ_mgr_global - base,
        freight_root - base,
        cargo_root - base,
        cancel_root.map_or_else(|| "none".to_string(), |r| format!("+{:#x}", r - base)),
        unit_name_va - base,
        off.company_cargo_offer,
    );

    // SAFETY: function entry points verified by unique anchors + `.pdata` roots in the game's `.text` section.
    Some(Engine {
        econ_mgr_global: econ_mgr_global as *const *mut u8,
        freight_fn: unsafe { std::mem::transmute::<usize, TakeFreightFn>(freight_root) },
        cargo_fn: unsafe { std::mem::transmute::<usize, TakeCargoFn>(cargo_root) },
        cancel_fn: cancel_root.map(|r| unsafe { std::mem::transmute::<usize, CancelJobFn>(r) }),
        unit_name_fn: unsafe { std::mem::transmute::<usize, UnitNameFn>(unit_name_va) },
        string_vtable,
        off,
    })
}

fn find_mov_rcx_rip_before(img: &PeImage, call_va: usize, back: usize) -> Option<usize> {
    let start = call_va.checked_sub(back)?;
    let off = start.checked_sub(img.text.0)?;
    let b = img.bytes(img.text).get(off..off + back)?;
    if b.len() < 7 {
        return None;
    }
    for i in (0..=b.len() - 7).rev() {
        if b[i] == 0x48 && b[i + 1] == 0x8B && b[i + 2] == 0x0D {
            let disp = i32::from_le_bytes(b[i + 3..i + 7].try_into().unwrap()) as isize;
            return Some((start + i + 7).wrapping_add_signed(disp));
        }
    }
    None
}

fn find_unit_name_helper(img: &PeImage, cargo_root: usize) -> Option<(usize, usize)> {
    let off = cargo_root.checked_sub(img.text.0)?;
    let b = img.bytes(img.text).get(off..off + 0x120)?;
    // Match `41 8b 55 0c` (`mov edx, [r13+0xc]`) followed within 0x40 bytes by `48 8d 05 <vtable>` and `e8 <unit_name_to_str>`.
    let start = b.windows(4).position(|w| w == [0x41, 0x8B, 0x55, 0x0C])?;
    let win = &b[start..(start + 0x45).min(b.len())];
    let mut vtable = None;
    let mut call_va = None;
    for i in 0..win.len().saturating_sub(7) {
        if vtable.is_none() && win[i] == 0x48 && win[i + 1] == 0x8D && win[i + 2] == 0x05 {
            let disp = i32::from_le_bytes(win[i + 3..i + 7].try_into().unwrap()) as isize;
            vtable = Some((cargo_root + start + i + 7).wrapping_add_signed(disp));
        }
        if win[i] == 0xE8 && vtable.is_some() {
            let disp = i32::from_le_bytes(win[i + 1..i + 5].try_into().unwrap()) as isize;
            call_va = Some((cargo_root + start + i + 5).wrapping_add_signed(disp));
            break;
        }
    }
    Some((call_va?, vtable?))
}

fn find_cargo_offer_offset(img: &PeImage, cargo_xref: usize) -> Option<usize> {
    let start = cargo_xref.checked_sub(0x80)?;
    let off = start.checked_sub(img.text.0)?;
    let b = img.bytes(img.text).get(off..off + 0x80)?;
    // `49 8d 8d <u32>`: `lea rcx, [r13 + cargo_offer]`
    for i in (0..=b.len().saturating_sub(7)).rev() {
        if b[i] == 0x49 && b[i + 1] == 0x8D && b[i + 2] == 0x8D {
            let val = u32::from_le_bytes(b[i + 3..i + 7].try_into().unwrap()) as usize;
            if (0x40..=0x300).contains(&val) {
                return Some(val);
            }
        }
    }
    None
}

fn find_actor_trailer_offsets(img: &PeImage, cargo_root: usize) -> Option<(usize, usize)> {
    let off = cargo_root.checked_sub(img.text.0)?;
    let b = img.bytes(img.text).get(off..off + 0x70)?;
    // In `cargo_root` prologue: `48 8b 81 <u32>` (`mov rax, [rcx + player_actor]`) followed immediately by
    // `4c 8b 40 <u8>` (`mov r8, [rax + actor_trailer]`).
    for i in 0..b.len().saturating_sub(11) {
        if b[i] == 0x48 && b[i + 1] == 0x8B && b[i + 2] == 0x81 && b[i + 7] == 0x4C && b[i + 8] == 0x8B && b[i + 9] == 0x40 {
            let actor = u32::from_le_bytes(b[i + 3..i + 7].try_into().unwrap()) as usize;
            let trailer = b[i + 10] as usize;
            if (0x40..=0x300).contains(&actor) && (0x10..=0x100).contains(&trailer) {
                return Some((actor, trailer));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_company_unit_and_target() {
        assert_eq!(
            parse_company_unit_name("company.volatile.posped.berlin"),
            Some(("posped".into(), "berlin".into()))
        );
        assert_eq!(parse_company_unit_name("company.volatile.wal_mkt.los_angeles"), Some(("wal_mkt".into(), "los_angeles".into())));
        assert_eq!(parse_company_unit_name("other.foo.bar"), None);
        assert_eq!(parse_target("trameri.hamburg"), Some(("trameri".into(), "hamburg".into())));
        assert_eq!(parse_target("invalid"), None);
    }

    #[test]
    fn formats_pretty_ids_and_json_escaping() {
        assert_eq!(pretty_id("los_angeles"), "Los Angeles");
        let mut s = String::new();
        push_json_str(&mut s, "A \"B\" \\ C\n");
        assert_eq!(s, r#""A \"B\" \\ C\n""#);
    }

    #[test]
    fn estimates_reward_monotonically() {
        let low = estimate_reward(300, 1.5, 10, 0, 0, false, false, 0, Skills::default());
        let high = estimate_reward(
            300,
            1.5,
            10,
            200,
            2,
            true,
            true,
            1,
            Skills { adr: 1, long_dist: 6, heavy: 6, fragile: 6, urgent: 6 },
        );
        assert!(high > low);
        assert!(low >= 600);
    }
}
