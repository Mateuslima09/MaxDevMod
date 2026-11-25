//! The reversing camera through the game's own rear-view mirrors (owner 2026-11-13: the camera drawn from the map
//! is empty on protected maps, "use the mirror, it's already a camera looking back"; docs/camera-de-re/RETROVISOR.md).
//!
//! The game draws its mirrors' views, every frame the camera is in the cab, into one render target of their own:
//! 512 x 1024, RGBA8 sRGB with a full mip chain (found with the probe, `mirror_probe`, in the owner's captures of
//! 2026-11-13: the first such target of a frame holds the two main mirrors side by side in its top two thirds,
//! left and right as the driver sees them, the wide-angle ones under them; the second is the same upside down).
//!
//! As the pass drawing it ends (the game setting its next targets), it's copied ([`on_set_render_targets`]) while
//! the page wants it. At each present, the screen's frame is composed over it ([`on_present`]): where the page
//! says (`HostBlock::mirror_rect`, the reversing camera's picture, left see-through there), the two main mirrors,
//! fitted and centred on black, and the app's frame over them (its badge and parking sensors' panel). The screens
//! (`screens::refresh`) then show that in place of the frame alone. The telemetry tells the page whether the mirrors
//! were drawn lately ([`seen`]): it shows the map's picture otherwise (the camera outside the cab).

use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

use westplay_shared::{SCREEN_HEIGHT, SCREEN_WIDTH};
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D_SRV_DIMENSION_TEXTURE2D, ID3DBlob};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BLEND_DESC,
    D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD, D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL,
    D3D11_CREATE_DEVICE_SINGLETHREADED, D3D11_CULL_NONE, D3D11_DEVICE_CONTEXT_IMMEDIATE, D3D11_FILL_SOLID,
    D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
    D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RENDER_TARGET_VIEW_DESC_0, D3D11_RTV_DIMENSION_TEXTURE2D, D3D11_SAMPLER_DESC,
    D3D11_SDK_VERSION, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_RTV,
    D3D11_TEX2D_SRV, D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT,
    ID3D11BlendState, ID3D11Buffer, ID3D11Device, ID3D11Device1, ID3D11DeviceContext, ID3D11DeviceContext1,
    ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11SamplerState, ID3D11ShaderResourceView,
    ID3D11Texture2D, ID3D11VertexShader, ID3DDeviceContextState,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_TYPELESS, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_FORMAT_R11G11B10_FLOAT, DXGI_FORMAT_R16G16B16A16_FLOAT,
    DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::{Interface, PCSTR, s};

use crate::{bridge, log};

/// The mirrors' target: its size, mips and format.
const MIRROR_WIDTH: u32 = 512;
const MIRROR_HEIGHT: u32 = 1024;
const MIRROR_MIPS: u32 = 11;
/// The part of its height the two main mirrors take, from its top (the wide-angle ones are under them, from 0.67).
const MAIN_MIRRORS: f32 = 0.665;
/// How much wider than they are the mirrors are shown (they're tall and narrow; the screen is wide).
const STRETCH: f32 = 1.3;
/// The mirrors count as drawn lately within this many game frames (half a second at 60 fps).
const LATELY_FRAMES: u64 = 30;
/// A composed screen is used this many frames after it was made, at most (a present skipped).
const COMPOSED_FOR_FRAMES: u64 = 3;

/// The game frame (counted at each present).
static FRAME: AtomicU64 = AtomicU64::new(0);
/// The frame our mirror last changed at, and how long the same before the camera is shown (owner 2026-11-19: going
/// into reverse the camera flicked from mirror to mirror a second before it held: the page covers it till then).
static OURS_CHANGED_AT: AtomicU64 = AtomicU64::new(0);
const OURS_STEADY: u64 = 30;

/// Whether our mirror has stayed the same a while (`OURS_STEADY` frames).
pub fn ours_steady() -> bool {
    FRAME.load(Ordering::Relaxed).saturating_sub(OURS_CHANGED_AT.load(Ordering::Relaxed)) >= OURS_STEADY
}
/// The frame the mirrors' target was last seen drawn in (0: never).
static SEEN: AtomicU64 = AtomicU64::new(0);
/// Whether our mirror's target is the one drawn to now (`rear_camera` moves its camera then).
static DRAWING_OURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// How many main mirrors (portrait R11G11B10 targets) the last frame drew: ours is the last of them.
static LAST_PORTRAITS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Whether this frame's mirror is our camera's (every other frame while our camera is on): the game uses one
/// picture for the glass mirror in the cab and the virtual one (owner 2026-11-14: in reverse the glass mirror showed
/// the camera), so the frames alternate: one the game's own view (kept, `normal`), the next our camera's (copied
/// for the screen), and the glass mirror given back the game's view before it's drawn (`on_ps_resources`).
static CAMERA_FRAME: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Set when a `CAMERA_FRAME` landed on the other mirror's view (`!view_matches`): slipping one frame realigns
/// `CAMERA_FRAME` with a game that alternates left/right mirror views every frame (owner 2026-11-22).
static MISSED_CAMERA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SLIPPED_LAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Our mirror's texture while it holds our camera's view, to be given back the game's (0: none).
static RESTORE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// Given back and not, since the last log.
/// And our camera's pictures not shown (of the other mirror).
static RESTORES: Mutex<[u64; 3]> = Mutex::new([0; 3]);

