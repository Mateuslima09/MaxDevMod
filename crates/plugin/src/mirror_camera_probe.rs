//! Finding where the game keeps its mirrors' camera (the reversing camera's step 3, owner 2026-11-14: "our own
//! mirror, one behind the truck and one behind the trailer", not the two side mirrors put together;
//! docs/camera-de-re/RETROVISOR.md).
//!
//! The game draws each mirror's view (`mirror`: one 512 x 1024 target, each mirror a viewport in it) with the
//! same shaders as the main view; where the view is seen from is in the vertex shaders' constant buffers. Which
//! buffer and where in it isn't known, so this is a one-off probe: a second after the truck is put in reverse with
//! the mirrors drawn, one whole frame's draws are followed. For each pass and viewport (each mirror, and the main
//! view), the first [`DRAWS_PER_PART`] draws' vertex-shader constant buffers are read back. Each buffer's contents
//! are written in full the first time they're seen, and after that only whether they changed. A per-view buffer
//! keeps the same contents for all of a viewport's draws and differs between the mirrors. The truck's and
//! trailer's places (the SDK's) are written with it, to match against.
//!
//! Each read stalls the GPU, so the frame it's taken in stutters for a moment: [`MAX_CAPTURES`] a game,
//! [`COOLDOWN_FRAMES`] apart (move the truck back a little between them). Written to
//! `Documents\WestPlay\logs\retrovisor\camera-N.txt`.

use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

use windows::Win32::Graphics::Direct3D11::{
    D3D11_BUFFER_DESC, D3D11_CPU_ACCESS_READ, D3D11_DEVICE_CONTEXT_IMMEDIATE, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11_VIEWPORT, ID3D11Buffer, ID3D11DeviceContext,
    ID3D11RenderTargetView, ID3D11Texture2D,
};
use windows::core::{BOOL, Interface};

use crate::hooks::guarded;
use crate::{bridge, log};

/// Captures a game at most (each stutters a frame).
/// 0 since the camera was placed through the draws (2026-11-14 12:10, `rear_camera`): no more stutters.
const MAX_CAPTURES: u32 = 0;
/// Game frames between two captures (~20 s at 60 fps).
const COOLDOWN_FRAMES: u64 = 1200;
/// Game frames in reverse, with the mirrors drawn, before a capture (~1 s).
const SETTLE_FRAMES: u64 = 60;
/// Draws read back in each pass-and-viewport, at most.
const DRAWS_PER_PART: u32 = 24;
/// Parts (pass-and-viewport) followed in a frame, at most.
const MAX_PARTS: usize = 48;
/// Vertex-shader constant buffer slots followed.
const SLOTS: usize = 14;
/// Draws written in the table (`Capture::table`) per target, at most.
const TABLE_ROWS: u32 = 900;
/// A buffer is read up to this many bytes.
const MAX_BYTES: u32 = 4096;

/// Whether this frame's draws are being followed (checked first in every draw: cheap when off).
static ARMED: AtomicBool = AtomicBool::new(false);
static CAPTURES: AtomicU32 = AtomicU32::new(0);
static LAST_CAPTURE: AtomicU64 = AtomicU64::new(0);
/// The frame the truck was first seen in reverse with the mirrors drawn, this time (0: not).
static READY_SINCE: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
struct Capture {
    /// The vertex shader's constant buffers bound now: the buffer's address, its first constant and how many
    /// (16 bytes each; `VSSetConstantBuffers1`'s range, or 0 and 0 for all of it).
    slots: [(usize, u32, u32); SLOTS],
    /// The target bound now: its address, and what it is (size, format; "MIRRORS" for the mirrors').
    target: (usize, String),
    viewport: [f32; 4],
    /// The parts in the order they came, and how many draws each had read.
    parts: Vec<(String, u32)>,
    /// The text written so far.
    text: String,
    /// Contents already written: (slot, address, hash) → which part and draw first had them.
    seen: HashMap<(usize, usize, u64), String>,
    draws: u64,
    /// The calls followed this frame, by kind and context (immediate or deferred).
    calls: HashMap<String, u64>,
    /// Each mirror's turn (and the main view's), seen in a draw of an object at the camera: searched for in the
    /// game's memory after the frame (`mirror_memory_scan`).
    views: Vec<crate::mirror_memory_scan::View>,
    /// The truck's position (the SDK's) as the capture was armed.
    truck: [f64; 3],
    /// Every draw of the main mirrors and the main view (`on_draw`), and how many each target has had.
    table: String,
    table_rows: HashMap<usize, u32>,
}

