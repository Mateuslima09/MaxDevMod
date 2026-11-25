//! MAXDEV MOD contextual Liquid Glass interaction HUD (`hints`).
//!
//! Renders the bottom-centre interaction bar in the MAXDEV MOD automotive Liquid Glass identity:
//! - Palette: crimson `#D71920`, obsidian/graphite `#101010` / `#25282C`, pure white `#FFFFFF`, silver `#AEB4BA`.
//! - Zero per-frame CPU text/icon painting: the layered Liquid Glass bitmap is painted once when the active
//!   actions or screen height change, and uploaded as a premultiplied BGRA texture.
//! - Smooth GPU transitions in its own `ID3DDeviceContextState` (`HudQuad`):
//!   - Entry: 200 ms ease-out fade + subtle upward glide.
//!   - Exit: 170 ms ease-in fade + subtle downward glide (holding the last texture until gone, no blink).
//!   - Context switch: updates in place without resetting opacity, smoothly gliding horizontal bounds.
//!   - Key press feedback: 120 ms crimson glass pulse on the pressed key's capsule via shader UV bounds.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{COLORREF, SIZE};
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, ID3DBlob};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
    D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_CREATE_DEVICE_SINGLETHREADED, D3D11_CULL_NONE,
    D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
    D3D11_SAMPLER_DESC, D3D11_SDK_VERSION, D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_VIEWPORT, ID3D11BlendState, ID3D11Buffer, ID3D11Device, ID3D11Device1, ID3D11DeviceContext,
    ID3D11DeviceContext1, ID3D11PixelShader, ID3D11RasterizerState, ID3D11SamplerState, ID3D11ShaderResourceView,
    ID3D11Texture2D, ID3D11VertexShader, ID3DDeviceContextState,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLIP_DEFAULT_PRECIS, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DIB_RGB_COLORS, DeleteDC, DeleteObject, FW_BOLD, FW_EXTRABOLD,
    FW_SEMIBOLD, GdiFlush, GetTextExtentPoint32W, HDC, HFONT, HGDIOBJ, OUT_DEFAULT_PRECIS, SelectObject, SetBkMode,
    SetTextColor, TRANSPARENT, TextOutW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::core::{Interface, PCSTR, s, w};

use crate::hints::{Action, Hint};

/// Animation timings specified in the MAXDEV MOD HUD design document (§5).
const ENTER_DURATION: Duration = Duration::from_millis(200);
const EXIT_DURATION: Duration = Duration::from_millis(170);
const RESIZE_EASE_PER_SEC: f32 = 16.0;
const KEY_FEEDBACK_DURATION: Duration = Duration::from_millis(120);

/// Screen proportions: compact, bottom-central, never covering cab instruments or the road.
pub(crate) const LETTERS_OF_SCREEN: f32 = 0.013;
const BOTTOM_GAP_OF_SCREEN: f32 = 0.014;
const SLIDE_OFFSET_OF_SCREEN: f32 = 0.008;
const MAX_WIDTH_OF_SCREEN: f32 = 0.78;

/// Brand palette (RGB).
const RED: [f32; 3] = [215.0, 25.0, 32.0]; // #D71920
const OBSIDIAN: [f32; 3] = [16.0, 16.0, 16.0]; // #101010
const GRAPHITE: [f32; 3] = [37.0, 40.0, 44.0]; // #25282C
const SILVER: [f32; 3] = [174.0, 180.0, 186.0]; // #AEB4BA
const WHITE: [f32; 3] = [255.0, 255.0, 255.0]; // #FFFFFF

/// Painted HUD surface and the UV rectangle `[u0, v0, u1, v1]` of each hint's key capsule.
pub(crate) struct PaintedHud {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub capsules_uv: Vec<[f32; 4]>,
}

/// Smooth entry, exit, width-glide and key-press feedback state.
pub(crate) struct Animator {
    visible: bool,
    opacity: f32,
    last_tick: Option<Instant>,
    smoothed_size: Option<(f32, f32)>,
    was_down: Vec<bool>,
    active_slot: Option<(usize, Instant)>,
}

impl Animator {
    pub const fn new() -> Self {
        Self {
            visible: false,
            opacity: 0.0,
            last_tick: None,
            smoothed_size: None,
            was_down: Vec::new(),
            active_slot: None,
        }
    }

    /// Advances visibility (`want_visible`) and key-press feedback for `hints` at `now`.
    /// Returns `(opacity, slide_progress, active_capsule_index_and_pulse)` when the HUD should be drawn.
    pub fn step(&mut self, want_visible: bool, hints: &[Hint], now: Instant) -> Option<(f32, f32, Option<(usize, f32)>)> {
        let dt = self
            .last_tick
            .map_or(0.016, |prev| now.saturating_duration_since(prev).as_secs_f32().clamp(0.0, 0.1));
        self.last_tick = Some(now);
        self.visible = want_visible;

        if want_visible {
            let step = dt / ENTER_DURATION.as_secs_f32();
            self.opacity = (self.opacity + step).min(1.0);
        } else {
            let step = dt / EXIT_DURATION.as_secs_f32();
            self.opacity = (self.opacity - step).max(0.0);
        }

        if self.opacity <= 0.001 {
            self.smoothed_size = None;
            self.was_down.clear();
            self.active_slot = None;
            return None;
        }

        // Poll bound keys for discrete 120 ms press feedback while visible.
        if want_visible {
            if self.was_down.len() != hints.len() {
                self.was_down.resize(hints.len(), false);
            }
            for (idx, hint) in hints.iter().enumerate() {
                let down = key_is_pressed(hint.vk, hint.second_vk);
                if down && !self.was_down[idx] {
                    self.active_slot = Some((idx, now));
                }
                self.was_down[idx] = down;
            }
        }

        let pulse = self.active_slot.and_then(|(idx, started)| {
            let elapsed = now.saturating_duration_since(started);
            if elapsed >= KEY_FEEDBACK_DURATION {
                None
            } else {
                let t = elapsed.as_secs_f32() / KEY_FEEDBACK_DURATION.as_secs_f32();
                // Smooth bell/decay curve: instant crisp response, gentle fade.
                let strength = (1.0 - t) * (1.0 - t * 0.5);
                Some((idx, strength))
            }
        });
        if pulse.is_none() {
            self.active_slot = None;
        }

        // Cubic ease-out on entry, smooth ease-in on exit.
        let eased = if want_visible {
            1.0 - (1.0 - self.opacity).powi(3)
        } else {
            self.opacity * self.opacity * (3.0 - 2.0 * self.opacity)
        };
        let slide = 1.0 - eased;
        Some((eased, slide, pulse))
    }