/// The mirrors' high quality (`on_set_render_targets`): whether a main mirror's steps are being drawn, whether
/// they're ours, and how many main mirrors' steps this frame has had. Ours: the first's.
static IN_GROUP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static GROUP_OURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static GROUPS_NOW: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Which high-quality mirror group (0 or 1) is the driver's side mirror for our reversing camera; flipped when
/// `rear_camera::ours_wrong_side()` reports it held the passenger-side view (owner 2026-11-22).
static OUR_GROUP: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Whether the last frame drew its mirrors with the high quality; the widest main mirror of the last frame, and of
/// this one so far.
static HIGH_QUALITY_LAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static MAX_PORTRAIT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static MAX_NOW: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Whether the mirrors are drawn with the high quality this frame (a main mirror's steps seen).
pub fn high_quality() -> bool {
    GROUPS_NOW.load(Ordering::Relaxed) > 0
}

/// Whether the frames alternate now (our camera on).
fn alternating() -> bool {
    crate::rear_camera::active()
}

/// Which mirror texture is ours (its address; 0: none yet).
static OURS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Our mirror's texture (its address; 0: none yet): which virtual mirror is which side is told from it
/// (`hide_virtual`).
pub fn ours_texture() -> usize {
    OURS.load(Ordering::Relaxed)
}

/// The main mirror being drawn now, ours or not (its texture's address, or a small group tag `1..` during a
/// high-quality mirror's G-buffer pass before its final target is bound; 0: none): each one's side is voted from its
/// own blocks (`rear_camera::side_of_texture`).
static MAIN_PASS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// Group tag (`index + 1`) of the high-quality mirror pass currently in flight (`IN_GROUP`), moved onto the final
/// R11G11B10 texture's address when it binds.
static GROUP_TAG: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn main_pass() -> usize {
    MAIN_PASS.load(Ordering::Relaxed)
}

/// Whether our mirror is being drawn now.
pub fn drawing_ours() -> bool {
    DRAWING_OURS.load(Ordering::Relaxed)
}

/// Whether the game drew its mirrors lately (for the telemetry: the page shows them then).
pub fn seen() -> bool {
    let seen = SEEN.load(Ordering::Relaxed);
    seen != 0 && FRAME.load(Ordering::Relaxed).saturating_sub(seen) <= LATELY_FRAMES
}

/// The page's picture's shape (wide over tall), while it shows the mirrors: our camera's lens fits it.
pub fn shape() -> Option<f32> {
    wanted().map(|[_, _, width, height]| width as f32 / height.max(1) as f32)
}

/// Whether the page shows the mirrors' picture now (the reversing camera on the screen).
pub fn showing() -> bool {
    wanted().is_some()
}

/// Where the page wants the mirrors on the screen's frame (left, top, width, height in its pixels), if it does.
fn wanted() -> Option<[u32; 4]> {
    let bridge = bridge::get()?;
    if !bridge::host_alive() {
        return None;
    }
    let rect: [u32; 4] = std::array::from_fn(|i| bridge.host.mirror_rect[i].load(Ordering::Acquire));
    (rect[2] >= 8 && rect[3] >= 8).then_some(rect)
}

#[derive(Default)]
struct State {
    /// The mirrors' target bound now (it's this frame's first): copied when its pass ends.
    bound: Option<ID3D11Texture2D>,
    /// The main mirrors set this frame: the frame, how many.
    portraits: (u64, u32),
    /// The main mirrors (their textures' addresses) set this frame.
    portraits_now: (u64, Vec<usize>),
    /// How often each main mirror is drawn (each frame drawn adds 1, all fade): ours is the steadiest.
    scores: std::collections::HashMap<usize, f32>,
    /// Ours, locked on (0: none yet: the frame's last), and the frame it was last drawn in.
    ours: usize,
    ours_seen: u64,
    /// The mirrors (R11G11B10 targets 256 wide) set this frame, as their sizes, and as last logged.
    mirrors_now: (u64, String),
    mirrors_logged: String,
    mirrors_logged_at: u64,
    /// Our copy of the mirrors, and its view.
    copy: Option<(ID3D11Texture2D, ID3D11ShaderResourceView)>,
    /// The game's own view of our mirror, kept from the last frame that wasn't our camera's.
    normal: Option<ID3D11Texture2D>,
    /// Our mirror, while it holds our camera's view (given back the game's when it's first used).
    restore: Option<ID3D11Texture2D>,
    /// The latest of the app's screen frames (`screens::refresh` passes it on).
    app_frame: Option<ID3D11Texture2D>,
    /// What composing is done with, made on the first present it's wanted; `Err` once that failed.
    drawing: Option<Result<Drawing, ()>>,
    /// The screen composed: its texture, the frame it was made in, how many have been made.
    composed: Option<ID3D11Texture2D>,
    composed_frame: u64,
    composed_count: u64,
}

struct Shared(State);
// SAFETY: used only on the game's render thread (its hooks and present), through the mutex.
unsafe impl Send for Shared {}

static STATE: LazyLock<Mutex<Shared>> = LazyLock::new(|| Mutex::new(Shared(State::default())));

fn state() -> std::sync::MutexGuard<'static, Shared> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether a target is one of the game's mirrors: its own picture, lit (R11G11B10), 128 to 2048 wide (the game's
/// mirror resolution: 256 by default, 512 and more set higher, owner 2026-11-14: set higher, the camera broke), square
/// or twice as tall or as wide. Not RGBA16F: with the mirrors' quality set high the game draws them in steps into
/// such targets first (its objects at half size, four targets at once, then their light), then into one of these;
/// and the game's menu draws into RGBA16F targets too (taken for mirrors, its icons were hidden, owner 2026-11-14).
pub(crate) fn mirror_like(desc: &D3D11_TEXTURE2D_DESC) -> bool {
    desc.Format == DXGI_FORMAT_R11G11B10_FLOAT
        && (128..=2048).contains(&desc.Width)
        && desc.ArraySize == 1
        && desc.SampleDesc.Count == 1
        && (desc.Height == desc.Width * 2 || desc.Height == desc.Width || desc.Height * 2 == desc.Width)
}

pub(crate) fn is_mirrors(desc: &D3D11_TEXTURE2D_DESC) -> bool {
    desc.Width == MIRROR_WIDTH
        && desc.Height == MIRROR_HEIGHT
        && desc.MipLevels == MIRROR_MIPS
        && desc.ArraySize == 1
        && desc.Format == DXGI_FORMAT_R8G8B8A8_UNORM_SRGB
        && desc.SampleDesc.Count == 1
}

/// Called at the start of `OMSetRenderTargets*`, with the game's context: the mirrors' pass ending here is copied
/// (while the page wants them), and the mirrors' target, if it's set now, noted.
///
/// # Safety
/// From the `OMSetRenderTargets*` hooks, with the game's arguments, on its render thread.
pub unsafe fn on_set_render_targets(context: *mut c_void, count: u32, views: *const *mut c_void) {
    // SAFETY: the game's live context and its arguments (the caller's contract): `count` views (checked for a sane
    // number), each a live view or null; the textures copied are the game's, held, and ours.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return;
        }
        DRAWING_OURS.store(false, Ordering::Relaxed);
        MAIN_PASS.store(0, Ordering::Relaxed);
        crate::rear_camera::new_pass();
        let mut state = state();
        let state = &mut state.0;
        if let Some(texture) = state.bound.take()
            && wanted().is_some()
        {
            if alternating() && CAMERA_FRAME.load(Ordering::Relaxed) {
                // Shown only if it was the mirror our camera was moved for (the game hands its mirrors' views round
                // its textures): else the screen keeps the last one. Never copy an unmoved mirror before
                // `rear_camera` turns on (owner 2026-11-22: showed the raw sideways side mirror frozen on screen).
                if crate::rear_camera::view_matches() {
                    copy_mirrors(context, state, &texture);
                } else {
                    MISSED_CAMERA.store(true, Ordering::Relaxed);
                    RESTORES.lock().unwrap_or_else(|e| e.into_inner())[2] += 1;
                }
                if state.normal.is_some() {
                    RESTORE.store(texture.as_raw() as usize, Ordering::Relaxed);
                    state.restore = Some(texture);
                }
            } else if alternating() {
                keep_normal(context, state, &texture);
            }
        }
        if views.is_null() || count == 0 || count > 8 {
            return;
        }
        let Some(view) = windows::Win32::Graphics::Direct3D11::ID3D11RenderTargetView::from_raw_borrowed(&*views)
        else {
            return;
        };
        let Ok(texture) = view.GetResource().and_then(|resource| resource.cast::<ID3D11Texture2D>()) else { return };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc);
        // A main mirror: drawn into a target of its own, twice as tall as wide, R11G11B10 (every truck: the DAF XG's
        // camera mirrors two 256 x 512, a Scania's glass ones one, owner 2026-11-14). Not the mirrors put together
        // (512 x 1024, the DAF's only): a glass-mirrored truck has none.
        // (And the high quality's steps, RGBA16F, logged: so it's told in the log the mirrors' quality is high.)
        let step = desc.Format == DXGI_FORMAT_R16G16B16A16_FLOAT && (32..=2048).contains(&desc.Width);
        if mirror_like(&desc) || step {
            let frame = FRAME.load(Ordering::Relaxed);
            if state.mirrors_now.0 != frame {
                state.mirrors_now = (frame, String::new());
            }
            // Its size, and its format and how many targets with it when not the default's (one R11G11B10).
            let unusual = step || count != 1;
            state.mirrors_now.1.push_str(&if unusual {
                format!(" {}x{}f{}x{count}", desc.Width, desc.Height, desc.Format.0)
            } else {
                format!(" {}x{}", desc.Width, desc.Height)
            });
        }
        // The mirrors' high quality: a main mirror drawn in steps, its objects first (half its size, RGBA16F, four
        // targets at once), then their light (two, then one), then into its own picture. Our camera is made in all
        // its steps: its objects are drawn in the first (owner 2026-11-14: with the high quality the camera showed the
        // mirror). Which main mirror's steps these are is told by their order in the frame.
        let step_portrait = step && desc.Height == desc.Width * 2 && desc.ArraySize == 1;
        let camera_now = !alternating() || CAMERA_FRAME.load(Ordering::Relaxed);
        if step_portrait && count == 4 {
            let index = GROUPS_NOW.fetch_add(1, Ordering::Relaxed);
            let tag = index as usize + 1;
            let ours = index == OUR_GROUP.load(Ordering::Relaxed);
            IN_GROUP.store(true, Ordering::Relaxed);
            GROUP_TAG.store(tag, Ordering::Relaxed);
            GROUP_OURS.store(ours, Ordering::Relaxed);
            DRAWING_OURS.store(ours && camera_now, Ordering::Relaxed);
            MAIN_PASS.store(tag, Ordering::Relaxed);
            return;
        }
        if step_portrait && IN_GROUP.load(Ordering::Relaxed) {
            DRAWING_OURS.store(GROUP_OURS.load(Ordering::Relaxed) && camera_now, Ordering::Relaxed);
            return;
        }
        let portrait = mirror_like(&desc) && desc.Height == desc.Width * 2;
        // A main mirror's own picture, after its steps (the high quality's): ours if its steps were.
        let grouped = portrait && IN_GROUP.swap(false, Ordering::Relaxed);
        if !portrait {
            IN_GROUP.store(false, Ordering::Relaxed);
            return;
        }
        // Only the main mirrors: the largest drawn, and with the high quality the ones drawn in its steps. A DAF XD
        // has two small ones (128 x 256) drawn apart, without steps, and one was taken for ours (owner 2026-11-14:
        // its turn never looked back, every picture left unshown, the screen kept an old one).
        MAX_NOW.fetch_max(desc.Width, Ordering::Relaxed);
        if !grouped && (HIGH_QUALITY_LAST.load(Ordering::Relaxed) || desc.Width < MAX_PORTRAIT.load(Ordering::Relaxed)) {
            return;
        }
        let frame = FRAME.load(Ordering::Relaxed);
        if state.portraits.0 != frame {
            state.portraits = (frame, 0);
        }
        state.portraits.1 += 1;
        let address = texture.as_raw() as usize;
        if grouped {
            let tag = GROUP_TAG.swap(0, Ordering::Relaxed);
            if tag != 0 {
                crate::rear_camera::alias_texture_votes(tag, address);
            }
        }
        MAIN_PASS.store(address, Ordering::Relaxed);
        if state.portraits_now.0 != frame {
            state.portraits_now = (frame, Vec::new());
        }
        if !state.portraits_now.1.contains(&address) {
            state.portraits_now.1.push(address);
        }
        // Ours: the one locked on (its order among the others changes frame to frame with the virtual mirrors on,
        // owner 2026-11-14: the camera flickered between two); before any, the frame's last.
        let ours = if grouped {
            GROUP_OURS.load(Ordering::Relaxed)
        } else if state.ours != 0 {
            address == state.ours
        } else {
            state.portraits.1 == LAST_PORTRAITS.load(Ordering::Relaxed).max(1)
        };
        if !ours {
            return;
        }
        state.ours_seen = frame;
        DRAWING_OURS.store(!alternating() || CAMERA_FRAME.load(Ordering::Relaxed), Ordering::Relaxed);
        {
            static TIMES: AtomicU64 = AtomicU64::new(0);
            crate::mirror_stack::on_mirror(TIMES.fetch_add(1, Ordering::Relaxed) + 1);
        }
        if SEEN.swap(frame.max(1), Ordering::Relaxed) == 0 {
            log!("retrovisor: the game's main mirror found ({}x{})", desc.Width, desc.Height);
        }
        state.bound = Some(texture);
    }
}

