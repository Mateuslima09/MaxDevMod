//! Phase 2 + Phase 3: Deferred 3D Volume Decal Dirt & Mud Renderer (D3D11).
//!
//! Renders procedural triplanar dust, mud, and wet-road grime onto the player's truck and coupled trailers
//! (including bitrem/rodotrem and locked/encrypted mods) during `Present`, reusing `brake_smoke`'s
//! read-only G-buffer depth snapshot (`SAVED_DEPTH`) and per-vehicle GPU `MV` matrices (`UNIT_MV_VOTES`).
//!
//! - Zero `.scs` edits, zero texture hooks, zero UV requirements, zero mid-frame D3D state mutation.
//! - Zero GPU cost when all vehicles are clean (`!dirt::has_visible_dirt()`).
//! - Hard floor clip (`localPos.y < ground_y + 0.08`) prevents road staining; cab-camera guard prevents
//!   interior bleed; daylight factor darkens dirt realistically at night.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, ID3DBlob};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
    D3D11_BLEND_SRC_ALPHA, D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_CREATE_DEVICE_SINGLETHREADED,
    D3D11_CULL_FRONT, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ZERO, D3D11_FILL_SOLID,
    D3D11_FILTER_MIN_MAG_MIP_POINT, D3D11_MAP_WRITE_DISCARD, D3D11_MAPPED_SUBRESOURCE, D3D11_RASTERIZER_DESC,
    D3D11_RENDER_TARGET_BLEND_DESC, D3D11_SAMPLER_DESC, D3D11_SDK_VERSION, D3D11_TEXTURE_ADDRESS_CLAMP,
    D3D11_USAGE_DYNAMIC, D3D11_VIEWPORT, ID3D11BlendState, ID3D11Buffer, ID3D11DepthStencilState, ID3D11Device,
    ID3D11Device1, ID3D11DeviceContext, ID3D11DeviceContext1, ID3D11PixelShader, ID3D11RasterizerState,
    ID3D11RenderTargetView, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
    ID3DDeviceContextState,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::{Interface, PCSTR, s};

use crate::dirt::UnitDirt;
use crate::log;

pub type M4 = [[f32; 4]; 4];

static IN_DRAW: AtomicBool = AtomicBool::new(false);

pub fn in_draw() -> bool {
    IN_DRAW.load(Ordering::Acquire)
}

/// One dirty vehicle unit (tractor or trailer) ready for deferred 3D decal projection.
#[derive(Clone, Copy)]
pub struct DirtUnitDraw {
    /// Vehicle GPU model-view matrix (`view = mul(mv, local)`).
    pub mv: M4,
    /// Tight local-space bounding box `[min_x, min_y, min_z]` and `[max_x, max_y, max_z]`.
    pub obb_min: [f32; 3],
    pub obb_max: [f32; 3],
    /// Local Y of the road surface under the tires (pixels below `ground_y + 0.08` are discarded).
    pub ground_y: f32,
    /// Simulated dirt layers for this unit.
    pub dirt: UnitDirt,
}

/// Per-frame scene data passed from `brake_smoke::on_present` (zero duplicated GPU capture).
pub struct DirtScene<'a> {
    pub proj: M4,
    pub depth_srv: &'a ID3D11ShaderResourceView,
    pub z_min: f32,
    pub z_max: f32,
    /// Daylight illumination multiplier (`0.18` at night .. `1.0` in bright daylight).
    pub daylight: f32,
    pub units: &'a [DirtUnitDraw],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DirtConstants {
    mv: M4,
    proj: M4,
    inv_mv: M4,
    inv_proj: M4,
    obb_min: [f32; 4],   // xyz = min, w = ground_y
    obb_max: [f32; 4],   // xyz = max, w = daylight
    dirt: [f32; 4],      // x = dust, y = mud, z = grime, w = wetness
    viewport: [f32; 4],  // x = 1/w, y = 1/h, z = z_min, w = z_max
}

struct Renderer {
    device: ID3D11Device,
    vs: ID3D11VertexShader,
    ps: ID3D11PixelShader,
    cb: ID3D11Buffer,
    blend: ID3D11BlendState,
    raster: ID3D11RasterizerState,
    depth_off: ID3D11DepthStencilState,
    sampler: ID3D11SamplerState,
    ctx_state: Option<ID3DDeviceContextState>,
}

// SAFETY: accessed only on the game's D3D11 render thread inside `Present` under `RENDERER` mutex.
unsafe impl Send for Renderer {}

static RENDERER: Mutex<Option<Result<Renderer, ()>>> = Mutex::new(None);