    /// Smoothly interpolates quad width/height when context changes while already visible, preventing jumps.
    pub fn smooth_quad_size(&mut self, target: (u32, u32), now: Instant) -> (f32, f32) {
        let (tw, th) = (target.0 as f32, target.1 as f32);
        match &mut self.smoothed_size {
            Some((cw, ch)) => {
                let dt = self
                    .last_tick
                    .map_or(0.016, |prev| now.saturating_duration_since(prev).as_secs_f32().clamp(0.004, 0.05));
                let k = 1.0 - (-RESIZE_EASE_PER_SEC * dt).exp();
                *cw += (tw - *cw) * k;
                *ch += (th - *ch) * k;
                if (*cw - tw).abs() < 0.75 {
                    *cw = tw;
                }
                if (*ch - th).abs() < 0.75 {
                    *ch = th;
                }
                // Snap width once close, keeping text pixel-aligned at rest.
                (*cw, th)
            }
            None => {
                self.smoothed_size = Some((tw, th));
                (tw, th)
            }
        }
    }
}

fn key_is_pressed(vk: u32, second_vk: u32) -> bool {
    if vk == 0 {
        return false;
    }
    let down = |code: u32| -> bool {
        let mut is_down = false;
        // SAFETY: GetAsyncKeyState is a plain Win32 query taking an integer virtual-key code.
        crate::hooks::guarded(|| is_down = unsafe { GetAsyncKeyState(code as i32) } as u16 & 0x8000 != 0);
        is_down
    };
    if second_vk != 0 {
        down(vk) && down(second_vk)
    } else {
        down(vk)
    }
}

/// Clip-space placement `[left, top, right, bottom]` on `screen_w x screen_h`, shifted down by `slide` (`0..1`).
pub(crate) fn place_on(screen_w: f32, screen_h: f32, (w, h): (f32, f32), slide: f32) -> [f32; 4] {
    let left = ((screen_w - w) * 0.5).round();
    let slide_px = slide * screen_h * SLIDE_OFFSET_OF_SCREEN;
    let bottom = (screen_h - screen_h * BOTTOM_GAP_OF_SCREEN + slide_px).round();
    let top = bottom - h.round();
    let right = left + w.round();
    [
        left / screen_w * 2.0 - 1.0,
        1.0 - top / screen_h * 2.0,
        right / screen_w * 2.0 - 1.0,
        1.0 - bottom / screen_h * 2.0,
    ]
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HudConstants {
    rect: [f32; 4],
    active_uv: [f32; 4],
    opacity: f32,
    pulse: f32,
    _pad: [f32; 2],
}

const HUD_SHADERS: &str = r"
cbuffer HudParams : register(b0) {
    float4 rect;
    float4 active_uv;
    float opacity;
    float pulse;
    float2 _pad;
};
struct Vertex { float4 position : SV_Position; float2 uv : TEXCOORD0; };
Vertex vs(uint id : SV_VertexID) {
    float2 corner = float2(id & 1, id >> 1);
    Vertex v;
    v.position = float4(lerp(rect.x, rect.z, corner.x), lerp(rect.y, rect.w, corner.y), 0, 1);
    v.uv = corner;
    return v;
}
Texture2D hud_tex : register(t0);
SamplerState smooth_sampler : register(s0);
float4 ps(Vertex v) : SV_Target {
    float4 base = hud_tex.Sample(smooth_sampler, v.uv);
    if (pulse > 0.001 && active_uv.z > active_uv.x) {
        float2 center = 0.5 * (active_uv.xy + active_uv.zw);
        float2 half_ext = max(0.5 * (active_uv.zw - active_uv.xy), float2(0.001, 0.001));
        float2 d = abs(v.uv - center) / half_ext;
        float inside = saturate(1.25 - max(d.x, d.y));
        float rim = saturate(1.0 - abs(max(d.x, d.y) - 0.96) * 8.0);
        float3 red = float3(0.843, 0.098, 0.125);
        float boost = pulse * (inside * 0.32 + rim * 0.55) * saturate(base.a * 2.0);
        base.rgb = saturate(base.rgb + red * boost);
        base.a = saturate(base.a + boost * 0.25);
    }
    return base * opacity;
}
";

/// Dedicated D3D11 quad renderer for the HUD, keeping `handset::Quad` untouched.
pub(crate) struct HudQuad {
    state: ID3DDeviceContextState,
    vertex: ID3D11VertexShader,
    pixel: ID3D11PixelShader,
    constants: ID3D11Buffer,
    blend: ID3D11BlendState,
    rasterizer: ID3D11RasterizerState,
    sampler: ID3D11SamplerState,
}

impl HudQuad {
    /// # Safety
    /// `device` is the game's live D3D11 device on its render thread.
    pub unsafe fn new(device: &ID3D11Device) -> windows::core::Result<Self> {
        // SAFETY: `device` is live on the render thread; all descriptors and shader blobs are local.
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
            let vs_blob = compile_shader(s!("vs"), s!("vs_4_0"))?;
            let ps_blob = compile_shader(s!("ps"), s!("ps_4_0"))?;
            let mut vertex = None;
            device.CreateVertexShader(blob_slice(&vs_blob), None, Some(&mut vertex))?;
            let mut pixel = None;
            device.CreatePixelShader(blob_slice(&ps_blob), None, Some(&mut pixel))?;
            let mut constants = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: std::mem::size_of::<HudConstants>() as u32,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut constants),
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
            let empty = windows::core::Error::empty;
            Ok(Self {
                state: state.ok_or_else(empty)?,
                vertex: vertex.ok_or_else(empty)?,
                pixel: pixel.ok_or_else(empty)?,
                constants: constants.ok_or_else(empty)?,
                blend: blend.ok_or_else(empty)?,
                rasterizer: rasterizer.ok_or_else(empty)?,
                sampler: sampler.ok_or_else(empty)?,
            })
        }
    }

    /// # Safety
    /// `device`, `context`, `swapchain`, and `view` belong to the game's live D3D11 device on its render thread.
    pub unsafe fn draw(
        &self,
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        swapchain: &IDXGISwapChain,
        view: &ID3D11ShaderResourceView,
        rect: [f32; 4],
        opacity: f32,
        active: Option<([f32; 4], f32)>,
    ) -> windows::core::Result<()> {
        // SAFETY: swaps into our isolated device context state, draws the premultiplied HUD quad onto the back
        // buffer, and restores the game's exact device context state before returning.
        unsafe {
            let context1: ID3D11DeviceContext1 = context.cast()?;
            let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut desc);
            let mut target = None;
            device.CreateRenderTargetView(&back, None, Some(&mut target))?;
            let (active_uv, pulse) = active.unwrap_or(([0.0; 4], 0.0));
            let params = HudConstants { rect, active_uv, opacity, pulse, _pad: [0.0; 2] };

            let mut previous: Option<ID3DDeviceContextState> = None;
            context1.SwapDeviceContextState(&self.state, Some(&mut previous));
            context.UpdateSubresource(&self.constants, 0, None, (&params as *const HudConstants).cast(), 0, 0);
            context.OMSetRenderTargets(Some(&[target]), None);
            context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: desc.Width as f32,
                Height: desc.Height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
            context.RSSetState(&self.rasterizer);
            context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            context.VSSetShader(&self.vertex, None);
            context.VSSetConstantBuffers(0, Some(&[Some(self.constants.clone())]));
            context.PSSetShader(&self.pixel, None);
            context.PSSetConstantBuffers(0, Some(&[Some(self.constants.clone())]));
            context.PSSetShaderResources(0, Some(&[Some(view.clone())]));
            context.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            context.OMSetBlendState(&self.blend, None, u32::MAX);
            context.Draw(4, 0);
            context.OMSetRenderTargets(None, None);
            context.PSSetShaderResources(0, Some(&[None]));
            if let Some(previous) = previous.as_ref() {
                context1.SwapDeviceContextState(previous, None);
            }
            Ok(())
        }
    }
}

