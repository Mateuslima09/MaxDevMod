//! The game's virtual mirrors drawn but not shown (owner 2026-11-14: with them on, the game draws its mirrors even
//! zoomed in on the dashboard's screen, so our reversing camera, made from a mirror, stays; but "I don't want the
//! mirrors on my screen").
//!
//! The virtual mirrors are put on the screen with the rest of the game's interface, at the end of each frame: drawn
//! into the final picture (a full-size 8-bit target) while one of the mirrors' own pictures is bound to the pixel
//! shader. Every mirror picture is a target 256 wide of its own (R11G11B10 or RGBA8 sRGB) (or the camera mirrors put together, 512 x 1024),
//! noted as the game draws into it ([`on_set_render_targets`]); a draw into the final picture with one of them bound
//! ([`on_ps_resources`]) is skipped ([`skip_draw`]). The mirrors in the cab (drawn into the scene) and our camera
//! (composed by the plugin) aren't touched.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

use windows::Win32::Graphics::Direct3D11::{
    D3D11_DEVICE_CONTEXT_IMMEDIATE, D3D11_TEXTURE2D_DESC, ID3D11DeviceContext, ID3D11RenderTargetView,
    ID3D11ShaderResourceView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
};
use windows::core::Interface;

use crate::log;

/// Whether the virtual mirrors are hidden.
const HIDE: bool = true;

/// The mirrors' pictures (their textures' addresses), as the game draws into them.
/// Each with the frame it was last drawn into: only those of the last few seconds count (the game makes them anew,
/// switching trucks, zooming: a list that only grew, capped, stopped taking the new ones, owner 2026-11-14).
static MIRRORS: LazyLock<Mutex<std::collections::HashMap<usize, u64>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
/// Frames a mirror's picture counts after it was last drawn into.
const MIRROR_KEPT: u64 = 600;
/// Whether the target drawn to now is the final picture.
static FINAL: AtomicBool = AtomicBool::new(false);
/// The pixel shader's slots holding a mirror's picture now (bits), while drawing into the final picture; and those
/// holding one to be hidden (not of a side the player chose to see, owner 2026-11-18).
static BOUND: AtomicU32 = AtomicU32::new(0);
static BOUND_HIDDEN: AtomicU32 = AtomicU32::new(0);

/// Which side a virtual mirror's picture is of (-1 the truck's left, 1 its right, 0 not known): our camera's mirror's
/// view is known by its turn (`rear_camera::ours_side`), so ours is of that side and another of the other.
fn side_of(address: usize) -> i32 {
    // Its own view's side, voted from its blocks; else told from ours.
    let own = crate::rear_camera::side_of_texture(address);
    if own != 0 {
        return own;
    }
    let ours = crate::mirror::ours_texture();
    let side = crate::rear_camera::ours_side();
    if ours == 0 || side == 0 {
        0
    } else if address == ours {
        side
    } else {
        -side
    }
}

/// Whether a virtual mirror of this side is shown, as the player chose (the phone's Settings): the right one only
/// until the app says. Not known: hidden (as all were before the choice).
fn shown(side: i32) -> bool {
    let chosen = crate::bridge::get().map_or(0, |bridge| bridge.host.virtual_mirrors.load(Ordering::Acquire));
    match chosen {
        westplay_shared::VIRTUAL_MIRRORS_NONE => false,
        westplay_shared::VIRTUAL_MIRRORS_LEFT => side < 0,
        westplay_shared::VIRTUAL_MIRRORS_BOTH => side != 0,
        _ => side > 0,
    }
}

