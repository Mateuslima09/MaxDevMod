//! Live D3D11 high-resolution screen bypass.
//!
//! Many vanilla and modded ETS2/ATS trucks (e.g. Mercedes Actros 2014, Volvo FH16, DAF XF) draw their
//! stock `ui/gps.sii` screen into a tiny `256 x 256` render target, crushing WestPlay's `1280 x 720`
//! frame down to blurry pixels before the 3D cab pass samples it.
//!
//! Instead of rewriting truck `.sii` files (which breaks modded interiors and requires a game restart),
//! this module renders WestPlay's full-resolution frame (upright, as the GPS target holds it,
//! with full hardware mipmaps and optional subtle sharpening) into our own
//! `R8G8B8A8_UNORM_SRGB` target during `Present` (between frames, never mid-pass!) and substitutes its
//! SRV when the 3D cab pass binds the truck's GPS render-target SRV in `PSSetShaderResources`.
//!
//! Changing the resolution tier in the in-cabin Settings panel or companion app updates
//! `HostBlock::screen_resolution` and takes effect on the very next frame without restarting the game.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

use westplay_shared::{
    SCREEN_HEIGHT, SCREEN_RES_GAME_DEFAULT, SCREEN_RES_HIGH, SCREEN_RES_STANDARD, SCREEN_RES_ULTRA, SCREEN_WIDTH,
};
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D_SRV_DIMENSION_TEXTURE2D, ID3DBlob};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BLEND_DESC,
    D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_CREATE_DEVICE_SINGLETHREADED, D3D11_CULL_NONE,
    D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
    D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RENDER_TARGET_VIEW_DESC_0, D3D11_RESOURCE_MISC_GENERATE_MIPS,
    D3D11_RTV_DIMENSION_TEXTURE2D, D3D11_SAMPLER_DESC, D3D11_SDK_VERSION, D3D11_SHADER_RESOURCE_VIEW_DESC,
    D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_RTV, D3D11_TEX2D_SRV, D3D11_TEXTURE_ADDRESS_CLAMP,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT, ID3D11BlendState, ID3D11Buffer, ID3D11Device,
    ID3D11Device1, ID3D11DeviceContext, ID3D11DeviceContext1, ID3D11PixelShader, ID3D11RasterizerState,
    ID3D11RenderTargetView, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
    ID3DDeviceContextState,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::{Interface, PCSTR, s};

use crate::{bridge, log};

/// Maximum SRVs tracked simultaneously for the active GPS render-target texture.
const MAX_GPS_SRVS: usize = 8;
/// Frames without a GPS screen draw before disabling substitution (matches `screens::SUPPORTED_LATELY`).
const STALE_FRAMES: u64 = 30;
/// Mip levels of the high-res picture the game may sample (see `ensure_target`).
const SHOWN_MIPS: u32 = 2;

/// Re-entrancy guard: true while our internal D3D11 blit or texture creation runs in `Present`,
/// so D3D11 hooks immediately forward to the original functions without touching plugin state.
static IN_BLIT: AtomicBool = AtomicBool::new(false);

/// Raw pointers to all SRVs the game created for the active GPS render-target texture (0: unused slot).
static GPS_RT_SRVS: [AtomicUsize; MAX_GPS_SRVS] = [const { AtomicUsize::new(0) }; MAX_GPS_SRVS];
/// Raw pointer to our high-res mipmapped SRV substituted in place of `GPS_RT_SRVS`.
static HIRES_SRV: AtomicUsize = AtomicUsize::new(0);
/// Whether `HIRES_SRV` holds a fresh frame and is safe to substitute in the 3D pass.
static GPS_RT_READY: AtomicBool = AtomicBool::new(false);
/// Frame number when `note_frame` last observed WestPlay drawn into the GPS render target.
static GPS_RT_FRAME: AtomicU64 = AtomicU64::new(0);
/// Last logged `(tier, target_w, target_h, out_w)` key so we log once per change.
static LOGGED_KEY: AtomicU64 = AtomicU64::new(0);
/// Last resolution tier applied, for detecting live changes cleanly.
static LAST_TIER: AtomicU32 = AtomicU32::new(u32::MAX);
/// Whether we already logged that `substitute_views` replaced the GPS SRV in a 3D pass.
static SUBSTITUTED_LOGGED: AtomicBool = AtomicBool::new(false);