fn compile_shader(entry: PCSTR, target: PCSTR) -> windows::core::Result<ID3DBlob> {
    let mut code = None;
    // SAFETY: `HUD_SHADERS` is static ASCII HLSL source; outputs into `code`.
    unsafe {
        D3DCompile(
            HUD_SHADERS.as_ptr().cast(),
            HUD_SHADERS.len(),
            None,
            None,
            None,
            entry,
            target,
            0,
            0,
            &mut code,
            None,
        )?;
    }
    code.ok_or_else(windows::core::Error::empty)
}

fn blob_slice(blob: &ID3DBlob) -> &[u8] {
    // SAFETY: `blob` owns a contiguous buffer of `GetBufferSize()` bytes for its lifetime.
    unsafe { std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize()) }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[derive(Clone, Copy)]
struct ItemPlace {
    x: i32,
    box_w: i32,
    kw: i32,
    icon_x: i32,
    text_x: i32,
    action: Action,
}

/// Paints the complete MAXDEV MOD Liquid Glass HUD for `hints` onto a `screen_w x screen_h` target, automatically
/// scaling down when needed so the bar never exceeds `MAX_WIDTH_OF_SCREEN`.
pub(crate) fn paint(hints: &[Hint], screen_w: u32, screen_h: u32, language: &str) -> Option<PaintedHud> {
    if hints.is_empty() {
        return None;
    }
    let base_letters = ((screen_h as f32 * LETTERS_OF_SCREEN).round() as i32).clamp(11, 38);
    let max_w = (screen_w as f32 * MAX_WIDTH_OF_SCREEN).round() as u32;
    let first = paint_at_size(hints, base_letters, language)?;
    if first.width <= max_w || base_letters <= 10 {
        return Some(first);
    }
    let scaled = ((base_letters as f32 * (max_w as f32 / first.width as f32)).floor() as i32).clamp(10, base_letters);
    if scaled == base_letters {
        Some(first)
    } else {
        paint_at_size(hints, scaled, language).or(Some(first))
    }
}