struct Shared(Capture);
// SAFETY: only used on the game's render thread (its immediate context's calls), through the mutex.
unsafe impl Send for Shared {}

static CAPTURE: LazyLock<Mutex<Shared>> = LazyLock::new(|| Mutex::new(Shared(Capture::default())));

fn capture() -> std::sync::MutexGuard<'static, Shared> {
    CAPTURE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn armed() -> bool {
    ARMED.load(Ordering::Relaxed)
}

/// `VSSetConstantBuffers` / `VSSetConstantBuffers1` (`first`/`counts`: null for the whole buffers).
///
/// # Safety
/// The game's arguments, on its render thread: `count` buffers (each live or null), and as many firsts and counts
/// if those aren't null.
pub unsafe fn on_vs_constant_buffers(
    context: *mut c_void,
    start: u32,
    count: u32,
    buffers: *const *mut c_void,
    firsts: *const u32,
    counts: *const u32,
) {
    // SAFETY: the game's live context (the caller's contract).
    if !armed() || buffers.is_null() || !unsafe { immediate(context, "VSSetConstantBuffers") } {
        return;
    }
    let mut capture = capture();
    for i in 0..count as usize {
        let slot = start as usize + i;
        if slot >= SLOTS {
            break;
        }
        // SAFETY: within the game's `count` (the caller's contract).
        let (buffer, first, n) = unsafe {
            (
                *buffers.add(i) as usize,
                if firsts.is_null() { 0 } else { *firsts.add(i) },
                if counts.is_null() { 0 } else { *counts.add(i) },
            )
        };
        capture.0.slots[slot] = (buffer, first, n);
    }
}

/// `RSSetViewports`.
///
/// # Safety
/// The game's arguments: `count` viewports.
pub unsafe fn on_viewports(count: u32, viewports: *const D3D11_VIEWPORT) {
    if !armed() || count == 0 || viewports.is_null() {
        return;
    }
    // SAFETY: the first of the game's viewports.
    let v = unsafe { *viewports };
    capture().0.viewport = [v.TopLeftX, v.TopLeftY, v.Width, v.Height];
}

/// `OMSetRenderTargets*`: which target is drawn to now.
///
/// # Safety
/// The game's arguments: `count` views, each live or null.
pub unsafe fn on_set_render_targets(context: *mut c_void, count: u32, views: *const *mut c_void) {
    // SAFETY: the game's live context (the caller's contract).
    if !armed() || !unsafe { immediate(context, "OMSetRenderTargets") } {
        return;
    }
    let mut target = (0usize, "none".to_string());
    if count > 0 && count <= 8 && !views.is_null() {
        // SAFETY: the game's first view, live or null.
        unsafe {
            if let Some(view) = ID3D11RenderTargetView::from_raw_borrowed(&*views)
                && let Ok(texture) = view.GetResource().and_then(|r| r.cast::<ID3D11Texture2D>())
            {
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                texture.GetDesc(&mut desc);
                let what = if crate::mirror::is_mirrors(&desc) { " MIRRORS" } else { "" };
                target =
                    (texture.as_raw() as usize, format!("{}x{} f{}{what}", desc.Width, desc.Height, desc.Format.0));
            }
        }
    }
    capture().0.target = target;
}

