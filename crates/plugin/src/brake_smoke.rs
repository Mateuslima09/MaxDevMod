//! 3D brake smoke rising naturally from overheated truck and trailer wheels (owner 2026-11-23).
//!
//! - Completely independent between tractor (`brake_temp_truck`) and trailer(s) (`brake_temp_trailer`):
//!   - Below orange (blue/green/yellow): no smoke (0% GPU/CPU cost).
//!   - Orange (`290..360 °C` in Realistic; `+100 °C` in Light; `-50 °C` in Hard): light airy pale-grey wisps.
//!   - Red (`360..400 °C` in Realistic): fuller medium-dark grey smoke.
//!   - Very hot (`400..425+ °C` in Realistic): dense dark charcoal smoke (both solo tractor and trailer).
//! - Emits from the SDK's exact 3D wheel hub coordinates (`reversing::VEHICLES` for tractor & trailer 0, plus
//!   `telemetry::extra_trailers` for bitrem/rodotrem/triple combinations).
//! - Simulates in vehicle-local space with subtle speed-proportional backward wind drift, locked directly to each
//!   rendered vehicle's GPU `MV` matrix (`UNIT_MV_VOTES`) for zero wobble/lag when orbiting the camera.
//! - Snapshots the main 3D G-buffer depth buffer on G-buffer exit (`CopyResource` only — never mutating D3D state
//!   mid-frame so mirrors and dashboards are completely untouched) and performs read-only hardware depth testing at
//!   `Present` so smoke never bleeds through the truck cab, trailer bodywork, or AI traffic (with analytical OBB
//!   occlusion as fallback when ETS2 resolution scaling != 100%).

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D11_SRV_DIMENSION_TEXTURE2D, ID3DBlob};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BLEND_DESC,
    D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD, D3D11_BUFFER_DESC,
    D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_LESS_EQUAL, D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE,
    D3D11_CREATE_DEVICE_SINGLETHREADED, D3D11_CULL_NONE, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ZERO,
    D3D11_DEVICE_CONTEXT_IMMEDIATE, D3D11_FILL_SOLID, D3D11_MAP_READ, D3D11_MAP_WRITE_DISCARD,
    D3D11_MAP_WRITE_NO_OVERWRITE, D3D11_MAPPED_SUBRESOURCE, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
    D3D11_SDK_VERSION, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_SRV,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC, D3D11_USAGE_STAGING, D3D11_VIEWPORT,
    ID3D11BlendState, ID3D11Buffer, ID3D11DepthStencilState, ID3D11DepthStencilView, ID3D11Device, ID3D11Device1,
    ID3D11DeviceContext, ID3D11DeviceContext1, ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView,
    ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader, ID3DDeviceContextState,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_D16_UNORM, DXGI_FORMAT_D24_UNORM_S8_UINT, DXGI_FORMAT_D32_FLOAT,
    DXGI_FORMAT_D32_FLOAT_S8X24_UINT, DXGI_FORMAT_R16_TYPELESS, DXGI_FORMAT_R16_UNORM, DXGI_FORMAT_R24_UNORM_X8_TYPELESS,
    DXGI_FORMAT_R24G8_TYPELESS, DXGI_FORMAT_R32_FLOAT, DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS, DXGI_FORMAT_R32_TYPELESS,
    DXGI_FORMAT_R32G8X24_TYPELESS, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::{Interface, PCSTR, s};

use crate::{bridge, log};

type M3 = [[f32; 3]; 3];
type M4 = [[f32; 4]; 4];

/// Maximum simultaneous smoke puffs drawn in a single instanced draw call.
const MAX_PARTICLES: usize = 384;
/// Maximum G-buffer constant-buffer blocks inspected per frame for the main view while smoke is active.
const MAX_READS: u32 = 1200;
/// Maximum constants in a standard object transform block in ETS2/ATS (`b0`).
const MAX_BLOCK_CONSTANTS: u32 = 24;
/// Minimum G-buffer render targets identifying the main 3D scene pass.
const GBUFFER_TARGETS: u32 = 3;
/// Minimum votes required for a standalone camera candidate.
const MIN_VIEW_VOTES: u32 = 3;
/// Maximum distance (metres) from the driver's head considered inside the cab (no main-view smoke over dashboard).
const CAB_CAMERA_MAX_DIST: f64 = 1.4;

/// Whether smoke is enabled and either emitting or still has live puffs (0% work when false).
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// Re-entrancy guard while `Drawing::render_into` binds its own render targets inside `Present`.
static IN_DRAW: AtomicBool = AtomicBool::new(false);
/// Whether the main 3D G-buffer pass is currently bound on the immediate context.
static IN_MAIN_GBUFFER: AtomicBool = AtomicBool::new(false);
/// Constant-buffer blocks read this frame for the main view.
static READS: AtomicU32 = AtomicU32::new(0);
/// Valid world 3D blocks seen in the current G-buffer pass (to distinguish real 3D passes from clears).
static PASS_WORLD_BLOCKS: AtomicU32 = AtomicU32::new(0);
/// Presents counted.
static PRESENTS: AtomicU64 = AtomicU64::new(0);

/// Depth-stencil view currently bound by the game's main G-buffer pass (and one queued to copy right after unbind).
struct ActiveDsv(Option<ID3D11DepthStencilView>);
// SAFETY: accessed only on the game's render thread (OMSetRenderTargets / Present).
unsafe impl Send for ActiveDsv {}
static ACTIVE_GBUFFER_DSV: Mutex<ActiveDsv> = Mutex::new(ActiveDsv(None));
static PENDING_DEPTH_COPY: Mutex<ActiveDsv> = Mutex::new(ActiveDsv(None));

/// GPU-copied SRV snapshot of the main 3D G-buffer's depth texture, sampled in the pixel shader for soft volumetric
/// depth occlusion at any ETS2 resolution scaling (never bound as a backbuffer DSV).
struct SavedDepth {
    device: ID3D11Device,
    width: u32,
    height: u32,
    src_format: u32,
    min_depth: f32,
    max_depth: f32,
    texture: ID3D11Texture2D,
    srv: ID3D11ShaderResourceView,
    captured_present: u64,
    copies_this_frame: u32,
}
// SAFETY: accessed only on the game's render thread (OMSetRenderTargets / Present).
unsafe impl Send for SavedDepth {}
static SAVED_DEPTH: Mutex<Option<SavedDepth>> = Mutex::new(None);
/// Viewport `(MinDepth, MaxDepth)` read during the active G-buffer pass (ETS2 uses `[0.01, 1.0]` for the 3D world).
static PASS_VIEWPORT_DEPTH: Mutex<(f32, f32)> = Mutex::new((0.0, 1.0));
static PENDING_VIEWPORT_DEPTH: Mutex<(f32, f32)> = Mutex::new((0.0, 1.0));

/// Known vehicle transpose orientations (`R_vehicle^T`, rows = `[right, up, back]` in world space) for the truck
/// and all coupled trailers, used to verify `R_view` against vehicle submeshes in the G-buffer.
static VEHICLE_AXES: LazyLock<Mutex<Vec<M3>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Per-frame votes for the main 3D world view.
static WORLD_VOTES: LazyLock<Mutex<HashMap<[i32; 6], (u32, M3, M4)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static VEHICLE_VOTES: LazyLock<Mutex<HashMap<[i32; 6], (u32, M3, M4)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// Camera position snapshotted at the start of the main G-buffer pass (in phase with that frame's GPU draws).
static CAM_AT_GBUFFER: Mutex<Option<[f64; 3]>> = Mutex::new(None);
/// Per-unit (`0` = truck, `1` = trailer 0, `2..=4` = extra trailers) GPU `MV` votes from this frame's `b0` blocks,
/// keyed by quantized translation so smoke locks pixel-perfectly to each rendered vehicle with zero camera-motion wobble.
type UnitMvEntry = (u32, M4, M3, [f64; 3]);
static UNIT_MV_VOTES: LazyLock<Mutex<[HashMap<[i32; 3], UnitMvEntry>; 5]>> =
    LazyLock::new(|| Mutex::new(std::array::from_fn(|_| HashMap::new())));
/// Last resolved main camera `(R_view, P, present_index)` for temporal continuity (prevents 1-frame flicker).
static LAST_CAMERA: Mutex<Option<(M3, M4, u64)>> = Mutex::new(None);

/// Snapshot of live vehicle poses updated each frame for `UNIT_MV_VOTES` matching.
static POSES_SNAPSHOT: LazyLock<Mutex<[Option<VehiclePose>; 5]>> = LazyLock::new(|| Mutex::new([None; 5]));

/// Diagnostic counters `(window_start, cam_hits, cam_misses)` logged every 3 s while active.
static DIAG_START: LazyLock<Mutex<(Instant, u32, u32)>> = LazyLock::new(|| Mutex::new((Instant::now(), 0, 0)));

/// Temperature shift (°C) packed in `HostBlock::brake_hud` bits 9..10 for Light / Realistic / Hard brake fade.
pub fn level_shift(flags: u32) -> f32 {
    match (flags >> 9) & 3 {
        1 => 100.0, // Light: +100 °C later
        2 => -50.0, // Hard: -50 °C sooner
        _ => 0.0,   // Realistic
    }
}

/// Maps disc temperature (°C) to smoke intensity `0.0..=1.0`, matching `brake_hud::disc_colour` and `brake_fade`:
/// - `< 290 °C` (blue / green / yellow): `0.0` (no smoke)
/// - `290..360 °C` (orange): light airy pale-grey smoke `0.08..0.40`
/// - `360..425+ °C` (red / no brakes at 400 °C): fuller -> dark charcoal smoke `0.40..1.0`
pub fn smoke_intensity(celsius: f32) -> f32 {
    if !celsius.is_finite() || celsius < 290.0 {
        return 0.0;
    }
    if celsius < 360.0 {
        let t = (celsius - 290.0) / 70.0;
        0.08 + 0.32 * t
    } else {
        let t = ((celsius - 360.0) / 65.0).clamp(0.0, 1.0);
        0.40 + 0.60 * t
    }
}

/// Transforms a vehicle-local point `(x right, y up, z back)` into world space (`Pose::to_world`).
fn to_world(origin: [f64; 3], heading: f32, pitch: f32, roll: f32, local: [f32; 3]) -> [f64; 3] {
    let dir = rotate_to_world(heading, pitch, roll, local);
    [origin[0] + dir[0], origin[1] + dir[1], origin[2] + dir[2]]
}

/// Rotates a vehicle-local direction vector into world space.
fn rotate_to_world(heading: f32, pitch: f32, roll: f32, local: [f32; 3]) -> [f64; 3] {
    let turn = |t: f32| f64::from(t) * std::f64::consts::TAU;
    let (h, p, r) = (turn(heading), turn(pitch), turn(roll));
    let [mut x, mut y, mut z] = local.map(f64::from);
    (x, y) = (x * r.cos() - y * r.sin(), x * r.sin() + y * r.cos());
    (y, z) = (y * p.cos() - z * p.sin(), y * p.sin() + z * p.cos());
    (x, z) = (x * h.cos() + z * h.sin(), -x * h.sin() + z * h.cos());
    [x, y, z]
}

/// Vehicle's world axes as rows `[right, up, back]` (`= R_vehicle^T`), matching `rear_camera::tilted_axes`.
fn tilted_axes(heading: f32, pitch: f32, roll: f32) -> M3 {
    let (h, p, r) = (heading * std::f32::consts::TAU, pitch * std::f32::consts::TAU, roll * std::f32::consts::TAU);
    let turn_y = |v: [f32; 3]| [v[0] * h.cos() + v[2] * h.sin(), v[1], -v[0] * h.sin() + v[2] * h.cos()];
    let turn_x = |v: [f32; 3]| [v[0], v[1] * p.cos() - v[2] * p.sin(), v[1] * p.sin() + v[2] * p.cos()];
    let turn_z = |v: [f32; 3]| [v[0] * r.cos() - v[1] * r.sin(), v[0] * r.sin() + v[1] * r.cos(), v[2]];
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|v| turn_y(turn_x(turn_z(v))))
}

/// Whether the main camera is physically inside the cab (close to the driver's head and not outside the truck).
fn camera_inside_cab(cam: [f64; 3], truck: &crate::telemetry::Truck) -> bool {
    if truck.position == [0.0; 3] {
        return false;
    }
    let head_world = to_world(truck.position, truck.heading, truck.pitch, truck.roll, truck.head);
    let dist_sq: f64 = cam.iter().zip(head_world).map(|(c, h)| (c - h).powi(2)).sum();
    dist_sq <= CAB_CAMERA_MAX_DIST * CAB_CAMERA_MAX_DIST
}

// ---------------------------------------------------------------------------------------------------------------
// Vehicle poses & fallback OBB visibility (used only when ETS2 scaling != 100% prevents binding the G-buffer DSV).