/// The run of virtual mirrors' pictures in the final pass, in order, each one's side: this frame's so far, and the last
/// frame's. A plain quad in the run, a frame, is hidden if the picture after it was of a side hidden: by its place in
/// the run, not its texture (owner 2026-11-18: the game swaps its textures' views frame to frame, the left mirror's
/// put on the screen first all the same, and frames by the texture blinked).
static ORDER: Mutex<(Vec<i32>, Vec<i32>)> = Mutex::new((Vec::new(), Vec::new()));
/// The side of the virtual mirror's picture bound now (0: none, or not known).
static BOUND_SIDE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
/// Whether the pixel shader's first slot holds a 1 x 1 texture now (a plain colour: a virtual mirror's frame, drawn
/// just before its picture, owner's interface.txt 2026-11-14), while drawing into the final picture.
static PLAIN: AtomicBool = AtomicBool::new(false);
/// Whether the final pass's run of virtual mirrors is on: right after its full-screen draw (3 vertices), frames and
/// pictures follow one another (interface.txt); the first draw of anything else ends it, until the next pass.
static RUN: AtomicBool = AtomicBool::new(false);
/// Draws skipped (for the log, now and then).
static SKIPPED: AtomicU64 = AtomicU64::new(0);

/// `OMSetRenderTargets*` (the game's immediate context): a mirror's picture noted; whether the final one is set.
///
/// # Safety
/// The game's live context and arguments: `count` views, each live or null.
pub unsafe fn on_set_render_targets(context: *mut c_void, count: u32, views: *const *mut c_void) {
    if !HIDE {
        return;
    }
    // SAFETY: as the caller says.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return;
        }
        let mut is_final = false;
        if count > 0
            && count <= 8
            && !views.is_null()
            && let Some(view) = ID3D11RenderTargetView::from_raw_borrowed(&*views)
            && let Ok(texture) = view.GetResource().and_then(|r| r.cast::<ID3D11Texture2D>())
        {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc);
            // A mirror's picture: 256 wide, as the game lights it (R11G11B10) or toned (RGBA8 sRGB); or the camera
            // mirrors put together. Not other 256-wide targets (the interface's own, single-channel ones).
            // Any mirror resolution the game's set to (owner 2026-11-14: set higher, they weren't known).
            let toned = desc.Format == DXGI_FORMAT_R8G8B8A8_UNORM_SRGB && desc.Width == 256;
            if crate::mirror::mirror_like(&desc) || toned || crate::mirror::is_mirrors(&desc) {
                let now = FRAME.load(Ordering::Relaxed);
                let mut mirrors = MIRRORS.lock().unwrap_or_else(|e| e.into_inner());
                mirrors.insert(texture.as_raw() as usize, now);
                if mirrors.len() > 32 {
                    mirrors.retain(|_, seen| now.saturating_sub(*seen) <= MIRROR_KEPT);
                }
            }
            is_final = (desc.Format == DXGI_FORMAT_B8G8R8A8_UNORM || desc.Format == DXGI_FORMAT_B8G8R8A8_UNORM_SRGB)
                && desc.Width >= 1024;
        }
        if FINAL.swap(is_final, Ordering::Relaxed) != is_final {
            BOUND.store(0, Ordering::Relaxed);
            BOUND_HIDDEN.store(0, Ordering::Relaxed);
            PLAIN.store(false, Ordering::Relaxed);
            RUN.store(false, Ordering::Relaxed);
        }
    }
}