/// Paints at a specific font height (`letters` px).
pub(crate) fn paint_at_size(hints: &[Hint], letters: i32, language: &str) -> Option<PaintedHud> {
    if hints.is_empty() || letters <= 0 {
        return None;
    }
    let em = letters as f32;
    let halo_x = (em * 0.95).round() as i32;
    let halo_bottom = (em * 0.42).round() as i32;
    let brand_h = (em * 0.72).round().max(9.0) as i32;
    let pad_x = (em * 0.95).round() as i32;
    let pad_y = (em * 0.44).round() as i32;
    let key_pad_x = (em * 0.52).round() as i32;
    let after_key = (em * 0.48).round() as i32;
    let icon_size = (em * 0.98).round().max(11.0) as i32;
    let after_icon = (em * 0.38).round() as i32;
    let gap_side = (em * 0.68).round() as i32;
    let brand_letters = (em * 0.56).round().clamp(8.0, 22.0) as i32;

    // 1. Measure text metrics via GDI and lay out items horizontally.
    // SAFETY: All GDI handles (`dc`, fonts, DIB) are created, selected, flushed and deleted on this thread before returning.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let make_font = |height: i32, weight: i32, italic: u32| -> HFONT {
            CreateFontW(
                -height,
                0,
                0,
                0,
                weight,
                italic,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                0,
                w!("Segoe UI"),
            )
        };
        let action_font = make_font(letters, FW_SEMIBOLD.0 as i32, 0);
        let key_font = make_font((letters as f32 * 0.94).round() as i32, FW_BOLD.0 as i32, 0);
        let brand_font = make_font(brand_letters, FW_EXTRABOLD.0 as i32, 1);

        let measure = |dc: HDC, font: HFONT, text: &[u16]| -> (i32, i32) {
            SelectObject(dc, HGDIOBJ(font.0));
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, text, &mut size);
            (size.cx, size.cy)
        };

        let encoded: Vec<(Vec<u16>, Vec<u16>, Action)> = hints
            .iter()
            .map(|h| (wide(&h.key.to_uppercase()), wide(h.action.words(language)), h.action))
            .collect();

        let mut tall = letters;
        let mut cursor_x = pad_x;
        let mut places = Vec::with_capacity(encoded.len());
        for (idx, (key_w16, act_w16, action)) in encoded.iter().enumerate() {
            let (kw, kh) = measure(dc, key_font, key_w16);
            let (aw, ah) = measure(dc, action_font, act_w16);
            tall = tall.max(kh).max(ah);
            let box_h_est = tall + (em * 0.34).round() as i32;
            let box_w = (kw + 2 * key_pad_x).max((box_h_est as f32 * 1.18).round() as i32);
            let icon_x = cursor_x + box_w + after_key;
            let text_x = icon_x + icon_size + after_icon;
            places.push(ItemPlace { x: cursor_x, box_w, kw, icon_x, text_x, action: *action });
            cursor_x = text_x + aw;
            if idx + 1 < encoded.len() {
                cursor_x += gap_side * 2 + 1;
            }
        }

        // Brand header segments: "MA" (white) + "X" (red) + "DEV " (silver) + "MOD" (red).
        let b_ma = wide("MA");
        let b_x = wide("X");
        let b_dev = wide("DEV ");
        let b_mod = wide("MOD");
        let (w_ma, bh) = measure(dc, brand_font, &b_ma);
        let (w_x, _) = measure(dc, brand_font, &b_x);
        let (w_dev, _) = measure(dc, brand_font, &b_dev);
        let (w_mod, _) = measure(dc, brand_font, &b_mod);
        let brand_total_w = w_ma + w_x + w_dev + w_mod;

        let pill_w = (cursor_x + pad_x).max(brand_total_w + pad_x * 3);
        // Center items inside pill if brand header made a 1-item pill wider.
        let extra_shift = (pill_w - (cursor_x + pad_x)) / 2;
        if extra_shift > 0 {
            for p in &mut places {
                p.x += extra_shift;
                p.icon_x += extra_shift;
                p.text_x += extra_shift;
            }
        }

        let box_h = tall + (em * 0.34).round() as i32;
        let pill_h = box_h + 2 * pad_y;
        let pill_left = halo_x;
        let pill_top = brand_h;
        let width = pill_w + 2 * halo_x;
        let height = pill_top + pill_h + halo_bottom;
        if width <= 0 || height <= 0 {
            let _ = DeleteObject(HGDIOBJ(action_font.0));
            let _ = DeleteObject(HGDIOBJ(key_font.0));
            let _ = DeleteObject(HGDIOBJ(brand_font.0));
            let _ = DeleteDC(dc);
            return None;
        }

        // 2. Render text mask onto a black DIB section so GDI antialiased glyphs give clean coverage [0..255].
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let Ok(bitmap) = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
            let _ = DeleteObject(HGDIOBJ(action_font.0));
            let _ = DeleteObject(HGDIOBJ(key_font.0));
            let _ = DeleteObject(HGDIOBJ(brand_font.0));
            let _ = DeleteDC(dc);
            return None;
        };
        let old_bmp = SelectObject(dc, HGDIOBJ(bitmap.0));
        SetBkMode(dc, TRANSPARENT);

        // Encode glyph colour tint into channels on black DIB:
        // White glyphs -> RGB(255, 255, 255), Red brand glyphs -> RGB(255, 24, 32), Silver brand -> RGB(195, 202, 210).
        // (COLORREF is 0x00BBGGRR!)
        let col_white = COLORREF(0x00FF_FFFF);
        let col_red = COLORREF(0x0020_19D7);
        let col_silver = COLORREF(0x00BA_B4AE);

        // Draw top-centered MAXDEV MOD micro-signature straddling the top edge of the pill.
        let brand_x = pill_left + (pill_w - brand_total_w) / 2;
        let brand_y = (pill_top - bh / 2 - 1).max(0);
        SelectObject(dc, HGDIOBJ(brand_font.0));
        SetTextColor(dc, col_white);
        let _ = TextOutW(dc, brand_x, brand_y, &b_ma);
        SetTextColor(dc, col_red);
        let _ = TextOutW(dc, brand_x + w_ma, brand_y, &b_x);
        SetTextColor(dc, col_silver);
        let _ = TextOutW(dc, brand_x + w_ma + w_x, brand_y, &b_dev);
        SetTextColor(dc, col_red);
        let _ = TextOutW(dc, brand_x + w_ma + w_x + w_dev, brand_y, &b_mod);

        // Draw key labels and action labels.
        let box_top = pill_top + pad_y;
        let (_, key_th) = measure(dc, key_font, &wide("A"));
        let (_, act_th) = measure(dc, action_font, &wide("Ag"));
        for ((key_w16, act_w16, _), p) in encoded.iter().zip(&places) {
            let kx = pill_left + p.x + (p.box_w - p.kw) / 2;
            let ky = box_top + (box_h - key_th) / 2;
            SelectObject(dc, HGDIOBJ(key_font.0));
            SetTextColor(dc, col_white);
            let _ = TextOutW(dc, kx, ky, key_w16);

            let ax = pill_left + p.text_x;
            let ay = box_top + (box_h - act_th) / 2;
            SelectObject(dc, HGDIOBJ(action_font.0));
            SetTextColor(dc, col_white);
            let _ = TextOutW(dc, ax, ay, act_w16);
        }

        let _ = GdiFlush();
        let text_dib = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec();

        SelectObject(dc, old_bmp);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteObject(HGDIOBJ(action_font.0));
        let _ = DeleteObject(HGDIOBJ(key_font.0));
        let _ = DeleteObject(HGDIOBJ(brand_font.0));
        let _ = DeleteDC(dc);

        // 3. Compose Liquid Glass layers in premultiplied BGRA.
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let pill_rect = [
            pill_left as f32,
            pill_top as f32,
            (pill_left + pill_w) as f32,
            (pill_top + pill_h) as f32,
        ];
        let pill_radius = pill_h as f32 * 0.5;
        let brand_gap = [
            (brand_x - (em * 0.35) as i32) as f32,
            (brand_x + brand_total_w + (em * 0.35) as i32) as f32,
        ];

        compose_pill_glass(&mut pixels, width as u32, height as u32, pill_rect, pill_radius, brand_gap);

        // Draw key capsules, dividers, and vector action icons.
        let key_radius = (box_h as f32 * 0.34).max(4.0);
        let mut capsules_uv = Vec::with_capacity(places.len());
        for (idx, p) in places.iter().enumerate() {
            let cap_rect = [
                (pill_left + p.x) as f32,
                box_top as f32,
                (pill_left + p.x + p.box_w) as f32,
                (box_top + box_h) as f32,
            ];
            compose_key_capsule(&mut pixels, width as u32, height as u32, cap_rect, key_radius);
            capsules_uv.push([
                cap_rect[0] / width as f32,
                cap_rect[1] / height as f32,
                cap_rect[2] / width as f32,
                cap_rect[3] / height as f32,
            ]);

            // Action vector icon (centered vertically in `box_top..box_top+box_h`).
            let icon_top = box_top + (box_h - icon_size) / 2;
            draw_action_icon(
                &mut pixels,
                width as u32,
                height as u32,
                (pill_left + p.icon_x) as f32,
                icon_top as f32,
                icon_size as f32,
                p.action,
            );

            // Subtle vertical glass divider before next item.
            if idx + 1 < places.len() {
                let next_x = pill_left + places[idx + 1].x;
                let div_x = next_x - gap_side;
                let div_top = pill_top + (pill_h as f32 * 0.22).round() as i32;
                let div_bot = pill_top + pill_h - (pill_h as f32 * 0.22).round() as i32;
                draw_divider(&mut pixels, width as u32, height as u32, div_x, div_top, div_bot);
            }
        }

        // 4. Composite crisp GDI text glyphs (with subtle dark drop shadow for legibility on any road surface).
        composite_text(&mut pixels, &text_dib, width as u32, height as u32);

        Some(PaintedHud { width: width as u32, height: height as u32, pixels, capsules_uv })
    }
}