/// Game render-target texture -> SRV address map (weak raw addresses, never dereferenced).
static RT_VIEWS: LazyLock<Mutex<Vec<(usize, usize)>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Lightweight frame metadata captured during `screens::refresh` and blitted cleanly during `Present`.
struct Pending {
    source: ID3D11Texture2D,
    version: u64,
    target_tex_raw: usize,
    target_w: u32,
    target_h: u32,
    layout_id: &'static str,
    frame: u64,
}

struct SharedPending(Option<Pending>);
// SAFETY: accessed only on the game's render thread through `PENDING` mutex.
unsafe impl Send for SharedPending {}

static PENDING: LazyLock<Mutex<SharedPending>> = LazyLock::new(|| Mutex::new(SharedPending(None)));

struct BlitGuard;

impl BlitGuard {
    fn enter() -> Self {
        IN_BLIT.store(true, Ordering::Release);
        Self
    }
}

impl Drop for BlitGuard {
    fn drop(&mut self) {
        IN_BLIT.store(false, Ordering::Release);
    }
}

/// True while our internal D3D11 blit/allocation runs on the render thread.
pub fn in_blit() -> bool {
    IN_BLIT.load(Ordering::Acquire)
}

/// Layouts where `game_mod` places WestPlay inside a cropped sub-rectangle of the GPS target
/// (`SHOWN_PARTS` / coach window screens) rather than covering the full render target.
/// Those layouts already get `2048 x 2048` in `westplay.scs`.
fn is_cropped_layout(id: &str) -> bool {
    matches!(id, "scania-7" | "setra-516" | "travego" | "tourismo" | "man-lions-coach")
}

/// Layouts whose GPS target the cab samples somewhere else too, where the high-res picture must not go (owner
/// 2026-11-21: a Scania 2025 mod's instrument cluster flashed with WestPlay's screen in it, and stopped with the
/// game's own resolution). Their target is big already (1024 x 512), so little is lost.
fn shares_its_target(id: &str) -> bool {
    id.starts_with("scania-2025")
}

/// Target dimensions `(width, height)` and unsharp-mask sharpening strength for a live tier.
fn tier_spec(tier: u32) -> Option<(u32, u32, f32)> {
    match tier {
        SCREEN_RES_STANDARD => Some((640, 360, 0.0)),
        SCREEN_RES_HIGH => Some((SCREEN_WIDTH, SCREEN_HEIGHT, 0.0)),
        SCREEN_RES_ULTRA => Some((SCREEN_WIDTH, SCREEN_HEIGHT, 0.55)),
        _ => None,
    }
}

fn clear_gps_srvs() {
    for slot in &GPS_RT_SRVS {
        slot.store(0, Ordering::Release);
    }
}

/// The GPS render-target texture the SRVs in `GPS_RT_SRVS` were published for (a live one, this frame's).
static GPS_RT_TEX: AtomicUsize = AtomicUsize::new(0);

fn publish_gps_srvs(target_tex_raw: usize) -> bool {
    GPS_RT_TEX.store(target_tex_raw, Ordering::Release);
    let views = RT_VIEWS.lock().unwrap_or_else(|e| e.into_inner());
    let mut found = [0usize; MAX_GPS_SRVS];
    let mut count = 0usize;
    for &(res, srv) in views.iter().rev() {
        if res == target_tex_raw && !found[..count].contains(&srv) {
            found[count] = srv;
            count += 1;
            if count == MAX_GPS_SRVS {
                break;
            }
        }
    }
    for (slot, addr) in GPS_RT_SRVS.iter().zip(found) {
        slot.store(addr, Ordering::Release);
    }
    count > 0
}

/// Records an SRV the game created for a single-sample RGBA render-target texture.
pub fn on_rt_view_created(resource_raw: usize, view_raw: usize) {
    let mut views = RT_VIEWS.lock().unwrap_or_else(|e| e.into_inner());
    views.retain(|&(_, v)| v != view_raw);
    if views.len() >= 128 {
        views.remove(0);
    }
    views.push((resource_raw, view_raw));
}