/// `PSSetShaderResources`, while drawing into the final picture: which slots hold a mirror's picture.
///
/// # Safety
/// The game's arguments: `count` views from `start`, each live or null.
pub unsafe fn on_ps_resources(start: u32, count: u32, views: *const *mut c_void) {
    if !HIDE || !FINAL.load(Ordering::Relaxed) || views.is_null() {
        return;
    }
    let mirrors = MIRRORS.lock().unwrap_or_else(|e| e.into_inner());
    let mut bound = BOUND.load(Ordering::Relaxed);
    let mut bound_hidden = BOUND_HIDDEN.load(Ordering::Relaxed);
    for i in 0..count.min(32) {
        let slot = start + i;
        if slot >= 32 {
            break;
        }
        // SAFETY: within the game's `count` views, each live or null (the caller's contract).
        let picture = unsafe {
            ID3D11ShaderResourceView::from_raw_borrowed(&*views.add(i as usize))
                .and_then(|view| view.GetResource().ok())
                .map(|resource| resource.as_raw() as usize)
                .filter(|address| mirrors.contains_key(address))
        };
        let is_mirror = picture.is_some();
        // A mirror-like picture (256 wide, as the game lights its mirrors) the plugin doesn't know put on the screen:
        // a virtual mirror that isn't hidden (owner 2026-11-14: they flickered into sight while reversing).
        if picture.is_none() && UNKNOWN_LOGGED.load(Ordering::Relaxed) < 10 {
            // SAFETY: as above.
            let unknown = unsafe {
                ID3D11ShaderResourceView::from_raw_borrowed(&*views.add(i as usize))
                    .and_then(|view| view.GetResource().ok())
                    .and_then(|resource| resource.cast::<ID3D11Texture2D>().ok())
                    .map(|texture| {
                        let mut desc = D3D11_TEXTURE2D_DESC::default();
                        texture.GetDesc(&mut desc);
                        desc
                    })
                    .filter(crate::mirror::mirror_like)
            };
            if let Some(desc) = unknown {
                UNKNOWN_LOGGED.fetch_add(1, Ordering::Relaxed);
                log!("retrovisor: a mirror's picture not known put on the screen ({}x{}, slot {slot}): not hidden", desc.Width, desc.Height);
            }
        }
        // A mirror's picture put on the screen: a virtual mirror's (a glass one's goes into the scene, never here).
        if let Some(address) = picture {
            VIRTUAL.lock().unwrap_or_else(|e| e.into_inner()).insert(address, FRAME.load(Ordering::Relaxed));
        }
        if slot == 0 {
            // SAFETY: as above.
            let desc = unsafe {
                ID3D11ShaderResourceView::from_raw_borrowed(&*views.add(i as usize))
                    .and_then(|view| view.GetResource().ok())
                    .and_then(|resource| resource.cast::<ID3D11Texture2D>().ok())
                    .map(|texture| {
                        let mut desc = D3D11_TEXTURE2D_DESC::default();
                        texture.GetDesc(&mut desc);
                        desc
                    })
            };
            let plain = desc.is_some_and(|desc| desc.Width == 1 && desc.Height == 1);
            PLAIN.store(plain, Ordering::Relaxed);
            // The game's test picture where a virtual mirror is while walking (the mirrors aren't drawn then):
            // 64 x 128, BC3 (owner's interface-andando-3.txt 2026-11-14).
            TEST_PICTURE.store(desc.is_some_and(|desc| desc.Width == 64 && desc.Height == 128 && desc.Format.0 == 78), Ordering::Relaxed);
        }
        if is_mirror {
            bound |= 1 << slot;
        } else {
            bound &= !(1 << slot);
        }
        if let Some(address) = picture {
            let side = side_of(address);
            BOUND_SIDE.store(side, Ordering::Relaxed);
            let hidden = !shown(side);
            let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
            if !seen.iter().any(|(a, s, h)| (*a, *s, *h) == (address, side, hidden)) && seen.len() < 16 {
                seen.push((address, side, hidden));
            }
        }
        if picture.is_some_and(|address| !shown(side_of(address))) {
            bound_hidden |= 1 << slot;
        } else {
            bound_hidden &= !(1 << slot);
        }
        if PROBING.load(Ordering::Relaxed) && slot < 8 {
            // SAFETY: as above.
            let what = unsafe {
                ID3D11ShaderResourceView::from_raw_borrowed(&*views.add(i as usize))
                    .and_then(|view| view.GetResource().ok())
                    .and_then(|resource| resource.cast::<ID3D11Texture2D>().ok())
                    .map(|texture| {
                        let mut desc = D3D11_TEXTURE2D_DESC::default();
                        texture.GetDesc(&mut desc);
                        format!("t{slot}={}x{}f{}", desc.Width, desc.Height, desc.Format.0)
                    })
                    .unwrap_or_else(|| format!("t{slot}=-"))
            };
            SLOTS.lock().unwrap_or_else(|e| e.into_inner())[slot as usize] = what;
        }
    }
    BOUND.store(bound, Ordering::Relaxed);
    BOUND_HIDDEN.store(bound_hidden, Ordering::Relaxed);
}

/// The virtual mirrors' pictures put on the screen lately: each texture, its side, whether hidden (logged every few
/// seconds, owner 2026-11-18: which one showed wasn't the one chosen).
static SEEN: Mutex<Vec<(usize, i32, bool)>> = Mutex::new(Vec::new());