/// Signed distance to a rounded rectangle `[x0, y0, x1, y1]` with corner `radius`. Negative inside, positive outside.
#[inline]
fn sd_round_rect(px: f32, py: f32, [x0, y0, x1, y1]: [f32; 4], radius: f32) -> f32 {
    let cx = 0.5 * (x0 + x1);
    let cy = 0.5 * (y0 + y1);
    let hx = 0.5 * (x1 - x0) - radius;
    let hy = 0.5 * (y1 - y0) - radius;
    let dx = (px - cx).abs() - hx;
    let dy = (py - cy).abs() - hy;
    let out = dx.max(0.0).hypot(dy.max(0.0));
    let inside = dx.max(dy).min(0.0);
    out + inside - radius
}

/// Alpha-over blend (`src` non-premultiplied RGB `0..255` + `alpha 0..1`) onto premultiplied BGRA `dst`.
#[inline]
fn blend_over(dst: &mut [u8], rgb: [f32; 3], alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    if a <= 0.001 {
        return;
    }
    let inv = 1.0 - a;
    dst[0] = (rgb[2] * a + f32::from(dst[0]) * inv).round().clamp(0.0, 255.0) as u8;
    dst[1] = (rgb[1] * a + f32::from(dst[1]) * inv).round().clamp(0.0, 255.0) as u8;
    dst[2] = (rgb[0] * a + f32::from(dst[2]) * inv).round().clamp(0.0, 255.0) as u8;
    let out_a = a * 255.0 + f32::from(dst[3]) * inv;
    dst[3] = out_a.round().clamp(0.0, 255.0) as u8;
}