/// Called on every presented frame: drops the bypass if the GPS screen stopped being drawn
/// (electrics off, menu open, camera away from dashboard).
pub fn on_frame(frame: u64) {
    if GPS_RT_READY.load(Ordering::Acquire) && frame.saturating_sub(GPS_RT_FRAME.load(Ordering::Acquire)) > STALE_FRAMES
    {
        GPS_RT_READY.store(false, Ordering::Release);
    }
}

/// Clears truck-specific bypass tracking when the player switches trucks.
pub fn reset_truck() {
    GPS_RT_READY.store(false, Ordering::Release);
    clear_gps_srvs();
    LOGGED_KEY.store(0, Ordering::Release);
    SUBSTITUTED_LOGGED.store(false, Ordering::Release);
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).0 = None;
}

/// Clears all D3D11 resources if the game recreated its device.
pub fn on_device_changed() {
    reset_truck();
    HIRES_SRV.store(0, Ordering::Release);
    LAST_TIER.store(u32::MAX, Ordering::Release);
    RT_VIEWS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    BLITTER.lock().unwrap_or_else(|e| e.into_inner()).0 = None;
}

/// Disables bypass for this frame (e.g. WestPlay switched off or showing transparent).
pub fn disable_for_frame() {
    GPS_RT_READY.store(false, Ordering::Release);
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).0 = None;
}

/// Records the active GPS render target and latest source texture during `screens::refresh`.
/// Performs ZERO D3D11 draw calls or context-state switches so ETS2's active UI pass is never disturbed.
#[allow(clippy::too_many_arguments)]
pub fn note_frame(
    source: ID3D11Texture2D,
    version: u64,
    target_tex_raw: usize,
    target_w: u32,
    target_h: u32,
    layout_id: &'static str,
    full_viewport: bool,
    frame: u64,
) {
    let Some(bridge) = bridge::get() else {
        disable_for_frame();
        return;
    };
    let tier = bridge.host.screen_resolution.load(Ordering::Acquire);
    let split = bridge.host.screen_split.load(Ordering::Acquire);
    // Handed to the truck: the game's own screen (its map) shows, not our picture over it.
    let handed_over = bridge.host.screen_handed_over.load(Ordering::Acquire) != 0;
    if tier_spec(tier).is_none() || split != 0 || handed_over || !full_viewport || is_cropped_layout(layout_id)
        || shares_its_target(layout_id)
        || target_tex_raw == 0
    {
        if tier_spec(tier).is_none() && LAST_TIER.swap(SCREEN_RES_GAME_DEFAULT, Ordering::AcqRel) != SCREEN_RES_GAME_DEFAULT {
            log!("screen hires: off (game default {target_w}x{target_h})");
        }
        disable_for_frame();
        return;
    }
    if !publish_gps_srvs(target_tex_raw) {
        GPS_RT_READY.store(false, Ordering::Release);
        return;
    }
    GPS_RT_FRAME.store(frame, Ordering::Release);
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).0 = Some(Pending {
        source,
        version,
        target_tex_raw,
        target_w,
        target_h,
        layout_id,
        frame,
    });
}

