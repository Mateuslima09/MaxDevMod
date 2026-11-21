//! Whether the game asks the GPU what is seen (occlusion queries), and how it's answered: the reversing camera's
//! mirror isn't drawn when it's out of view (owner 2026-11-14: zoomed in on the dashboard's screen, the camera's
//! gone). If the game asks "is the mirror seen?" with a query, answering "yes" would have it drawn anyway.
//!
//! A probe: queries made (by kind), and every 2 s the calls since (`Begin`, `End`, `GetData` by kind, how many
//! answers were "nothing seen", `SetPredication`), written to the log while the truck is in reverse.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

use windows::Win32::Graphics::Direct3D11::{D3D11_QUERY_DESC, ID3D11Query};
use windows::core::{HRESULT, Interface};

use crate::hooks::{guarded, hook};
use crate::log;

const VTBL_DEVICE_CREATE_QUERY: usize = 24;
const VTBL_DEVICE_CREATE_PREDICATE: usize = 25;
const VTBL_CONTEXT_GET_DATA: usize = 29;
const VTBL_CONTEXT_SET_PREDICATION: usize = 30;

static ORIG_CREATE_QUERY: AtomicUsize = AtomicUsize::new(0);
static ORIG_CREATE_PREDICATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_DATA: AtomicUsize = AtomicUsize::new(0);
static ORIG_SET_PREDICATION: AtomicUsize = AtomicUsize::new(0);

type CreateQueryFn = unsafe extern "system" fn(*mut c_void, *const D3D11_QUERY_DESC, *mut *mut c_void) -> HRESULT;
type GetDataFn = unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, u32, u32) -> HRESULT;
type SetPredicationFn = unsafe extern "system" fn(*mut c_void, *mut c_void, i32);

#[derive(Default)]
struct Counts {
    made: BTreeMap<i32, u64>,
    /// By the query's kind: calls answered, of them "nothing seen" (0 samples, or false).
    asked: BTreeMap<i32, (u64, u64)>,
    predicated: u64,
}

static COUNTS: Mutex<Option<Counts>> = Mutex::new(None);
static LAST_LOG: AtomicU64 = AtomicU64::new(0);

fn counts(f: impl FnOnce(&mut Counts)) {
    let mut guard = COUNTS.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Counts::default));
}

/// # Safety
/// The throwaway device's and context's vtables, before the hooks are switched on.
pub unsafe fn install(device: *const usize, context: *const usize) -> Result<(), String> {
    // SAFETY: the slots are ID3D11Device's and ID3D11DeviceContext's own, hooked with detours of their signatures.
    unsafe {
        hook(*device.add(VTBL_DEVICE_CREATE_QUERY), create_query as *mut c_void, &ORIG_CREATE_QUERY, "CreateQuery")?;
        hook(
            *device.add(VTBL_DEVICE_CREATE_PREDICATE),
            create_predicate as *mut c_void,
            &ORIG_CREATE_PREDICATE,
            "CreatePredicate",
        )?;
        hook(*context.add(VTBL_CONTEXT_GET_DATA), get_data as *mut c_void, &ORIG_GET_DATA, "GetData")?;
        hook(
            *context.add(VTBL_CONTEXT_SET_PREDICATION),
            set_predication as *mut c_void,
            &ORIG_SET_PREDICATION,
            "SetPredication",
        )?;
    }
    Ok(())
}

unsafe extern "system" fn create_query(this: *mut c_void, desc: *const D3D11_QUERY_DESC, out: *mut *mut c_void) -> HRESULT {
    // SAFETY: the game's arguments, looked at only; then the original with them.
    unsafe {
        if !desc.is_null() {
            let kind = (*desc).Query.0;
            guarded(|| counts(|c| *c.made.entry(kind).or_insert(0) += 1));
        }
        let original: CreateQueryFn = std::mem::transmute(ORIG_CREATE_QUERY.load(Ordering::Acquire));
        original(this, desc, out)
    }
}

unsafe extern "system" fn create_predicate(
    this: *mut c_void,
    desc: *const D3D11_QUERY_DESC,
    out: *mut *mut c_void,
) -> HRESULT {
    // SAFETY: as `create_query`.
    unsafe {
        if !desc.is_null() {
            let kind = 1000 + (*desc).Query.0;
            guarded(|| counts(|c| *c.made.entry(kind).or_insert(0) += 1));
        }
        let original: CreateQueryFn = std::mem::transmute(ORIG_CREATE_PREDICATE.load(Ordering::Acquire));
        original(this, desc, out)
    }
}

unsafe extern "system" fn get_data(this: *mut c_void, query: *mut c_void, data: *mut c_void, size: u32, flags: u32) -> HRESULT {
    // SAFETY: the original first, with the game's arguments; then its answer looked at (the game's buffer, `size`
    // bytes, when it says it's there).
    unsafe {
        let original: GetDataFn = std::mem::transmute(ORIG_GET_DATA.load(Ordering::Acquire));
        let result = original(this, query, data, size, flags);
        if result.0 == 0 && !query.is_null() && !data.is_null() {
            guarded(|| {
                let Some(query) = ID3D11Query::from_raw_borrowed(&query) else { return };
                let desc = query.GetDesc();
                let kind = desc.Query.0;
                // Occlusion (2): a u64 of samples drawn; occlusion predicate (3): a BOOL.
                let nothing = match (kind, size) {
                    (2, 8..) => *(data as *const u64) == 0,
                    (3, 4..) => *(data as *const i32) == 0,
                    _ => false,
                };
                counts(|c| {
                    let entry = c.asked.entry(kind).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += u64::from(nothing);
                });
            });
        }
        result
    }
}

unsafe extern "system" fn set_predication(this: *mut c_void, predicate: *mut c_void, value: i32) {
    // SAFETY: looked at only; then the original with the game's arguments.
    unsafe {
        if !predicate.is_null() {
            guarded(|| counts(|c| c.predicated += 1));
        }
        let original: SetPredicationFn = std::mem::transmute(ORIG_SET_PREDICATION.load(Ordering::Acquire));
        original(this, predicate, value)
    }
}

/// Every 2 s (120 frames) in reverse: the counts since, logged and cleared.
pub fn on_present(frame: u64) {
    if frame.saturating_sub(LAST_LOG.load(Ordering::Relaxed)) < 120 {
        return;
    }
    LAST_LOG.store(frame, Ordering::Relaxed);
    let reversing = crate::bridge::get()
        .is_some_and(|b| b.plugin.telemetry.read().is_some_and(|t| t.displayed_gear < 0 || t.gear < 0));
    let taken = COUNTS.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or_default();
    if !reversing {
        return;
    }
    log!(
        "queries: made {:?} (kind: 0 event, 1 timestamp, 2 occlusion, 3 occlusion predicate; +1000 predicates), \
         answered {:?} (kind: (answers, nothing seen)), predicated draws set {} times; mirrors seen: {}",
        taken.made,
        taken.asked,
        taken.predicated,
        crate::mirror::seen()
    );
}