const SHADER_SRC: &str = r#"
cbuffer DirtCB : register(b0) {
    float4x4 gMV;
    float4x4 gProj;
    float4x4 gInvMV;
    float4x4 gInvProj;
    float4 gObbMin;   // xyz = min, w = ground_y
    float4 gObbMax;   // xyz = max, w = daylight
    float4 gDirt;     // x = dust, y = mud, z = grime, w = wetness
    float4 gViewport; // x = invW, y = invH, z = zMin, w = zMax
};

Texture2D<float> gDepth : register(t0);
SamplerState gSamp : register(s0);

struct VSOut {
    float4 pos : SV_Position;
};

static const float3 CUBE_VERTS[36] = {
    // -Z
    float3(0,0,0), float3(0,1,0), float3(1,1,0), float3(0,0,0), float3(1,1,0), float3(1,0,0),
    // +Z
    float3(1,0,1), float3(1,1,1), float3(0,1,1), float3(1,0,1), float3(0,1,1), float3(0,0,1),
    // -X
    float3(0,0,1), float3(0,1,1), float3(0,1,0), float3(0,0,1), float3(0,1,0), float3(0,0,0),
    // +X
    float3(1,0,0), float3(1,1,0), float3(1,1,1), float3(1,0,0), float3(1,1,1), float3(1,0,1),
    // +Y
    float3(0,1,0), float3(0,1,1), float3(1,1,1), float3(0,1,0), float3(1,1,1), float3(1,1,0),
    // -Y
    float3(0,0,1), float3(0,0,0), float3(1,0,0), float3(0,0,1), float3(1,0,0), float3(1,0,1),
};

VSOut VSMain(uint id : SV_VertexID) {
    VSOut o;
    float3 u = CUBE_VERTS[id];
    float3 localPos = lerp(gObbMin.xyz, gObbMax.xyz, u);
    float4 viewPos = mul(gMV, float4(localPos, 1.0));
    o.pos = mul(gProj, viewPos);
    return o;
}