/// A draw (`DrawIndexed`, `DrawIndexedInstanced`, `Draw`): its part's first draws' buffers read back.
///
/// # Safety
/// The game's live context, in its draw call, on its render thread.
pub unsafe fn on_draw(context: *mut c_void, kind: &'static str, count: u32, start: u32, base: i32) {
    if !armed() {
        return;
    }
    // SAFETY: the game's live context (the caller's contract).
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        let immediate = context.GetType() == D3D11_DEVICE_CONTEXT_IMMEDIATE;
        let mut guard = capture();
        let c = &mut guard.0;
        *c.calls.entry(format!("{kind} {}", if immediate { "imediato" } else { "diferido" })).or_insert(0) += 1;
        if !immediate {
            return;
        }
        c.draws += 1;
        let v = c.viewport;
        let part = format!("{:#x} {} viewport {} {} {}x{}", c.target.0, c.target.1, v[0], v[1], v[2], v[3]);
        let index = match c.parts.iter().position(|(p, _)| *p == part) {
            Some(i) => i,
            None if c.parts.len() < MAX_PARTS => {
                c.parts.push((part.clone(), 0));
                let _ =
                    writeln!(c.text, "\n== parte {}: alvo {part} (desenho nº {} do quadro)", c.parts.len(), c.draws);
                c.parts.len() - 1
            }
            None => return,
        };
        // Every draw of the two main mirrors and of the main view: its mesh (index buffer, count, start, base) and
        // where its object is seen from the camera (the view-space translation, and the turn's first row), to tell
        // where a mirror's camera is from the same mesh seen in both (`table`).
        let tabled = (c.target.1.starts_with("256x512 f26") || c.target.1.starts_with("1920x1080 f10"))
            && c.table_rows.get(&c.target.0).copied().unwrap_or(0) < TABLE_ROWS;
        if tabled && c.slots[0].0 != 0 {
            let raw = c.slots[0].0 as *mut c_void;
            let (first, n) = (c.slots[0].1, c.slots[0].2);
            if let Some(buffer) = ID3D11Buffer::from_raw_borrowed(&raw)
                && let Some(bytes) = read(context, buffer, first, n)
                && bytes.len() >= 48
            {
                let f =
                    |i: usize| f32::from_le_bytes([bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2], bytes[i * 4 + 3]]);
                let mut index_buffer = None;
                context.IAGetIndexBuffer(Some(&mut index_buffer), None, None);
                let ib = index_buffer.map_or(0, |b| b.as_raw() as usize);
                *c.table_rows.entry(c.target.0).or_insert(0) += 1;
                let _ = writeln!(
                    c.table,
                    "T {:#x} {} ib={ib:#x} n={count} s={start} b={base} t=({:.4} {:.4} {:.4}) r0=({:.5} {:.5} {:.5})",
                    c.target.0,
                    c.target.1,
                    f(3),
                    f(7),
                    f(11),
                    f(0),
                    f(1),
                    f(2)
                );
            }
        }
        if c.parts[index].1 >= DRAWS_PER_PART {
            return;
        }
        c.parts[index].1 += 1;
        let draw = c.parts[index].1;
        let mut line = format!("desenho {draw}:");
        for slot in 0..SLOTS {
            let (address, first, n) = c.slots[slot];
            if address == 0 {
                continue;
            }
            let raw = address as *mut c_void;
            let Some(buffer) = ID3D11Buffer::from_raw_borrowed(&raw) else { continue };
            let Some(bytes) = read(context, buffer, first, n) else {
                let _ = write!(line, " b{slot}=?");
                continue;
            };
            if slot == 0 && c.views.len() < 8 && !c.views.iter().any(|v| v.what == c.target.1) {
                let mirror = c.target.1.starts_with("256x") && c.target.1.contains(" f26");
                let main = c.target.1.starts_with("1920x1080 f10") || c.target.1.contains(" f10") && v[2] >= 1280.0;
                if (mirror || main)
                    && let Some(mut view) = crate::mirror_memory_scan::view_in(&c.target.1, &bytes)
                {
                    view.what = format!("{} ({:#x})", view.what, c.target.0);
                    if !c.views.iter().any(|seen| seen.what == view.what) {
                        c.views.push(view);
                    }
                }
            }
            let hash = fnv(&bytes);
            let key = (slot, address, hash);
            match c.seen.get(&key) {
                Some(at) => {
                    let _ = write!(line, " b{slot}={:08x}(igual a {at})", hash as u32);
                }
                None => {
                    let at = format!("p{}d{draw}", index + 1);
                    let _ = write!(line, " b{slot}={:08x}(NOVO)", hash as u32);
                    let mut dump = format!(
                        "   b{slot} {address:#x} constantes {first}+{n} ({} bytes) {:08x}:\n",
                        bytes.len(),
                        hash as u32
                    );
                    for (row, chunk) in bytes.chunks(16).enumerate() {
                        let f: Vec<String> = chunk
                            .chunks(4)
                            .map(|b| format!("{:12.5}", f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
                            .collect();
                        let _ = writeln!(dump, "     [{row:3}] {}", f.join(" "));
                    }
                    c.seen.insert(key, at);
                    line.push('\n');
                    line.push_str(&dump);
                }
            }
        }
        c.text.push_str(&line);
        c.text.push('\n');
    }
}