#[derive(Clone, Copy)]
struct VehiclePose {
    origin: [f64; 3],
    heading: f32,
    pitch: f32,
    roll: f32,
    axes: M3,
}

impl VehiclePose {
    fn new(origin: [f64; 3], heading: f32, pitch: f32, roll: f32) -> Self {
        Self { origin, heading, pitch, roll, axes: tilted_axes(heading, pitch, roll) }
    }

    fn local_to_world(&self, local: [f32; 3]) -> [f64; 3] {
        to_world(self.origin, self.heading, self.pitch, self.roll, local)
    }

    fn world_to_local(&self, world: [f64; 3]) -> [f32; 3] {
        let d = [
            (world[0] - self.origin[0]) as f32,
            (world[1] - self.origin[1]) as f32,
            (world[2] - self.origin[2]) as f32,
        ];
        [
            self.axes[0][0] * d[0] + self.axes[0][1] * d[1] + self.axes[0][2] * d[2],
            self.axes[1][0] * d[0] + self.axes[1][1] * d[1] + self.axes[1][2] * d[2],
            self.axes[2][0] * d[0] + self.axes[2][1] * d[1] + self.axes[2][2] * d[2],
        ]
    }
}

#[derive(Clone, Copy)]
struct BodyBox {
    pose: VehiclePose,
    min: [f32; 3],
    max: [f32; 3],
}

impl BodyBox {
    fn visibility(&self, cam_world: [f64; 3], point_world: [f64; 3]) -> f32 {
        let a = self.pose.world_to_local(cam_world);
        if (0..3).all(|i| a[i] >= self.min[i] && a[i] <= self.max[i]) {
            return 1.0;
        }
        let b = self.pose.world_to_local(point_world);
        if segment_hits_aabb(a, b, self.min, self.max) {
            return 0.0;
        }
        let pad = 0.24f32;
        let outer_min = [self.min[0] - pad, self.min[1] - pad, self.min[2] - pad];
        let outer_max = [self.max[0] + pad, self.max[1] + pad, self.max[2] + pad];
        if !segment_hits_aabb(a, b, outer_min, outer_max) {
            return 1.0;
        }
        (segment_distance_to_aabb(a, b, self.min, self.max) / pad).clamp(0.0, 1.0)
    }
}

fn segment_hits_aabb(a: [f32; 3], b: [f32; 3], min: [f32; 3], max: [f32; 3]) -> bool {
    let mut t_min = 0.02f32;
    let mut t_max = 0.94f32;
    for i in 0..3 {
        let d = b[i] - a[i];
        if d.abs() < 1e-6 {
            if a[i] < min[i] || a[i] > max[i] {
                return false;
            }
        } else {
            let inv = 1.0 / d;
            let t1 = (min[i] - a[i]) * inv;
            let t2 = (max[i] - a[i]) * inv;
            t_min = t_min.max(t1.min(t2));
            t_max = t_max.min(t1.max(t2));
            if t_min > t_max {
                return false;
            }
        }
    }
    t_min <= t_max
}

fn segment_distance_to_aabb(a: [f32; 3], b: [f32; 3], min: [f32; 3], max: [f32; 3]) -> f32 {
    let mut best = f32::MAX;
    for s in 1..=5 {
        let t = s as f32 / 6.0;
        let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
        let dx = (min[0] - p[0]).max(0.0).max(p[0] - max[0]);
        let dy = (min[1] - p[1]).max(0.0).max(p[1] - max[1]);
        let dz = (min[2] - p[2]).max(0.0).max(p[2] - max[2]);
        best = best.min((dx * dx + dy * dy + dz * dz).sqrt());
    }
    best
}

// ---------------------------------------------------------------------------------------------------------------
// Camera & G-buffer depth capture hooks (pure read/CopyResource only — zero D3D pipeline state mutation mid-frame).

/// Maps a D3D11 depth-stencil texture format to `(typeless_copy_format, float_srv_format)` in the same typeless group
/// so `CopyResource` + pixel-shader `Texture2D<float>` sampling works without binding a DSV to the swapchain.
fn depth_srv_formats(src: DXGI_FORMAT) -> Option<(DXGI_FORMAT, DXGI_FORMAT)> {
    match src {
        DXGI_FORMAT_D32_FLOAT_S8X24_UINT | DXGI_FORMAT_R32G8X24_TYPELESS => {
            Some((DXGI_FORMAT_R32G8X24_TYPELESS, DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS))
        }
        DXGI_FORMAT_D24_UNORM_S8_UINT | DXGI_FORMAT_R24G8_TYPELESS => {
            Some((DXGI_FORMAT_R24G8_TYPELESS, DXGI_FORMAT_R24_UNORM_X8_TYPELESS))
        }
        DXGI_FORMAT_D32_FLOAT | DXGI_FORMAT_R32_TYPELESS => Some((DXGI_FORMAT_R32_TYPELESS, DXGI_FORMAT_R32_FLOAT)),
        DXGI_FORMAT_D16_UNORM | DXGI_FORMAT_R16_TYPELESS => Some((DXGI_FORMAT_R16_TYPELESS, DXGI_FORMAT_R16_UNORM)),
        _ => None,
    }
}

/// Snapshots the just-finished G-buffer depth texture into a typeless SRV texture via pure GPU `CopyResource`
/// (called immediately after `OMSetRenderTargets` unbinds the G-buffer; never mutates D3D pipeline state).
///
/// # Safety
/// Live immediate `context` and `dsv` on the game's render thread.
unsafe fn snapshot_gbuffer_depth(
    context: &ID3D11DeviceContext,
    dsv: &ID3D11DepthStencilView,
    vp_depth: (f32, f32),
) {
    // SAFETY: live D3D11 context, DSV, and device on the render thread.
    unsafe {
        let Ok(src_tex) = dsv.GetResource().and_then(|r| r.cast::<ID3D11Texture2D>()) else { return };
        let mut src_desc = D3D11_TEXTURE2D_DESC::default();
        src_tex.GetDesc(&mut src_desc);
        if src_desc.Width < 640
            || src_desc.Width <= src_desc.Height
            || src_desc.ArraySize != 1
            || src_desc.SampleDesc.Count != 1
        {
            return;
        }
        let Some((tex_fmt, srv_fmt)) = depth_srv_formats(src_desc.Format) else { return };
        let Ok(device) = context.GetDevice() else { return };
        let mut saved = SAVED_DEPTH.lock().unwrap_or_else(|e| e.into_inner());
        let needs_new = !matches!(
            &*saved,
            Some(s)
                if s.device == device
                    && s.width == src_desc.Width
                    && s.height == src_desc.Height
                    && s.src_format == src_desc.Format.0 as u32
        );
        if needs_new {
            let mut dst_desc = src_desc;
            dst_desc.Format = tex_fmt;
            dst_desc.Usage = D3D11_USAGE_DEFAULT;
            dst_desc.BindFlags = D3D11_BIND_SHADER_RESOURCE.0 as u32;
            dst_desc.CPUAccessFlags = 0;
            dst_desc.MiscFlags = 0;
            let mut copy_tex = None;
            if device.CreateTexture2D(&dst_desc, None, Some(&mut copy_tex)).is_err() {
                return;
            }
            let Some(copy_tex) = copy_tex else { return };
            let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: srv_fmt,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
                },
            };
            let mut copy_srv = None;
            if device.CreateShaderResourceView(&copy_tex, Some(&srv_desc), Some(&mut copy_srv)).is_err() {
                return;
            }
            let Some(copy_srv) = copy_srv else { return };
            *saved = Some(SavedDepth {
                device,
                width: src_desc.Width,
                height: src_desc.Height,
                src_format: src_desc.Format.0 as u32,
                min_depth: vp_depth.0,
                max_depth: vp_depth.1,
                texture: copy_tex,
                srv: copy_srv,
                captured_present: 0,
                copies_this_frame: 0,
            });
        }
        if let Some(s) = saved.as_mut() {
            context.CopyResource(&s.texture, &src_tex);
            s.min_depth = vp_depth.0;
            s.max_depth = vp_depth.1;
            let target_present = PRESENTS.load(Ordering::Relaxed) + 1;
            if s.captured_present != target_present {
                s.captured_present = target_present;
                s.copies_this_frame = 1;
            } else {
                s.copies_this_frame += 1;
            }
        }
    }
}

/// `OMSetRenderTargets*` (before original): tracks the main 3D G-buffer pass and queues its depth texture for copy.
///
/// # Safety
/// The game's live context and arguments: `count` views, each live or null; `depth` live or null.
pub unsafe fn on_set_render_targets(context: *mut c_void, count: u32, views: *const *mut c_void, depth: *mut c_void) {
    if IN_DRAW.load(Ordering::Relaxed) || crate::dirt_render::in_draw() {
        return;
    }
    let was_gbuffer = IN_MAIN_GBUFFER.swap(false, Ordering::Relaxed);
    let pass_blocks = PASS_WORLD_BLOCKS.swap(0, Ordering::Relaxed);
    let prev_dsv = if was_gbuffer { ACTIVE_GBUFFER_DSV.lock().unwrap_or_else(|e| e.into_inner()).0.take() } else { None };
    let pass_vp = *PASS_VIEWPORT_DEPTH.lock().unwrap_or_else(|e| e.into_inner());

    if !ACTIVE.load(Ordering::Relaxed) {
        return;
    }

    // SAFETY: live immediate context on the game's render thread.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return;
        }
        if was_gbuffer && pass_blocks >= 3 && let Some(prev) = prev_dsv {
            *PENDING_VIEWPORT_DEPTH.lock().unwrap_or_else(|e| e.into_inner()) = pass_vp;
            PENDING_DEPTH_COPY.lock().unwrap_or_else(|e| e.into_inner()).0 = Some(prev);
        }
        if views.is_null() || count < GBUFFER_TARGETS || count > 8 || depth.is_null() {
            return;
        }
        let Some(view) = ID3D11RenderTargetView::from_raw_borrowed(&*views) else { return };
        let Ok(texture) = view.GetResource().and_then(|r| r.cast::<ID3D11Texture2D>()) else { return };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc);
        if desc.Width >= 640 && desc.Width > desc.Height && desc.ArraySize == 1 {
            IN_MAIN_GBUFFER.store(true, Ordering::Relaxed);
            *PASS_VIEWPORT_DEPTH.lock().unwrap_or_else(|e| e.into_inner()) = (0.0, 1.0);
            if let Some(c) = crate::camera::position() {
                *CAM_AT_GBUFFER.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some([f64::from(c[0]), f64::from(c[1]), f64::from(c[2])]);
            }
            if let Some(dsv) = ID3D11DepthStencilView::from_raw_borrowed(&depth) {
                ACTIVE_GBUFFER_DSV.lock().unwrap_or_else(|e| e.into_inner()).0 = Some(dsv.clone());
            }
        }
    }
}

/// `OMSetRenderTargets*` (immediately after original): executes any queued G-buffer depth copy now that the G-buffer
/// render targets have been unbound. Pure `CopyResource` only — zero D3D pipeline state mutation.
///
/// # Safety
/// The game's live context on the render thread.
pub unsafe fn after_set_render_targets(context: *mut c_void) {
    if IN_DRAW.load(Ordering::Relaxed) || crate::dirt_render::in_draw() {
        return;
    }
    let Some(prev) = PENDING_DEPTH_COPY.lock().unwrap_or_else(|e| e.into_inner()).0.take() else { return };
    let vp_depth = *PENDING_VIEWPORT_DEPTH.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: live immediate context on the game's render thread.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return;
        }
        snapshot_gbuffer_depth(context, &prev, vp_depth);
    }
}