/// # Safety
/// The game's live immediate context and its mirrors' target, on its render thread.
unsafe fn copy_mirrors(context: &ID3D11DeviceContext, state: &mut State, texture: &ID3D11Texture2D) {
    // SAFETY: as the caller says; our copy is made on the game's own device, of the mirrors' size and format, and
    // the first mip copied whole.
    unsafe {
        let Ok(device) = context.GetDevice() else { return };
        let fresh = match &state.copy {
            Some((copy, _)) => {
                let (mut had, mut now) = (D3D11_TEXTURE2D_DESC::default(), D3D11_TEXTURE2D_DESC::default());
                copy.GetDesc(&mut had);
                texture.GetDesc(&mut now);
                copy.GetDevice().ok().as_ref() != Some(&device)
                    || (had.Width, had.Height, had.Format) != (now.Width, now.Height, now.Format)
            }
            None => true,
        };
        if fresh {
            state.copy = None;
            let mut source = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut source);
            let desc = D3D11_TEXTURE2D_DESC {
                Width: source.Width,
                Height: source.Height,
                // All its mips, made from the first each frame: the smallest is its mean colour (our exposure).
                MipLevels: 0,
                ArraySize: 1,
                Format: source.Format,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
                CPUAccessFlags: 0,
                MiscFlags: windows::Win32::Graphics::Direct3D11::D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
            };
            let mut copy = None;
            if let Err(e) = device.CreateTexture2D(&desc, None, Some(&mut copy)) {
                log!("retrovisor: copy not made: {e}");
                return;
            }
            let Some(copy) = copy else { return };
            let mut view = None;
            if let Err(e) = device.CreateShaderResourceView(&copy, None, Some(&mut view)) {
                log!("retrovisor: copy's view not made: {e}");
                return;
            }
            let Some(view) = view else { return };
            state.copy = Some((copy, view));
        }
        if let Some((copy, view)) = &state.copy {
            context.CopySubresourceRegion(copy, 0, 0, 0, 0, texture, 0, None);
            context.GenerateMips(view);
            // Now and then, what the copy holds (its middle's mean colour), to tell a black screen from a dark one.
            if FRAME.load(Ordering::Relaxed).is_multiple_of(300) {
                check_copy(context, copy);
            }
        }
    }
}