/// Draws the outer crimson edge halo, layered dark Liquid Glass stadium body, specular sheen, and metallic/red rim.
fn compose_pill_glass(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rect: [f32; 4],
    radius: f32,
    brand_gap: [f32; 2],
) {
    let [x0, y0, x1, y1] = rect;
    let pill_w = (x1 - x0).max(1.0);
    let pill_h = (y1 - y0).max(1.0);
    let cx = 0.5 * (x0 + x1);

    for y in 0..height {
        let py = y as f32 + 0.5;
        let v = ((py - y0) / pill_h).clamp(0.0, 1.0);
        for x in 0..width {
            let px = x as f32 + 0.5;
            let d = sd_round_rect(px, py, rect, radius);
            let out = &mut pixels[((y * width + x) * 4) as usize..][..4];

            // Lateral factor [0..1]: 0 in centre, 1 at the curved left/right ends (matches PDF red side-cap glow).
            let end_bias = (((px - cx).abs() / (0.5 * pill_w)).clamp(0.0, 1.0)).powi(2);

            // 1. Subtle outer crimson ambient glow concentrated at the stadium caps.
            if d > -1.0 && d < radius * 0.65 {
                let falloff = (-d.max(0.0) / (radius * 0.24)).exp();
                let glow_a = falloff * (0.06 + 0.26 * end_bias);
                blend_over(out, RED, glow_a);
            }

            // 2. Interior Liquid Glass surface (smooth 1px anti-aliased edge).
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                // Vertical gradient from smoked graphite (#25282C) to deep obsidian (#101010).
                let mix_g = (1.0 - v).powf(1.35);
                let mut r = OBSIDIAN[0] + (GRAPHITE[0] - OBSIDIAN[0]) * mix_g;
                let mut g = OBSIDIAN[1] + (GRAPHITE[1] - OBSIDIAN[1]) * mix_g;
                let mut b = OBSIDIAN[2] + (GRAPHITE[2] - OBSIDIAN[2]) * mix_g;

                // Subtle iOS Liquid Glass top specular reflection band (upper 36% of pill).
                if v < 0.36 {
                    let sheen = (1.0 - v / 0.36).powi(2) * 0.11;
                    r += (WHITE[0] - r) * sheen;
                    g += (WHITE[1] - g) * sheen;
                    b += (WHITE[2] - b) * sheen;
                }

                // Subtle warm crimson reflection near the curved caps inside the glass.
                let inner_edge = ((-d) / (radius * 0.45)).clamp(0.0, 1.0);
                let cap_tint = (1.0 - inner_edge) * end_bias * 0.18;
                r += (RED[0] - r) * cap_tint;
                g += (RED[1] - g) * cap_tint;
                b += (RED[2] - b) * cap_tint;

                // High-contrast translucent alpha (0.85 top -> 0.91 bottom).
                let glass_alpha = (0.85 + 0.06 * v) * cov;
                blend_over(out, [r, g, b], glass_alpha);
            }

            // 3. Crisp 1.3px Liquid Glass border (paused cleanly under the MAXDEV MOD top micro-header).
            let in_brand_cutout = py < y0 + 2.5 && px >= brand_gap[0] && px <= brand_gap[1];
            if !in_brand_cutout {
                let border = (1.15 - (d + 0.65).abs()).clamp(0.0, 1.0);
                if border > 0.0 {
                    // Top rim catches silver light (#AEB4BA) + crimson accent lines flanking the brand badge;
                    // curved ends glow crimson (#D71920); bottom rim carries a discrete red/silver reflection.
                    let top_flank = if py < y0 + 2.5 {
                        let dist_from_gap = if px < brand_gap[0] {
                            brand_gap[0] - px
                        } else {
                            px - brand_gap[1]
                        };
                        (1.0 - dist_from_gap / (pill_w * 0.32)).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let red_mix = (end_bias * 0.85 + top_flank * 0.9 + v * 0.25).clamp(0.0, 1.0);
                    let br = SILVER[0] + (RED[0] - SILVER[0]) * red_mix;
                    let bg = SILVER[1] + (RED[1] - SILVER[1]) * red_mix;
                    let bb = SILVER[2] + (RED[2] - SILVER[2]) * red_mix;
                    let rim_alpha = border * (0.52 + 0.36 * red_mix);
                    blend_over(out, [br, bg, bb], rim_alpha);
                }
            }
        }
    }
}

/// Draws a recessed/elevated Liquid Glass keycap capsule (`#25282C` -> `#121417` with `#AEB4BA` specular rim).
fn compose_key_capsule(pixels: &mut [u8], width: u32, height: u32, rect: [f32; 4], radius: f32) {
    let [x0, y0, x1, y1] = rect;
    let cap_h = (y1 - y0).max(1.0);
    let min_x = (x0.floor() as i32 - 2).max(0) as u32;
    let max_x = (x1.ceil() as i32 + 2).min(width as i32) as u32;
    let min_y = (y0.floor() as i32 - 2).max(0) as u32;
    let max_y = (y1.ceil() as i32 + 2).min(height as i32) as u32;

    for y in min_y..max_y {
        let py = y as f32 + 0.5;
        let v = ((py - y0) / cap_h).clamp(0.0, 1.0);
        for x in min_x..max_x {
            let px = x as f32 + 0.5;
            let d = sd_round_rect(px, py, rect, radius);
            let out = &mut pixels[((y * width + x) * 4) as usize..][..4];

            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                // Crisp smoked glass keycap body.
                let top_sheen = (1.0 - v).powi(2);
                let r = 20.0 + 32.0 * top_sheen;
                let g = 22.0 + 34.0 * top_sheen;
                let b = 25.0 + 38.0 * top_sheen;
                blend_over(out, [r, g, b], 0.92 * cov);
            }

            // Crisp metallic silver border (brighter on top specular edge).
            let border = (1.05 - (d + 0.5).abs()).clamp(0.0, 1.0);
            if border > 0.0 {
                let spec = 1.0 - 0.55 * v;
                blend_over(out, SILVER, border * 0.68 * spec);
            }
        }
    }
}