/// `VSSetConstantBuffers1`: votes for the main camera `(R_view, P)` and locks each vehicle unit's GPU `MV`.
///
/// # Safety
/// The game's live context and arguments: `count` buffers, and as many firsts and counts if not null.
pub unsafe fn on_vs_constant_buffers1(
    context: *mut c_void,
    start: u32,
    count: u32,
    buffers: *const *mut c_void,
    firsts: *const u32,
    counts: *const u32,
) {
    if !ACTIVE.load(Ordering::Relaxed) || !IN_MAIN_GBUFFER.load(Ordering::Relaxed) {
        return;
    }
    if start != 0 || count == 0 || buffers.is_null() || firsts.is_null() {
        return;
    }
    // SAFETY: the game's first buffer and its range (the caller's contract).
    let (raw, first, constants) = unsafe { (*buffers, *firsts, if counts.is_null() { 0 } else { *counts }) };
    if raw.is_null() || !(8..=MAX_BLOCK_CONSTANTS).contains(&constants) {
        return;
    }
    if READS.fetch_add(1, Ordering::Relaxed) >= MAX_READS {
        return;
    }
    // SAFETY: live context and buffer, bound range >= 8 constants (32 floats).
    let numbers = unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return;
        }
        let Some(buffer) = ID3D11Buffer::from_raw_borrowed(&raw) else { return };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        if context.Map(buffer, 0, D3D11_MAP_WRITE_NO_OVERWRITE, 0, Some(&mut mapped)).is_err() {
            return;
        }
        let mut numbers = [0f32; 32];
        std::ptr::copy_nonoverlapping((mapped.pData as *const f32).add(first as usize * 4), numbers.as_mut_ptr(), 32);
        context.Unmap(buffer, 0);
        numbers
    };
    let Some((rot, proj)) = extract_view_and_proj(&numbers) else { return };
    let block_idx = PASS_WORLD_BLOCKS.fetch_add(1, Ordering::Relaxed);
    if block_idx == 1 {
        // Read the exact viewport `(MinDepth, MaxDepth)` used by the game's 3D G-buffer draws (ETS2 uses `[0.01, 1.0]`).
        // SAFETY: live immediate context on the render thread; pure read-only query.
        unsafe {
            if let Some(ctx) = ID3D11DeviceContext::from_raw_borrowed(&context) {
                let mut n = 1u32;
                let mut vp = D3D11_VIEWPORT::default();
                ctx.RSGetViewports(&mut n, Some(&mut vp));
                if n >= 1 && vp.MaxDepth > vp.MinDepth {
                    *PASS_VIEWPORT_DEPTH.lock().unwrap_or_else(|e| e.into_inner()) = (vp.MinDepth, vp.MaxDepth);
                }
            }
        }
    }
    let mv = rows(&numbers, 0);

    let key = rot_key(&rot, 200.0);
    WORLD_VOTES.lock().unwrap_or_else(|e| e.into_inner()).entry(key).or_insert((0, rot, proj)).0 += 1;

    let axes_list = VEHICLE_AXES.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if !axes_list.is_empty() {
        let mut seen = Vec::with_capacity(axes_list.len());
        let mut v_votes = VEHICLE_VOTES.lock().unwrap_or_else(|e| e.into_inner());
        for axes in axes_list {
            let cand = mul3(&rot, &axes);
            if cand[1][1] < -0.1 {
                continue;
            }
            let ckey = rot_key(&cand, 100.0);
            if !seen.contains(&ckey) {
                seen.push(ckey);
                v_votes.entry(ckey).or_insert((0, cand, proj)).0 += 1;
            }
        }
    }

    // Match this block directly against each vehicle unit's origin (`MV_trans ≈ R_view * (origin - cam)`)
    // so wheel smoke locks pixel-perfectly to the GPU's exact `MV` matrix for this frame with zero wobble.
    if let Some(cam_approx) = *CAM_AT_GBUFFER.lock().unwrap_or_else(|e| e.into_inner()) {
        let poses = *POSES_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
        let t = [mv[0][3], mv[1][3], mv[2][3]];
        let mut mv_votes = UNIT_MV_VOTES.lock().unwrap_or_else(|e| e.into_inner());
        for (u, pose) in poses.iter().enumerate() {
            let Some(pose) = pose else { continue };
            let cand = mul3(&rot, &pose.axes);
            if cand[1][1] < -0.1 {
                continue;
            }
            let off = [
                cand[0][0] * t[0] + cand[1][0] * t[1] + cand[2][0] * t[2],
                cand[0][1] * t[0] + cand[1][1] * t[1] + cand[2][1] * t[2],
                cand[0][2] * t[0] + cand[1][2] * t[1] + cand[2][2] * t[2],
            ];
            let cam_imp = [
                pose.origin[0] - f64::from(off[0]),
                pose.origin[1] - f64::from(off[1]),
                pose.origin[2] - f64::from(off[2]),
            ];
            let err_sq = (cam_imp[0] - cam_approx[0]).powi(2)
                + (cam_imp[1] - cam_approx[1]).powi(2)
                + (cam_imp[2] - cam_approx[2]).powi(2);
            if err_sq < 1.5 * 1.5 {
                let tkey = [
                    (t[0] * 25.0).round() as i32,
                    (t[1] * 25.0).round() as i32,
                    (t[2] * 25.0).round() as i32,
                ];
                mv_votes[u].entry(tkey).or_insert((0, mv, cand, cam_imp)).0 += 1;
            }
        }
    }
}

fn rot_key(rot: &M3, scale: f32) -> [i32; 6] {
    std::array::from_fn(|i| (rot[i / 3][i % 3] * scale).round() as i32)
}

fn rot_close(a: &M3, b: &M3, tol: f32) -> bool {
    (0..3).all(|r| (0..3).all(|c| (a[r][c] - b[r][c]).abs() < tol))
}

/// Extracts `(R_mv, P)` from a 32-float `b0` block (`MV` in rows 0..3, `PMV` in rows 4..7), rejecting screen-space
/// passes and short-far-plane cab-interior projections.
fn extract_view_and_proj(numbers: &[f32; 32]) -> Option<(M3, M4)> {
    let mv = rows(numbers, 0);
    if mv[3] != [0.0, 0.0, 0.0, 1.0] {
        return None;
    }
    let rot: M3 = std::array::from_fn(|r| [mv[r][0], mv[r][1], mv[r][2]]);
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    if !rot.iter().all(|r| (dot(*r, *r) - 1.0).abs() < 0.02) {
        return None;
    }
    if dot(rot[0], rot[1]).abs() > 0.02 || dot(rot[1], rot[2]).abs() > 0.02 {
        return None;
    }
    let det = rot[0][0] * (rot[1][1] * rot[2][2] - rot[1][2] * rot[2][1])
        - rot[0][1] * (rot[1][0] * rot[2][2] - rot[1][2] * rot[2][0])
        + rot[0][2] * (rot[1][0] * rot[2][1] - rot[1][1] * rot[2][0]);
    if det < 0.9 || rot[1][1] < -0.1 {
        return None;
    }
    let identity = (0..3).all(|r| (0..3).all(|c| (rot[r][c] - if r == c { 1.0 } else { 0.0 }).abs() < 1e-4));
    if identity {
        return None;
    }
    let inv = affine_inverse(&mv)?;
    let proj = product(&rows(numbers, 4), &inv);
    if (proj[3][2] + 1.0).abs() >= 0.05 || proj[0][0] <= 0.2 || proj[1][1] <= 0.2 {
        return None;
    }
    let z_ndc = |z: f32| (proj[2][2] * z + proj[2][3]) / (-z);
    let (zn2, zn50) = (z_ndc(-2.0), z_ndc(-50.0));
    if !(0.0..=1.0).contains(&zn2) || !(0.0..=1.0).contains(&zn50) {
        return None;
    }
    Some((rot, proj))
}

struct CameraPick {
    rot: M3,
    proj: M4,
    source: &'static str,
}

fn take_camera(presents: u64) -> Option<CameraPick> {
    let mut w_votes = WORLD_VOTES.lock().unwrap_or_else(|e| e.into_inner());
    let mut v_votes = VEHICLE_VOTES.lock().unwrap_or_else(|e| e.into_inner());
    READS.store(0, Ordering::Relaxed);

    let last = *LAST_CAMERA.lock().unwrap_or_else(|e| e.into_inner());
    let recent_rot = last.filter(|(_, _, at)| presents.saturating_sub(*at) <= 6).map(|(r, _, _)| r);
    let continuous = |rot: &M3| recent_rot.is_some_and(|prev| rot_close(rot, &prev, 0.30));

    let w_list: Vec<(u32, M3, M4)> = w_votes.values().copied().collect();

    let agreed = w_list
        .iter()
        .filter(|(wc, _, _)| *wc >= 2)
        .filter_map(|&(wc, wrot, wproj)| {
            let vc: u32 = v_votes
                .values()
                .filter(|(_, vrot, _)| rot_close(&wrot, vrot, 0.06))
                .map(|(c, _, _)| *c)
                .sum();
            (vc >= 2).then_some((continuous(&wrot), wc + vc, wrot, wproj))
        })
        .max_by_key(|(cont, score, _, _)| (*cont, *score))
        .map(|(_, _, rot, proj)| (rot, proj, "agreed"));

    let by_world = agreed.or_else(|| {
        w_list
            .iter()
            .filter(|(c, _, _)| *c >= MIN_VIEW_VOTES)
            .max_by_key(|(c, rot, _)| (continuous(rot), *c))
            .map(|&(_, rot, proj)| (rot, proj, "world"))
    });

    let chosen = by_world.or_else(|| {
        v_votes
            .values()
            .filter(|(c, rot, _)| *c >= MIN_VIEW_VOTES && (recent_rot.is_none() || continuous(rot)))
            .max_by_key(|(c, _, _)| *c)
            .map(|&(_, rot, proj)| (rot, proj, "vehicle"))
    });

    w_votes.clear();
    v_votes.clear();

    if let Some((rot, proj, source)) = chosen {
        *LAST_CAMERA.lock().unwrap_or_else(|e| e.into_inner()) = Some((rot, proj, presents));
        return Some(CameraPick { rot, proj, source });
    }
    last.filter(|(_, _, at)| presents.saturating_sub(*at) <= 3).map(|(rot, proj, _)| CameraPick {
        rot,
        proj,
        source: "held",
    })
}

fn take_unit_mvs(rot: &M3) -> ([Option<M4>; 5], Option<[f64; 3]>) {
    let mut all = UNIT_MV_VOTES.lock().unwrap_or_else(|e| e.into_inner());
    let mut out = [None; 5];
    let mut best_cam: Option<(u32, [f64; 3])> = None;
    for (u, map) in all.iter_mut().enumerate() {
        let winner = map
            .values()
            .filter(|(c, _, cand, _)| *c >= 2 && rot_close(cand, rot, 0.08))
            .max_by_key(|(c, _, _, _)| *c)
            .copied();
        map.clear();
        if let Some((count, mv, _, cam_imp)) = winner {
            out[u] = Some(mv);
            if best_cam.is_none_or(|(bc, _)| count > bc) {
                best_cam = Some((count, cam_imp));
            }
        }
    }
    (out, best_cam.map(|(_, c)| c))
}