/// Unknown mirror-like pictures put on the screen, logged (at most 10).
static UNKNOWN_LOGGED: AtomicU32 = AtomicU32::new(0);

/// Whether the game's draw now is a virtual mirror's (into the final picture, a mirror's picture bound): skipped.
/// Never one of the plugin's own (drawn from inside its hooks).
pub fn skip_draw(kind: &str, count: u32, start: u32) -> bool {
    if PROBING.load(Ordering::Relaxed) && FINAL.load(Ordering::Relaxed) && !crate::hooks::in_westplay() {
        let slots = SLOTS.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mirror = BOUND.load(Ordering::Relaxed) != 0;
        let mut text = PROBE.lock().unwrap_or_else(|e| e.into_inner());
        if text.len() < 200_000 {
            use std::fmt::Write as _;
            let _ = writeln!(text, "{kind} {count} from {start}{} | {}", if mirror { " ESPELHO" } else { "" }, slots.join(" "));
        }
    }
    // Nothing hidden with the game paused (its menu): no virtual mirrors are shown there, and its own pictures could
    // be taken for them (owner 2026-11-14: the menu's icons went missing in reverse).
    if !HIDE || !FINAL.load(Ordering::Relaxed) || crate::hooks::in_westplay() || crate::telemetry::paused() {
        return false;
    }
    // A mirror's picture, or its frame: a plain quad (6 indices, a 1 x 1 texture) drawn just before a mirror's
    // picture (owner's interface.txt). Only those: the menus' and messages' panels are plain quads too, drawn
    // into the final picture as well (owner 2026-11-14: hiding every plain quad took them all away). Which draws
    // of the final pass were frames is known from the frame before (the order is the same frame to frame).
    let ordinal = ORDINAL.fetch_add(1, Ordering::Relaxed);
    let plain = kind == "DrawIndexed" && count == 6 && PLAIN.load(Ordering::Relaxed);
    // Walking, the game's test picture in a virtual mirror's place is one (owner 2026-11-14: "far" shown walking).
    let walking = crate::character::walking();
    // A virtual mirror's picture (`any_mirror`), and one to be hidden (`mirror`: not of a side chosen to be seen).
    let test_picture = walking && TEST_PICTURE.load(Ordering::Relaxed) && kind == "DrawIndexed" && count == 6;
    let any_mirror = BOUND.load(Ordering::Relaxed) != 0 || test_picture;
    let mirror = BOUND_HIDDEN.load(Ordering::Relaxed) != 0 || test_picture;
    if any_mirror {
        LAST_VIRTUAL.store(FRAME.load(Ordering::Relaxed).max(1), Ordering::Relaxed);
        DRAWN_NOW.fetch_add(1, Ordering::Relaxed);
    }
    // Which picture of the run this is (the frames before it belong to it), and whether the last frame's was hidden.
    let hidden_last = {
        let mut order = ORDER.lock().unwrap_or_else(|e| e.into_inner());
        let side_last = order.1.get(order.0.len()).copied().unwrap_or(0);
        if any_mirror && order.0.len() < 8 {
            let side = if BOUND.load(Ordering::Relaxed) != 0 { BOUND_SIDE.load(Ordering::Relaxed) } else { 0 };
            order.0.push(side);
        }
        !shown(side_last)
    };
    {
        let mut frames = FRAMES.lock().unwrap_or_else(|e| e.into_inner());
        // Its place among the frame's plain quads alone: another of the interface's draws now and then (an icon
        // coming and going) put the frames a place later in the final pass, at 7 and 9 for 6 and 8, and they
        // flickered (owner's log 2026-11-14); among the plain quads they stay where they were.
        let plain_index = plain.then(|| {
            frames.plain_count += 1;
            frames.plain_count - 1
        });
        if mirror && frames.last_plain == Some(ordinal.wrapping_sub(1)) {
            frames.next.push(ordinal - 1);
            if let Some(index) = frames.last_plain_index {
                frames.plain_next.push(index);
                // A frame there after all: no longer taken for something else.
                frames.not_frames.retain(|(place, _)| *place != (ordinal - 1, index));
            }
        }
        // A plain quad hidden as a frame, and no mirror's picture after it: it wasn't one (a message's icon, the
        // game's messages put in among the draws: owner 2026-11-14, the parking brake message without its key's
        // icon). Its place isn't taken for a frame for two seconds: gone a frame, it's back.
        let now = FRAME.load(Ordering::Relaxed);
        if let Some(place) = frames.hidden_plain.take()
            && !mirror
        {
            let new = !frames.not_frames.iter().any(|(seen, _)| *seen == place);
            frames.not_frames.retain(|(seen, _)| *seen != place);
            frames.not_frames.push((place, now));
            if new && NOT_FRAMES_LOGGED.fetch_add(1, Ordering::Relaxed) < 20 {
                log!("retrovisor: a plain quad hidden as a frame wasn't one (draw {}, plain quad {}): shown again", place.0, place.1);
            }
        }
        frames.not_frames.retain(|(_, seen)| now.saturating_sub(*seen) <= 120);
        let not_frame = plain_index.is_some_and(|index| frames.not_frames.iter().any(|(place, _)| *place == (ordinal, index)));
        frames.last_plain = plain.then_some(ordinal);
        frames.last_plain_index = plain_index;
        // Frames at the places of the last 30 frames' (the order can change for a frame, a mirror coming or going:
        // one frame let through flickered, owner 2026-11-14), only while virtual mirrors are drawn (not the menus).
        let known = frames.recent.iter().any(|seen| seen.contains(&ordinal))
            || plain_index.is_some_and(|index| frames.plain_recent.iter().any(|seen| seen.contains(&index)));
        // The run: a frame's place can change for a frame (a flicker of the frame's shadow went on, owner
        // 2026-11-14): any plain quad right after the full-screen draw, among the mirrors, is a frame.
        let in_run = RUN.load(Ordering::Relaxed);
        if kind == "Draw" && count == 3 {
            RUN.store(virtual_count() > 0 || walking, Ordering::Relaxed);
        } else if in_run && !plain && !any_mirror {
            RUN.store(false, Ordering::Relaxed);
        }
        // Walking, only the run's (right after the full-screen draw: the test pictures and their frames come first),
        // the places known from the cab no longer holding.
        let frames_on = virtual_count() > 0;
        // In the run, a frame only before a picture hidden last frame (owner 2026-11-18: one virtual mirror shown,
        // its frame with it).
        let hide = mirror
            || (plain
                && !not_frame
                && ((known && frames_on && !walking) || (in_run && (walking || (frames_on && hidden_last)))));
        if hide && !mirror && let Some(index) = plain_index {
            frames.hidden_plain = Some((ordinal, index));
        }
        // A frame let through (owner 2026-11-14: their black frames still flicker now and then): a plain quad shown
        // and a mirror's picture right after it. Logged with what was known of it when it was let through.
        let mut let_through = LET_THROUGH.lock().unwrap_or_else(|e| e.into_inner());
        if mirror
            && let Some((at, what)) = let_through.take()
            && at == ordinal.wrapping_sub(1)
            && LET_THROUGH_LOGGED.fetch_add(1, Ordering::Relaxed) < 200
        {
            log!("retrovisor: a virtual mirror's frame let through: {what}");
        }
        if plain && !hide {
            *let_through = Some((
                ordinal,
                format!(
                    "draw {ordinal} of the final pass, after the full-screen one by {}, run {in_run}, {} virtual mirrors counted, frames known at {:?}",
                    ordinal.wrapping_sub(LAST_FULL_SCREEN.load(Ordering::Relaxed)),
                    virtual_count(),
                    frames.recent.back()
                ),
            ));
        }
        drop(let_through);
        if kind == "Draw" && count == 3 {
            LAST_FULL_SCREEN.store(ordinal, Ordering::Relaxed);
        }
        if !hide {
            return false;
        }
    }
    let skipped = SKIPPED.fetch_add(1, Ordering::Relaxed) + 1;
    if skipped == 1 || skipped.is_multiple_of(10_000) {
        log!("retrovisor: the virtual mirrors hidden ({skipped} draws left out so far)");
    }
    true
}