fn draw_divider(pixels: &mut [u8], width: u32, height: u32, x: i32, top: i32, bottom: i32) {
    if x < 0 || x >= width as i32 || bottom <= top {
        return;
    }
    let span = (bottom - top) as f32;
    for y in top.max(0)..bottom.min(height as i32) {
        let t = (y - top) as f32 / span;
        // Soft vertical fade at ends of the divider.
        let fade = (t * std::f32::consts::PI).sin();
        let out = &mut pixels[((y as u32 * width + x as u32) * 4) as usize..][..4];
        blend_over(out, [195.0, 75.0, 82.0], 0.34 * fade);
    }
}

/// Composites GDI antialiased text glyphs (with a 1px soft shadow below) onto `pixels`.
fn composite_text(pixels: &mut [u8], text_dib: &[u8], width: u32, height: u32) {
    // Pass 1: subtle 1px drop shadow for crisp readability over sunlit dashboards/roads.
    for y in 1..height {
        for x in 0..width {
            let src = &text_dib[(((y - 1) * width + x) * 4) as usize..][..4];
            let lum = src[0].max(src[1]).max(src[2]);
            if lum > 8 {
                let dst = &mut pixels[((y * width + x) * 4) as usize..][..4];
                blend_over(dst, [0.0, 0.0, 0.0], (f32::from(lum) / 255.0) * 0.55);
            }
        }
    }
    // Pass 2: crisp coloured glyphs.
    for i in 0..(width * height) as usize {
        let src = &text_dib[i * 4..i * 4 + 4];
        let b = f32::from(src[0]);
        let g = f32::from(src[1]);
        let r = f32::from(src[2]);
        let a = r.max(g).max(b) / 255.0;
        if a > 0.01 {
            let scale = 1.0 / a.max(0.001);
            let rgb = [(r * scale).min(255.0), (g * scale).min(255.0), (b * scale).min(255.0)];
            blend_over(&mut pixels[i * 4..i * 4 + 4], rgb, a);
        }
    }
}