fn clear_unit_mvs() {
    let mut all = UNIT_MV_VOTES.lock().unwrap_or_else(|e| e.into_inner());
    for map in all.iter_mut() {
        map.clear();
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Wheel-concentrated vehicle-local particle simulation with subtle, natural wind drift.

#[derive(Clone, Copy, PartialEq, Eq)]
struct UnitId(u8);

#[derive(Clone, Copy)]
struct Emitter {
    unit: UnitId,
    local: [f32; 3],
    side: f32,
}

#[derive(Clone, Copy)]
struct Particle {
    unit: UnitId,
    local_pos: [f32; 3],
    local_vel: [f32; 3],
    age: f32,
    life: f32,
    r_start: f32,
    r_end: f32,
    angle: f32,
    spin: f32,
    opacity: f32,
    darkness: f32,
    seed: f32,
}

struct Sim {
    particles: Vec<Particle>,
    truck_accum: f32,
    trailer_accum: f32,
    wheel_cursor_truck: usize,
    wheel_cursor_trailer: usize,
    rng: u32,
    last_tick: Option<Instant>,
}

impl Default for Sim {
    fn default() -> Self {
        Self {
            particles: Vec::with_capacity(MAX_PARTICLES),
            truck_accum: 0.0,
            trailer_accum: 0.0,
            wheel_cursor_truck: 0,
            wheel_cursor_trailer: 0,
            rng: 0x9e37_79b9,
            last_tick: None,
        }
    }
}

impl Sim {
    fn rand01(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32) * (1.0 / (u32::MAX as f32))
    }

    fn rand_range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.rand01()
    }

    fn clear(&mut self) {
        self.particles.clear();
        self.truck_accum = 0.0;
        self.trailer_accum = 0.0;
        self.last_tick = None;
    }

    fn step(
        &mut self,
        dt: f32,
        truck_intensity: f32,
        trailer_intensity: f32,
        truck_wheels: &[Emitter],
        trailer_wheels: &[Emitter],
        speed: f32,
    ) {
        let lat_drag = (-2.2 * dt).exp();
        let wind_back = (speed * 0.085).clamp(-1.4, 2.2);
        let mut i = 0;
        while i < self.particles.len() {
            let p = &mut self.particles[i];
            p.age += dt;
            if p.age >= p.life {
                self.particles.swap_remove(i);
                continue;
            }
            p.local_pos[0] += p.local_vel[0] * dt;
            p.local_pos[1] += p.local_vel[1] * dt;
            p.local_pos[2] += p.local_vel[2] * dt;
            p.local_vel[0] *= lat_drag;
            p.local_vel[2] += (wind_back - p.local_vel[2]) * (2.2 * dt).min(1.0);
            let target_rise = (0.34 + 0.34 * p.darkness) / (1.0 + speed.abs() * 0.035);
            p.local_vel[1] += (target_rise - p.local_vel[1]) * (2.2 * dt).min(1.0);
            p.angle += p.spin * dt;
            i += 1;
        }

        if truck_intensity > 0.0 && !truck_wheels.is_empty() {
            let per_wheel = 2.6 + 9.5 * truck_intensity * truck_intensity.sqrt();
            let rate = (per_wheel * truck_wheels.len() as f32).min(100.0);
            self.truck_accum = (self.truck_accum + rate * dt).min(12.0);
            while self.truck_accum >= 1.0 {
                self.truck_accum -= 1.0;
                let idx = self.wheel_cursor_truck % truck_wheels.len();
                self.wheel_cursor_truck = self.wheel_cursor_truck.wrapping_add(1);
                self.spawn(truck_wheels[idx], truck_intensity, wind_back);
            }
        } else {
            self.truck_accum = 0.0;
        }

        if trailer_intensity > 0.0 && !trailer_wheels.is_empty() {
            let per_wheel = 2.6 + 9.5 * trailer_intensity * trailer_intensity.sqrt();
            let rate = (per_wheel * trailer_wheels.len() as f32).min(150.0);
            self.trailer_accum = (self.trailer_accum + rate * dt).min(14.0);
            while self.trailer_accum >= 1.0 {
                self.trailer_accum -= 1.0;
                let idx = self.wheel_cursor_trailer % trailer_wheels.len();
                self.wheel_cursor_trailer = self.wheel_cursor_trailer.wrapping_add(1);
                self.spawn(trailer_wheels[idx], trailer_intensity, wind_back);
            }
        } else {
            self.trailer_accum = 0.0;
        }
    }

    fn spawn(&mut self, emitter: Emitter, intensity: f32, wind_back: f32) {
        let theta = self.rand_range(0.0, std::f32::consts::TAU);
        let ring = self.rand_range(0.06, 0.28);
        let local_pos = [
            emitter.local[0] + emitter.side * self.rand_range(-0.04, 0.08),
            emitter.local[1] + theta.sin() * ring + self.rand_range(0.0, 0.10),
            emitter.local[2] + theta.cos() * ring,
        ];
        // Subtle outward curl clears the outer rim/wheel-arch lip so hardware depth testing keeps the puff crisp.
        let out_push = self.rand_range(0.22, 0.46) * (0.7 + 0.4 * intensity);
        let local_vel = [
            emitter.side * out_push + self.rand_range(-0.08, 0.08),
            self.rand_range(0.16, 0.42) * (0.75 + 0.45 * intensity),
            wind_back * self.rand_range(0.65, 1.25) + self.rand_range(-0.18, 0.24),
        ];
        let life = self.rand_range(0.95, 1.65) * (0.85 + 0.30 * intensity);
        let r_start = self.rand_range(0.20, 0.32) * (0.88 + 0.22 * intensity);
        let r_end = self.rand_range(0.55, 0.92) * (0.85 + 0.35 * intensity);
        let opacity = (0.14 + 0.32 * intensity) * self.rand_range(0.82, 1.18);
        let p = Particle {
            unit: emitter.unit,
            local_pos,
            local_vel,
            age: 0.0,
            life: life.max(0.55),
            r_start,
            r_end,
            angle: self.rand_range(0.0, std::f32::consts::TAU),
            spin: self.rand_range(-0.75, 0.75),
            opacity: opacity.min(0.48),
            darkness: intensity,
            seed: self.rand_range(0.0, 100.0),
        };
        if self.particles.len() < MAX_PARTICLES {
            self.particles.push(p);
        } else if let Some((oldest, _)) =
            self.particles.iter().enumerate().max_by(|(_, a), (_, b)| a.age.total_cmp(&b.age))
        {
            self.particles[oldest] = p;
        }
    }
}

static SIM: LazyLock<Mutex<Sim>> = LazyLock::new(|| Mutex::new(Sim::default()));

// ---------------------------------------------------------------------------------------------------------------
// D3D11 instanced smoke shader & drawing (pixel-shader SRV soft depth testing against snapshotted G-buffer depth).

const SHADERS: &str = r"
cbuffer Camera : register(b0) {
    row_major float4x4 proj;
    float4 depth_params;
    float4 depth_extra;
};
cbuffer Particles : register(b1) {
    float4 pos_radius[384];
    float4 meta[384];
};
Texture2D<float> scene_depth : register(t0);

struct Vertex {
    float4 position   : SV_Position;
    float2 uv         : TEXCOORD0;
    float4 meta       : TEXCOORD1;
    float2 depth_info : TEXCOORD2;
};

Vertex vs(uint vid : SV_VertexID, uint iid : SV_InstanceID) {
    float2 corner = float2((vid & 1) ? 1.0 : -1.0, (vid >> 1) ? -1.0 : 1.0);
    float4 pr = pos_radius[iid];
    float4 m  = meta[iid];
    float s = sin(m.y);
    float c = cos(m.y);
    float2 rot = float2(corner.x * c - corner.y * s, corner.x * s + corner.y * c);
    float4 view_pos = float4(pr.xy + rot * pr.w, pr.z, 1.0);
    Vertex v;
    v.position = mul(proj, view_pos);
    v.uv = corner;
    v.meta = m;
    // True linear distance (-view_z) plus gentle volumetric wrap bias around the outer tyre/rim.
    v.depth_info = float2(-pr.z, min(pr.w * 0.65, 0.45));
    return v;
}