/// The game's own view of our mirror kept (a frame that isn't our camera's), to give back after our camera's.
///
/// # Safety
/// As `copy_mirrors`.
unsafe fn keep_normal(context: &ID3D11DeviceContext, state: &mut State, texture: &ID3D11Texture2D) {
    // SAFETY: as the caller says; our texture made on the game's device, of the mirror's size and format.
    unsafe {
        let Ok(device) = context.GetDevice() else { return };
        let mut now = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut now);
        let fits = state.normal.as_ref().is_some_and(|normal| {
            let mut had = D3D11_TEXTURE2D_DESC::default();
            normal.GetDesc(&mut had);
            normal.GetDevice().ok().as_ref() == Some(&device) && (had.Width, had.Height, had.Format) == (now.Width, now.Height, now.Format)
        });
        if !fits {
            state.normal = None;
            let desc = D3D11_TEXTURE2D_DESC {
                Width: now.Width,
                Height: now.Height,
                MipLevels: 1,
                ArraySize: 1,
                Format: now.Format,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            let mut normal = None;
            if let Err(e) = device.CreateTexture2D(&desc, None, Some(&mut normal)) {
                log!("retrovisor: the mirror's own view not kept: {e}");
                return;
            }
            state.normal = normal;
        }
        if let Some(normal) = &state.normal {
            context.CopySubresourceRegion(normal, 0, 0, 0, 0, texture, 0, None);
        }
    }
}

/// `PSSetShaderResources`: our mirror, holding our camera's view, about to be used (the glass mirror in the cab, the
/// virtual one): given back the game's own view first.
///
/// # Safety
/// The game's live context and arguments: `count` views, each live or null.
pub unsafe fn on_ps_resources(context: *mut c_void, count: u32, views: *const *mut c_void) {
    let target = RESTORE.load(Ordering::Relaxed);
    if target == 0 || views.is_null() {
        return;
    }
    // SAFETY: the game's views and context (the caller's contract); our textures, held in the state.
    unsafe {
        let uses = (0..count.min(32)).any(|i| {
            ID3D11ShaderResourceView::from_raw_borrowed(&*views.add(i as usize))
                .and_then(|view| view.GetResource().ok())
                .is_some_and(|resource| resource.as_raw() as usize == target)
        });
        if !uses {
            return;
        }
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        RESTORE.store(0, Ordering::Relaxed);
        let mut state = state();
        let state = &mut state.0;
        if let (Some(restore), Some(normal)) = (state.restore.take(), &state.normal) {
            context.CopySubresourceRegion(&restore, 0, 0, 0, 0, normal, 0, None);
            RESTORES.lock().unwrap_or_else(|e| e.into_inner())[0] += 1;
        }
    }
}

/// The app's latest screen frame (`screens::refresh`), for the next composing.
pub fn note_frame(frame: &ID3D11Texture2D) {
    let mut state = state();
    if state.0.app_frame.as_ref() != Some(frame) {
        state.0.app_frame = Some(frame.clone());
    }
}

/// The app's frame from the floating screen (`float_screen`), taken only if no screen gave one: in a truck whose own
/// screen isn't WestPlay's the reversing camera is composed for the floating screen alone.
pub fn note_frame_if_none(frame: &ID3D11Texture2D) {
    let mut state = state();
    if state.0.app_frame.is_none() {
        state.0.app_frame = Some(frame.clone());
    }
}

