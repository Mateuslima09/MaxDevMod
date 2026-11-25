//! Floating 3D world-space payment speech bubble ("balãozinho") above the fuel attendant's head.
//!
//! When the player finishes refuelling and exits Refueling Mode, an attendant stands in front of the truck
//! waiting for payment. This overlay projects the attendant's 3D head world position onto the screen using
//! the walking camera's live pose and renders a crisp Liquid Glass speech bubble with payment icon,
//! formatted amount owed, and `[E]` prompt (or a directional edge pill when the attendant is off-screen).

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::Instant;

use windows::Win32::Foundation::{COLORREF, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_TEXTURE2D_DESC, ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLIP_DEFAULT_PRECIS, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    Ellipse, FW_BOLD, FW_EXTRABOLD, FW_SEMIBOLD, FillRect, GdiFlush, GetStockObject, GetTextExtentPoint32W,
    HFONT, HGDIOBJ, NULL_PEN, OUT_DEFAULT_PRECIS, Polygon, RoundRect, SelectObject, SetBkMode, SetTextColor,
    TRANSPARENT, TextOutW,
};
use windows::core::{Interface, w};

use crate::handset::{Quad, view_format};
use crate::log;

/// Height above the attendant's ground feet where the speech bubble anchors (metres).
const HEAD_OFFSET_Y: f64 = 2.08;
/// Vertical FOV of camera 0 (radians, matches Prism3D's default free-camera projection).
const CAMERA_FOV_Y_DEG: f32 = 55.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BubbleKind {
    /// Fuel station attendant waiting for payment.
    FuelPayment,
    /// Freight clerk waiting to hand over / sign the pickup or load Nota Fiscal.
    FreightPickup,
    /// Freight clerk waiting to receive / sign the delivery Nota Fiscal.
    FreightDelivery,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BubbleState {
    /// Which NPC / interaction this bubble represents.
    pub kind: BubbleKind,
    /// NPC ground world coordinates `[x, y, z]`.
    pub world: [f64; 3],
    /// Camera eye world coordinates `[x, y, z]`.
    pub cam_pos: [f64; 3],
    /// Camera look yaw (radians) & pitch (radians).
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    /// Primary detail line (e.g. `"R$ 450,00"` or cargo / action label).
    pub detail_text: String,
    /// Two-letter game language (`"pt"`, `"es"`, `"en"`, …).
    pub language: String,
    /// Player is within E-key interaction range of the NPC.
    pub in_range: bool,
    /// Horizontal ground distance from player to NPC (metres).
    pub distance_m: f32,
}

static STATE: Mutex<Option<BubbleState>> = Mutex::new(None);
static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Updates or clears the floating payment speech bubble (called from game thread).
pub fn set(next: Option<BubbleState>) {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = next;
}

/// Colors in 0xBBGGRR (Win32 COLORREF).
const BG_DARK: u32 = 0x221A14; // rich obsidian glass
const BORDER_GOLD: u32 = 0x26B5F5; // warm gold/amber accent (#F5B526)
const BORDER_BLUE: u32 = 0xF8BD38; // crisp sky blue for pickup Nota Fiscal (#38BDF8)
const BORDER_ORANGE: u32 = 0x3C92FB; // warm amber-orange for delivery Nota Fiscal (#FB923C)
const BORDER_GREEN: u32 = 0x58D130; // vivid emerald when in range (#30D158)
const BADGE_BG_NEAR: u32 = 0x58D130;
const BADGE_TEXT: u32 = 0x101010;
const TEXT_WHITE: u32 = 0xFFFFFF;
const TEXT_MUTED: u32 = 0xC8D0D8;
const CARD_BODY: u32 = 0x45C9FF;
const CARD_STRIPE: u32 = 0x221A14;
const DOC_PAPER: u32 = 0xF5F5F5;
const DOC_LINE: u32 = 0x5A4E44;

#[derive(Clone, Debug, PartialEq, Eq)]
struct PaintKey {
    kind: BubbleKind,
    detail_text: String,
    language: String,
    in_range: bool,
    dist_rounded: i32,
    arrow_dir: i8, // -1 left, 0 bubble (on-screen), +1 right
    height_px: u32,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn paint_bubble(key: &PaintKey) -> Option<(u32, u32, Vec<u8>)> {
    let em = ((key.height_px as f32) * 0.0145).clamp(11.0, 24.0);
    let pad_x = (em * 0.85).round() as i32;
    let pad_y = (em * 0.55).round() as i32;
    let gap = (em * 0.45).round() as i32;
    let tail_h = if key.arrow_dir == 0 { (em * 0.72).round() as i32 } else { 0 };
    let border = 2i32;

    let (title, action_badge) = match (key.kind, key.language.as_str(), key.in_range) {
        (BubbleKind::FuelPayment, "pt", true) => ("PAGAR FRENTISTA", "E · PAGAR"),
        (BubbleKind::FuelPayment, "pt", false) => ("PAGAR FRENTISTA", ""),
        (BubbleKind::FuelPayment, "es", true) => ("PAGAR EMPLEADO", "E · PAGAR"),
        (BubbleKind::FuelPayment, "es", false) => ("PAGAR EMPLEADO", ""),
        (BubbleKind::FuelPayment, _, true) => ("PAY ATTENDANT", "E · PAY"),
        (BubbleKind::FuelPayment, _, false) => ("PAY ATTENDANT", ""),
        (BubbleKind::FreightPickup, "pt", true) => ("NOTA FISCAL · COLETA", "E · ASSINAR"),
        (BubbleKind::FreightPickup, "pt", false) => ("NOTA FISCAL · COLETA", ""),
        (BubbleKind::FreightPickup, "es", true) => ("FACTURA · RECOGIDA", "E · FIRMAR"),
        (BubbleKind::FreightPickup, "es", false) => ("FACTURA · RECOGIDA", ""),
        (BubbleKind::FreightPickup, _, true) => ("FREIGHT PAPERS · PICKUP", "E · SIGN"),
        (BubbleKind::FreightPickup, _, false) => ("FREIGHT PAPERS · PICKUP", ""),
        (BubbleKind::FreightDelivery, "pt", true) => ("NOTA FISCAL · ENTREGA", "E · ENTREGAR"),
        (BubbleKind::FreightDelivery, "pt", false) => ("NOTA FISCAL · ENTREGA", ""),
        (BubbleKind::FreightDelivery, "es", true) => ("FACTURA · ENTREGA", "E · ENTREGAR"),
        (BubbleKind::FreightDelivery, "es", false) => ("FACTURA · ENTREGA", ""),
        (BubbleKind::FreightDelivery, _, true) => ("FREIGHT PAPERS · DELIVERY", "E · DELIVER"),
        (BubbleKind::FreightDelivery, _, false) => ("FREIGHT PAPERS · DELIVERY", ""),
    };
    let badge_text = if !action_badge.is_empty() {
        action_badge.to_owned()
    } else if key.arrow_dir < 0 {
        format!("◀ {} m", key.dist_rounded.max(1))
    } else if key.arrow_dir > 0 {
        format!("{} m ▶", key.dist_rounded.max(1))
    } else {
        format!("{} m", key.dist_rounded.max(1))
    };

    // SAFETY: GDI objects created, drawn into a local DIB, flushed, copied out, and destroyed here.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let mk_font = |size: i32, weight: i32| -> HFONT {
            CreateFontW(
                -size,
                0,
                0,
                0,
                weight,
                0,
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
        let title_font = mk_font((em * 0.78).round() as i32, FW_BOLD.0 as i32);
        let price_font = mk_font((em * 1.25).round() as i32, FW_EXTRABOLD.0 as i32);
        let badge_font = mk_font((em * 0.82).round() as i32, FW_SEMIBOLD.0 as i32);

        let measure = |font: HFONT, text: &[u16]| -> (i32, i32) {
            SelectObject(dc, HGDIOBJ(font.0));
            let mut sz = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, text, &mut sz);
            (sz.cx, sz.cy)
        };

        let title_w = wide(title);
        let detail_w = wide(&key.detail_text);
        let badge_w = wide(&badge_text);
        let (tw, th) = measure(title_font, &title_w);
        let (pw, ph) = measure(price_font, &detail_w);
        let (bw, bh) = measure(badge_font, &badge_w);

        let icon_sz = (em * 1.9).round() as i32;
        let badge_pad_x = (em * 0.55).round() as i32;
        let badge_pad_y = (em * 0.28).round() as i32;
        let badge_box_w = bw + badge_pad_x * 2;
        let badge_box_h = bh + badge_pad_y * 2;

        let text_col_w = tw.max(pw);
        let text_col_h = th + (em * 0.12).round() as i32 + ph;
        let body_h = (text_col_h.max(icon_sz).max(badge_box_h) + pad_y * 2).max(40);
        let width = (pad_x + icon_sz + gap + text_col_w + gap + badge_box_w + pad_x).max(160);
        let height = body_h + tail_h;

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
            let _ = DeleteDC(dc);
            return None;
        };
        let old_bmp = SelectObject(dc, HGDIOBJ(bitmap.0));
        let old_pen = SelectObject(dc, GetStockObject(NULL_PEN));

        // Clear to transparent black (0x00000000).
        std::ptr::write_bytes(bits as *mut u8, 0, (width * height * 4) as usize);

        let base_accent = match key.kind {
            BubbleKind::FuelPayment => BORDER_GOLD,
            BubbleKind::FreightPickup => BORDER_BLUE,
            BubbleKind::FreightDelivery => BORDER_ORANGE,
        };
        let accent = if key.in_range { BORDER_GREEN } else { base_accent };
        let radius = (em * 1.1).round() as i32;

        let draw_round = |l: i32, t: i32, r: i32, b: i32, rad: i32, color: u32| {
            let brush = CreateSolidBrush(COLORREF(color));
            let prev = SelectObject(dc, HGDIOBJ(brush.0));
            let _ = RoundRect(dc, l, t, r + 1, b + 1, rad, rad);
            SelectObject(dc, prev);
            let _ = DeleteObject(HGDIOBJ(brush.0));
        };

        let draw_poly = |pts: &[POINT], color: u32| {
            let brush = CreateSolidBrush(COLORREF(color));
            let prev = SelectObject(dc, HGDIOBJ(brush.0));
            let _ = Polygon(dc, pts);
            SelectObject(dc, prev);
            let _ = DeleteObject(HGDIOBJ(brush.0));
        };

        // Speech bubble body: outer accent rim + inner obsidian glass panel.
        draw_round(0, 0, width, body_h, radius, accent);
        draw_round(border, border, width - border, body_h - border, (radius - border).max(4), BG_DARK);

        // Downward pointer tail when anchored above the attendant.
        if tail_h > 0 {
            let cx = width / 2;
            let tw_half = (tail_h as f32 * 0.85).round() as i32;
            draw_poly(
                &[
                    POINT { x: cx - tw_half, y: body_h - border },
                    POINT { x: cx + tw_half, y: body_h - border },
                    POINT { x: cx, y: height },
                ],
                accent,
            );
            draw_poly(
                &[
                    POINT { x: cx - tw_half + border, y: body_h - border - 1 },
                    POINT { x: cx + tw_half - border, y: body_h - border - 1 },
                    POINT { x: cx, y: height - border - 1 },
                ],
                BG_DARK,
            );
        }

        // Left emblem: payment card for FuelPayment, Nota Fiscal document for FreightPickup/Delivery.
        let ix = pad_x;
        let iy = (body_h - icon_sz) / 2;
        let circle_brush = CreateSolidBrush(COLORREF(0x362C24));
        let prev_b = SelectObject(dc, HGDIOBJ(circle_brush.0));
        let _ = Ellipse(dc, ix, iy, ix + icon_sz, iy + icon_sz);
        SelectObject(dc, prev_b);
        let _ = DeleteObject(HGDIOBJ(circle_brush.0));

        if key.kind == BubbleKind::FuelPayment {
            // Mini payment card inside the circle.
            let cw = (icon_sz as f32 * 0.62).round() as i32;
            let ch = (icon_sz as f32 * 0.44).round() as i32;
            let cx0 = ix + (icon_sz - cw) / 2;
            let cy0 = iy + (icon_sz - ch) / 2;
            draw_round(cx0, cy0, cx0 + cw, cy0 + ch, 4, CARD_BODY);
            let stripe_rect = RECT {
                left: cx0 + 1,
                top: cy0 + (ch / 4),
                right: cx0 + cw - 1,
                bottom: cy0 + (ch / 4) + (ch / 5).max(2),
            };
            let stripe_brush = CreateSolidBrush(COLORREF(CARD_STRIPE));
            FillRect(dc, &stripe_rect, stripe_brush);
            let _ = DeleteObject(HGDIOBJ(stripe_brush.0));
        } else {
            // Mini Nota Fiscal document sheet inside the circle.
            let dw = (icon_sz as f32 * 0.48).round() as i32;
            let dh = (icon_sz as f32 * 0.62).round() as i32;
            let dx0 = ix + (icon_sz - dw) / 2;
            let dy0 = iy + (icon_sz - dh) / 2;
            draw_round(dx0, dy0, dx0 + dw, dy0 + dh, 3, DOC_PAPER);
            // Colored header bar + 2 text lines on the document.
            let hdr_brush = CreateSolidBrush(COLORREF(base_accent));
            let hdr_rect = RECT {
                left: dx0 + 2,
                top: dy0 + 2,
                right: dx0 + dw - 2,
                bottom: dy0 + 2 + (dh / 5).max(2),
            };
            FillRect(dc, &hdr_rect, hdr_brush);
            let _ = DeleteObject(HGDIOBJ(hdr_brush.0));

            let line_brush = CreateSolidBrush(COLORREF(DOC_LINE));
            let l1_top = dy0 + (dh * 45 / 100);
            let l2_top = dy0 + (dh * 68 / 100);
            for (top, w_frac) in [(l1_top, 100), (l2_top, 72)] {
                let r = RECT {
                    left: dx0 + 3,
                    top,
                    right: dx0 + 3 + ((dw - 6) * w_frac / 100).max(3),
                    bottom: top + 2,
                };
                FillRect(dc, &r, line_brush);
            }
            let _ = DeleteObject(HGDIOBJ(line_brush.0));
        }

        // Texts: Title + Detail line.
        SetBkMode(dc, TRANSPARENT);
        let tx = ix + icon_sz + gap;
        let ty = (body_h - text_col_h) / 2;
        SelectObject(dc, HGDIOBJ(title_font.0));
        SetTextColor(dc, COLORREF(TEXT_MUTED));
        let _ = TextOutW(dc, tx, ty, &title_w);

        SelectObject(dc, HGDIOBJ(price_font.0));
        SetTextColor(dc, COLORREF(TEXT_WHITE));
        let _ = TextOutW(dc, tx, ty + th + (em * 0.08).round() as i32, &detail_w);

        // Right pill badge (`E · PAGAR` / `E · ASSINAR` / `E · ENTREGAR` or distance).
        let bx = width - pad_x - badge_box_w;
        let by = (body_h - badge_box_h) / 2;
        draw_round(
            bx,
            by,
            bx + badge_box_w,
            by + badge_box_h,
            (badge_box_h / 2).max(6),
            if key.in_range { BADGE_BG_NEAR } else { base_accent },
        );
        SelectObject(dc, HGDIOBJ(badge_font.0));
        SetTextColor(dc, COLORREF(BADGE_TEXT));
        let _ = TextOutW(dc, bx + badge_pad_x, by + badge_pad_y, &badge_w);

        let _ = GdiFlush();
        let raw = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize);
        let mut pixels = vec![0u8; raw.len()];
        // Convert GDI RGB pixels into premultiplied BGRA (non-zero RGB -> opaque 240 alpha; untouched stays 0).
        for (src, dst) in raw.chunks_exact(4).zip(pixels.chunks_exact_mut(4)) {
            if src[0] != 0 || src[1] != 0 || src[2] != 0 {
                let a = 240u32;
                dst[0] = ((u32::from(src[0]) * a) / 255) as u8;
                dst[1] = ((u32::from(src[1]) * a) / 255) as u8;
                dst[2] = ((u32::from(src[2]) * a) / 255) as u8;
                dst[3] = a as u8;
            }
        }

        SelectObject(dc, old_pen);
        SelectObject(dc, old_bmp);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        for font in [title_font, price_font, badge_font] {
            let _ = DeleteObject(HGDIOBJ(font.0));
        }
        let _ = DeleteDC(dc);
        Some((width as u32, height as u32, pixels))
    }
}