float hash21(float2 p) {
    p = frac(p * float2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return frac(p.x * p.y);
}

float noise(float2 p) {
    float2 i = floor(p);
    float2 f = frac(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = hash21(i);
    float b = hash21(i + float2(1.0, 0.0));
    float c = hash21(i + float2(0.0, 1.0));
    float d = hash21(i + float2(1.0, 1.0));
    return lerp(lerp(a, b, f.x), lerp(c, d, f.x), f.y);
}

float fbm(float2 p) {
    float v = 0.0;
    float amp = 0.52;
    v += amp * noise(p); p *= 2.03; amp *= 0.5;
    v += amp * noise(p); p *= 2.01; amp *= 0.5;
    v += amp * noise(p); p *= 2.02; amp *= 0.5;
    v += amp * noise(p);
    return v;
}

float4 ps(Vertex v) : SV_Target {
    float age = v.meta.x;
    float opacity = v.meta.z;
    float intensity = v.meta.w;
    float r = length(v.uv);
    if (r >= 1.0) discard;

    float2 seed_off = float2(v.meta.y * 1.7 + age * 0.55, v.meta.y * 2.3 - age * 0.75);
    float warp = fbm(v.uv * 1.65 + seed_off);
    float n = fbm(v.uv * 2.65 + seed_off * 1.3 + float2(warp - 0.5, 0.5 - warp) * 0.65);
    float radial = saturate(1.0 - r * r);
    float edge = r + (n - 0.5) * 0.55;
    float shape = (1.0 - smoothstep(0.04, 0.94, edge)) * radial;
    float cloud = shape * (0.58 + 0.62 * n);

    float fade_in  = smoothstep(0.0, 0.16, age);
    float fade_out = 1.0 - smoothstep(0.32, 1.0, age);
    float alpha = saturate(cloud * fade_in * fade_out * opacity);
    if (alpha < 0.004) discard;

    if (depth_params.x > 0.5) {
        int2 max_px = int2(int(depth_params.w), int(depth_extra.x));
        int2 px = clamp(int2(v.position.xy * depth_params.yz), int2(0, 0), max_px);
        float raw_d = scene_depth.Load(int3(px, 0));
        float vp_min = depth_extra.y;
        float vp_span = max(depth_extra.z, 1e-5);
        float scene_dist;
        if (depth_params.x < 1.5) {
            // Reversed-Z (ETS2 uses MinDepth = 0.01, MaxDepth = 1.0; sky is cleared to 0.0 < vp_min).
            if (raw_d <= vp_min + 1e-6) {
                scene_dist = 1e6;
            } else {
                float ndc_d = (raw_d - vp_min) / vp_span;
                scene_dist = proj[2][3] / max(ndc_d + proj[2][2], 1e-5);
            }
        } else if (depth_params.x < 2.5) {
            float ndc_d = saturate((raw_d - vp_min) / vp_span);
            scene_dist = proj[2][3] / max((1.0 - ndc_d) + proj[2][2], 1e-5);
        } else {
            float ndc_d = saturate((raw_d - vp_min) / vp_span);
            scene_dist = (-proj[2][3]) / max((-proj[2][2]) - ndc_d, 1e-5);
        }
        float diff = scene_dist - v.depth_info.x;
        float soft = smoothstep(-0.08, 0.35, diff + v.depth_info.y);
        alpha *= soft;
        if (alpha < 0.004) discard;
    }

    // Natural smoke palette:
    // - Orange (intensity 0.08..0.40, 290..360 C): airy light whitish-grey wisps
    // - Red    (intensity 0.40..0.75, 360..400 C): fuller medium-dark grey
    // - Very hot / no brakes (0.75..1.0, 400..425+ C): dark smoky charcoal (both solo truck & trailer)
    float mid_t  = smoothstep(0.12, 0.52, intensity);
    float hot_t  = smoothstep(0.48, 0.88, intensity);
    float lit = saturate(0.56 - v.uv.y * 0.28 + (n - 0.5) * 0.32);
    float3 light_col = lerp(float3(0.86, 0.87, 0.89), float3(0.50, 0.49, 0.51), mid_t);
    light_col = lerp(light_col, float3(0.24, 0.23, 0.24), hot_t);
    float3 dark_col  = lerp(float3(0.62, 0.64, 0.67), float3(0.26, 0.25, 0.27), mid_t);
    dark_col = lerp(dark_col, float3(0.10, 0.10, 0.11), hot_t);
    float3 col = lerp(dark_col, light_col, lit);
    return float4(col * alpha, alpha);
}
";

const PROBE_SHADER: &str = r"
cbuffer ProbeCoords : register(b0) {
    uint4 coords[4];
};
Texture2D<float> scene_depth : register(t0);

float4 probe_vs(uint vid : SV_VertexID) : SV_Position {
    float2 p = float2((vid & 1) ? 1.0 : -1.0, (vid >> 1) ? -1.0 : 1.0);
    return float4(p, 0.0, 1.0);
}

float probe_ps(float4 pos : SV_Position) : SV_Target {
    uint idx = min(uint(pos.x), 3u);
    return scene_depth.Load(int3(coords[idx].xy, 0));
}
";

#[repr(C)]
#[derive(Clone, Copy)]
struct CameraConstants {
    proj: M4,
    depth_params: [f32; 4],
    depth_extra: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ProbeCoords {
    coords: [[u32; 4]; 4],
}

/// Classifies the snapshotted depth buffer from 4 probed pixels `[top_center, top_left, bottom_road, p0]`:
/// - `0`: constant/empty/invalid depth buffer (fall back to analytical OBB occlusion so smoke never disappears)
/// - `1`: Reversed-Z matching `proj` (`0 = far, 1 = near`, `proj[2][3] > 0`)
/// - `2`: Standard-Z `1 - z_rev` (`1 = far, 0 = near`, while `proj[2][3] > 0`)
/// - `3`: Standard-Z matching `proj` (`1 = far, 0 = near`, `proj[2][3] <= 0`)
fn classify_depth_mode(samples: [f32; 4], proj_z_offset: f32) -> u32 {
    if !samples.iter().all(|v| v.is_finite()) {
        return 0;
    }
    let [d_top, d_corner, d_road, d_p0] = samples;
    let min_d = d_top.min(d_corner).min(d_road).min(d_p0);
    let max_d = d_top.max(d_corner).max(d_road).max(d_p0);
    if (max_d - min_d) < 1e-6 {
        return 0;
    }
    let std_mode = if proj_z_offset > 0.0 { 2 } else { 3 };
    if d_top < 0.5 && d_corner < 0.5 && d_road < 0.5 {
        1
    } else if d_top > 0.5 && d_corner > 0.5 && d_road > 0.5 {
        std_mode
    } else if d_top <= d_road {
        1
    } else {
        std_mode
    }
}

fn depth_mode_name(mode: u32) -> &'static str {
    match mode {
        1 => "rev-z",
        2 => "inv-z",
        3 => "std-z",
        _ => "off",
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ParticleConstants {
    pos_radius: [[f32; 4]; MAX_PARTICLES],
    meta: [[f32; 4]; MAX_PARTICLES],
}

struct Drawing {
    device: ID3D11Device,
    state: ID3DDeviceContextState,
    vertex: ID3D11VertexShader,
    pixel: ID3D11PixelShader,
    probe_vs: ID3D11VertexShader,
    probe_ps: ID3D11PixelShader,
    camera_buf: ID3D11Buffer,
    particle_buf: ID3D11Buffer,
    probe_buf: ID3D11Buffer,
    probe_rt: ID3D11Texture2D,
    probe_rtv: ID3D11RenderTargetView,
    probe_staging: ID3D11Texture2D,
    probe_pending: u64,
    probe_last_present: u64,
    probe_samples: Option<[f32; 4]>,
    blend: ID3D11BlendState,
    rasterizer: ID3D11RasterizerState,
    depth_off: ID3D11DepthStencilState,
}

// SAFETY: used only on the game's render thread, through `DRAWING`.
unsafe impl Send for Drawing {}

static DRAWING: Mutex<Option<Result<Drawing, ()>>> = Mutex::new(None);
static FAILURES: AtomicU64 = AtomicU64::new(0);

impl Drawing {
    /// # Safety
    /// Live D3D11 device on the game's render thread.
    unsafe fn new(device: &ID3D11Device) -> windows::core::Result<Self> {
        // SAFETY: local descriptors, static shader source, live device.
        unsafe {
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
            let vs_blob = compile_src(SHADERS, s!("vs"), s!("vs_4_0"))?;
            let ps_blob = compile_src(SHADERS, s!("ps"), s!("ps_4_0"))?;
            let pvs_blob = compile_src(PROBE_SHADER, s!("probe_vs"), s!("vs_4_0"))?;
            let pps_blob = compile_src(PROBE_SHADER, s!("probe_ps"), s!("ps_4_0"))?;
            let mut vertex = None;
            device.CreateVertexShader(blob_bytes(&vs_blob), None, Some(&mut vertex))?;
            let mut pixel = None;
            device.CreatePixelShader(blob_bytes(&ps_blob), None, Some(&mut pixel))?;
            let mut probe_vs = None;
            device.CreateVertexShader(blob_bytes(&pvs_blob), None, Some(&mut probe_vs))?;
            let mut probe_ps = None;
            device.CreatePixelShader(blob_bytes(&pps_blob), None, Some(&mut probe_ps))?;

            let mut camera_buf = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: std::mem::size_of::<CameraConstants>() as u32,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut camera_buf),
            )?;
            let mut particle_buf = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: std::mem::size_of::<ParticleConstants>() as u32,
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut particle_buf),
            )?;
            let mut probe_buf = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: std::mem::size_of::<ProbeCoords>() as u32,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut probe_buf),
            )?;
            let mut probe_rt = None;
            device.CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: 4,
                    Height: 1,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_R32_FLOAT,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut probe_rt),
            )?;
            let probe_rt = probe_rt.unwrap();
            let mut probe_rtv = None;
            device.CreateRenderTargetView(&probe_rt, None, Some(&mut probe_rtv))?;
            let mut probe_staging = None;
            device.CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: 4,
                    Height: 1,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_R32_FLOAT,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Usage: D3D11_USAGE_STAGING,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut probe_staging),
            )?;

            let mut blend_desc = D3D11_BLEND_DESC::default();
            blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: true.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            let mut blend = None;
            device.CreateBlendState(&blend_desc, Some(&mut blend))?;
            let mut rasterizer = None;
            device.CreateRasterizerState(
                &D3D11_RASTERIZER_DESC {
                    FillMode: D3D11_FILL_SOLID,
                    CullMode: D3D11_CULL_NONE,
                    DepthClipEnable: false.into(),
                    ..Default::default()
                },
                Some(&mut rasterizer),
            )?;
            let mut depth_off = None;
            device.CreateDepthStencilState(
                &D3D11_DEPTH_STENCIL_DESC {
                    DepthEnable: false.into(),
                    DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
                    DepthFunc: D3D11_COMPARISON_LESS_EQUAL,
                    ..Default::default()
                },
                Some(&mut depth_off),
            )?;
            log!("brake smoke: ready to be drawn");
            Ok(Self {
                device: device.clone(),
                state: state.unwrap(),
                vertex: vertex.unwrap(),
                pixel: pixel.unwrap(),
                probe_vs: probe_vs.unwrap(),
                probe_ps: probe_ps.unwrap(),
                camera_buf: camera_buf.unwrap(),
                particle_buf: particle_buf.unwrap(),
                probe_buf: probe_buf.unwrap(),
                probe_rt,
                probe_rtv: probe_rtv.unwrap(),
                probe_staging: probe_staging.unwrap(),
                probe_pending: 0,
                probe_last_present: 0,
                probe_samples: None,
                blend: blend.unwrap(),
                rasterizer: rasterizer.unwrap(),
                depth_off: depth_off.unwrap(),
            })
        }
    }

    /// Reads back the 4×1 depth probe if a previous probe finished at least 2 frames ago (zero GPU stall).
    ///
    /// # Safety
    /// Live immediate context of this device on the render thread.
    unsafe fn poll_probe(&mut self, context: &ID3D11DeviceContext, presents: u64, force_sync: bool) {
        if self.probe_pending == 0 || (!force_sync && presents < self.probe_pending + 2) {
            return;
        }
        // SAFETY: `probe_staging` is a 4×1 R32_FLOAT staging texture on this device.
        unsafe {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            if context.Map(&self.probe_staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).is_ok() {
                let vals = std::slice::from_raw_parts(mapped.pData as *const f32, 4);
                self.probe_samples = Some([vals[0], vals[1], vals[2], vals[3]]);
                context.Unmap(&self.probe_staging, 0);
            }
            self.probe_pending = 0;
        }
    }

    /// Draws `count` sorted view-space particles into `rtv` (`width` × `height`), sampling `depth_srv` in the pixel
    /// shader for smooth soft-particle occlusion when `depth_mode != 0`. Optionally issues a 4×1 probe pass.
    ///
    /// # Safety
    /// Live immediate context, render target, and optional depth SRV of this device; `count <= MAX_PARTICLES`.
    #[allow(clippy::too_many_arguments)]
    unsafe fn render_into(
        &mut self,
        context: &ID3D11DeviceContext,
        rtv: &ID3D11RenderTargetView,
        width: u32,
        height: u32,
        proj: &M4,
        data: &ParticleConstants,
        count: u32,
        depth_srv: Option<(&ID3D11ShaderResourceView, u32, u32, f32, f32)>,
        depth_mode: u32,
        probe_coords: Option<ProbeCoords>,
        presents: u64,
    ) -> windows::core::Result<()> {
        if count == 0 {
            return Ok(());
        }
        // SAFETY: live device/context/views; DeviceContextState restores the game's D3D state cleanly.
        unsafe {
            let context1: ID3D11DeviceContext1 = context.cast()?;
            let mut previous = None;
            IN_DRAW.store(true, Ordering::Relaxed);
            context1.SwapDeviceContextState(&self.state, Some(&mut previous));

            // Optional 4×1 depth probe (runs at most once per ~1 s or on first frame with depth SRV).
            if let (Some((srv, _, _, _, _)), Some(coords)) = (depth_srv, probe_coords) {
                context.UpdateSubresource(&self.probe_buf, 0, None, &coords as *const ProbeCoords as *const c_void, 0, 0);
                let vp = D3D11_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: 4.0,
                    Height: 1.0,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                };
                context.RSSetViewports(Some(&[vp]));
                context.RSSetState(&self.rasterizer);
                context.OMSetRenderTargets(Some(&[Some(self.probe_rtv.clone())]), None);
                context.OMSetDepthStencilState(&self.depth_off, 0);
                context.OMSetBlendState(None, None, u32::MAX);
                context.IASetInputLayout(None);
                context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
                context.VSSetShader(&self.probe_vs, None);
                context.PSSetShader(&self.probe_ps, None);
                context.PSSetConstantBuffers(0, Some(&[Some(self.probe_buf.clone())]));
                context.PSSetShaderResources(0, Some(&[Some(srv.clone())]));
                context.Draw(4, 0);
                context.OMSetRenderTargets(Some(&[None]), None);
                context.PSSetShaderResources(0, Some(&[None]));
                context.CopyResource(&self.probe_staging, &self.probe_rt);
                self.probe_pending = presents;
                self.probe_last_present = presents;
            }

            let cam_consts = if let Some((_, dw, dh, vp_min, vp_max)) = depth_srv.filter(|_| depth_mode != 0) {
                CameraConstants {
                    proj: *proj,
                    depth_params: [
                        depth_mode as f32,
                        dw as f32 / width.max(1) as f32,
                        dh as f32 / height.max(1) as f32,
                        dw.saturating_sub(1) as f32,
                    ],
                    depth_extra: [dh.saturating_sub(1) as f32, vp_min, (vp_max - vp_min).max(1e-5), 0.0],
                }
            } else {
                CameraConstants { proj: *proj, depth_params: [0.0; 4], depth_extra: [0.0; 4] }
            };
            context.UpdateSubresource(&self.camera_buf, 0, None, &cam_consts as *const CameraConstants as *const c_void, 0, 0);
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            if context.Map(&self.particle_buf, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped)).is_ok() {
                std::ptr::copy_nonoverlapping(
                    data as *const ParticleConstants as *const u8,
                    mapped.pData as *mut u8,
                    std::mem::size_of::<ParticleConstants>(),
                );
                context.Unmap(&self.particle_buf, 0);
            }

            let viewport = D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: width as f32,
                Height: height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            };
            context.RSSetViewports(Some(&[viewport]));
            context.RSSetState(&self.rasterizer);
            context.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
            context.OMSetDepthStencilState(&self.depth_off, 0);
            context.OMSetBlendState(&self.blend, None, u32::MAX);
            context.IASetInputLayout(None);
            context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            context.VSSetShader(&self.vertex, None);
            context.VSSetConstantBuffers(0, Some(&[Some(self.camera_buf.clone()), Some(self.particle_buf.clone())]));
            context.PSSetShader(&self.pixel, None);
            context.PSSetConstantBuffers(0, Some(&[Some(self.camera_buf.clone())]));
            if let Some((srv, _, _, _, _)) = depth_srv.filter(|_| depth_mode != 0) {
                context.PSSetShaderResources(0, Some(&[Some(srv.clone())]));
            }
            context.DrawInstanced(4, count, 0, 0);
            context.PSSetShaderResources(0, Some(&[None]));
            context.OMSetRenderTargets(Some(&[None]), None);

            if let Some(previous) = previous {
                context1.SwapDeviceContextState(&previous, None);
            }
            IN_DRAW.store(false, Ordering::Relaxed);
            Ok(())
        }
    }

    /// Draws `count` sorted view-space particles into the swapchain backbuffer, sampling `depth_srv` in the pixel
    /// shader for smooth depth occlusion.
    ///
    /// # Safety
    /// Live swapchain on the game's render thread; `count <= MAX_PARTICLES`.
    #[allow(clippy::too_many_arguments)]
    unsafe fn draw(
        &mut self,
        swapchain: &IDXGISwapChain,
        proj: &M4,
        data: &ParticleConstants,
        count: u32,
        depth_srv: Option<(&ID3D11ShaderResourceView, u32, u32, f32, f32)>,
        depth_mode: u32,
        p0_view: Option<[f32; 3]>,
        presents: u64,
    ) -> windows::core::Result<()> {
        if count == 0 {
            return Ok(());
        }
        // SAFETY: live device/context/swapchain on the render thread.
        unsafe {
            let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut desc);
            let mut rtv = None;
            self.device.CreateRenderTargetView(&back, None, Some(&mut rtv))?;
            let rtv = rtv.unwrap();

            let context: ID3D11DeviceContext = self.device.GetImmediateContext()?;
            self.poll_probe(&context, presents, false);

            let want_probe = depth_srv.is_some()
                && self.probe_pending == 0
                && (self.probe_samples.is_none() || presents.saturating_sub(self.probe_last_present) >= 60);
            let probe_coords = if want_probe && let Some((_, dw, dh, _, _)) = depth_srv {
                let p0_px = p0_view
                    .and_then(|v| {
                        let cw = proj[3][2] * v[2];
                        if cw <= 0.05 {
                            return None;
                        }
                        let nx = (proj[0][0] * v[0] + proj[0][2] * v[2]) / cw;
                        let ny = (proj[1][1] * v[1] + proj[1][2] * v[2]) / cw;
                        let px = ((nx * 0.5 + 0.5) * dw as f32).clamp(0.0, dw.saturating_sub(1) as f32) as u32;
                        let py = ((0.5 - ny * 0.5) * dh as f32).clamp(0.0, dh.saturating_sub(1) as f32) as u32;
                        Some([px, py])
                    })
                    .unwrap_or([dw / 2, dh / 2]);
                Some(ProbeCoords {
                    coords: [
                        [dw / 2, 8.min(dh.saturating_sub(1)), 0, 0],
                        [8.min(dw.saturating_sub(1)), 8.min(dh.saturating_sub(1)), 0, 0],
                        [dw / 2, dh.saturating_sub(16), 0, 0],
                        [p0_px[0], p0_px[1], 0, 0],
                    ],
                })
            } else {
                None
            };

            self.render_into(
                &context,
                &rtv,
                desc.Width,
                desc.Height,
                proj,
                data,
                count,
                depth_srv,
                depth_mode,
                probe_coords,
                presents,
            )
        }
    }
}

