//! Full-screen D3D11 transition loading overlay for Quick Job accept / cancel.
//!
//! When accepting (or cancelling) a Quick Job, Prism3D teleports the player across the map
//! and streams in the company truck and surrounding sector models over ~2–3 seconds.
//! Drawing a sleek dark loading screen ("Aguardando liberação do caminhão...") masks that
//! raw object pop-in and fades smoothly back into the 3D world once streaming settles.

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11ShaderResourceView};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, FW_BOLD, FW_MEDIUM, FW_SEMIBOLD, FillRect, GdiFlush, HFONT, HGDIOBJ,
    OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, ANTIALIASED_QUALITY, SelectObject, SetBkMode,
    SetTextColor, TRANSPARENT, TextOutW, GetTextExtentPoint32W,
};
use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::core::{Interface, w};

use crate::handset::Quad;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionKind {
    QuickAccept,
    QuickCancel,
    TruckSwap,
    TrailerAttach,
    TrailerStore,
}

struct ActiveTransition {
    kind: TransitionKind,
    started: Instant,
    hold: Duration,
    fade: Duration,
}

static ACTIVE: Mutex<Option<ActiveTransition>> = Mutex::new(None);
static RENDERER: Mutex<Option<Renderer>> = Mutex::new(None);

/// Starts the full-screen loading transition overlay.
pub fn trigger(kind: TransitionKind) {
    let (hold, fade) = match kind {
        TransitionKind::QuickAccept => (Duration::from_millis(5000), Duration::from_millis(550)),
        TransitionKind::QuickCancel => (Duration::from_millis(5500), Duration::from_millis(550)),
        TransitionKind::TruckSwap => (Duration::from_millis(3400), Duration::from_millis(450)),
        TransitionKind::TrailerAttach => (Duration::from_millis(2600), Duration::from_millis(400)),
        TransitionKind::TrailerStore => (Duration::from_millis(2000), Duration::from_millis(380)),
    };
    if let Ok(mut guard) = ACTIVE.lock() {
        *guard = Some(ActiveTransition { kind, started: Instant::now(), hold, fade });
    }
}

/// Clears any active transition immediately (e.g. if an accept/cancel failed or wasn't a Quick Job).
pub fn clear() {
    if let Ok(mut guard) = ACTIVE.lock() {
        *guard = None;
    }
}

/// Drops D3D11 resources on swapchain resize / shutdown.
pub fn reset() {
    if let Ok(mut guard) = RENDERER.lock() {
        *guard = None;
    }
}

/// Called once per frame from `d3d::on_present`, after all other overlays.
///
/// # Safety
/// `swapchain` must be the live D3D11 `IDXGISwapChain*` on the game's render thread.
pub unsafe fn on_present(swapchain: *mut c_void) {
    let state = {
        let Ok(mut guard) = ACTIVE.lock() else { return };
        let Some(active) = guard.as_ref() else { return };
        let elapsed = active.started.elapsed();
        let total = active.hold + active.fade;
        if elapsed >= total {
            *guard = None;
            return;
        }
        let progress = (elapsed.as_secs_f32() / active.hold.as_secs_f32()).clamp(0.0, 1.0);
        let alpha = if elapsed <= active.hold {
            1.0
        } else {
            let fade_t = (elapsed - active.hold).as_secs_f32() / active.fade.as_secs_f32();
            (1.0 - fade_t).clamp(0.0, 1.0)
        };
        (active.kind, progress, alpha)
    };

    // SAFETY: live swapchain on the D3D11 render thread.
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let Ok(mut guard) = RENDERER.lock() else { return };
        if matches!(&*guard, Some(r) if r.device != device) {
            *guard = None;
        }
        if guard.is_none() {
            *guard = Renderer::new(swapchain).ok();
        }
        if let Some(renderer) = guard.as_mut()
            && renderer.draw(swapchain, state.0, state.1, state.2).is_err()
        {
            *guard = None;
        }
    }
}

struct Renderer {
    device: ID3D11Device,
    quad: Quad,
    backdrop: Option<(DXGI_FORMAT, u8, ID3D11ShaderResourceView)>,
    card: Option<(DXGI_FORMAT, TransitionKind, u32, u8, (u32, u32), ID3D11ShaderResourceView)>,
}

// SAFETY: D3D11 device & SRVs accessed exclusively under `RENDERER` mutex on the present thread.
unsafe impl Send for Renderer {}

impl Renderer {
    unsafe fn new(swapchain: &IDXGISwapChain) -> windows::core::Result<Self> {
        let device: ID3D11Device = unsafe { swapchain.GetDevice()? };
        let quad = unsafe { Quad::new(&device)? };
        Ok(Self { device, quad, backdrop: None, card: None })
    }