/// Runs once per frame during `Present` (between frames, while ETS2's immediate context is idle):
/// blits the latest WestPlay frame into our upright mipmapped high-res target.
///
/// # Safety
/// `swapchain` must be the game's live `IDXGISwapChain` on its render thread inside `Present`.
pub unsafe fn on_present(swapchain: *mut c_void) {
    let pending = {
        let guard = PENDING.lock().unwrap_or_else(|e| e.into_inner());
        guard.0.as_ref().map(|p| Pending {
            source: p.source.clone(),
            version: p.version,
            target_tex_raw: p.target_tex_raw,
            target_w: p.target_w,
            target_h: p.target_h,
            layout_id: p.layout_id,
            frame: p.frame,
        })
    };
    let Some(pending) = pending else { return };
    let Some(bridge) = bridge::get() else {
        GPS_RT_READY.store(false, Ordering::Release);
        return;
    };
    let tier = bridge.host.screen_resolution.load(Ordering::Acquire);
    let split = bridge.host.screen_split.load(Ordering::Acquire);
    let Some((out_w, out_h, sharpen)) = tier_spec(tier) else {
        if LAST_TIER.swap(SCREEN_RES_GAME_DEFAULT, Ordering::AcqRel) != SCREEN_RES_GAME_DEFAULT {
            log!("screen hires: off (game default {}x{})", pending.target_w, pending.target_h);
        }
        GPS_RT_READY.store(false, Ordering::Release);
        return;
    };
    if split != 0 {
        GPS_RT_READY.store(false, Ordering::Release);
        return;
    }
    // SAFETY: `swapchain` is the game's live swap chain in `Present`, on its render thread.
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let Ok(context) = device.GetImmediateContext() else { return };
        if pending.source.GetDevice().ok().as_ref() != Some(&device) {
            return;
        }
        let _guard = BlitGuard::enter();
        let mut blitter_lock = BLITTER.lock().unwrap_or_else(|e| e.into_inner());
        let device_raw = device.as_raw() as usize;
        if blitter_lock.0.as_ref().is_some_and(|b| b.device.as_raw() as usize != device_raw) {
            blitter_lock.0 = None;
            HIRES_SRV.store(0, Ordering::Release);
        }
        if blitter_lock.0.is_none() {
            match Blitter::new(&device) {
                Ok(b) => blitter_lock.0 = Some(b),
                Err(e) => {
                    log!("screen hires: failed to init D3D11 blitter: {e}");
                    GPS_RT_READY.store(false, Ordering::Release);
                    return;
                }
            }
        }
        let Some(blitter) = blitter_lock.0.as_mut() else {
            GPS_RT_READY.store(false, Ordering::Release);
            return;
        };
        match blitter.blit_if_needed(&context, &pending.source, pending.version, out_w, out_h, tier, sharpen) {
            Ok(srv_raw) => {
                if publish_gps_srvs(pending.target_tex_raw) {
                    HIRES_SRV.store(srv_raw, Ordering::Release);
                    GPS_RT_FRAME.store(pending.frame, Ordering::Release);
                    GPS_RT_READY.store(true, Ordering::Release);
                    LAST_TIER.store(tier, Ordering::Release);
                    let key = ((tier as u64) << 48)
                        | ((pending.target_w as u64 & 0xffff) << 32)
                        | ((pending.target_h as u64 & 0xffff) << 16)
                        | (out_w as u64 & 0xffff);
                    if LOGGED_KEY.swap(key, Ordering::AcqRel) != key {
                        log!(
                            "screen hires: {} {}x{} -> {}x{} (tier {}, sharpen {:.2}, live D3D11 in Present)",
                            pending.layout_id,
                            pending.target_w,
                            pending.target_h,
                            out_w,
                            out_h,
                            tier,
                            sharpen
                        );
                    }
                }
            }
            Err(e) => {
                log!("screen hires: blit failed: {e}");
                GPS_RT_READY.store(false, Ordering::Release);
            }
        }
    }
}