/// The final pass's draws counted this frame; the frames found (plain quads just before a mirror's picture), as
/// their places in that count, this frame and the one before (skipped this frame).
static ORDINAL: AtomicU64 = AtomicU64::new(0);
/// The place of this frame's full-screen draw in the final pass, and the plain quads let through logged (at most 200).
static LAST_FULL_SCREEN: AtomicU64 = AtomicU64::new(0);
static LET_THROUGH_LOGGED: AtomicU32 = AtomicU32::new(0);
/// Whether the pixel shader's first slot holds the game's test picture now (`on_ps_resources`).
static TEST_PICTURE: AtomicBool = AtomicBool::new(false);
static NOT_FRAMES_LOGGED: AtomicU32 = AtomicU32::new(0);
/// The last plain quad shown (its place, and what was known of it): a frame let through if a picture follows.
static LET_THROUGH: Mutex<Option<(u64, String)>> = Mutex::new(None);
/// Frames (counted at each present) and the last one a virtual mirror's picture was drawn in (`virtual_drawn`).
static FRAME: AtomicU64 = AtomicU64::new(0);
static LAST_VIRTUAL: AtomicU64 = AtomicU64::new(0);

/// The virtual mirrors drawn this frame, and the most of the last few frames' (F2 turns on one, then both).
static DRAWN_NOW: AtomicU64 = AtomicU64::new(0);
static DRAWN_RECENT: Mutex<[u64; 10]> = Mutex::new([0; 10]);