/// Anti-aliased geometric icon primitives in normalized `[0..1] x [0..1]` icon space.
fn sd_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (vx, vy) = (bx - ax, by - ay);
    let (wx, wy) = (px - ax, py - ay);
    let len2 = vx * vx + vy * vy;
    let t = if len2 > 1e-6 { ((wx * vx + wy * vy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    (wx - vx * t).hypot(wy - vy * t)
}

fn draw_action_icon(pixels: &mut [u8], width: u32, height: u32, x0: f32, y0: f32, size: f32, action: Action) {
    let min_x = (x0.floor() as i32 - 1).max(0) as u32;
    let max_x = ((x0 + size).ceil() as i32 + 1).min(width as i32) as u32;
    let min_y = (y0.floor() as i32 - 1).max(0) as u32;
    let max_y = ((y0 + size).ceil() as i32 + 1).min(height as i32) as u32;
    let stroke = (1.55 / size).clamp(0.055, 0.095);
    let aa = 0.95 / size;

    for y in min_y..max_y {
        let v = (y as f32 + 0.5 - y0) / size;
        for x in min_x..max_x {
            let u = (x as f32 + 0.5 - x0) / size;
            let d = icon_sdf(u, v, stroke, action);
            let alpha = ((aa - d) / (2.0 * aa)).clamp(0.0, 1.0);
            if alpha > 0.005 {
                let dst = &mut pixels[((y * width + x) * 4) as usize..][..4];
                blend_over(dst, WHITE, alpha * 0.95);
            }
        }
    }
}

/// Signed distance for each `Action`'s vector glyph in `[0..1]²` (`< 0` inside the glyph).
fn icon_sdf(u: f32, v: f32, w: f32, action: Action) -> f32 {
    let line = |ax: f32, ay: f32, bx: f32, by: f32| sd_segment(u, v, ax, ay, bx, by) - w;
    let ring = |cx: f32, cy: f32, r: f32| ((u - cx).hypot(v - cy) - r).abs() - w;
    let disc = |cx: f32, cy: f32, r: f32| (u - cx).hypot(v - cy) - r;
    let box_outline = |x0: f32, y0: f32, x1: f32, y1: f32, r: f32| sd_round_rect(u, v, [x0, y0, x1, y1], r).abs() - w;

    match action {
        Action::GetOut => {
            // Cab door frame + arrow pointing out to left/right
            let door = line(0.55, 0.16, 0.22, 0.16)
                .min(line(0.22, 0.16, 0.22, 0.84))
                .min(line(0.22, 0.84, 0.55, 0.84));
            let arrow = line(0.42, 0.50, 0.84, 0.50)
                .min(line(0.68, 0.34, 0.84, 0.50))
                .min(line(0.68, 0.66, 0.84, 0.50));
            door.min(arrow)
        }
        Action::GetIn => {
            // Cab door frame on right + arrow pointing inside
            let door = line(0.48, 0.16, 0.82, 0.16)
                .min(line(0.82, 0.16, 0.82, 0.84))
                .min(line(0.82, 0.84, 0.48, 0.84));
            let arrow = line(0.16, 0.50, 0.62, 0.50)
                .min(line(0.46, 0.34, 0.62, 0.50))
                .min(line(0.46, 0.66, 0.62, 0.50));
            door.min(arrow)
        }
        Action::Phone => {
            // Smartphone silhouette + bottom home indicator
            box_outline(0.28, 0.12, 0.72, 0.88, 0.09).min(line(0.43, 0.76, 0.57, 0.76))
        }
        Action::Screen => {
            // Multimedia screen frame + play triangle inside
            let frame = box_outline(0.14, 0.20, 0.86, 0.80, 0.08);
            let play = line(0.43, 0.38, 0.43, 0.62)
                .min(line(0.43, 0.38, 0.63, 0.50))
                .min(line(0.43, 0.62, 0.63, 0.50));
            frame.min(play)
        }
        Action::Character => {
            // Clean person head + shoulders silhouette (filled)
            let head = disc(0.50, 0.31, 0.16);
            let shoulders = sd_round_rect(u, v, [0.22, 0.54, 0.78, 0.86], 0.16);
            head.min(shoulders)
        }
        Action::Couple | Action::Uncouple | Action::LowerLegs | Action::RaiseLegs => {
            // Interlocking chain/fifth-wheel links (matches PDF "Próximo da carreta")
            let left_link = ring(0.38, 0.54, 0.19);
            let right_link = ring(0.62, 0.46, 0.19);
            let base = left_link.min(right_link);
            if action == Action::Uncouple {
                base.min(line(0.22, 0.80, 0.78, 0.20))
            } else {
                base
            }
        }
        Action::SignNote => {
            // Freight clipboard / document with signature lines
            box_outline(0.22, 0.14, 0.78, 0.86, 0.07)
                .min(line(0.35, 0.38, 0.65, 0.38))
                .min(line(0.35, 0.54, 0.65, 0.54))
                .min(line(0.35, 0.70, 0.53, 0.70))
        }
        Action::Refuel | Action::StopRefuel | Action::PumpFuel => {
            // Fuel pump + nozzle hose
            let pump = box_outline(0.20, 0.22, 0.60, 0.84, 0.06).min(line(0.28, 0.42, 0.52, 0.42));
            let hose = line(0.60, 0.38, 0.78, 0.38)
                .min(line(0.78, 0.38, 0.78, 0.66))
                .min(line(0.68, 0.66, 0.78, 0.66));
            pump.min(hose)
        }
        Action::PayFuel => {
            // Payment card
            box_outline(0.14, 0.24, 0.86, 0.76, 0.07)
                .min(line(0.16, 0.41, 0.84, 0.41))
                .min(line(0.26, 0.60, 0.44, 0.60))
        }
        Action::Flashlight | Action::FlashlightOff => {
            // Flashlight body + light rays
            let body = box_outline(0.18, 0.38, 0.58, 0.62, 0.05);
            let rays = line(0.68, 0.32, 0.84, 0.22)
                .min(line(0.70, 0.50, 0.86, 0.50))
                .min(line(0.68, 0.68, 0.84, 0.78));
            let base = body.min(rays);
            if action == Action::FlashlightOff {
                base.min(line(0.20, 0.82, 0.80, 0.18))
            } else {
                base
            }
        }
        Action::Camera => {
            // Camera body + lens ring
            box_outline(0.16, 0.28, 0.84, 0.78, 0.08)
                .min(line(0.36, 0.20, 0.64, 0.20))
                .min(ring(0.50, 0.53, 0.14))
        }
        Action::Crouch => {
            // Double downward chevron
            line(0.26, 0.30, 0.50, 0.52)
                .min(line(0.74, 0.30, 0.50, 0.52))
                .min(line(0.26, 0.54, 0.50, 0.76))
                .min(line(0.74, 0.54, 0.50, 0.76))
        }
        Action::HideBar => {
            // Eye with subtle slash
            ring(0.50, 0.50, 0.22).min(disc(0.50, 0.50, 0.07)).min(line(0.22, 0.78, 0.78, 0.22))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquid_glass_hud_scales_cleanly_and_records_capsule_uvs() {
        let hints = vec![
            Hint { key: "F8".into(), action: Action::GetOut, vk: 0x77, second_vk: 0 },
            Hint { key: "Alt + 4".into(), action: Action::Phone, vk: 0xA4, second_vk: 0x34 },
            Hint { key: "Alt + 5".into(), action: Action::Character, vk: 0xA4, second_vk: 0x35 },
        ];
        let painted = paint(&hints, 1920, 1080, "pt").expect("HUD painted");
        assert_eq!(painted.capsules_uv.len(), 3);
        assert_eq!(painted.pixels.len(), (painted.width * painted.height * 4) as usize);
        assert!(painted.width <= (1920.0 * MAX_WIDTH_OF_SCREEN) as u32);
        // Premultiplied invariant: RGB never exceeds alpha.
        assert!(painted.pixels.chunks_exact(4).all(|px| px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3]));
        if let Ok(dest) = std::env::var("MAXDEV_HUD_PREVIEW") {
            let mut out = format!("{} {}\n", painted.width, painted.height).into_bytes();
            out.extend_from_slice(&painted.pixels);
            let _ = std::fs::write(dest, out);
        }
    }

    #[test]
    fn animator_fades_in_and_out_without_blinking() {
        let mut anim = Animator::new();
        let t0 = Instant::now();
        let hints = [Hint { key: "F8".into(), action: Action::GetOut, vk: 0, second_vk: 0 }];
        assert!(anim.step(false, &hints, t0).is_none());
        let (op1, slide1, _) = anim.step(true, &hints, t0 + Duration::from_millis(100)).unwrap();
        assert!(op1 > 0.1 && op1 < 1.0 && slide1 > 0.0);
        let (op2, slide2, _) = anim.step(true, &hints, t0 + Duration::from_millis(350)).unwrap();
        assert!((op2 - 1.0).abs() < 1e-4 && slide2.abs() < 1e-4);
        // Switching off keeps drawing during exit fade rather than vanishing abruptly.
        let (op3, slide3, _) = anim.step(false, &hints, t0 + Duration::from_millis(420)).unwrap();
        assert!(op3 > 0.0 && op3 < 1.0 && slide3 > 0.0);
        assert!(anim.step(false, &hints, t0 + Duration::from_millis(700)).is_none());
    }
}