/// The screen composed with the mirrors, and a number that changes with every one made, while it's fresh.
pub fn composed() -> Option<(ID3D11Texture2D, u64)> {
    let state = state();
    let state = &state.0;
    let frame = FRAME.load(Ordering::Relaxed);
    let fresh = state.composed_count != 0 && frame.saturating_sub(state.composed_frame) <= COMPOSED_FOR_FRAMES;
    if !fresh || wanted().is_none() {
        return None;
    }
    state.composed.clone().map(|composed| (composed, state.composed_count))
}

/// At each of the game's presents: the screen composed anew while the page wants the mirrors and they're drawn.
///
/// # Safety
/// The game's live swap chain, in its `Present`, on its render thread.
pub unsafe fn on_present(swapchain: *mut c_void) {
    {
        let state = state();
        let mut state = state;
        let (counted, count) = state.0.portraits;
        let now = FRAME.load(Ordering::Relaxed);
        // The next frame our camera's if this one wasn't (and our camera's on); if this camera frame landed on the
        // wrong mirror (out of phase with a game alternating left/right every frame), slip one frame to realign.
        let missed = MISSED_CAMERA.swap(false, Ordering::Relaxed);
        let was_camera = CAMERA_FRAME.load(Ordering::Relaxed);
        let slip = missed && !SLIPPED_LAST.load(Ordering::Relaxed);
        SLIPPED_LAST.store(slip, Ordering::Relaxed);
        let camera_next = alternating() && (!was_camera || slip);
        HIGH_QUALITY_LAST.store(GROUPS_NOW.swap(0, Ordering::Relaxed) > 0, Ordering::Relaxed);
        IN_GROUP.store(false, Ordering::Relaxed);
        MAX_PORTRAIT.store(MAX_NOW.swap(0, Ordering::Relaxed), Ordering::Relaxed);
        CAMERA_FRAME.store(camera_next, Ordering::Relaxed);
        if RESTORE.swap(0, Ordering::Relaxed) != 0 {
            RESTORES.lock().unwrap_or_else(|e| e.into_inner())[1] += 1;
        }
        state.0.restore = None;
        if now.is_multiple_of(600) && alternating() {
            let mut restores = RESTORES.lock().unwrap_or_else(|e| e.into_inner());
            log!(
                "retrovisor: the glass mirror given back its own view {} times, not {}; our camera's picture not shown {} times, of the other mirror (the last 600 frames)",
                restores[0], restores[1], restores[2]
            );
            *restores = [0; 3];
        }
        LAST_PORTRAITS.store(if counted == now { count } else { 0 }, Ordering::Relaxed);
        // The main mirrors' steadiness: drawn this frame +1, every one fading. Ours the steadiest, changed only when
        // it's gone a few frames or another is drawn far more often (the virtual mirrors: drawn even zoomed in).
        let drawn_now = if state.0.portraits_now.0 == now { std::mem::take(&mut state.0.portraits_now.1) } else { Vec::new() };
        for score in state.0.scores.values_mut() {
            *score *= 0.98;
        }
        state.0.scores.retain(|_, score| *score > 0.05);
        for address in &drawn_now {
            *state.0.scores.entry(*address).or_insert(0.0) += 1.0;
        }
        // Only the virtual mirrors, when there are (hidden: the glass ones in the cab stay as they are).
        // Prefer the driver's side mirror when its side is known, so a higher score on the passenger mirror never
        // yanks `ours` onto the wrong side (owner 2026-11-22: 142 camera frames dropped until `ours` switched back).
        let any_virtual = state.0.scores.keys().any(|address| crate::hide_virtual::is_virtual(*address));
        let driver = crate::rear_camera::driver_side();
        let any_driver = state.0.scores.keys().any(|address| {
            (!any_virtual || crate::hide_virtual::is_virtual(*address))
                && crate::rear_camera::side_of_texture(*address) == driver
        });
        let any_not_wrong = state.0.scores.keys().any(|address| {
            (!any_virtual || crate::hide_virtual::is_virtual(*address))
                && crate::rear_camera::side_of_texture(*address) != -driver
        });
        let best = state.0.scores.iter()
            .filter(|(address, _)| !any_virtual || crate::hide_virtual::is_virtual(**address))
            .filter(|(address, _)| !any_driver || crate::rear_camera::side_of_texture(**address) == driver)
            .filter(|(address, _)| !any_not_wrong || crate::rear_camera::side_of_texture(**address) != -driver)
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(a, s)| (*a, *s));
        if let Some((best, best_score)) = best {
            let ours = state.0.ours;
            let ours_score = state.0.scores.get(&ours).copied().unwrap_or(0.0);
            // Gone a few frames: another one at once (a glass mirror out of view, the virtual one always drawn: a
            // second's wait made the camera flicker, owner 2026-11-14).
            let gone = now.saturating_sub(state.0.ours_seen) > 3;
            let ours_glass = any_virtual && !crate::hide_virtual::is_virtual(ours);
            let ours_wrong = any_driver && crate::rear_camera::side_of_texture(ours) == -driver;
            if best != ours && (ours == 0 || gone || ours_glass || ours_wrong || best_score > ours_score * 1.5) {
                log!("retrovisor: our mirror is now {best:#x} (drawn {best_score:.1}, the one before {ours_score:.1})");
                state.0.ours = best;
                OURS.store(best, Ordering::Relaxed);
                OURS_CHANGED_AT.store(now, Ordering::Relaxed);
                state.0.ours_seen = now;
            } else if ours != 0 && crate::rear_camera::ours_wrong_side() {
                // Ours holding the other side's view a while (the game hands the views round its textures): the
                // other main mirror drawn is ours now, the driver's side one's (owner 2026-11-18).
                if HIGH_QUALITY_LAST.load(Ordering::Relaxed) {
                    let next = OUR_GROUP.fetch_xor(1, Ordering::Relaxed) ^ 1;
                    log!("retrovisor: our high-quality mirror group is now {next} (the one before held the other side's view)");
                }
                let other = state.0.scores.iter()
                    .filter(|(address, _)| **address != ours && (!any_virtual || crate::hide_virtual::is_virtual(**address)))
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(a, _)| *a);
                if let Some(other) = other {
                    log!("retrovisor: our mirror is now {other:#x} (the one before held the other side's view)");
                    state.0.ours = other;
                    OURS.store(other, Ordering::Relaxed);
                    OURS_CHANGED_AT.store(now, Ordering::Relaxed);
                    state.0.ours_seen = now;
                }
            }
        }
        // Which mirrors the game drew, when that changes (looking about, zooming: a mirror out of view isn't drawn).
        let drawn = if state.0.mirrors_now.0 == now { state.0.mirrors_now.1.clone() } else { String::new() };
        if drawn != state.0.mirrors_logged && now.saturating_sub(state.0.mirrors_logged_at) >= 30 {
            state.0.mirrors_logged_at = now;
            log!("retrovisor: the game drew these mirrors this frame:{}", if drawn.is_empty() { " none" } else { &drawn });
            state.0.mirrors_logged = drawn;
        }
    }
    let frame = FRAME.fetch_add(1, Ordering::Relaxed) + 1;
    let Some(rect) = wanted() else { return };
    if !seen() {
        return;
    }
    // SAFETY: the game's live swap chain, on its render thread (the caller's contract).
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let Ok(context) = device.GetImmediateContext() else { return };
        let mut state = state();
        let state = &mut state.0;
        let (Some((copy, mirrors)), Some(app_frame)) = (state.copy.clone(), state.app_frame.clone()) else { return };
        if copy.GetDevice().ok().as_ref() != Some(&device) {
            return;
        }
        if matches!(&state.drawing, Some(Ok(made)) if made.device != device) {
            state.drawing = None;
        }
        if state.drawing.is_none() {
            state.drawing = Some(Drawing::new(&device).map_err(|e| log!("retrovisor: can't be composed: {e}")));
            if matches!(state.drawing, Some(Ok(_))) {
                log!("retrovisor: ready to be shown on the screen");
            }
        }
        let Some(Ok(drawing)) = state.drawing.as_mut() else { return };
        match drawing.compose(&context, &mirrors, &app_frame, rect) {
            Ok(()) => {
                state.composed = Some(drawing.target.clone());
                state.composed_frame = frame;
                if state.composed_count == 0 {
                    log!("retrovisor: shown on the screen, at {rect:?}");
                }
                state.composed_count += 1;
            }
            Err(e) => {
                log!("retrovisor: not composed: {e}");
                state.drawing = Some(Err(()));
            }
        }
    }
}