/// How many virtual mirrors the game draws now (0, 1 or 2), hidden or not.
pub fn virtual_count() -> u64 {
    DRAWN_RECENT.lock().unwrap_or_else(|e| e.into_inner()).iter().copied().max().unwrap_or(0)
}

struct Frames {
    now: Vec<u64>,
    /// The last 30 frames' frames' places.
    recent: std::collections::VecDeque<Vec<u64>>,
    next: Vec<u64>,
    last_plain: Option<u64>,
    /// The same, as places among the frame's plain quads (this frame's count so far).
    plain_count: u64,
    plain_recent: std::collections::VecDeque<Vec<u64>>,
    plain_next: Vec<u64>,
    last_plain_index: Option<u64>,
    /// The plain quad just hidden as a frame (its place and plain index), and places found not to be frames (with
    /// the frame they were found in).
    hidden_plain: Option<(u64, u64)>,
    not_frames: Vec<((u64, u64), u64)>,
}
static FRAMES: Mutex<Frames> =
    Mutex::new(Frames {
        now: Vec::new(),
        recent: std::collections::VecDeque::new(),
        next: Vec::new(),
        last_plain: None,
        plain_count: 0,
        plain_recent: std::collections::VecDeque::new(),
        plain_next: Vec::new(),
        last_plain_index: None,
        hidden_plain: None,
        not_frames: Vec::new(),
    });

/// The virtual mirrors' pictures (their textures' addresses), each with the frame it was last put on the screen in.
static VIRTUAL: LazyLock<Mutex<std::collections::HashMap<usize, u64>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Whether a mirror's picture is a virtual mirror's (put on the screen in the last two seconds): our camera is made
/// from one of those, hidden, and the glass mirrors in the cab are left as they are (owner 2026-11-14: "in reverse I
/// can't look in my mirror, it's the camera").
pub fn is_virtual(address: usize) -> bool {
    let now = FRAME.load(Ordering::Relaxed);
    VIRTUAL.lock().unwrap_or_else(|e| e.into_inner()).get(&address).is_some_and(|seen| now.saturating_sub(*seen) <= 120)
}

/// A probe of the interface's draws (the virtual mirrors' frames, left after their pictures were hidden, owner
/// 2026-11-14): one frame's draws into the final picture, each with what its pixel shader has bound, written to
/// `logs\retrovisor\interface.txt`, once a game, after the mirrors were hidden a while.
static PROBING: AtomicBool = AtomicBool::new(false);
/// Whether the probes of the interface's draws run (off from 2026-11-18: each file written is a hitch, owner).
const INTERFACE_PROBE: bool = false;
static PROBED: AtomicBool = AtomicBool::new(false);
static PROBE: Mutex<String> = Mutex::new(String::new());
static SLOTS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(vec![String::new(); 8]));