/// Projects vehicle-local particles into a camera's view space, using each unit's exact GPU `MV` when locked.
/// When `use_obb_fallback` is true (e.g. ETS2 resolution scaling != 100% where hardware DSV cannot be bound),
/// applies analytical OBB fading against the vehicle bodies.
fn project_particles(
    particles: &[Particle],
    poses: &[Option<VehiclePose>; 5],
    unit_mvs: &[Option<M4>; 5],
    boxes: &[BodyBox],
    cam: [f64; 3],
    rot: &M3,
    use_obb_fallback: bool,
) -> Vec<([f32; 4], [f32; 4])> {
    particles
        .iter()
        .filter_map(|p| {
            let u = p.unit.0 as usize;
            let pose = poses.get(u).and_then(|o| *o)?;
            let mut vis = 1.0f32;
            if use_obb_fallback {
                let world = pose.local_to_world(p.local_pos);
                for b in boxes {
                    vis = vis.min(b.visibility(cam, world));
                    if vis <= 0.01 {
                        return None;
                    }
                }
            }
            let (vx, vy, vz) = if let Some(mv) = unit_mvs.get(u).and_then(|m| *m) {
                let [lx, ly, lz] = p.local_pos;
                (
                    mv[0][0] * lx + mv[0][1] * ly + mv[0][2] * lz + mv[0][3],
                    mv[1][0] * lx + mv[1][1] * ly + mv[1][2] * lz + mv[1][3],
                    mv[2][0] * lx + mv[2][1] * ly + mv[2][2] * lz + mv[2][3],
                )
            } else {
                let world = pose.local_to_world(p.local_pos);
                let dx = (world[0] - cam[0]) as f32;
                let dy = (world[1] - cam[1]) as f32;
                let dz = (world[2] - cam[2]) as f32;
                (
                    rot[0][0] * dx + rot[0][1] * dy + rot[0][2] * dz,
                    rot[1][0] * dx + rot[1][1] * dy + rot[1][2] * dz,
                    rot[2][0] * dx + rot[2][1] * dy + rot[2][2] * dz,
                )
            };
            if vz > -0.35 || vz < -160.0 {
                return None;
            }
            let age01 = (p.age / p.life).clamp(0.0, 1.0);
            let radius = p.r_start + (p.r_end - p.r_start) * age01.sqrt();
            Some(([vx, vy, vz, radius], [age01, p.angle + p.seed, p.opacity * vis, p.darkness]))
        })
        .collect()
}

/// Called each `Present` before UI overlays: updates smoke particles and renders them into exterior 3D views.
///
/// # Safety
/// Live swapchain on the game's render thread.
pub unsafe fn on_present(swapchain: *mut c_void) {
    let presents = PRESENTS.fetch_add(1, Ordering::Relaxed) + 1;
    ACTIVE_GBUFFER_DSV.lock().unwrap_or_else(|e| e.into_inner()).0 = None;
    PENDING_DEPTH_COPY.lock().unwrap_or_else(|e| e.into_inner()).0 = None;

    let Some(bridge) = bridge::get().filter(|_| bridge::host_alive()) else {
        deactivate();
        return;
    };
    let flags = bridge.host.brake_hud.load(Ordering::Acquire);
    let smoke_enabled = (flags & 0x100) != 0 && (flags & 0x40) != 0;
    let paused = crate::telemetry::paused();
    let dirt_active = !paused && crate::dirt::has_visible_dirt();
    if !smoke_enabled && !dirt_active {
        deactivate();
        return;
    }

    let shift = level_shift(flags);
    let truck_temp = f32::from_bits(bridge.host.brake_temp_truck.load(Ordering::Acquire));
    let trailer_temp = f32::from_bits(bridge.host.brake_temp_trailer.load(Ordering::Acquire));
    let truck_intensity = if smoke_enabled { smoke_intensity(truck_temp - shift) } else { 0.0 };
    let trailer_intensity = if smoke_enabled { smoke_intensity(trailer_temp - shift) } else { 0.0 };

    let mut sim = SIM.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    let dt = sim
        .last_tick
        .replace(now)
        .map_or(1.0 / 60.0, |prev| now.saturating_duration_since(prev).as_secs_f32().clamp(0.001, 0.05));

    let truck = crate::telemetry::truck();
    let trailer = crate::telemetry::trailer();
    let extras = crate::telemetry::extra_trailers();

    let (poses, boxes, truck_wheels, trailer_wheels, trailer_unit_count) =
        collect_vehicles(&truck, &trailer, &extras, truck_intensity, trailer_intensity);
    let truck_wheel_count = truck_wheels.len();
    let trailer_wheel_count = trailer_wheels.len();

    if !paused && (truck_intensity > 0.0 || trailer_intensity > 0.0 || !sim.particles.is_empty()) {
        sim.step(dt, truck_intensity, trailer_intensity, &truck_wheels, &trailer_wheels, truck.speed);
    }

    let still_active =
        !paused && (dirt_active || truck_intensity > 0.0 || trailer_intensity > 0.0 || !sim.particles.is_empty());
    ACTIVE.store(still_active, Ordering::Relaxed);

    if still_active {
        update_vehicle_axes(&truck, &trailer, &extras);
        *POSES_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = poses;
    } else {
        *SAVED_DEPTH.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    if paused || (sim.particles.is_empty() && !dirt_active) {
        WORLD_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        VEHICLE_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        clear_unit_mvs();
        READS.store(0, Ordering::Relaxed);
        return;
    }

    let cam_now = crate::camera::position().map(|c| [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])]);
    let cam_gb = *CAM_AT_GBUFFER.lock().unwrap_or_else(|e| e.into_inner());
    let particle_count = sim.particles.len();

    // Never draw main-view smoke inside the cab over the steering wheel or dashboard.
    if let Some(c) = cam_now
        && camera_inside_cab(c, &truck)
    {
        drop(sim);
        WORLD_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        VEHICLE_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        clear_unit_mvs();
        READS.store(0, Ordering::Relaxed);
        *LAST_CAMERA.lock().unwrap_or_else(|e| e.into_inner()) = None;
        return;
    }

    let Some(pick) = take_camera(presents) else {
        drop(sim);
        clear_unit_mvs();
        let mut diag = DIAG_START.lock().unwrap_or_else(|e| e.into_inner());
        diag.2 += 1;
        if diag.0.elapsed() >= Duration::from_secs(3) {
            let (hits, misses) = (diag.1, diag.2);
            *diag = (Instant::now(), 0, 0);
            drop(diag);
            log!(
                "brake smoke: camera miss (hits={hits}, miss={misses}, cam={}) | truck {truck_temp:.0}C ({truck_intensity:.2}, {truck_wheel_count}w) trailer {trailer_temp:.0}C ({trailer_intensity:.2}, {trailer_wheel_count}w/{trailer_unit_count}t) | p={particle_count}",
                cam_now.is_some()
            );
        }
        return;
    };
    let (rot, proj) = (pick.rot, pick.proj);
    let (unit_mvs, cam_gpu) = take_unit_mvs(&rot);
    let locked_units = unit_mvs.iter().filter(|m| m.is_some()).count();
    let Some(cam) = cam_gpu.or(cam_gb).or(cam_now) else { return };

    // Use the SRV snapshot of the main 3D G-buffer depth copied right after G-buffer exit this frame.
    let saved_depth = {
        let saved = SAVED_DEPTH.lock().unwrap_or_else(|e| e.into_inner());
        saved.as_ref().filter(|s| s.captured_present == presents).map(|s| {
            (s.srv.clone(), s.width, s.height, s.src_format, s.copies_this_frame, s.min_depth, s.max_depth)
        })
    };

    // Determine active depth mode from our zero-stall 4×1 depth probe (falls back to OBB if probe detects empty/constant depth).
    let (probe_samples, depth_mode) = {
        let drawing = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        let samples = match drawing.as_ref() {
            Some(Ok(made)) => made.probe_samples,
            _ => None,
        };
        let mode = if saved_depth.is_some() {
            match samples {
                Some(s) => classify_depth_mode(s, proj[2][3]),
                None => if proj[2][3] > 0.0 { 1 } else { 3 },
            }
        } else {
            0
        };
        (samples, mode)
    };

    let mut projected = project_particles(&sim.particles, &poses, &unit_mvs, &boxes, cam, &rot, depth_mode == 0);
    drop(sim);

    // Phase 2 + Phase 3: render deferred 3D dirt decals onto truck & coupled trailers before brake smoke.
    if dirt_active
        && let Some((ref depth_srv, _, _, _, _, z_min, z_max)) = saved_depth
    {
        let dirt_units = crate::dirt::all_units();
        let mut draws: Vec<crate::dirt_render::DirtUnitDraw> = Vec::with_capacity(5);
        for (idx, body) in boxes.iter().enumerate().take(5) {
            let Some(mv) = unit_mvs[idx] else { continue };
            let dirt = dirt_units[idx];
            if dirt.total() <= 0.015 {
                continue;
            }
            // Slightly pad the tight interior OBB outward to encompass fenders, skirts, fuel tanks, and wheels.
            let pad_x = 0.28f32;
            let ground_y = body.min[1] - 0.35;
            draws.push(crate::dirt_render::DirtUnitDraw {
                mv,
                obb_min: [body.min[0] - pad_x, ground_y, body.min[2] - 0.25],
                obb_max: [body.max[0] + pad_x, body.max[1] + 0.35, body.max[2] + 0.25],
                ground_y,
                dirt,
            });
        }
        if !draws.is_empty() {
            let game_min = bridge.plugin.telemetry.read().map_or(720u32, |t| t.game_time % 1440);
            let hour = game_min as f32 / 60.0;
            // Smooth daylight curve: 0.20 at night, 1.0 between 08:00 and 18:00.
            let daylight = if (8.0..=18.0).contains(&hour) {
                1.0
            } else if (5.0..8.0).contains(&hour) {
                0.20 + 0.80 * ((hour - 5.0) / 3.0)
            } else if (18.0..21.0).contains(&hour) {
                1.0 - 0.80 * ((hour - 18.0) / 3.0)
            } else {
                0.20
            };
            // SAFETY: live swapchain on the game's render thread inside Present.
            unsafe {
                if let Some(sc) = IDXGISwapChain::from_raw_borrowed(&swapchain) {
                    let scene = crate::dirt_render::DirtScene {
                        proj,
                        depth_srv,
                        z_min,
                        z_max,
                        daylight,
                        units: &draws,
                    };
                    crate::dirt_render::render_into(sc, &scene);
                }
            }
        }
    }

    let first_view = projected.first().map(|(pr, _)| [pr[0], pr[1], pr[2]]);
    let projected_count = projected.len();

    if !projected.is_empty() {
        projected.sort_by(|a, b| a.0[2].total_cmp(&b.0[2]));
        let count = projected.len().min(MAX_PARTICLES);
        let mut constants = ParticleConstants { pos_radius: [[0.0; 4]; MAX_PARTICLES], meta: [[0.0; 4]; MAX_PARTICLES] };
        for (i, (pr, m)) in projected.into_iter().take(count).enumerate() {
            constants.pos_radius[i] = pr;
            constants.meta[i] = m;
        }
        let srv_arg = saved_depth.as_ref().map(|(srv, w, h, _, _, zmin, zmax)| (srv, *w, *h, *zmin, *zmax));

        // SAFETY: live swapchain on the game's render thread.
        unsafe {
            let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
            let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
            let mut drawing = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
            if matches!(&*drawing, Some(Ok(made)) if made.device != device) {
                *drawing = None;
            }
            if drawing.is_none() {
                *drawing = Some(Drawing::new(&device).map_err(|e| log!("brake smoke: can't be drawn: {e}")));
            }
            if let Some(Ok(made)) = drawing.as_mut()
                && let Err(e) =
                    made.draw(swapchain, &proj, &constants, count as u32, srv_arg, depth_mode, first_view, presents)
            {
                let n = FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
                if n <= 3 {
                    log!("brake smoke: draw failed ({n}): {e}");
                }
            }
        }
    }

    let mut diag = DIAG_START.lock().unwrap_or_else(|e| e.into_inner());
    diag.1 += 1;
    if diag.0.elapsed() >= Duration::from_secs(3) {
        let (hits, misses) = (diag.1, diag.2);
        *diag = (Instant::now(), 0, 0);
        drop(diag);
        let depth_desc = match saved_depth.as_ref() {
            Some((_, w, h, fmt, copies, zmin, zmax)) => format!("{w}x{h}f{fmt}x{copies}[{zmin:.3}..{zmax:.2}]"),
            None => String::from("none"),
        };
        let probe_desc = match probe_samples {
            Some([a, b, c, d]) => format!("[{a:.4},{b:.4},{c:.4},{d:.4}]"),
            None => String::from("pending"),
        };
        log!(
            "brake smoke: truck {truck_temp:.0}C ({truck_intensity:.2}, {truck_wheel_count}w) trailer {trailer_temp:.0}C ({trailer_intensity:.2}, {trailer_wheel_count}w/{trailer_unit_count}t) | p={particle_count} vis={projected_count} locked={locked_units} | cam={hits}/{} src={} depth={depth_desc}({}) probe={probe_desc} p0={first_view:.1?}",
            hits + misses,
            pick.source,
            depth_mode_name(depth_mode),
        );
    }
}