/// Fast-path substitution called from `PSSetShaderResources` hook during non-screen-target passes.
/// If one of `views[..count]` matches a tracked GPS render-target SRV, copies `views[..count]` into `out`,
/// replaces that slot with our high-res mipmapped `HIRES_SRV`, and returns `true`.
///
/// # Safety
/// `views` must point to at least `count` readable pointer slots (the caller's D3D11 contract).
pub unsafe fn substitute_views(count: u32, views: *const *mut c_void, out: &mut [*mut c_void; 128]) -> bool {
    // Not while the GPS target itself is being drawn into. Any other target is fine (owner 2026-11-20: a mod's
    // bigger screen blurry whatever the tier): the 3D cab pass in ETS2 1.62 draws into a 1920 x 1080
    // `R8G8B8A8_UNORM_SRGB` target, the same shape `screens::is_screen_target` takes for a screen's, and the old check
    // (any such target bound: skip) so never let the high-res picture in. No log ever said "3D pass SRV substituted".
    if !GPS_RT_READY.load(Ordering::Acquire)
        || crate::screens::current_target_texture() == GPS_RT_TEX.load(Ordering::Acquire)
        || views.is_null()
        || count == 0
        || count > 128
    {
        return false;
    }
    let hires_srv = HIRES_SRV.load(Ordering::Acquire);
    if hires_srv == 0 {
        return false;
    }
    // SAFETY: `views` is non-null and `count <= 128` (checked above), per `PSSetShaderResources` contract.
    let slice = unsafe { std::slice::from_raw_parts(views, count as usize) };
    let mut matched = false;
    for (idx, &v) in slice.iter().enumerate() {
        let addr = v as usize;
        if addr == 0 {
            continue;
        }
        let is_gps = GPS_RT_SRVS.iter().any(|slot| {
            let s = slot.load(Ordering::Relaxed);
            s != 0 && s == addr
        });
        // Only a view that is the GPS target's still (owner 2026-11-19: black flashes over the whole screen after
        // "engine only off", on into walking): the addresses are kept without a hold on the views, and as the game
        // makes the screen's textures again the address of one let go is taken by another of its views, a
        // full-screen pass's; the screen's picture put there flashed the whole screen. The view is the game's own,
        // bound now: alive to ask.
        if is_gps && !view_of(addr, GPS_RT_TEX.load(Ordering::Acquire)) {
            if !STALE_LOGGED.swap(true, Ordering::AcqRel) {
                log!("screen hires: a GPS view's address now another texture's: not substituted");
            }
            continue;
        }
        if is_gps {
            if !matched {
                out[..count as usize].copy_from_slice(slice);
                matched = true;
            }
            out[idx] = hires_srv as *mut c_void;
        }
    }
    if matched && !SUBSTITUTED_LOGGED.swap(true, Ordering::AcqRel) {
        log!("screen hires: 3D pass SRV substituted");
    }
    matched
}

static STALE_LOGGED: AtomicBool = AtomicBool::new(false);

/// Whether the shader resource view at `view` (one the game binds now: alive) is of the texture at `texture`.
fn view_of(view: usize, texture: usize) -> bool {
    if texture == 0 {
        return false;
    }
    let raw = view as *mut c_void;
    // SAFETY: `view` is a view the game passes to `PSSetShaderResources` this call (the caller's contract): alive.
    let Some(view) = (unsafe { ID3D11ShaderResourceView::from_raw_borrowed(&raw) }) else { return false };
    // SAFETY: a live view; the resource is returned with a reference, released as it drops.
    let resource = unsafe { view.GetResource() };
    resource.is_ok_and(|resource| resource.as_raw() as usize == texture)
}

const SHADERS: &str = r"
cbuffer Params : register(b0) { float4 params; };
struct Vertex { float4 position : SV_Position; float2 uv : TEXCOORD0; };
Vertex vs(uint id : SV_VertexID) {
    float2 corner = float2(id & 1, id >> 1);
    Vertex v;
    v.position = float4(corner.x * 2.0 - 1.0, 1.0 - corner.y * 2.0, 0.0, 1.0);
    v.uv = corner; // not flipped: the owner saw it upside down flipped (2026-11-20, the first time it was let in)
    return v;
}
Texture2D picture : register(t0);
SamplerState smooth : register(s0);
float4 ps(Vertex v) : SV_Target {
    float4 c = picture.SampleLevel(smooth, v.uv, 0);
    if (params.z <= 0.0) return float4(c.rgb, 1.0);
    float2 dx = float2(params.x, 0.0);
    float2 dy = float2(0.0, params.y);
    float3 n = picture.SampleLevel(smooth, v.uv - dy, 0).rgb;
    float3 s = picture.SampleLevel(smooth, v.uv + dy, 0).rgb;
    float3 e = picture.SampleLevel(smooth, v.uv + dx, 0).rgb;
    float3 w = picture.SampleLevel(smooth, v.uv - dx, 0).rgb;
    float3 blur = (n + s + e + w) * 0.25;
    float3 sharp = saturate(c.rgb + (c.rgb - blur) * params.z);
    return float4(sharp, 1.0);
}
";