/// A constant buffer's contents (its bound range, or all of it), read back now: the GPU waits.
///
/// # Safety
/// The game's live immediate context and one of its buffers, on its render thread.
unsafe fn read(context: &ID3D11DeviceContext, buffer: &ID3D11Buffer, first: u32, count: u32) -> Option<Vec<u8>> {
    // SAFETY: as the caller says; the staging buffer is ours, of the buffer's size, mapped and unmapped here.
    unsafe {
        let mut desc = D3D11_BUFFER_DESC::default();
        buffer.GetDesc(&mut desc);
        let device = context.GetDevice().ok()?;
        let staging_desc = D3D11_BUFFER_DESC {
            ByteWidth: desc.ByteWidth,
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
            StructureByteStride: 0,
        };
        let mut staging = None;
        device.CreateBuffer(&staging_desc, None, Some(&mut staging)).ok()?;
        let staging = staging?;
        context.CopyResource(&staging, buffer);
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).ok()?;
        let start = (first * 16).min(desc.ByteWidth);
        let len = if count == 0 { desc.ByteWidth - start } else { (count * 16).min(desc.ByteWidth - start) };
        let len = len.min(MAX_BYTES);
        let bytes = std::slice::from_raw_parts((mapped.pData as *const u8).add(start as usize), len as usize).to_vec();
        context.Unmap(&staging, 0);
        Some(bytes)
    }
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3))
}

/// At each present: a capture taken in the frame just ended is saved; the next frame is armed once the truck has
/// been in reverse a second with the mirrors drawn (within the limits).
pub fn on_present(frame: u64) {
    if ARMED.swap(false, Ordering::Relaxed) {
        finish(frame);
        return;
    }
    if CAPTURES.load(Ordering::Relaxed) >= MAX_CAPTURES {
        return;
    }
    let Some(bridge) = bridge::get() else { return };
    let reversing =
        bridge.plugin.telemetry.read().is_some_and(|t| t.electric_enabled && (t.displayed_gear < 0 || t.gear < 0));
    if !reversing || !crate::mirror::seen() || crate::telemetry::paused() {
        READY_SINCE.store(0, Ordering::Relaxed);
        return;
    }
    let since = READY_SINCE.load(Ordering::Relaxed);
    if since == 0 {
        READY_SINCE.store(frame, Ordering::Relaxed);
        return;
    }
    let last = LAST_CAPTURE.load(Ordering::Relaxed);
    if frame - since < SETTLE_FRAMES || (last != 0 && frame - last < COOLDOWN_FRAMES) {
        return;
    }
    let mut c = capture();
    c.0 = Capture::default();
    let truck = crate::telemetry::truck();
    let trailer = crate::telemetry::trailer();
    let _ = writeln!(
        c.0.text,
        "# Câmera dos espelhos: os constant buffers do vertex shader de um quadro (quadro {}).\n\
         # caminhão: posição {:?}, direção {} voltas, inclinação {} e rolagem {} voltas, cabeça {:?}\n\
         # carreta: engatada {}, posição {:?}, direção {} voltas, pino {:?}, quinta roda {:?}",
        frame + 1,
        truck.position,
        truck.heading,
        truck.pitch,
        truck.roll,
        truck.head,
        trailer.connected,
        trailer.position,
        trailer.heading,
        trailer.hook,
        crate::telemetry::truck_hook()
    );
    c.0.truck = truck.position;
    LAST_CAPTURE.store(frame, Ordering::Relaxed);
    ARMED.store(true, Ordering::Relaxed);
}