float hash31(float3 p) {
    p = frac(p * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return frac((p.x + p.y) * p.z);
}

float noise3(float3 p) {
    float3 i = floor(p);
    float3 f = frac(p);
    f = f * f * (3.0 - 2.0 * f);
    float n000 = hash31(i + float3(0,0,0));
    float n100 = hash31(i + float3(1,0,0));
    float n010 = hash31(i + float3(0,1,0));
    float n110 = hash31(i + float3(1,1,0));
    float n001 = hash31(i + float3(0,0,1));
    float n101 = hash31(i + float3(1,0,1));
    float n011 = hash31(i + float3(0,1,1));
    float n111 = hash31(i + float3(1,1,1));
    float nx00 = lerp(n000, n100, f.x);
    float nx10 = lerp(n010, n110, f.x);
    float nx01 = lerp(n001, n101, f.x);
    float nx11 = lerp(n011, n111, f.x);
    return lerp(lerp(nx00, nx10, f.y), lerp(nx01, nx11, f.y), f.z);
}

float fbm3(float3 p) {
    float v = 0.0;
    float a = 0.52;
    v += a * noise3(p); p *= 2.07; a *= 0.5;
    v += a * noise3(p); p *= 2.11; a *= 0.5;
    v += a * noise3(p);
    return v;
}

float4 PSMain(VSOut inp) : SV_Target {
    float2 uv = inp.pos.xy * gViewport.xy;
    float rawD = gDepth.SampleLevel(gSamp, uv, 0);
    // Skip sky / far plane (handles both reversed-Z and standard-Z cleanly).
    if (rawD <= 1e-5 || rawD >= 0.99999) discard;

    float zSpan = max(abs(gViewport.w - gViewport.z), 1e-4);
    float ndcZ = (rawD - gViewport.z) / zSpan;
    float2 ndcXY = float2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    float4 clipPos = float4(ndcXY, ndcZ, 1.0);
    float4 viewH = mul(gInvProj, clipPos);
    if (abs(viewH.w) < 1e-6) discard;
    float3 viewPos = viewH.xyz / viewH.w;

    float4 localH = mul(gInvMV, float4(viewPos, 1.0));
    float3 lp = localH.xyz;

    // Tight vehicle OBB clip + floor clip so dirt NEVER stains the road or nearby traffic.
    if (lp.x < gObbMin.x || lp.x > gObbMax.x ||
        lp.y < gObbMin.y || lp.y > gObbMax.y ||
        lp.z < gObbMin.z || lp.z > gObbMax.z) {
        discard;
    }
    float groundY = gObbMin.w;
    if (lp.y < groundY + 0.08) discard;

    // Surface normal in vehicle-local space for triplanar-style shading & top-dust bias.
    float3 nrm = normalize(cross(ddx(lp), ddy(lp)) + 1e-6);
    float h = max(lp.y - groundY, 0.0);

    // Height masks: mud concentrates low (chassis, fenders, tanks), dust climbs higher on sides/rear.
    float lowMask = saturate(1.0 - smoothstep(0.65, 1.65, h));
    float midMask = saturate(1.0 - smoothstep(1.35, 2.85, h));
    float topBias = saturate(abs(nrm.y) * 0.75 + 0.25);

    // Multi-scale procedural dirt & mud splatters in vehicle-local 3D space (zero UVs needed).
    float macro = fbm3(lp * float3(1.1, 1.4, 0.85));
    float fine = fbm3(lp * float3(4.5, 5.2, 3.8));
    float splatters = smoothstep(0.56, 0.80, fbm3(lp * float3(3.2, 4.0, 2.2) + float3(0.0, 0.0, lp.y * 0.6)));

    float dustAmt = gDirt.x * midMask * (0.35 + 0.65 * macro) * (0.65 + 0.35 * topBias);
    float mudAmt = gDirt.y * (lowMask * (0.45 + 0.55 * macro) + midMask * splatters * 0.65);
    float grimeAmt = gDirt.z * midMask * (0.30 + 0.70 * fine);

    // Keep upper cab / windshield region subtle so exterior glass stays clear.
    float glassGuard = 1.0 - 0.55 * smoothstep(1.85, 2.45, h);
    float alpha = saturate((dustAmt * 0.62 + mudAmt * 0.82 + grimeAmt * 0.55) * glassGuard);
    if (alpha < 0.015) discard;

    // Color palette: dry ochre dust, rich dark brown mud, dark grey asphalt grime.
    float3 dustCol = float3(0.54, 0.44, 0.33);
    float3 mudCol = lerp(float3(0.28, 0.20, 0.13), float3(0.14, 0.10, 0.06), gDirt.w);
    float3 grimeCol = float3(0.16, 0.16, 0.17);

    float wSum = max(dustAmt + mudAmt + grimeAmt, 1e-4);
    float3 col = (dustCol * dustAmt + mudCol * mudAmt + grimeCol * grimeAmt) / wSum;
    col *= lerp(1.0, 0.68, gDirt.w * 0.65) * gObbMax.w;

    // Soft edge fade near OBB boundaries to avoid any sharp cut line.
    float3 edgeDist = min(lp - gObbMin.xyz, gObbMax.xyz - lp);
    float edgeFade = saturate(min(edgeDist.x, min(edgeDist.y, edgeDist.z)) / 0.12);
    return float4(col, alpha * edgeFade * 0.78);
}
"#;

fn compile_shader(entry: PCSTR, target: PCSTR) -> Result<ID3DBlob, String> {
    let mut blob = None;
    let mut errors = None;
    // SAFETY: valid static HLSL source slice and null-terminated entry/target strings.
    let hr = unsafe {
        D3DCompile(
            SHADER_SRC.as_ptr() as *const c_void,
            SHADER_SRC.len(),
            None,
            None,
            None,
            entry,
            target,
            0,
            0,
            &mut blob,
            Some(&mut errors),
        )
    };
    if let Err(e) = hr {
        let msg = errors
            .map(|b| unsafe {
                std::str::from_utf8(std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize()))
                    .unwrap_or("utf8")
                    .to_owned()
            })
            .unwrap_or_else(|| e.to_string());
        return Err(msg);
    }
    blob.ok_or_else(|| "empty shader blob".into())
}