fn deactivate() {
    ACTIVE.store(false, Ordering::Relaxed);
    SIM.lock().unwrap_or_else(|e| e.into_inner()).clear();
    WORLD_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
    VEHICLE_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
    clear_unit_mvs();
    *CAM_AT_GBUFFER.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *SAVED_DEPTH.lock().unwrap_or_else(|e| e.into_inner()) = None;
    PENDING_DEPTH_COPY.lock().unwrap_or_else(|e| e.into_inner()).0 = None;
    READS.store(0, Ordering::Relaxed);
}

fn update_vehicle_axes(
    truck: &crate::telemetry::Truck,
    trailer: &crate::telemetry::Trailer,
    extras: &[crate::telemetry::ExtraTrailer; crate::telemetry::EXTRA_TRAILERS_COUNT],
) {
    let mut axes = Vec::with_capacity(10);
    if truck.position != [0.0; 3] {
        for (ps, rs) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            axes.push(tilted_axes(truck.heading, truck.pitch * ps, truck.roll * rs));
        }
    }
    if trailer.connected && trailer.placed && trailer.position != [0.0; 3] {
        axes.push(tilted_axes(trailer.heading, trailer.pitch, trailer.roll));
        axes.push(tilted_axes(trailer.heading, -trailer.pitch, -trailer.roll));
    }
    for extra in extras {
        if extra.connected && extra.placed && extra.position != [0.0; 3] {
            axes.push(tilted_axes(extra.heading, extra.pitch, extra.roll));
        }
    }
    *VEHICLE_AXES.lock().unwrap_or_else(|e| e.into_inner()) = axes;
}

/// Builds live vehicle poses, fallback OBBs, and wheel emitters for truck & all coupled trailers.
fn collect_vehicles(
    truck: &crate::telemetry::Truck,
    trailer: &crate::telemetry::Trailer,
    extras: &[crate::telemetry::ExtraTrailer; crate::telemetry::EXTRA_TRAILERS_COUNT],
    truck_intensity: f32,
    trailer_intensity: f32,
) -> ([Option<VehiclePose>; 5], Vec<BodyBox>, Vec<Emitter>, Vec<Emitter>, usize) {
    let vehicles = crate::reversing::VEHICLES.lock().unwrap_or_else(|e| e.into_inner());
    let mut poses: [Option<VehiclePose>; 5] = [None; 5];
    let mut boxes = Vec::with_capacity(5);
    let mut truck_out = Vec::new();
    let mut trailer_out = Vec::new();
    let mut trailer_units = 0usize;

    // Unit 0: tractor.
    if truck.position != [0.0; 3] {
        let pose = VehiclePose::new(truck.position, truck.heading, truck.pitch, truck.roll);
        poses[0] = Some(pose);
        let n = vehicles.truck_wheels.min(crate::reversing::MOST_WHEELS);
        let max_x = vehicles.truck_wheel_at.iter().take(n).map(|w| w[0].abs()).fold(0.85f32, f32::max);
        let min_z = vehicles.truck.front_axle_z.min(-1.5) - 1.0;
        let max_z = vehicles.truck.rear_z.max(3.0);
        let g = vehicles.truck.ground_y;
        boxes.push(BodyBox {
            pose,
            min: [-(max_x - 0.16).max(0.50), g + 0.35, min_z],
            max: [(max_x - 0.16).max(0.50), g + 3.5, max_z],
        });
        if truck_intensity > 0.0 {
            for at in vehicles.truck_wheel_at.iter().take(n) {
                let side = if at[0] >= 0.0 { 1.0 } else { -1.0 };
                let local = [at[0] + side * 0.16, at[1] + 0.05, at[2]];
                truck_out.push(Emitter { unit: UnitId(0), local, side });
            }
        }
    }

    // Unit 1: first trailer (`trailer.0`).
    if trailer.connected && trailer.placed && trailer.position != [0.0; 3] {
        let pose = VehiclePose::new(trailer.position, trailer.heading, trailer.pitch, trailer.roll);
        poses[1] = Some(pose);
        let n = vehicles.trailer_wheels.min(crate::reversing::MOST_WHEELS);
        let max_x = vehicles.trailer_wheel_at.iter().take(n).map(|w| w[0].abs()).fold(0.85f32, f32::max);
        let min_z = vehicles.trailer.hook_z.min(-2.0) - 0.8;
        let max_z = vehicles.trailer.rear_z.max(3.0);
        let g = vehicles.trailer.ground_y;
        boxes.push(BodyBox {
            pose,
            min: [-(max_x - 0.14).max(0.50), g + 0.35, min_z],
            max: [(max_x - 0.14).max(0.50), g + 3.6, max_z],
        });
        if trailer_intensity > 0.0 && n > 0 {
            trailer_units += 1;
            for at in vehicles.trailer_wheel_at.iter().take(n) {
                let side = if at[0] >= 0.0 { 1.0 } else { -1.0 };
                let local = [at[0] + side * 0.18, at[1] + 0.05, at[2]];
                trailer_out.push(Emitter { unit: UnitId(1), local, side });
            }
        }
    }
    drop(vehicles);

    // Units 2..=4: additional trailers (`trailer.1`..`trailer.3`, e.g. bitrem / rodotrem).
    for (idx, extra) in extras.iter().enumerate() {
        if extra.connected && extra.placed && extra.position != [0.0; 3] && extra.wheel_count > 0 {
            let unit = UnitId((idx + 2) as u8);
            let pose = VehiclePose::new(extra.position, extra.heading, extra.pitch, extra.roll);
            poses[unit.0 as usize] = Some(pose);
            let n = extra.wheel_count.min(crate::reversing::MOST_WHEELS);
            let max_x = extra.wheel_at.iter().take(n).map(|w| w[0].abs()).fold(0.85f32, f32::max);
            let min_wz = extra.wheel_at.iter().take(n).map(|w| w[2]).fold(f32::INFINITY, f32::min);
            let max_wz = extra.wheel_at.iter().take(n).map(|w| w[2]).fold(f32::NEG_INFINITY, f32::max);
            let avg_y = extra.wheel_at.iter().take(n).map(|w| w[1]).sum::<f32>() / n as f32;
            let g = (avg_y - 0.50).min(0.0);
            boxes.push(BodyBox {
                pose,
                min: [-(max_x - 0.14).max(0.50), g + 0.35, min_wz - 2.0],
                max: [(max_x - 0.14).max(0.50), g + 3.6, max_wz + 1.8],
            });
            if trailer_intensity > 0.0 {
                trailer_units += 1;
                for at in extra.wheel_at.iter().take(n) {
                    let side = if at[0] >= 0.0 { 1.0 } else { -1.0 };
                    let local = [at[0] + side * 0.18, at[1] + 0.05, at[2]];
                    trailer_out.push(Emitter { unit, local, side });
                }
            }
        }
    }

    (poses, boxes, truck_out, trailer_out, trailer_units)
}

// ---------------------------------------------------------------------------------------------------------------
// Matrix & shader compile helpers.

fn rows(numbers: &[f32; 32], first: usize) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| numbers[(first + r) * 4 + c]))
}

fn product(a: &M4, b: &M4) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| a[r][k] * b[k][c]).sum()))
}