fn compile(entry: PCSTR, target: PCSTR) -> windows::core::Result<ID3DBlob> {
    let mut code = None;
    let mut errors = None;
    // SAFETY: `SHADERS`, `entry`, and `target` are valid static strings.
    let result = unsafe {
        D3DCompile(
            SHADERS.as_ptr().cast(),
            SHADERS.len(),
            None,
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
    if let Err(error) = result {
        if let Some(errors) = errors {
            // SAFETY: `errors` is a valid D3D blob returned by `D3DCompile`.
            let bytes =
                unsafe { std::slice::from_raw_parts(errors.GetBufferPointer().cast::<u8>(), errors.GetBufferSize()) };
            log!("screen hires: shader error: {}", String::from_utf8_lossy(bytes));
        }
        return Err(error);
    }
    code.ok_or_else(windows::core::Error::empty)
}

fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
    // SAFETY: `blob` is a live compiled shader bytecode blob.
    unsafe { std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize()) }
}

struct Target {
    width: u32,
    height: u32,
    _texture: ID3D11Texture2D,
    rtv: ID3D11RenderTargetView,
    srv: ID3D11ShaderResourceView,
}

struct Blitter {
    device: ID3D11Device,
    state: ID3DDeviceContextState,
    vertex: ID3D11VertexShader,
    pixel: ID3D11PixelShader,
    params: ID3D11Buffer,
    blend: ID3D11BlendState,
    rasterizer: ID3D11RasterizerState,
    sampler: ID3D11SamplerState,
    target: Option<Target>,
    source_view: Option<(usize, ID3D11ShaderResourceView)>,
    last_version: u64,
    last_source: usize,
    last_tier: u32,
}

struct SharedBlitter(Option<Blitter>);
// SAFETY: used only on the game's render thread under `BLITTER` mutex.
unsafe impl Send for SharedBlitter {}

static BLITTER: LazyLock<Mutex<SharedBlitter>> = LazyLock::new(|| Mutex::new(SharedBlitter(None)));