const SHADERS: &str = r"
cbuffer Place : register(b0) { float4 rect; float4 uv; float4 keep; float4 mode; };
struct Vertex { float4 position : SV_Position; float2 uv : TEXCOORD0; };
Vertex vs(uint id : SV_VertexID) {
    float2 corner = float2(id & 1, id >> 1);
    Vertex v;
    v.position = float4(lerp(rect.x, rect.z, corner.x), lerp(rect.y, rect.w, corner.y), 0, 1);
    v.uv = lerp(uv.xy, uv.zw, corner);
    return v;
}
Texture2D picture : register(t0);
SamplerState smooth : register(s0);
float4 ps(Vertex v) : SV_Target {
    if (mode.x < 0.5) return picture.Sample(smooth, v.uv);
    if (mode.x > 1.5) {
        float2 turned = float2(v.uv.y, v.uv.x);
        if (mode.y > 0.5) turned.x = 1 - turned.x;
        if (mode.z > 0.5) turned.y = 1 - turned.y;
        // The mirror's own picture, as the game lit it (R11G11B10, before its exposure: some 0.001, owner's Scania
        // 2026-11-14): exposed as a camera does, its mean (the smallest mip) brought to a mid grey, then toned.
        float3 lit = picture.SampleLevel(smooth, lerp(keep.xy, keep.zw, turned), 0).rgb;
        float mean = dot(picture.SampleLevel(smooth, float2(0.5, 0.5), 12).rgb, float3(0.2126, 0.7152, 0.0722));
        float exposure = clamp(mode.w / max(mean, 1e-6), 1, 20000);
        return float4(1 - exp(-lit * exposure), 1);
    }
    if (v.uv.x < keep.x || v.uv.y < keep.y || v.uv.x > keep.z || v.uv.y > keep.w) return float4(0, 0, 0, 1);
    return float4(picture.SampleLevel(smooth, v.uv, 0).rgb, 1);
}
";

/// What the screen is composed with: a context state of its own, the shaders, the place, blends (none for the
/// mirrors, premultiplied for the app's frame over them), and the screen it makes.
struct Drawing {
    device: ID3D11Device,
    state: ID3DDeviceContextState,
    vertex: ID3D11VertexShader,
    pixel: ID3D11PixelShader,
    place: ID3D11Buffer,
    opaque: ID3D11BlendState,
    over: ID3D11BlendState,
    rasterizer: ID3D11RasterizerState,
    sampler: ID3D11SamplerState,
    target: ID3D11Texture2D,
    target_view: ID3D11RenderTargetView,
    /// The view of the app's frame, for the frame texture it was made for.
    frame_view: Option<(ID3D11Texture2D, ID3D11ShaderResourceView)>,
}