struct CachedTex {
    key: PaintKey,
    format: DXGI_FORMAT,
    size: (u32, u32),
    view: ID3D11ShaderResourceView,
}

struct Renderer {
    device: ID3D11Device,
    quad: Quad,
    cached: Option<CachedTex>,
}

// SAFETY: accessed only on the game's render thread (`Present`), guarded by Mutex.
unsafe impl Send for Renderer {}

static DRAWING: Mutex<Option<Result<Renderer, ()>>> = Mutex::new(None);

/// Projects world position `target` through camera `(cam_pos, cam_yaw, cam_pitch)` into screen pixels `(sx, sy)`
/// plus `arrow_dir`: `0` = visible on screen, `-1` = clamped left edge, `+1` = clamped right edge.
fn project_to_screen(state: &BubbleState, width: f32, height: f32) -> (f32, f32, i8, f32) {
    let target = [state.world[0], state.world[1] + HEAD_OFFSET_Y, state.world[2]];
    let dx = target[0] - state.cam_pos[0];
    let dy = target[1] - state.cam_pos[1];
    let dz = target[2] - state.cam_pos[2];

    let yaw = f64::from(state.cam_yaw);
    let pitch = f64::from(state.cam_pitch);
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();

    // Exact camera basis matching `character.rs`:
    // forward = (-sin(yaw)*cos(pitch), sin(pitch), -cos(yaw)*cos(pitch))
    // right   = (cos(yaw), 0, -sin(yaw))
    // up      = (sin(yaw)*sin(pitch), cos(pitch), cos(yaw)*sin(pitch))
    let forward = [-sy * cp, sp, -cy * cp];
    let right = [cy, 0.0, -sy];
    let up = [sy * sp, cp, cy * sp];

    let z_cam = dx * forward[0] + dy * forward[1] + dz * forward[2];
    let x_cam = dx * right[0] + dy * right[1] + dz * right[2];
    let y_cam = dx * up[0] + dy * up[1] + dz * up[2];

    let aspect = (width / height.max(1.0)) as f64;
    let tan_half_y = (f64::from(CAMERA_FOV_Y_DEG).to_radians() * 0.5).tan();
    let tan_half_x = tan_half_y * aspect;

    let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.5) as f32;
    let scale = (4.8 / dist).clamp(0.76, 1.08);

    if z_cam > 0.25 {
        let ndc_x = (x_cam / (z_cam * tan_half_x)) as f32;
        let ndc_y = (y_cam / (z_cam * tan_half_y)) as f32;
        if ndc_x.abs() <= 0.92 && ndc_y.abs() <= 0.88 {
            let sx = (ndc_x * 0.5 + 0.5) * width;
            let sy_px = (0.5 - ndc_y * 0.5) * height;
            return (sx, sy_px, 0, scale);
        }
    }

    // Off-screen or behind camera: clamp cleanly to the left or right screen edge so the player spots the attendant.
    let dir: i8 = if x_cam < 0.0 { -1 } else { 1 };
    let sx = if dir < 0 { width * 0.13 } else { width * 0.87 };
    let sy_px = height * 0.42;
    (sx, sy_px, dir, 0.92)
}