impl Blitter {
    /// # Safety
    /// `device` must be the game's live D3D11 device on its render thread.
    unsafe fn new(device: &ID3D11Device) -> windows::core::Result<Self> {
        // SAFETY: `device` is live on the render thread; all descriptors are stack locals.
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
            let mut params = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: 16,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut params),
            )?;
            let mut blend_desc = D3D11_BLEND_DESC::default();
            blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: false.into(),
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
                ..Default::default()
            };
            let mut blend = None;
            device.CreateBlendState(&blend_desc, Some(&mut blend))?;
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
            Ok(Self {
                device: device.clone(),
                state: state.ok_or_else(none)?,
                vertex: vertex.ok_or_else(none)?,
                pixel: pixel.ok_or_else(none)?,
                params: params.ok_or_else(none)?,
                blend: blend.ok_or_else(none)?,
                rasterizer: rasterizer.ok_or_else(none)?,
                sampler: sampler.ok_or_else(none)?,
                target: None,
                source_view: None,
                last_version: u64::MAX,
                last_source: 0,
                last_tier: u32::MAX,
            })
        }
    }

    /// # Safety
    /// Must be called on the render thread with live `device`.
    unsafe fn ensure_target(&mut self, width: u32, height: u32) -> windows::core::Result<&Target> {
        let needs_new = !matches!(&self.target, Some(t) if t.width == width && t.height == height);
        if needs_new {
            let none = windows::core::Error::empty;
            // SAFETY: `self.device` is live on the render thread; all descriptors are stack locals.
            unsafe {
                let mut texture = None;
                self.device.CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: width,
                        Height: height,
                        MipLevels: 0,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                        CPUAccessFlags: 0,
                        MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
                    },
                    None,
                    Some(&mut texture),
                )?;
                let texture = texture.ok_or_else(none)?;
                let mut rtv = None;
                self.device.CreateRenderTargetView(
                    &texture,
                    Some(&D3D11_RENDER_TARGET_VIEW_DESC {
                        Format: DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
                        ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_RENDER_TARGET_VIEW_DESC_0 { Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 } },
                    }),
                    Some(&mut rtv),
                )?;
                let mut srv = None;
                self.device.CreateShaderResourceView(
                    &texture,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
                        ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            // Two levels only (owner 2026-11-20: soft and greyish with all of them): the cab sees the
                            // 1280-wide picture at ~500 px, where trilinear filtering blends in the 320-wide level;
                            // stopping at the 640-wide one keeps text sharp without the old jaggies.
                            Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: SHOWN_MIPS },
                        },
                    }),
                    Some(&mut srv),
                )?;
                self.target = Some(Target {
                    width,
                    height,
                    _texture: texture,
                    rtv: rtv.ok_or_else(none)?,
                    srv: srv.ok_or_else(none)?,
                });
                self.last_version = u64::MAX;
            }
        }
        self.target.as_ref().ok_or_else(windows::core::Error::empty)
    }

    /// # Safety
    /// `context` and `source` must be live D3D11 objects on the render thread under `BlitGuard` inside `Present`.
    #[allow(clippy::too_many_arguments)]
    unsafe fn blit_if_needed(
        &mut self,
        context: &ID3D11DeviceContext,
        source: &ID3D11Texture2D,
        version: u64,
        width: u32,
        height: u32,
        tier: u32,
        sharpen: f32,
    ) -> windows::core::Result<usize> {
        let source_raw = source.as_raw() as usize;
        // SAFETY: `self.device` is live on the render thread.
        let srv_raw = unsafe { self.ensure_target(width, height)? }.srv.as_raw() as usize;
        if self.last_version == version && self.last_source == source_raw && self.last_tier == tier {
            return Ok(srv_raw);
        }
        // SAFETY: `self.device`, `context`, and `source` are live on the render thread; game state is swapped back.
        unsafe {
            if !matches!(&self.source_view, Some((ptr, _)) if *ptr == source_raw) {
                let mut view = None;
                self.device.CreateShaderResourceView(
                    source,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
                        ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
                        },
                    }),
                    Some(&mut view),
                )?;
                self.source_view = Some((source_raw, view.ok_or_else(windows::core::Error::empty)?));
            }
            let Some((_, source_srv)) = self.source_view.clone() else {
                return Err(windows::core::Error::empty());
            };
            let Some(target) = self.target.as_ref() else {
                return Err(windows::core::Error::empty());
            };
            let context1: ID3D11DeviceContext1 = context.cast()?;
            let mut saved = None;
            context1.SwapDeviceContextState(&self.state, Some(&mut saved));
            let params: [f32; 4] = [1.0 / SCREEN_WIDTH as f32, 1.0 / SCREEN_HEIGHT as f32, sharpen, 0.0];
            context.UpdateSubresource(&self.params, 0, None, params.as_ptr().cast(), 0, 0);
            context.OMSetRenderTargets(Some(&[Some(target.rtv.clone())]), None);
            context.RSSetState(&self.rasterizer);
            context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: width as f32,
                Height: height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
            context.OMSetBlendState(&self.blend, None, 0xffff_ffff);
            context.IASetInputLayout(None);
            context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            context.VSSetShader(&self.vertex, None);
            context.PSSetShader(&self.pixel, None);
            context.PSSetConstantBuffers(0, Some(&[Some(self.params.clone())]));
            context.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            context.PSSetShaderResources(0, Some(&[Some(source_srv)]));
            context.Draw(4, 0);
            context.OMSetRenderTargets(Some(&[None]), None);
            context.PSSetShaderResources(0, Some(&[None]));
            context.GenerateMips(&target.srv);
            if let Some(saved) = &saved {
                context1.SwapDeviceContextState(saved, None);
            }
            self.last_version = version;
            self.last_source = source_raw;
            self.last_tier = tier;
            Ok(srv_raw)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scania_2025_keeps_its_own_screen() {
        assert!(shares_its_target("scania-2025") && shares_its_target("scania-2025-advanced"));
        assert!(!shares_its_target("scania-7") && !shares_its_target("generic"));
    }
}