impl Drawing {
    /// # Safety
    /// The game's live device, on its render thread.
    unsafe fn new(device: &ID3D11Device) -> windows::core::Result<Self> {
        // SAFETY: the game's live device (the caller's contract); every description is a local, the shaders' code
        // our own.
        unsafe {
            let none = windows::core::Error::empty;
            let device1: ID3D11Device1 = device.cast()?;
            let flags = if device.GetCreationFlags() & D3D11_CREATE_DEVICE_SINGLETHREADED.0 != 0 { 1 } else { 0 };
            let mut state = None;
            device1.CreateDeviceContextState(
                flags,
                &[device.GetFeatureLevel()],
                D3D11_SDK_VERSION,
                &ID3D11Device::IID,
                None,
                Some(&mut state),
            )?;
            let vertex_code = compile(s!("vs"), s!("vs_4_0"))?;
            let pixel_code = compile(s!("ps"), s!("ps_4_0"))?;
            let mut vertex = None;
            device.CreateVertexShader(blob_bytes(&vertex_code), None, Some(&mut vertex))?;
            let mut pixel = None;
            device.CreatePixelShader(blob_bytes(&pixel_code), None, Some(&mut pixel))?;
            let mut place = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: 64,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut place),
            )?;
            let blend = |enabled: bool| -> windows::core::Result<ID3D11BlendState> {
                let mut desc = D3D11_BLEND_DESC::default();
                desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: enabled.into(),
                    SrcBlend: D3D11_BLEND_ONE,
                    DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                    BlendOp: D3D11_BLEND_OP_ADD,
                    SrcBlendAlpha: D3D11_BLEND_ONE,
                    DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                    BlendOpAlpha: D3D11_BLEND_OP_ADD,
                    RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
                };
                let mut made = None;
                device.CreateBlendState(&desc, Some(&mut made))?;
                made.ok_or_else(none)
            };
            let opaque = blend(false)?;
            let over = blend(true)?;
            let mut rasterizer = None;
            device.CreateRasterizerState(
                &D3D11_RASTERIZER_DESC {
                    FillMode: D3D11_FILL_SOLID,
                    CullMode: D3D11_CULL_NONE,
                    DepthClipEnable: true.into(),
                    ..Default::default()
                },
                Some(&mut rasterizer),
            )?;
            let mut sampler = None;
            device.CreateSamplerState(
                &D3D11_SAMPLER_DESC {
                    Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                    AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                    MaxLOD: f32::MAX,
                    ..Default::default()
                },
                Some(&mut sampler),
            )?;
            // The screen made: the app frame's size and format family (it's copied into the screens as a frame is).
            let mut target = None;
            device.CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: SCREEN_WIDTH,
                    Height: SCREEN_HEIGHT,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_TYPELESS,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                    CPUAccessFlags: 0,
                    MiscFlags: 0,
                },
                None,
                Some(&mut target),
            )?;
            let target = target.ok_or_else(none)?;
            let mut target_view = None;
            device.CreateRenderTargetView(
                &target,
                Some(&D3D11_RENDER_TARGET_VIEW_DESC {
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
                    ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_RENDER_TARGET_VIEW_DESC_0 { Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 } },
                }),
                Some(&mut target_view),
            )?;
            Ok(Self {
                device: device.clone(),
                state: state.ok_or_else(none)?,
                vertex: vertex.ok_or_else(none)?,
                pixel: pixel.ok_or_else(none)?,
                place: place.ok_or_else(none)?,
                opaque,
                over,
                rasterizer: rasterizer.ok_or_else(none)?,
                sampler: sampler.ok_or_else(none)?,
                target,
                target_view: target_view.ok_or_else(none)?,
                frame_view: None,
            })
        }
    }

    /// The screen made anew: transparent, the mirrors in `rect` (fitted and centred on black), the app's frame over.
    ///
    /// # Safety
    /// The game's live immediate context, of this drawing's device, on its render thread.
    unsafe fn compose(
        &mut self,
        context: &ID3D11DeviceContext,
        mirrors: &ID3D11ShaderResourceView,
        app_frame: &ID3D11Texture2D,
        rect: [u32; 4],
    ) -> windows::core::Result<()> {
        // SAFETY: as the caller says; the game's context state is swapped back as it was before this returns.
        unsafe {
            if !matches!(&self.frame_view, Some((texture, _)) if texture == app_frame) {
                let mut view = None;
                self.device.CreateShaderResourceView(
                    app_frame,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
                        ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
                        },
                    }),
                    Some(&mut view),
                )?;
                self.frame_view = Some((app_frame.clone(), view.ok_or_else(windows::core::Error::empty)?));
            }
            let Some((_, frame_view)) = self.frame_view.clone() else { return Ok(()) };
            let context1: ID3D11DeviceContext1 = context.cast()?;
            let mut game = None;
            context1.SwapDeviceContextState(&self.state, Some(&mut game));
            context.ClearRenderTargetView(&self.target_view, &[0.0, 0.0, 0.0, 0.0]);
            context.OMSetRenderTargets(Some(&[Some(self.target_view.clone())]), None);
            context.RSSetState(&self.rasterizer);
            context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: SCREEN_WIDTH as f32,
                Height: SCREEN_HEIGHT as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
            context.IASetInputLayout(None);
            context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            context.VSSetShader(&self.vertex, None);
            context.VSSetConstantBuffers(0, Some(&[Some(self.place.clone())]));
            context.PSSetShader(&self.pixel, None);
            context.PSSetConstantBuffers(0, Some(&[Some(self.place.clone())]));
            context.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            // The mirrors in the page's rectangle.
            let place = if crate::rear_camera::active() { ours_place(rect) } else { mirrors_place(rect) };
            context.UpdateSubresource(&self.place, 0, None, place.as_ptr().cast(), 0, 0);
            context.OMSetBlendState(&self.opaque, None, 0xffff_ffff);
            context.PSSetShaderResources(0, Some(&[Some(mirrors.clone())]));
            context.Draw(4, 0);
            // The app's frame over the whole screen.
            let whole: [f32; 16] = [-1.0, 1.0, 1.0, -1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0];
            context.UpdateSubresource(&self.place, 0, None, whole.as_ptr().cast(), 0, 0);
            context.OMSetBlendState(&self.over, None, 0xffff_ffff);
            context.PSSetShaderResources(0, Some(&[Some(frame_view)]));
            context.Draw(4, 0);
            context.OMSetRenderTargets(Some(&[None]), None);
            context.PSSetShaderResources(0, Some(&[None]));
            if let Some(game) = &game {
                context1.SwapDeviceContextState(game, None);
            }
            Ok(())
        }
    }
}