/// Draws the floating payment speech bubble above the attendant on each presented frame.
///
/// # Safety
/// `swapchain` must be the game's live swap chain, called on the render thread inside `Present`.
pub unsafe fn on_present(swapchain: *mut c_void) {
    if crate::telemetry::paused()
        || !crate::bridge::host_alive()
        || crate::bridge::phone_out()
        || crate::bridge::paper_out()
    {
        return;
    }
    let Some(state) = STATE.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
        return;
    };

    // SAFETY: live swap chain on the game's render thread (caller's contract).
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let mut guard = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(&*guard, Some(Ok(r)) if r.device != device) {
            *guard = None;
        }
        if guard.is_none() {
            *guard = Some(
                Quad::new(&device)
                    .map(|quad| Renderer { device: device.clone(), quad, cached: None })
                    .map_err(|e| log!("fuel_bubble: couldn't initialize quad: {e}")),
            );
        }
        let Some(Ok(renderer)) = guard.as_mut() else { return };
        let Ok(back) = swapchain.GetBuffer::<ID3D11Texture2D>(0) else { return };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        back.GetDesc(&mut desc);
        drop(back);
        let (sw, sh) = (desc.Width as f32, desc.Height as f32);
        let format = view_format(desc.Format);

        let (sx, sy, arrow_dir, scale) = project_to_screen(&state, sw, sh);
        let key = PaintKey {
            kind: state.kind,
            detail_text: state.detail_text,
            language: state.language,
            in_range: state.in_range,
            dist_rounded: state.distance_m.round() as i32,
            arrow_dir,
            height_px: desc.Height,
        };

        let stale = renderer.cached.as_ref().is_none_or(|c| c.key != key || c.format != format);
        if stale {
            let Some((w, h, pixels)) = paint_bubble(&key) else { return };
            match crate::hints::upload(&device, w, h, &pixels, format) {
                Ok(view) => renderer.cached = Some(CachedTex { key, format, size: (w, h), view }),
                Err(e) => {
                    log!("fuel_bubble: texture upload failed: {e}");
                    return;
                }
            }
        }
        let Some(tex) = &renderer.cached else { return };

        // Gentle floating bobbing animation.
        let t = START_TIME.get_or_init(Instant::now).elapsed().as_secs_f32();
        let bob_px = (t * 3.4).sin() * (sh * 0.0055);
        let pulse = if state.in_range { 1.0 + (t * 5.0).sin() * 0.025 } else { 1.0 };

        let bw = tex.size.0 as f32 * scale * pulse;
        let bh = tex.size.1 as f32 * scale * pulse;

        // Anchor bottom-center of speech bubble right above the attendant's head (or centered on edge pill).
        let left = (sx - bw * 0.5).clamp(sw * 0.02, sw - bw - sw * 0.02);
        let top = if arrow_dir == 0 {
            (sy - bh + bob_px).clamp(sh * 0.04, sh - bh - sh * 0.08)
        } else {
            (sy - bh * 0.5 + bob_px * 0.5).clamp(sh * 0.08, sh - bh - sh * 0.12)
        };
        let right = left + bw;
        let bottom = top + bh;

        let rect = [
            left / sw * 2.0 - 1.0,
            1.0 - top / sh * 2.0,
            right / sw * 2.0 - 1.0,
            1.0 - bottom / sh * 2.0,
        ];

        let Ok(context) = device.GetImmediateContext() else { return };
        if let Err(e) = renderer.quad.draw(&renderer.device, &context, swapchain, &tex.view, rect) {
            log!("fuel_bubble: draw failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubble_projects_in_front_and_clamps_behind() {
        // Looking along -Z (yaw = 0, pitch = 0) from origin toward attendant at (0, 0, -5).
        let front = BubbleState {
            kind: BubbleKind::FuelPayment,
            world: [0.0, 0.0, -5.0],
            cam_pos: [0.0, 1.6, 0.0],
            cam_yaw: 0.0,
            cam_pitch: 0.0,
            detail_text: "R$ 245,50".into(),
            language: "pt".into(),
            in_range: false,
            distance_m: 5.0,
        };
        let (sx, sy, dir, scale) = project_to_screen(&front, 1920.0, 1080.0);
        assert_eq!(dir, 0);
        assert!((sx - 960.0).abs() < 2.0);
        assert!(sy > 0.0 && sy < 1080.0);
        assert!(scale >= 0.76 && scale <= 1.08);

        // Attendant behind camera to the right: clamps to right edge (+1).
        let behind = BubbleState { world: [3.0, 0.0, 5.0], ..front };
        let (sx, _, dir, _) = project_to_screen(&behind, 1920.0, 1080.0);
        assert_eq!(dir, 1);
        assert!(sx > 1500.0);
    }

    #[test]
    fn bubble_paints_valid_bgra_bitmap() {
        for kind in [BubbleKind::FuelPayment, BubbleKind::FreightPickup, BubbleKind::FreightDelivery] {
            let key = PaintKey {
                kind,
                detail_text: "Barras de Reforço".into(),
                language: "pt".into(),
                in_range: true,
                dist_rounded: 1,
                arrow_dir: 0,
                height_px: 1080,
            };
            let (w, h, pixels) = paint_bubble(&key).expect("bubble should paint");
            assert!(w > 100 && h > 40);
            assert_eq!(pixels.len(), (w * h * 4) as usize);
            // Must have non-zero alpha pixels.
            assert!(pixels.chunks_exact(4).any(|px| px[3] > 200));
        }
    }
}