fn finish(frame: u64) {
    let number = CAPTURES.fetch_add(1, Ordering::Relaxed) + 1;
    let capture = std::mem::take(&mut capture().0);
    let mut text = capture.text;
    let _ = writeln!(text, "\n# Tabela: cada desenho dos espelhos principais e da vista principal\n{}", capture.table);
    let mut calls: Vec<_> = capture.calls.iter().collect();
    calls.sort();
    let _ = writeln!(text, "\n# chamadas no quadro: {calls:?}");
    let _ = writeln!(text, "# {} desenhos no contexto imediato; partes:", capture.draws);
    for (n, (part, draws)) in capture.parts.iter().enumerate() {
        let _ = writeln!(text, "#   {:2}: {part} ({draws} lidos)", n + 1);
    }
    log!(
        "retrovisor: camera capture {number} taken in frame {frame}: {} draws, {} parts",
        capture.draws,
        capture.parts.len()
    );
    let dir = crate::log::log_dir().join("retrovisor");
    let (views, truck) = (capture.views, capture.truck);
    log!("retrovisor: views to look for in memory: {:?}", views.iter().map(|v| v.what.as_str()).collect::<Vec<_>>());
    let _ = std::thread::Builder::new().name("westplay-mirror-camera".into()).spawn(move || {
        let file = dir.join(format!("camera-{number}.txt"));
        let saved = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, text));
        match saved {
            Ok(()) => log!("retrovisor: camera capture saved in {}", file.display()),
            Err(e) => log!("retrovisor: camera capture not saved ({}): {e}", file.display()),
        }
        if !views.is_empty() {
            crate::mirror_memory_scan::scan(views, truck, dir.join(format!("camera-{number}-memoria.txt")));
        }
    });
}

/// Whether `context` is the immediate one; a deferred one's call is counted (`calls`) and left.
///
/// # Safety
/// `context` is a live device context.
unsafe fn immediate(context: *mut c_void, kind: &'static str) -> bool {
    // SAFETY: as the caller says.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return false };
        if context.GetType() == D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return true;
        }
    }
    *capture().0.calls.entry(format!("{kind} diferido")).or_insert(0) += 1;
    false
}

// The hooks. The draw calls' functions are taken from the game's own contexts too (`hook_context`), not only from
// a throwaway device's (`install`): Direct3D can give a context of another kind (single-threaded, deferred) other
// functions for the same call, and the first try (2026-11-14) followed one draw a frame. A hook keeps each of its
// functions' originals, picked by the calling context's own function.

/// One call hooked in up to four functions: each one's address and its original (the trampoline).
struct Twin {
    slot: usize,
    name: &'static str,
    targets: [AtomicUsize; 4],
    originals: [AtomicUsize; 4],
}

impl Twin {
    const fn new(slot: usize, name: &'static str) -> Self {
        Twin { slot, name, targets: [const { AtomicUsize::new(0) }; 4], originals: [const { AtomicUsize::new(0) }; 4] }
    }

    /// The original for `this` context's own function.
    ///
    /// # Safety
    /// `this` is a live context whose vtable has the slot.
    unsafe fn original(&self, this: *mut c_void) -> usize {
        // SAFETY: as the caller says.
        let target = unsafe { crate::hooks::vtable_entry(this, self.slot) };
        for i in 0..4 {
            if self.targets[i].load(Ordering::Acquire) == target {
                return self.originals[i].load(Ordering::Acquire);
            }
        }
        self.originals[0].load(Ordering::Acquire)
    }