/// The mirrors' quad over the page's rectangle (frame pixels: left, top, width, height): its place in clip space,
/// the mirrors' coordinates at its corners (the two main mirrors fitted, a little wider than they are, and centred),
/// the part of them kept (black round it), and drawn as the mirrors.
fn mirrors_place([x, y, width, height]: [u32; 4]) -> [f32; 16] {
    let (x, y, w, h) = (x as f32, y as f32, width as f32, height as f32);
    let (sw, sh) = (SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
    // The two main mirrors' shape as shown: wide over tall.
    let aspect = MIRROR_WIDTH as f32 / (MIRROR_HEIGHT as f32 * MAIN_MIRRORS) * STRETCH;
    let shown_h = h.min(w / aspect);
    let shown_w = shown_h * aspect;
    let (left, top) = (x + (w - shown_w) / 2.0, y + (h - shown_h) / 2.0);
    let u = |px: f32| (px - left) / shown_w;
    let v = |py: f32| (py - top) / shown_h * MAIN_MIRRORS;
    [
        x / sw * 2.0 - 1.0,
        1.0 - y / sh * 2.0,
        (x + w) / sw * 2.0 - 1.0,
        1.0 - (y + h) / sh * 2.0,
        u(x),
        v(y),
        u(x + w),
        v(y + h),
        0.0,
        0.0,
        1.0,
        MAIN_MIRRORS,
        1.0,
        0.0,
        0.0,
        0.0,
    ]
}

/// Our camera (`rear_camera`) over the whole of the page's rectangle: its part of the mirrors' picture, lying on
/// its side there, turned back.
fn ours_place([x, y, width, height]: [u32; 4]) -> [f32; 16] {
    let (x, y, w, h) = (x as f32, y as f32, width as f32, height as f32);
    let (sw, sh) = (SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
    let [u0, v0, u1, v1] = crate::rear_camera::PLACE;
    let [flip_x, flip_y] = crate::rear_camera::FLIP.map(|f| if f { 1.0 } else { 0.0 });
    [
        x / sw * 2.0 - 1.0,
        1.0 - y / sh * 2.0,
        (x + w) / sw * 2.0 - 1.0,
        1.0 - (y + h) / sh * 2.0,
        0.0,
        0.0,
        1.0,
        1.0,
        u0,
        v0,
        u1,
        v1,
        2.0,
        flip_x,
        flip_y,
        crate::rear_camera::EXPOSURE,
    ]
}

fn compile(entry: PCSTR, target: PCSTR) -> windows::core::Result<ID3DBlob> {
    let mut code = None;
    let mut errors = None;
    // SAFETY: the source is a static string of the length given; the blobs out are locals.
    let compiled = unsafe {
        D3DCompile(
            SHADERS.as_ptr().cast(),
            SHADERS.len(),
            s!("westplay-mirrors"),
            None,
            None,
            entry,
            target,
            0,
            0,
            &mut code,
            Some(&mut errors),
        )
    };
    if let Err(e) = compiled {
        let said = errors.map(|blob| String::from_utf8_lossy(blob_bytes(&blob)).into_owned()).unwrap_or_default();
        return Err(windows::core::Error::new(e.code(), said));
    }
    code.ok_or_else(windows::core::Error::empty)
}

fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
    // SAFETY: the blob's buffer is its own, alive and of its size while the blob is.
    unsafe { std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_shaders_compile() {
        assert!(compile(s!("vs"), s!("vs_4_0")).is_ok());
        assert!(compile(s!("ps"), s!("ps_4_0")).map_err(|e| e.message()).is_ok());
    }

    #[test]
    fn the_mirrors_fill_the_height_centred_in_a_wide_place() {
        let place = mirrors_place([0, 0, 900, 720]);
        // The quad covers the place.
        assert_eq!(&place[0..4], &[-1.0, 1.0, 900.0 / 1280.0 * 2.0 - 1.0, -1.0]);
        // Full height: the top of the mirrors at the top, their main part's bottom at the bottom.
        assert!(place[5].abs() < 1e-5 && (place[7] - MAIN_MIRRORS).abs() < 1e-5);
        // Centred across: as much cut at the left as at the right.
        assert!(place[4] < 0.0 && (place[4] + (place[6] - 1.0)).abs() < 1e-4);
    }

    #[test]
    fn a_narrow_place_fits_the_mirrors_across() {
        let place = mirrors_place([100, 0, 300, 720]);
        assert!(place[4].abs() < 1e-5 && (place[6] - 1.0).abs() < 1e-5);
        assert!(place[5] < 0.0);
    }
}

/// Logs the mean colour of a 16 x 16 square in the middle of our mirror's copy (read back: the GPU waits once).
///
/// # Safety
/// The game's live immediate context and our copy, on its render thread.
unsafe fn check_copy(context: &ID3D11DeviceContext, copy: &ID3D11Texture2D) {
    use windows::Win32::Graphics::Direct3D11::{D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_USAGE_STAGING};
    // SAFETY: as the caller says; the staging texture is ours, of the copy's format, 16 x 16, mapped and unmapped here.
    unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        copy.GetDesc(&mut desc);
        let Ok(device) = context.GetDevice() else { return };
        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: 16,
            Height: 16,
            MipLevels: 1,
            ArraySize: 1,
            Format: desc.Format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };
        let mut staging = None;
        if device.CreateTexture2D(&staging_desc, None, Some(&mut staging)).is_err() {
            return;
        }
        let Some(staging) = staging else { return };
        let (x, y) = (desc.Width / 2 - 8, desc.Height / 2 - 8);
        let area = D3D11_BOX { left: x, top: y, front: 0, right: x + 16, bottom: y + 16, back: 1 };
        context.CopySubresourceRegion(&staging, 0, 0, 0, 0, copy, 0, Some(&area));
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        if context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).is_err() {
            return;
        }
        let mut sum = [0f32; 3];
        for row in 0..16 {
            let line = (mapped.pData as *const u8).add(row * mapped.RowPitch as usize) as *const u32;
            for col in 0..16 {
                let v = *line.add(col);
                let rgb = [
                    crate::mirror_probe::small_float(v & 0x7ff, 6),
                    crate::mirror_probe::small_float(v >> 11 & 0x7ff, 6),
                    crate::mirror_probe::small_float(v >> 22 & 0x3ff, 5),
                ];
                for i in 0..3 {
                    sum[i] += rgb[i] / 256.0;
                }
            }
        }
        context.Unmap(&staging, 0);
        log!("retrovisor: our mirror's copy, {}x{} format {}, middle's mean colour {sum:?}", desc.Width, desc.Height, desc.Format.0);
    }
}