impl Renderer {
    fn new(device: &ID3D11Device) -> Result<Self, String> {
        let vs_blob = compile_shader(s!("VSMain"), s!("vs_5_0"))?;
        let ps_blob = compile_shader(s!("PSMain"), s!("ps_5_0"))?;
        // SAFETY: valid compiled bytecode blobs and live D3D11 device.
        unsafe {
            let mut vs = None;
            device
                .CreateVertexShader(
                    std::slice::from_raw_parts(vs_blob.GetBufferPointer() as *const u8, vs_blob.GetBufferSize()),
                    None,
                    Some(&mut vs),
                )
                .map_err(|e| format!("VS: {e}"))?;
            let vs = vs.ok_or("VS null")?;

            let mut ps = None;
            device
                .CreatePixelShader(
                    std::slice::from_raw_parts(ps_blob.GetBufferPointer() as *const u8, ps_blob.GetBufferSize()),
                    None,
                    Some(&mut ps),
                )
                .map_err(|e| format!("PS: {e}"))?;
            let ps = ps.ok_or("PS null")?;

            let cb_desc = D3D11_BUFFER_DESC {
                ByteWidth: std::mem::size_of::<DirtConstants>() as u32,
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                CPUAccessFlags: windows::Win32::Graphics::Direct3D11::D3D11_CPU_ACCESS_WRITE.0 as u32,
                ..Default::default()
            };
            let mut cb = None;
            device.CreateBuffer(&cb_desc, None, Some(&mut cb)).map_err(|e| format!("CB: {e}"))?;
            let cb = cb.ok_or("CB null")?;

            let mut blend_desc = D3D11_BLEND_DESC::default();
            blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: true.into(),
                SrcBlend: D3D11_BLEND_SRC_ALPHA,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            let mut blend = None;
            device.CreateBlendState(&blend_desc, Some(&mut blend)).map_err(|e| format!("Blend: {e}"))?;
            let blend = blend.ok_or("Blend null")?;

            let raster_desc = D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                CullMode: D3D11_CULL_FRONT,
                DepthClipEnable: false.into(),
                ..Default::default()
            };
            let mut raster = None;
            device.CreateRasterizerState(&raster_desc, Some(&mut raster)).map_err(|e| format!("Raster: {e}"))?;
            let raster = raster.ok_or("Raster null")?;

            let ds_desc = D3D11_DEPTH_STENCIL_DESC {
                DepthEnable: false.into(),
                DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
                ..Default::default()
            };
            let mut depth_off = None;
            device
                .CreateDepthStencilState(&ds_desc, Some(&mut depth_off))
                .map_err(|e| format!("DS: {e}"))?;
            let depth_off = depth_off.ok_or("DS null")?;

            let samp_desc = D3D11_SAMPLER_DESC {
                Filter: D3D11_FILTER_MIN_MAG_MIP_POINT,
                AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                ..Default::default()
            };
            let mut sampler = None;
            device.CreateSamplerState(&samp_desc, Some(&mut sampler)).map_err(|e| format!("Samp: {e}"))?;
            let sampler = sampler.ok_or("Samp null")?;

            let ctx_state = device.cast::<ID3D11Device1>().ok().and_then(|d1| {
                let mut state = None;
                let levels = [d1.GetFeatureLevel()];
                let _ = d1.CreateDeviceContextState(
                    D3D11_CREATE_DEVICE_SINGLETHREADED.0,
                    &levels,
                    D3D11_SDK_VERSION,
                    &ID3D11Device::IID,
                    None,
                    Some(&mut state),
                );
                state
            });

            Ok(Self {
                device: device.clone(),
                vs,
                ps,
                cb,
                blend,
                raster,
                depth_off,
                sampler,
                ctx_state,
            })
        }
    }
}