fn mul3(a: &M3, b: &M3) -> M3 {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

fn affine_inverse(m: &M4) -> Option<M4> {
    let [a, b, c] = [m[0][0], m[0][1], m[0][2]];
    let [d, e, f] = [m[1][0], m[1][1], m[1][2]];
    let [g, h, i] = [m[2][0], m[2][1], m[2][2]];
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-9 {
        return None;
    }
    let r = [
        [(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det],
        [(f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det],
        [(d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det],
    ];
    let t = [m[0][3], m[1][3], m[2][3]];
    let mut out = [[0.0; 4]; 4];
    for row in 0..3 {
        out[row][..3].copy_from_slice(&r[row]);
        out[row][3] = -(r[row][0] * t[0] + r[row][1] * t[1] + r[row][2] * t[2]);
    }
    out[3][3] = 1.0;
    Some(out)
}

fn compile_src(source: &str, entry: PCSTR, target: PCSTR) -> windows::core::Result<ID3DBlob> {
    let mut code = None;
    let mut errors = None;
    // SAFETY: `source`, `entry`, and `target` are static slices/C-strings.
    let result = unsafe {
        D3DCompile(
            source.as_ptr() as *const c_void,
            source.len(),
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
    if let Err(err) = result {
        if let Some(blob) = errors {
            let text = String::from_utf8_lossy(blob_bytes(&blob));
            log!("brake smoke: shader compile error: {text}");
        }
        return Err(err);
    }
    Ok(code.unwrap())
}

fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
    // SAFETY: valid D3D blob returned by `D3DCompile`.
    unsafe { std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_shaders_compile() {
        compile_src(SHADERS, s!("vs"), s!("vs_4_0")).expect("vertex shader compiles");
        compile_src(SHADERS, s!("ps"), s!("ps_4_0")).expect("pixel shader compiles");
        compile_src(PROBE_SHADER, s!("probe_vs"), s!("vs_4_0")).expect("probe vertex shader compiles");
        compile_src(PROBE_SHADER, s!("probe_ps"), s!("ps_4_0")).expect("probe pixel shader compiles");
    }

    #[test]
    fn classifies_reversed_standard_and_empty_depth_modes() {
        // Reversed-Z (0 = far sky, >0 = closer road/truck)
        assert_eq!(classify_depth_mode([0.0, 0.0, 0.04, 0.015], 0.20), 1);
        // Inverted/Standard-Z with reversed-Z proj (1 = far sky, <1 = closer road/truck)
        assert_eq!(classify_depth_mode([1.0, 1.0, 0.96, 0.985], 0.20), 2);
        // Standard-Z with standard-Z proj
        assert_eq!(classify_depth_mode([1.0, 1.0, 0.96, 0.985], -0.20), 3);
        // Constant/empty depth buffer -> disable depth test (fall back to OBB)
        assert_eq!(classify_depth_mode([0.0, 0.0, 0.0, 0.0], 0.20), 0);
        assert_eq!(classify_depth_mode([1.0, 1.0, 1.0, 1.0], 0.20), 0);
    }

    #[test]
    fn smoke_intensity_follows_orange_and_red_thresholds() {
        assert_eq!(smoke_intensity(f32::NAN), 0.0);
        assert_eq!(smoke_intensity(25.0), 0.0);
        assert_eq!(smoke_intensity(289.9), 0.0);
        let orange_start = smoke_intensity(290.0);
        let orange_mid = smoke_intensity(325.0);
        let red_start = smoke_intensity(360.0);
        let red_full = smoke_intensity(425.0);
        assert!(orange_start > 0.0 && orange_start < 0.15, "orange_start={orange_start}");
        assert!(orange_mid > orange_start && orange_mid < red_start, "orange_mid={orange_mid}");
        assert!(red_start > 0.38 && red_start < 0.45, "red_start={red_start}");
        assert!((red_full - 1.0).abs() < 1e-4, "red_full={red_full}");
        assert_eq!(smoke_intensity(350.0 - level_shift(1 << 9)), 0.0);
        assert!(smoke_intensity(395.0 - level_shift(1 << 9)) > 0.0);
        assert_eq!(smoke_intensity(235.0 - level_shift(2 << 9)), 0.0);
        assert!(smoke_intensity(245.0 - level_shift(2 << 9)) > 0.0);
        assert!(smoke_intensity(320.0 - level_shift(2 << 9)) > 0.40);
    }

    #[test]
    fn trailer_smoke_stays_close_with_gentle_wind_drift() {
        let mut sim = Sim::default();
        let truck_wheels = [Emitter { unit: UnitId(0), local: [1.1, 0.5, 2.0], side: 1.0 }];
        let trailer_wheels = [
            Emitter { unit: UnitId(1), local: [1.1, 0.5, 3.0], side: 1.0 },
            Emitter { unit: UnitId(1), local: [-1.1, 0.5, 3.0], side: -1.0 },
        ];
        for _ in 0..40 {
            sim.step(0.05, 0.0, smoke_intensity(380.0), &truck_wheels, &trailer_wheels, 15.0);
        }
        assert!(!sim.particles.is_empty(), "trailer in red should emit smoke");
        assert!(sim.particles.iter().all(|p| p.unit == UnitId(1)), "only trailer wheels should emit");
        let avg_z = sim.particles.iter().map(|p| p.local_pos[2]).sum::<f32>() / sim.particles.len() as f32;
        assert!(avg_z > 3.1 && avg_z < 5.5, "gentle backward wind drift expected around wheel (avg_z={avg_z})");
    }

    #[test]
    fn obb_occludes_far_side_wheels_while_keeping_near_side_visible() {
        let pose = VehiclePose::new([0.0, 0.0, 0.0], 0.0, 0.0, 0.0);
        let body = BodyBox { pose, min: [-0.85, 0.3, -4.0], max: [0.85, 3.5, 5.0] };
        let cam_left = [-5.0, 1.5, 0.0];
        let near_left_wheel = [-1.25, 0.6, 2.5];
        let far_right_wheel = [1.25, 0.6, 2.5];
        assert!(body.visibility(cam_left, near_left_wheel) > 0.9, "near-side left wheel must be clearly visible");
        assert_eq!(body.visibility(cam_left, far_right_wheel), 0.0, "far-side right wheel must be occluded behind trailer");
    }

    #[test]
    fn extracts_world_camera_and_rejects_short_range_cab_projection() {
        let c = std::f32::consts::FRAC_1_SQRT_2;
        let mv: M4 = [[c, 0.0, c, 5.0], [0.0, 1.0, 0.0, -1.0], [-c, 0.0, c, -12.0], [0.0, 0.0, 0.0, 1.0]];
        let p_world: M4 = [[1.2, 0.0, 0.0, 0.0], [0.0, 2.1, 0.0, 0.0], [0.0, 0.0, 0.000125, 0.25], [0.0, 0.0, -1.0, 0.0]];
        let pmv = product(&p_world, &mv);
        let mut buf = [0.0f32; 32];
        for r in 0..4 {
            for col in 0..4 {
                buf[r * 4 + col] = mv[r][col];
                buf[(r + 4) * 4 + col] = pmv[r][col];
            }
        }
        let (rot, proj) = extract_view_and_proj(&buf).expect("valid world camera block");
        assert!((rot[0][0] - c).abs() < 1e-4);
        assert!((proj[0][0] - 1.2).abs() < 1e-4);

        let p_cab: M4 = [[1.2, 0.0, 0.0, 0.0], [0.0, 2.1, 0.0, 0.0], [0.0, 0.0, 0.00673, 0.1006], [0.0, 0.0, -1.0, 0.0]];
        let pmv_cab = product(&p_cab, &mv);
        for r in 0..4 {
            for col in 0..4 {
                buf[(r + 4) * 4 + col] = pmv_cab[r][col];
            }
        }
        assert!(extract_view_and_proj(&buf).is_none(), "cab-interior projection should be rejected");
    }

    #[test]
    fn truck_submeshes_cannot_hijack_world_view_rotation() {
        WORLD_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        VEHICLE_VOTES.lock().unwrap_or_else(|e| e.into_inner()).clear();
        *LAST_CAMERA.lock().unwrap_or_else(|e| e.into_inner()) = None;

        let a = 30.0f32.to_radians();
        let r_view: M3 = [[a.cos(), 0.0, a.sin()], [0.0, 1.0, 0.0], [-a.sin(), 0.0, a.cos()]];
        let axes = tilted_axes(110.0 / 360.0, 0.0, 0.0);
        let r_truck: M3 = std::array::from_fn(|r| std::array::from_fn(|c| axes[c][r]));
        let r_mv_truck = mul3(&r_view, &r_truck);
        let proj: M4 = [[1.2, 0.0, 0.0, 0.0], [0.0, 2.1, 0.0, 0.0], [0.0, 0.0, 0.000125, 0.25], [0.0, 0.0, -1.0, 0.0]];

        {
            let mut wv = WORLD_VOTES.lock().unwrap_or_else(|e| e.into_inner());
            let mut vv = VEHICLE_VOTES.lock().unwrap_or_else(|e| e.into_inner());
            wv.insert(rot_key(&r_mv_truck, 200.0), (100, r_mv_truck, proj));
            wv.insert(rot_key(&r_view, 200.0), (12, r_view, proj));
            let undone = mul3(&r_mv_truck, &axes);
            vv.insert(rot_key(&undone, 100.0), (100, undone, proj));
        }

        let pick = take_camera(10).expect("camera resolved");
        assert_eq!(pick.source, "agreed");
        assert!(rot_close(&pick.rot, &r_view, 1e-3), "must pick true R_view, not truck's R_view*R_truck");
    }

    #[test]
    fn it_renders_and_depth_occludes_smoke_on_warp() {
        use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP;
        use windows::Win32::Graphics::Direct3D11::{
            D3D11_BIND_DEPTH_STENCIL, D3D11_CLEAR_DEPTH, D3D11_CREATE_DEVICE_FLAG, D3D11CreateDevice,
        };
        use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

        // SAFETY: self-contained WARP software D3D11 device & offscreen textures for unit testing.
        unsafe {
            let mut device = None;
            let mut context = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .expect("WARP device created");
            let (device, context): (ID3D11Device, ID3D11DeviceContext) = (device.unwrap(), context.unwrap());
            let mut drawing = Drawing::new(&device).expect("Drawing initialized");

            let color_size = 128u32;
            let depth_size = 256u32; // Different size from color target to verify ETS2 resolution scaling works!
            let mut color_tex = None;
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: color_size,
                        Height: color_size,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
                        ..Default::default()
                    },
                    None,
                    Some(&mut color_tex),
                )
                .unwrap();
            let color_tex = color_tex.unwrap();
            let mut rtv = None;
            device.CreateRenderTargetView(&color_tex, None, Some(&mut rtv)).unwrap();
            let rtv = rtv.unwrap();

            // ETS2's real G-buffer depth format: D32_FLOAT_S8X24_UINT with BIND_DEPTH_STENCIL only.
            let mut depth_tex = None;
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: depth_size,
                        Height: depth_size,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_D32_FLOAT_S8X24_UINT,
                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_DEPTH_STENCIL.0 as u32,
                        ..Default::default()
                    },
                    None,
                    Some(&mut depth_tex),
                )
                .unwrap();
            let depth_tex = depth_tex.unwrap();
            let mut dsv = None;
            device.CreateDepthStencilView(&depth_tex, None, Some(&mut dsv)).unwrap();
            let dsv = dsv.unwrap();

            // Copy into typeless SRV texture using the exact same format mapping as `snapshot_gbuffer_depth`.
            let (tex_fmt, srv_fmt) = depth_srv_formats(DXGI_FORMAT_D32_FLOAT_S8X24_UINT).unwrap();
            let mut depth_copy = None;
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: depth_size,
                        Height: depth_size,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: tex_fmt,
                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                        ..Default::default()
                    },
                    None,
                    Some(&mut depth_copy),
                )
                .unwrap();
            let depth_copy = depth_copy.unwrap();
            let mut depth_srv = None;
            device
                .CreateShaderResourceView(
                    &depth_copy,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: srv_fmt,
                        ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
                        },
                    }),
                    Some(&mut depth_srv),
                )
                .unwrap();
            let depth_srv = depth_srv.unwrap();

            let mut staging = None;
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: color_size,
                        Height: color_size,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        Usage: D3D11_USAGE_STAGING,
                        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                        ..Default::default()
                    },
                    None,
                    Some(&mut staging),
                )
                .unwrap();
            let staging = staging.unwrap();

            let proj: M4 = [[1.5, 0.0, 0.0, 0.0], [0.0, 1.5, 0.0, 0.0], [0.0, 0.0, 0.00012, 0.20], [0.0, 0.0, -1.0, 0.0]];
            let mut constants =
                ParticleConstants { pos_radius: [[0.0; 4]; MAX_PARTICLES], meta: [[0.0; 4]; MAX_PARTICLES] };
            constants.pos_radius[0] = [0.0, 0.0, -10.0, 1.2];
            constants.meta[0] = [0.25, 0.5, 0.44, 0.9];

            let read_center_alpha = |ctx: &ID3D11DeviceContext| -> u8 {
                ctx.CopyResource(&staging, &color_tex);
                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                ctx.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).unwrap();
                let row = (color_size / 2) as usize * mapped.RowPitch as usize;
                let px = std::slice::from_raw_parts(
                    (mapped.pData as *const u8).add(row + (color_size / 2) as usize * 4),
                    4,
                );
                let alpha = px[3];
                ctx.Unmap(&staging, 0);
                alpha
            };

            let probe = ProbeCoords {
                coords: [[128, 4, 0, 0], [4, 4, 0, 0], [128, 250, 0, 0], [128, 128, 0, 0]],
            };

            // Case 1: ETS2 real MinDepth=0.01..MaxDepth=1.0!
            // Smoke at 24.3 m: wheel depth = 0.01 + 0.99 * ((0.20 - 0.00012*24.3)/24.3) = 0.01803 -> VISIBLE!
            constants.pos_radius[0] = [0.0, 0.0, -24.3, 1.2];
            context.ClearRenderTargetView(&rtv, &[0.0, 0.0, 0.0, 0.0]);
            context.ClearDepthStencilView(&dsv, D3D11_CLEAR_DEPTH.0 as u32, 0.01803, 0);
            context.CopyResource(&depth_copy, &depth_tex);
            drawing
                .render_into(
                    &context,
                    &rtv,
                    color_size,
                    color_size,
                    &proj,
                    &constants,
                    1,
                    Some((&depth_srv, depth_size, depth_size, 0.01, 1.0)),
                    1,
                    Some(probe),
                    1,
                )
                .unwrap();
            drawing.poll_probe(&context, 1, true);
            assert!((drawing.probe_samples.unwrap()[0] - 0.01803).abs() < 1e-4);
            let wheel_alpha = read_center_alpha(&context);
            assert!(wheel_alpha > 20, "smoke at 24.3 m must be visible at wheel depth 0.01803 with MinDepth=0.01 (got {wheel_alpha})");

            // Case 2: Sky cleared to 0.0 (< MinDepth 0.01) -> smoke MUST be visible!
            context.ClearRenderTargetView(&rtv, &[0.0, 0.0, 0.0, 0.0]);
            context.ClearDepthStencilView(&dsv, D3D11_CLEAR_DEPTH.0 as u32, 0.0, 0);
            context.CopyResource(&depth_copy, &depth_tex);
            drawing
                .render_into(
                    &context,
                    &rtv,
                    color_size,
                    color_size,
                    &proj,
                    &constants,
                    1,
                    Some((&depth_srv, depth_size, depth_size, 0.01, 1.0)),
                    1,
                    None,
                    2,
                )
                .unwrap();
            assert!(read_center_alpha(&context) > 20, "smoke must draw visibly against 0.0 sky");

            // Case 3: Closer truck cab / rear tailgate / AI traffic at 20.3 m (depth = 0.01963) -> 100% OCCLUDED!
            context.ClearRenderTargetView(&rtv, &[0.0, 0.0, 0.0, 0.0]);
            context.ClearDepthStencilView(&dsv, D3D11_CLEAR_DEPTH.0 as u32, 0.01963, 0);
            context.CopyResource(&depth_copy, &depth_tex);
            drawing
                .render_into(
                    &context,
                    &rtv,
                    color_size,
                    color_size,
                    &proj,
                    &constants,
                    1,
                    Some((&depth_srv, depth_size, depth_size, 0.01, 1.0)),
                    1,
                    None,
                    3,
                )
                .unwrap();
            assert_eq!(read_center_alpha(&context), 0, "smoke at 24.3 m behind cab/traffic at 20.3 m must be occluded");

            // Case 4: Distant hill at 200 m (depth = 0.01087) -> smoke at 24.3 m MUST be visible!
            context.ClearRenderTargetView(&rtv, &[0.0, 0.0, 0.0, 0.0]);
            context.ClearDepthStencilView(&dsv, D3D11_CLEAR_DEPTH.0 as u32, 0.01087, 0);
            context.CopyResource(&depth_copy, &depth_tex);
            drawing
                .render_into(
                    &context,
                    &rtv,
                    color_size,
                    color_size,
                    &proj,
                    &constants,
                    1,
                    Some((&depth_srv, depth_size, depth_size, 0.01, 1.0)),
                    1,
                    None,
                    4,
                )
                .unwrap();
            assert!(read_center_alpha(&context) > 20, "smoke at 24.3 m must be visible in front of 200 m hill");
        }
    }
}