    /// Hooks `target` (if it isn't yet), switched on now if `now`.
    ///
    /// # Safety
    /// `target` is this call's function in d3d11.dll, `detour` of its signature.
    unsafe fn hook(&self, target: usize, detour: *mut c_void, now: bool) {
        if (0..4).any(|i| self.targets[i].load(Ordering::Acquire) == target) {
            return;
        }
        let Some(i) = (0..4).find(|&i| self.targets[i].load(Ordering::Acquire) == 0) else {
            return log!("retrovisor: no room to hook {} again at {target:#x}", self.name);
        };
        // SAFETY: as the caller says; the original is stored before the hook is switched on.
        let made = unsafe { crate::hooks::hook(target, detour, &self.originals[i], self.name) };
        if let Err(e) = made {
            return log!("retrovisor: {} not hooked at {target:#x}: {e}", self.name);
        }
        self.targets[i].store(target, Ordering::Release);
        if now {
            // SAFETY: the hook just made, its original stored.
            if let Err(e) = unsafe { crate::hooks::enable(target) } {
                log!("retrovisor: {} not switched on at {target:#x}: {e}", self.name);
            }
        }
    }
}

static VS_CB: Twin = Twin::new(7, "VSSetConstantBuffers");
static DRAW_INDEXED: Twin = Twin::new(12, "DrawIndexed");
static DRAW: Twin = Twin::new(13, "Draw");
static DRAW_INDEXED_INSTANCED: Twin = Twin::new(20, "DrawIndexedInstanced");
static DRAW_INSTANCED: Twin = Twin::new(21, "DrawInstanced");
static DRAW_INDEXED_INDIRECT: Twin = Twin::new(39, "DrawIndexedInstancedIndirect");
static DRAW_INSTANCED_INDIRECT: Twin = Twin::new(40, "DrawInstancedIndirect");
static VIEWPORTS: Twin = Twin::new(44, "RSSetViewports");
static EXECUTE: Twin = Twin::new(58, "ExecuteCommandList");
static VS_CB1: Twin = Twin::new(119, "VSSetConstantBuffers1");

fn hooks() -> [(&'static Twin, *mut c_void); 10] {
    [
        (&VS_CB, vs_cb as *mut c_void),
        (&DRAW_INDEXED, draw_indexed as *mut c_void),
        (&DRAW, draw as *mut c_void),
        (&DRAW_INDEXED_INSTANCED, draw_indexed_instanced as *mut c_void),
        (&DRAW_INSTANCED, draw_instanced as *mut c_void),
        (&DRAW_INDEXED_INDIRECT, draw_indexed_indirect as *mut c_void),
        (&DRAW_INSTANCED_INDIRECT, draw_instanced_indirect as *mut c_void),
        (&VIEWPORTS, viewports as *mut c_void),
        (&EXECUTE, execute as *mut c_void),
        (&VS_CB1, vs_cb1 as *mut c_void),
    ]
}

/// At start, from a throwaway device's context (its vtable; `context1`: whether it has `ID3D11DeviceContext1`'s).
///
/// # Safety
/// `vtable` is a live device context's vtable, before the hooks are switched on.
pub unsafe fn install(vtable: *const usize, context1: bool) {
    for (twin, detour) in hooks() {
        if twin.slot >= 115 && !context1 {
            continue;
        }
        // SAFETY: the slot is within the vtable (ID3D11DeviceContext's 115, ID3D11DeviceContext1's past them).
        unsafe { twin.hook(*vtable.add(twin.slot), detour, false) };
    }
}

/// The contexts' vtables already looked at.
static CONTEXTS: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];