/// Renders deferred 3D dirt decals for all dirty units in `scene` directly into the swapchain backbuffer.
///
/// # Safety
/// Must be called on the game's render thread inside `Present` with a live `IDXGISwapChain`.
pub unsafe fn render_into(swapchain: &IDXGISwapChain, scene: &DirtScene<'_>) {
    if scene.units.is_empty() {
        return;
    }
    let Some(inv_proj) = invert_m4(&scene.proj) else { return };

    // SAFETY: live swapchain on render thread inside Present; all D3D11 state is restored via SwapDeviceContextState.
    unsafe {
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let Ok(context) = device.GetImmediateContext() else { return };
        let Ok(backbuffer) = swapchain.GetBuffer::<ID3D11Texture2D>(0) else { return };
        let mut bb_desc = windows::Win32::Graphics::Direct3D11::D3D11_TEXTURE2D_DESC::default();
        backbuffer.GetDesc(&mut bb_desc);
        if bb_desc.Width == 0 || bb_desc.Height == 0 {
            return;
        }

        let mut guard = RENDERER.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(&*guard, Some(Ok(r)) if r.device != device) {
            *guard = None;
        }
        if guard.is_none() {
            *guard = Some(Renderer::new(&device).map_err(|e| {
                log!("dirt_render: failed to initialize D3D11 pipeline: {e}");
            }));
        }
        let Some(Ok(r)) = guard.as_ref() else { return };

        let mut rtv: Option<ID3D11RenderTargetView> = None;
        if device.CreateRenderTargetView(&backbuffer, None, Some(&mut rtv)).is_err() {
            return;
        }
        let Some(rtv) = rtv else { return };

        IN_DRAW.store(true, Ordering::Release);
        let ctx1 = context.cast::<ID3D11DeviceContext1>().ok();
        let mut prev_state: Option<ID3DDeviceContextState> = None;
        if let (Some(c1), Some(st)) = (&ctx1, &r.ctx_state) {
            c1.SwapDeviceContextState(st, Some(&mut prev_state));
        }

        let vp = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: bb_desc.Width as f32,
            Height: bb_desc.Height as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        context.RSSetViewports(Some(&[vp]));
        context.OMSetRenderTargets(Some(&[Some(rtv)]), None);
        context.OMSetBlendState(&r.blend, None, 0xFFFF_FFFF);
        context.OMSetDepthStencilState(&r.depth_off, 0);
        context.RSSetState(&r.raster);
        context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        context.IASetInputLayout(None);
        context.VSSetShader(&r.vs, None);
        context.PSSetShader(&r.ps, None);
        context.PSSetShaderResources(0, Some(&[Some(scene.depth_srv.clone())]));
        context.PSSetSamplers(0, Some(&[Some(r.sampler.clone())]));
        context.VSSetConstantBuffers(0, Some(&[Some(r.cb.clone())]));
        context.PSSetConstantBuffers(0, Some(&[Some(r.cb.clone())]));

        let proj_t = transpose_m4(&scene.proj);
        let inv_proj_t = transpose_m4(&inv_proj);

        for unit in scene.units {
            let Some(inv_mv) = invert_m4(&unit.mv) else { continue };
            let data = DirtConstants {
                mv: transpose_m4(&unit.mv),
                proj: proj_t,
                inv_mv: transpose_m4(&inv_mv),
                inv_proj: inv_proj_t,
                obb_min: [unit.obb_min[0], unit.obb_min[1], unit.obb_min[2], unit.ground_y],
                obb_max: [unit.obb_max[0], unit.obb_max[1], unit.obb_max[2], scene.daylight],
                dirt: [unit.dirt.dust, unit.dirt.mud, unit.dirt.grime, unit.dirt.wetness],
                viewport: [1.0 / bb_desc.Width as f32, 1.0 / bb_desc.Height as f32, scene.z_min, scene.z_max],
            };
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            if context.Map(&r.cb, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped)).is_ok() {
                std::ptr::copy_nonoverlapping(&data, mapped.pData as *mut DirtConstants, 1);
                context.Unmap(&r.cb, 0);
                context.Draw(36, 0);
            }
        }

        // Unbind SRV & RTV cleanly before restoring ETS2's context state.
        context.PSSetShaderResources(0, Some(&[None]));
        context.OMSetRenderTargets(Some(&[None]), None);
        if let (Some(c1), Some(prev)) = (&ctx1, prev_state) {
            c1.SwapDeviceContextState(&prev, None);
        }
        IN_DRAW.store(false, Ordering::Release);
    }
}

fn transpose_m4(m: &M4) -> M4 {
    [
        [m[0][0], m[1][0], m[2][0], m[3][0]],
        [m[0][1], m[1][1], m[2][1], m[3][1]],
        [m[0][2], m[1][2], m[2][2], m[3][2]],
        [m[0][3], m[1][3], m[2][3], m[3][3]],
    ]
}

/// General 4×4 matrix inverse (Gauss-Jordan with partial pivoting). Returns `None` if singular.
pub fn invert_m4(m: &M4) -> Option<M4> {
    let mut a = [[0.0f64; 8]; 4];
    for r in 0..4 {
        for c in 0..4 {
            a[r][c] = f64::from(m[r][c]);
            a[r][c + 4] = if r == c { 1.0 } else { 0.0 };
        }
    }
    for col in 0..4 {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..4 {
            let v = a[r][col].abs();
            if v > best {
                best = v;
                pivot = r;
            }
        }
        if best < 1e-12 || !best.is_finite() {
            return None;
        }
        a.swap(col, pivot);
        let div = a[col][col];
        for c in 0..8 {
            a[col][c] /= div;
        }
        for r in 0..4 {
            if r == col {
                continue;
            }
            let factor = a[r][col];
            for c in 0..8 {
                a[r][c] -= factor * a[col][c];
            }
        }
    }
    let mut out = [[0.0f32; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] = a[r][c + 4] as f32;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invert_m4_recovers_identity() {
        let m: M4 = [
            [1.5, 0.2, 0.0, 3.0],
            [-0.2, 1.2, 0.3, -1.5],
            [0.0, -0.3, 2.0, 4.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let inv = invert_m4(&m).expect("invertible");
        for r in 0..4 {
            for c in 0..4 {
                let sum: f32 = (0..4).map(|k| m[r][k] * inv[k][c]).sum();
                let expected = if r == c { 1.0 } else { 0.0 };
                assert!((sum - expected).abs() < 1e-4, "cell ({r},{c}) = {sum}");
            }
        }
    }
}