/// At each present: the probe armed for one frame, then written.
pub fn on_present() {
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    if frame.is_multiple_of(300) {
        let seen = std::mem::take(&mut *SEEN.lock().unwrap_or_else(|e| e.into_inner()));
        if !seen.is_empty() {
            let chosen = crate::bridge::get().map_or(0, |bridge| bridge.host.virtual_mirrors.load(Ordering::Acquire));
            let what: Vec<String> = seen
                .iter()
                .map(|(address, side, hidden)| {
                    let side = match side {
                        -1 => "left",
                        1 => "right",
                        _ => "?",
                    };
                    format!("{address:#x} {side} {}", if *hidden { "hidden" } else { "shown" })
                })
                .collect();
            log!(
                "retrovisor: virtual mirrors (chosen {chosen}, ours {:#x}): {}",
                crate::mirror::ours_texture(),
                what.join(", ")
            );
        }
    }
    DRAWN_RECENT.lock().unwrap_or_else(|e| e.into_inner())[(frame % 10) as usize] = DRAWN_NOW.swap(0, Ordering::Relaxed);
    ORDINAL.store(0, Ordering::Relaxed);
    {
        let mut order = ORDER.lock().unwrap_or_else(|e| e.into_inner());
        order.1 = std::mem::take(&mut order.0);
    }
    {
        let mut frames = FRAMES.lock().unwrap_or_else(|e| e.into_inner());
        frames.now = std::mem::take(&mut frames.next);
        let latest = frames.now.clone();
        frames.recent.push_back(latest);
        let plain_latest = std::mem::take(&mut frames.plain_next);
        frames.plain_recent.push_back(plain_latest);
        if frames.plain_recent.len() > 30 {
            frames.plain_recent.pop_front();
        }
        frames.plain_count = 0;
        frames.last_plain_index = None;
        if frames.recent.len() > 30 {
            frames.recent.pop_front();
        }
        frames.last_plain = None;
    }
    if PROBING.swap(false, Ordering::Relaxed) {
        let text = std::mem::take(&mut *PROBE.lock().unwrap_or_else(|e| e.into_inner()));
        let dir = crate::log::log_dir().join("retrovisor");
        let _ = std::fs::create_dir_all(&dir);
        let name = if WALK_PROBING.swap(false, Ordering::Relaxed) {
            format!("interface-andando-{}.txt", WALK_FILES.fetch_add(1, Ordering::Relaxed) + 1)
        } else {
            "interface.txt".to_owned()
        };
        match std::fs::write(dir.join(&name), text) {
            Ok(()) => log!("retrovisor: the interface's draws written ({name})"),
            Err(e) => log!("retrovisor: the interface's draws not written: {e}"),
        }
        return;
    }
    if !INTERFACE_PROBE {
        return;
    }
    // Once a game, a second and a half into walking too: the game's test picture where a virtual mirror is (owner
    // 2026-11-14: shown walking; the mirrors aren't drawn then, so it isn't known as one).
    if crate::character::walking() {
        // At 1.5, 10 and 30 seconds of a walk, three files at most a game (the first, at 1.5 s, caught none: the
        // mirrors were off then, owner 2026-11-14).
        let walked = WALKING_FOR.fetch_add(1, Ordering::Relaxed);
        if [90, 600, 1800].contains(&walked) && WALK_FILES.load(Ordering::Relaxed) < 3 {
            WALK_PROBING.store(true, Ordering::Relaxed);
            PROBING.store(true, Ordering::Relaxed);
        }
    } else {
        WALKING_FOR.store(0, Ordering::Relaxed);
    }
    if !PROBED.load(Ordering::Relaxed) && SKIPPED.load(Ordering::Relaxed) >= 600 {
        PROBED.store(true, Ordering::Relaxed);
        PROBING.store(true, Ordering::Relaxed);
    }
}

/// The walking probe: frames walked, whether it was made this game, whether the one now is it.
static WALKING_FOR: AtomicU64 = AtomicU64::new(0);
static WALK_FILES: AtomicU32 = AtomicU32::new(0);
static WALK_PROBING: AtomicBool = AtomicBool::new(false);