    unsafe fn draw(
        &mut self,
        swapchain: &IDXGISwapChain,
        kind: TransitionKind,
        progress: f32,
        alpha: f32,
    ) -> windows::core::Result<()> {
        let desc = unsafe { swapchain.GetDesc()? };
        let width = desc.BufferDesc.Width.max(1);
        let height = desc.BufferDesc.Height.max(1);
        let format = desc.BufferDesc.Format;
        let context = unsafe { self.device.GetImmediateContext()? };

        // 1. Full-screen pitch-dark backdrop (100% opaque while holding so zero object pop-in leaks through).
        let bg_a = ((alpha * 255.0).round() as u8).clamp(0, 255);
        let need_bg = self.backdrop.as_ref().is_none_or(|(f, a, _)| *f != format || *a != bg_a);
        if need_bg {
            // Premultiplied BGRA (deep black #030305).
            let r = (3u32 * u32::from(bg_a) / 255) as u8;
            let g = (3u32 * u32::from(bg_a) / 255) as u8;
            let b = (5u32 * u32::from(bg_a) / 255) as u8;
            let pixels = [b, g, r, bg_a].repeat(16);
            let view = unsafe { crate::hints::upload(&self.device, 4, 4, &pixels, format)? };
            self.backdrop = Some((format, bg_a, view));
        }
        if let Some((_, _, view)) = &self.backdrop {
            unsafe { self.quad.draw(&self.device, &context, swapchain, view, [-1.0, 1.0, 1.0, -1.0])? };
        }

        // 2. Centered loading card with animated progress bar (24 discrete steps for smooth bar without per-frame GDI).
        let step = (progress * 24.0).round() as u32;
        let card_a = ((alpha * 255.0).round() as u8).clamp(0, 255);
        let need_card = self
            .card
            .as_ref()
            .is_none_or(|(f, k, s, a, _, _)| *f != format || *k != kind || *s != step || *a != card_a);
        if need_card {
            let lang = crate::character::language();
            let scale = (height as f32 / 1080.0).clamp(0.75, 1.6);
            if let Some((w, h, pixels)) = paint_card(kind, step as f32 / 24.0, card_a, scale, lang) {
                let view = unsafe { crate::hints::upload(&self.device, w, h, &pixels, format)? };
                self.card = Some((format, kind, step, card_a, (w, h), view));
            }
        }
        if let Some((_, _, _, _, (cw, ch), view)) = &self.card {
            let half_w = (*cw as f32) / (width as f32);
            let half_h = (*ch as f32) / (height as f32);
            let rect = [-half_w, half_h, half_w, -half_h];
            unsafe { self.quad.draw(&self.device, &context, swapchain, view, rect)? };
        }
        Ok(())
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn paint_card(kind: TransitionKind, progress: f32, alpha: u8, scale: f32, language: &str) -> Option<(u32, u32, Vec<u8>)> {
    let w = (540.0 * scale).round() as i32;
    let h = (168.0 * scale).round() as i32;
    let (kicker, title, subtitle) = match (language, kind) {
        ("pt", TransitionKind::QuickAccept) => (
            "MAXDEVMOD  ·  FRETE RÁPIDO",
            "Aguardando liberação do caminhão...",
            "Carregando veículo da empresa e cenário",
        ),
        ("pt", TransitionKind::QuickCancel) => (
            "MAXDEVMOD  ·  FRETE RÁPIDO",
            "Retornando ao seu caminhão...",
            "Preparando seu veículo na estrada",
        ),
        ("pt", TransitionKind::TruckSwap) => (
            "MAXDEVMOD  ·  MINHA FROTA",
            "Trocando de caminhão...",
            "Preparando veículo da garagem e cabine",
        ),
        ("pt", TransitionKind::TrailerAttach) => (
            "MAXDEVMOD  ·  GARAGEM DE CARRETAS",
            "Preparando sua carreta...",
            "Acoplando implemento ao caminhão",
        ),
        ("pt", TransitionKind::TrailerStore) => (
            "MAXDEVMOD  ·  GARAGEM DE CARRETAS",
            "Guardando carreta na garagem...",
            "Liberando engate do caminhão",
        ),
        ("es", TransitionKind::QuickAccept) => (
            "MAXDEVMOD  ·  TRABAJO RÁPIDO",
            "Esperando liberación del camión...",
            "Cargando vehículo de la empresa y mapa",
        ),
        ("es", TransitionKind::QuickCancel) => (
            "MAXDEVMOD  ·  TRABAJO RÁPIDO",
            "Volviendo a tu camión...",
            "Preparando tu vehículo en carretera",
        ),
        ("es", TransitionKind::TruckSwap) => (
            "MAXDEVMOD  ·  MI FLOTA",
            "Cambiando de camión...",
            "Preparando vehículo del garaje y cabina",
        ),
        ("es", TransitionKind::TrailerAttach) => (
            "MAXDEVMOD  ·  REMOLQUES",
            "Preparando tu remolque...",
            "Acoplando remolque al camión",
        ),
        ("es", TransitionKind::TrailerStore) => (
            "MAXDEVMOD  ·  REMOLQUES",
            "Guardando remolque en el garaje...",
            "Desacoplando del camión",
        ),
        (_, TransitionKind::QuickAccept) => (
            "MAXDEVMOD  ·  QUICK JOB",
            "Waiting for truck clearance...",
            "Loading company vehicle and world scenery",
        ),
        (_, TransitionKind::QuickCancel) => (
            "MAXDEVMOD  ·  QUICK JOB",
            "Returning to your truck...",
            "Restoring your vehicle on the road",
        ),
        (_, TransitionKind::TruckSwap) => (
            "MAXDEVMOD  ·  MY FLEET",
            "Switching trucks...",
            "Preparing your garage truck and cabin",
        ),
        (_, TransitionKind::TrailerAttach) => (
            "MAXDEVMOD  ·  TRAILER MANAGER",
            "Preparing your trailer...",
            "Coupling trailer to your truck",
        ),
        (_, TransitionKind::TrailerStore) => (
            "MAXDEVMOD  ·  TRAILER MANAGER",
            "Storing trailer in garage...",
            "Detaching trailer from your truck",
        ),
    };
    // Distinct accent palette per transition kind (BGR format for Win32 COLORREF).
    let (accent_bar, kicker_color, border_color) = match kind {
        TransitionKind::QuickAccept | TransitionKind::QuickCancel => (0x0035_39E5, 0x0048_4AE5, 0x0026_2022), // Crimson red
        TransitionKind::TruckSwap => (0x00FF_840A, 0x00FF_A23A, 0x0032_2418),                                 // iOS Blue
        TransitionKind::TrailerAttach => (0x000B_9EF5, 0x002C_B8FB, 0x001E_2630),                             // Amber gold
        TransitionKind::TrailerStore => (0x005E_C522, 0x007A_DE4A, 0x001E_2C22),                              // Emerald green
    };

    // SAFETY: local GDI objects cleaned up before returning.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
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
        let prev_bmp = SelectObject(dc, HGDIOBJ(bitmap.0));

        let fill = |rect: RECT, bgr: u32| {
            let brush = CreateSolidBrush(COLORREF(bgr));
            FillRect(dc, &rect, brush);
            let _ = DeleteObject(HGDIOBJ(brush.0));
        };

        // Border + card surface + top accent bar (BGR format for COLORREF).
        fill(RECT { left: 0, top: 0, right: w, bottom: h }, border_color);
        fill(RECT { left: 2, top: 2, right: w - 2, bottom: h - 2 }, 0x0014_1213);
        fill(RECT { left: 2, top: 2, right: w - 2, bottom: (6.0 * scale).round() as i32 }, accent_bar);

        let make_font = |px: f32, weight: i32| -> HFONT {
            CreateFontW(
                -(px * scale).round() as i32,
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
        let f_kicker = make_font(12.5, FW_BOLD.0 as i32);
        let f_title = make_font(22.0, FW_BOLD.0 as i32);
        let f_sub = make_font(14.0, FW_MEDIUM.0 as i32);
        let f_pct = make_font(12.0, FW_SEMIBOLD.0 as i32);

        SetBkMode(dc, TRANSPARENT);
        let draw_centered = |font: HFONT, text: &str, y: i32, color: u32| {
            SelectObject(dc, HGDIOBJ(font.0));
            SetTextColor(dc, COLORREF(color));
            let wt = wide(text);
            let mut sz = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, &wt, &mut sz);
            let x = (w - sz.cx) / 2;
            let _ = TextOutW(dc, x, y, &wt);
        };

        draw_centered(f_kicker, kicker, (22.0 * scale).round() as i32, kicker_color);
        draw_centered(f_title, title, (48.0 * scale).round() as i32, 0x00FA_F7F5);
        draw_centered(f_sub, subtitle, (84.0 * scale).round() as i32, 0x00A8_A29E);

        // Progress bar track + themed accent fill
        let pad_x = (42.0 * scale).round() as i32;
        let bar_y = (124.0 * scale).round() as i32;
        let bar_h = (8.0 * scale).round().max(4.0) as i32;
        let bar_w = w - pad_x * 2;
        fill(RECT { left: pad_x, top: bar_y, right: pad_x + bar_w, bottom: bar_y + bar_h }, 0x002A_2628);
        let fill_w = ((bar_w as f32) * progress.clamp(0.06, 1.0)).round() as i32;
        fill(RECT { left: pad_x, top: bar_y, right: pad_x + fill_w, bottom: bar_y + bar_h }, accent_bar);

        let pct = format!("{}%", (progress * 100.0).round() as u32);
        draw_centered(f_pct, &pct, (139.0 * scale).round() as i32, 0x0090_8A86);

        let _ = GdiFlush();
        let byte_len = (w as usize) * (h as usize) * 4;
        let mut pixels = std::slice::from_raw_parts(bits as *const u8, byte_len).to_vec();
        // GDI writes alpha=0; premultiply all pixels by `alpha`.
        for px in pixels.chunks_exact_mut(4) {
            px[0] = (u16::from(px[0]) * u16::from(alpha) / 255) as u8;
            px[1] = (u16::from(px[1]) * u16::from(alpha) / 255) as u8;
            px[2] = (u16::from(px[2]) * u16::from(alpha) / 255) as u8;
            px[3] = alpha;
        }

        for f in [f_kicker, f_title, f_sub, f_pct] {
            let _ = DeleteObject(HGDIOBJ(f.0));
        }
        SelectObject(dc, prev_bmp);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
        Some((w as u32, h as u32, pixels))
    }
}