/// Each context the game sets targets on: its own functions hooked too, the first time its kind is seen.
///
/// # Safety
/// `context` is the game's live context, on its thread.
pub unsafe fn hook_context(context: *mut c_void) {
    // SAFETY: a COM object starts with its vtable's address.
    let vtable = unsafe { *(context as *const usize) };
    if CONTEXTS.iter().any(|seen| seen.load(Ordering::Relaxed) == vtable) {
        return;
    }
    let Some(free) = CONTEXTS.iter().find(|seen| seen.load(Ordering::Relaxed) == 0) else { return };
    free.store(vtable, Ordering::Relaxed);
    // SAFETY: the game's live context (the caller's contract).
    let context1 = unsafe {
        ID3D11DeviceContext::from_raw_borrowed(&context)
            .is_some_and(|c| c.cast::<windows::Win32::Graphics::Direct3D11::ID3D11DeviceContext1>().is_ok())
    };
    let mut new = Vec::new();
    for (twin, detour) in hooks() {
        if twin.slot >= 115 && !context1 {
            continue;
        }
        // SAFETY: the slot is within the context's vtable (checked for ID3D11DeviceContext1's).
        let target = unsafe { crate::hooks::vtable_entry(context, twin.slot) };
        if !(0..4).any(|i| twin.targets[i].load(Ordering::Acquire) == target) {
            new.push(twin.name);
            // SAFETY: that context's own function for the call.
            unsafe { twin.hook(target, detour, true) };
        }
    }
    log!("retrovisor: a game context's functions (vtable {vtable:#x}): newly hooked {new:?}");
}

type VsCb = unsafe extern "system" fn(*mut c_void, u32, u32, *const *mut c_void);
type VsCb1 = unsafe extern "system" fn(*mut c_void, u32, u32, *const *mut c_void, *const u32, *const u32);
type DrawIndexedFn = unsafe extern "system" fn(*mut c_void, u32, u32, i32);
type DrawFn = unsafe extern "system" fn(*mut c_void, u32, u32);
type DrawIndexedInstancedFn = unsafe extern "system" fn(*mut c_void, u32, u32, u32, i32, u32);
type DrawInstancedFn = unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32);
type IndirectFn = unsafe extern "system" fn(*mut c_void, *mut c_void, u32);
type ViewportsFn = unsafe extern "system" fn(*mut c_void, u32, *const D3D11_VIEWPORT);
type ExecuteFn = unsafe extern "system" fn(*mut c_void, *mut c_void, BOOL);

// Each detour: looks (only while a capture is armed), then the original for the calling context's function, with
// the game's arguments as they came.

unsafe extern "system" fn vs_cb(this: *mut c_void, start: u32, count: u32, buffers: *const *mut c_void) {
    // SAFETY: see above.
    unsafe {
        if armed() {
            guarded(|| on_vs_constant_buffers(this, start, count, buffers, std::ptr::null(), std::ptr::null()));
        }
        crate::rear_camera::note_vs_cb(start, count);
        guarded(|| crate::float_screen::on_vs_constant_buffers(this, start, count, buffers));
        let original: VsCb = std::mem::transmute(VS_CB.original(this));
        original(this, start, count, buffers)
    }
}

unsafe extern "system" fn vs_cb1(
    this: *mut c_void,
    start: u32,
    count: u32,
    buffers: *const *mut c_void,
    firsts: *const u32,
    counts: *const u32,
) {
    // SAFETY: see above.
    unsafe {
        if armed() {
            guarded(|| on_vs_constant_buffers(this, start, count, buffers, firsts, counts));
        }
        guarded(|| crate::rear_camera::on_vs_constant_buffers1(this, start, count, buffers, firsts, counts));
        guarded(|| crate::float_screen::on_vs_constant_buffers1(this, start, count, buffers, firsts, counts));
        guarded(|| crate::brake_smoke::on_vs_constant_buffers1(this, start, count, buffers, firsts, counts));
        let original: VsCb1 = std::mem::transmute(VS_CB1.original(this));
        original(this, start, count, buffers, firsts, counts)
    }
}

unsafe extern "system" fn draw_indexed(this: *mut c_void, count: u32, start: u32, base: i32) {
    // SAFETY: see above.
    unsafe {
        if crate::hide_virtual::skip_draw("DrawIndexed", count, start) || crate::rear_camera::skip_draw("DrawIndexed", count) {
            return;
        }
        if armed() {
            guarded(|| on_draw(this, "DrawIndexed", count, start, base));
        }
        let original: DrawIndexedFn = std::mem::transmute(DRAW_INDEXED.original(this));
        original(this, count, start, base)
    }
}

unsafe extern "system" fn draw(this: *mut c_void, count: u32, start: u32) {
    // SAFETY: see above.
    unsafe {
        if crate::hide_virtual::skip_draw("Draw", count, start) || crate::rear_camera::skip_draw("Draw", count) {
            return;
        }
        if armed() {
            guarded(|| on_draw(this, "Draw", count, start, 0));
        }
        let original: DrawFn = std::mem::transmute(DRAW.original(this));
        original(this, count, start)
    }
}

unsafe extern "system" fn draw_indexed_instanced(
    this: *mut c_void,
    count: u32,
    instances: u32,
    start: u32,
    base: i32,
    first_instance: u32,
) {
    // SAFETY: see above.
    unsafe {
        if crate::hide_virtual::skip_draw("DrawIndexedInstanced", count, start) || crate::rear_camera::skip_draw("DrawIndexedInstanced", count) {
            return;
        }
        if armed() {
            guarded(|| on_draw(this, "DrawIndexedInstanced", count, start, base));
        }
        let original: DrawIndexedInstancedFn = std::mem::transmute(DRAW_INDEXED_INSTANCED.original(this));
        original(this, count, instances, start, base, first_instance)
    }
}

unsafe extern "system" fn draw_instanced(this: *mut c_void, count: u32, instances: u32, start: u32, first: u32) {
    // SAFETY: see above.
    unsafe {
        if crate::hide_virtual::skip_draw("DrawInstanced", count, start) || crate::rear_camera::skip_draw("DrawInstanced", count) {
            return;
        }
        if armed() {
            guarded(|| on_draw(this, "DrawInstanced", count, start, 0));
        }
        let original: DrawInstancedFn = std::mem::transmute(DRAW_INSTANCED.original(this));
        original(this, count, instances, start, first)
    }
}

unsafe extern "system" fn draw_indexed_indirect(this: *mut c_void, args: *mut c_void, offset: u32) {
    // SAFETY: see above.
    unsafe {
        if armed() {
            guarded(|| on_draw(this, "DrawIndexedInstancedIndirect", 0, 0, 0));
        }
        let original: IndirectFn = std::mem::transmute(DRAW_INDEXED_INDIRECT.original(this));
        original(this, args, offset)
    }
}

unsafe extern "system" fn draw_instanced_indirect(this: *mut c_void, args: *mut c_void, offset: u32) {
    // SAFETY: see above.
    unsafe {
        if armed() {
            guarded(|| on_draw(this, "DrawInstancedIndirect", 0, 0, 0));
        }
        let original: IndirectFn = std::mem::transmute(DRAW_INSTANCED_INDIRECT.original(this));
        original(this, args, offset)
    }
}

unsafe extern "system" fn viewports(this: *mut c_void, count: u32, list: *const D3D11_VIEWPORT) {
    // SAFETY: see above.
    unsafe {
        if armed() {
            guarded(|| {
                if immediate(this, "RSSetViewports") {
                    on_viewports(count, list)
                }
            });
        }
        let original: ViewportsFn = std::mem::transmute(VIEWPORTS.original(this));
        original(this, count, list)
    }
}

unsafe extern "system" fn execute(this: *mut c_void, list: *mut c_void, restore: BOOL) {
    // SAFETY: see above.
    unsafe {
        if armed() {
            guarded(|| {
                *capture().0.calls.entry("ExecuteCommandList".into()).or_insert(0) += 1;
            });
        }
        let original: ExecuteFn = std::mem::transmute(EXECUTE.original(this));
        original(this, list, restore)
    }
}
