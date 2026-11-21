//! The brakes' HUD (owner 2026-11-21: "a little HUD of its own, apart from the keys' bar, two brake discs with the
//! temperature beside them and the colour changing as they heat"): the truck's and the trailer's discs as the app
//! models them (host brake_fade.rs, `HostBlock::brake_temp_*`), each with its °C under it, in the corner and at the
//! size the player chose on the phone (`HostBlock::brake_hud`). Pulsing while the brake is weakened.
//!
//! Painted on the CPU only when what it shows changes (the temperatures to 5 °C), drawn each frame with the HUD's own
//! quad (`hud::HudQuad`), as the fade is (fade.rs). Shown only driving: the electrics on, the game not paused.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::Instant;

use windows::Win32::Graphics::Direct3D11::{D3D11_TEXTURE2D_DESC, ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::Interface;

use crate::hud::HudQuad;
use crate::{bridge, log};

/// The HUD's height, of the screen's, by size (small, medium, large); its width is [`ASPECT`] times it.
const HEIGHTS: [f32; 3] = [0.085, 0.115, 0.15];
const ASPECT: f32 = 1.9;
/// Its margin from the screen's edges, of the screen's height (above the game's own bottom bar).
const MARGIN: f32 = 0.025;

/// What the HUD shows: the discs' °C (rounded to 5; the trailer's `None` without one) and its pixel size.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shown {
    truck: i32,
    trailer: Option<i32>,
    width: u32,
    height: u32,
}

struct Drawing {
    device: ID3D11Device,
    quad: HudQuad,
    painted: Option<(Shown, ID3D11ShaderResourceView)>,
}

// SAFETY: used only on the game's render thread, through the mutex.
unsafe impl Send for Drawing {}

static DRAWING: Mutex<Option<Drawing>> = Mutex::new(None);
static STARTED: Mutex<Option<Instant>> = Mutex::new(None);

/// A disc's colour by its temperature (as the phone's Freios app draws it): blue cold, green, yellow, orange, red.
pub fn disc_colour(celsius: f32) -> [u8; 3] {
    let stops: [(f32, [f32; 3]); 5] = [
        (60.0, [0x3a as f32, 0x8d as f32, 0xff as f32]),
        (180.0, [0x2e as f32, 0xcc as f32, 0x71 as f32]),
        (300.0, [0xf5 as f32, 0xc4 as f32, 0x00 as f32]),
        (400.0, [0xff as f32, 0x7a as f32, 0x00 as f32]),
        (500.0, [0xd7 as f32, 0x19 as f32, 0x20 as f32]),
    ];
    let c = if celsius <= stops[0].0 {
        stops[0].1
    } else if celsius >= stops[4].0 {
        stops[4].1
    } else {
        let i = stops.iter().position(|s| celsius < s.0).unwrap_or(4).max(1);
        let ((a, ca), (b, cb)) = (stops[i - 1], stops[i]);
        let t = (celsius - a) / (b - a);
        [ca[0] + (cb[0] - ca[0]) * t, ca[1] + (cb[1] - ca[1]) * t, ca[2] + (cb[2] - ca[2]) * t]
    };
    c.map(|v| v.round().clamp(0.0, 255.0) as u8)
}

/// A 5×7 font for what the HUD writes (digits, °, C and the discs' names), a row a byte, its high 5 bits.
fn glyph(ch: char) -> [u8; 7] {
    match ch {
        '0' => [0x70, 0x88, 0x98, 0xa8, 0xc8, 0x88, 0x70],
        '1' => [0x20, 0x60, 0x20, 0x20, 0x20, 0x20, 0x70],
        '2' => [0x70, 0x88, 0x08, 0x10, 0x20, 0x40, 0xf8],
        '3' => [0xf8, 0x10, 0x20, 0x10, 0x08, 0x88, 0x70],
        '4' => [0x10, 0x30, 0x50, 0x90, 0xf8, 0x10, 0x10],
        '5' => [0xf8, 0x80, 0xf0, 0x08, 0x08, 0x88, 0x70],
        '6' => [0x30, 0x40, 0x80, 0xf0, 0x88, 0x88, 0x70],
        '7' => [0xf8, 0x08, 0x10, 0x20, 0x40, 0x40, 0x40],
        '8' => [0x70, 0x88, 0x88, 0x70, 0x88, 0x88, 0x70],
        '9' => [0x70, 0x88, 0x88, 0x78, 0x08, 0x10, 0x60],
        '°' => [0x60, 0x90, 0x90, 0x60, 0x00, 0x00, 0x00],
        'C' => [0x70, 0x88, 0x80, 0x80, 0x80, 0x88, 0x70],
        'A' => [0x70, 0x88, 0x88, 0xf8, 0x88, 0x88, 0x88],
        'V' => [0x88, 0x88, 0x88, 0x88, 0x88, 0x50, 0x20],
        'L' => [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0xf8],
        'O' => [0x70, 0x88, 0x88, 0x88, 0x88, 0x88, 0x70],
        'R' => [0xf0, 0x88, 0x88, 0xf0, 0xa0, 0x90, 0x88],
        'E' => [0xf8, 0x80, 0x80, 0xf0, 0x80, 0x80, 0xf8],
        'T' => [0xf8, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20],
        '-' => [0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00],
        _ => [0; 7],
    }
}

/// Premultiplied BGRA pixels, `width` × `height`.
struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Self {
        Self { width, height, pixels: vec![0; (width * height * 4) as usize] }
    }

    /// `rgb` at `alpha` (0..1) over pixel (x, y).
    fn blend(&mut self, x: i32, y: i32, rgb: [u8; 3], alpha: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || alpha <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        let a = alpha.min(1.0);
        let p = &mut self.pixels[i..i + 4];
        for (k, c) in [rgb[2], rgb[1], rgb[0]].into_iter().enumerate() {
            p[k] = (f32::from(c) * a + f32::from(p[k]) * (1.0 - a)).round() as u8;
        }
        p[3] = (255.0 * a + f32::from(p[3]) * (1.0 - a)).round() as u8;
    }

    /// A filled shape by its signed distance (negative inside), antialiased over a pixel.
    fn fill(&mut self, rgb: [u8; 3], alpha: f32, bounds: [f32; 4], distance: impl Fn(f32, f32) -> f32) {
        let [x0, y0, x1, y1] = bounds;
        for y in (y0.floor() as i32 - 1)..=(y1.ceil() as i32 + 1) {
            for x in (x0.floor() as i32 - 1)..=(x1.ceil() as i32 + 1) {
                let d = distance(x as f32 + 0.5, y as f32 + 0.5);
                let coverage = (0.5 - d).clamp(0.0, 1.0);
                self.blend(x, y, rgb, alpha * coverage);
            }
        }
    }

    /// Text centred on `cx` from `top`, `scale` pixels a font dot.
    fn text(&mut self, text: &str, cx: f32, top: f32, scale: f32, rgb: [u8; 3]) {
        let count = text.chars().count() as f32;
        let left = cx - (count * 6.0 - 1.0) * scale / 2.0;
        for (n, ch) in text.chars().enumerate() {
            let rows = glyph(ch);
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..5 {
                    if bits & (0x80 >> col) != 0 {
                        let x = left + (n as f32 * 6.0 + col as f32) * scale;
                        let y = top + row as f32 * scale;
                        self.fill(rgb, 1.0, [x, y, x + scale, y + scale], |px, py| {
                            let dx = (px - (x + scale / 2.0)).abs() - scale / 2.0;
                            let dy = (py - (y + scale / 2.0)).abs() - scale / 2.0;
                            dx.max(dy)
                        });
                    }
                }
            }
        }
    }
}

/// A rounded rectangle's signed distance.
fn round_rect(px: f32, py: f32, [x0, y0, x1, y1]: [f32; 4], r: f32) -> f32 {
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (hx, hy) = ((x1 - x0) / 2.0 - r, (y1 - y0) / 2.0 - r);
    let (qx, qy) = ((px - cx).abs() - hx, (py - cy).abs() - hy);
    (qx.max(0.0).hypot(qy.max(0.0))) + qx.max(qy).min(0.0) - r
}

/// One disc at (cx, cy), radius `r`: its colour, the hub, six vents, the caliper over its top right.
fn disc(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, colour: [u8; 3]) {
    let dark = [0x1c, 0x1c, 0x1e];
    canvas.fill(colour, 1.0, [cx - r, cy - r, cx + r, cy + r], |x, y| (x - cx).hypot(y - cy) - r);
    canvas.fill(dark, 1.0, [cx - r, cy - r, cx + r, cy + r], |x, y| (x - cx).hypot(y - cy) - r * 0.34);
    for k in 0..6 {
        let a = k as f32 * std::f32::consts::PI / 3.0;
        let (vx, vy) = (cx + r * 0.66 * a.cos(), cy + r * 0.66 * a.sin());
        let vr = r * 0.09;
        canvas.fill(dark, 1.0, [vx - vr, vy - vr, vx + vr, vy + vr], |x, y| (x - vx).hypot(y - vy) - vr);
    }
    // The caliper: a thick arc piece over the disc's top right.
    let caliper = [0x3a, 0x3a, 0x3c];
    canvas.fill(caliper, 1.0, [cx - r * 1.1, cy - r * 1.1, cx + r * 1.1, cy + r * 1.1], |x, y| {
        let (dx, dy) = (x - cx, y - cy);
        let ring = ((dx.hypot(dy)) - r * 0.92).abs() - r * 0.16;
        let angle = dy.atan2(dx).to_degrees();
        let span = (angle + 55.0).abs() - 22.0;
        ring.max(span * r / 30.0)
    });
}

/// The HUD's picture: its glass, the two discs and their °C and names.
fn paint(shown: Shown) -> Canvas {
    let (w, h) = (shown.width as f32, shown.height as f32);
    let mut canvas = Canvas::new(shown.width, shown.height);
    let glass = [0x10, 0x10, 0x10];
    canvas.fill(glass, 0.72, [0.0, 0.0, w, h], |x, y| round_rect(x, y, [0.5, 0.5, w - 0.5, h - 0.5], h * 0.16));
    let r = h * 0.27;
    let scale = (h / 46.0).max(1.0);
    let grey = [0x8e, 0x8e, 0x93];
    for (i, (name, celsius)) in [("CAVALO", Some(shown.truck)), ("CARRETA", shown.trailer)].into_iter().enumerate() {
        let cx = w * (0.27 + 0.46 * i as f32);
        let cy = h * 0.36;
        let colour = celsius.map_or(grey, |c| disc_colour(c as f32));
        disc(&mut canvas, cx, cy, r, colour);
        let label = celsius.map_or("-".to_owned(), |c| format!("{c}°C"));
        canvas.text(&label, cx, h * 0.68, scale, [0xff, 0xff, 0xff]);
        canvas.text(name, cx, h * 0.86, (scale * 0.6).max(1.0), [0xae, 0xb4, 0xba]);
    }
    canvas
}

/// Where the HUD goes in clip space (left, top, right, bottom), for a corner (`BRAKE_HUD_*`) and its pixel size.
fn place(screen_w: f32, screen_h: f32, corner: u32, (w, h): (f32, f32)) -> [f32; 4] {
    let margin = screen_h * MARGIN;
    let left = if corner & 1 == 0 { margin } else { screen_w - margin - w };
    let top = if corner & 2 == 0 { screen_h - margin - h } else { margin };
    let x = |px: f32| px / screen_w * 2.0 - 1.0;
    let y = |py: f32| 1.0 - py / screen_h * 2.0;
    [x(left), y(top), x(left + w), y(top + h)]
}

/// Draws the brakes' HUD over the frame, if it's shown.
///
/// # Safety
/// `swapchain` is the game's live swap chain, on its render thread, in `Present`.
pub unsafe fn on_present(swapchain: *mut c_void) {
    let Some(bridge) = bridge::get().filter(|_| bridge::host_alive()) else { return };
    let said = bridge.host.brake_hud.load(Ordering::Acquire);
    let driving = bridge.plugin.telemetry.read().is_some_and(|t| !t.paused && t.electric_enabled);
    if said & 0x100 == 0 || said & 1 == 0 || !driving {
        *STARTED.lock().unwrap_or_else(|e| e.into_inner()) = None;
        return;
    }
    let truck = f32::from_bits(bridge.host.brake_temp_truck.load(Ordering::Acquire));
    let trailer = f32::from_bits(bridge.host.brake_temp_trailer.load(Ordering::Acquire));
    let strength = f32::from_bits(bridge.host.brake_strength.load(Ordering::Acquire));
    if !truck.is_finite() {
        return;
    }
    let corner = (said >> 1) & 3;
    let size = ((said >> 3) & 3).min(2) as usize;
    let five = |c: f32| (c / 5.0).round() as i32 * 5;
    // SAFETY: the caller's contract: the live swap chain on the render thread.
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let Ok(back) = swapchain.GetBuffer::<ID3D11Texture2D>(0) else { return };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        back.GetDesc(&mut desc);
        let (screen_w, screen_h) = (desc.Width as f32, desc.Height as f32);
        let height = (screen_h * HEIGHTS[size]).round().max(24.0);
        let shown = Shown {
            truck: five(truck),
            trailer: trailer.is_finite().then(|| five(trailer)),
            width: (height * ASPECT).round() as u32,
            height: height as u32,
        };
        let mut drawing = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        if drawing.as_ref().is_some_and(|d| d.device != device) {
            *drawing = None;
        }
        if drawing.is_none() {
            match HudQuad::new(&device) {
                Ok(quad) => *drawing = Some(Drawing { device: device.clone(), quad, painted: None }),
                Err(e) => {
                    log!("brake hud: can't be drawn: {e}");
                    return;
                }
            }
        }
        let Some(made) = drawing.as_mut() else { return };
        if made.painted.as_ref().is_none_or(|(was, _)| *was != shown) {
            let canvas = paint(shown);
            let format = crate::handset::view_format(desc.Format);
            match crate::hints::upload(&made.device, canvas.width, canvas.height, &canvas.pixels, format) {
                Ok(view) => made.painted = Some((shown, view)),
                Err(e) => {
                    log!("brake hud: not painted: {e}");
                    return;
                }
            }
        }
        let Some((_, view)) = made.painted.as_ref() else { return };
        // Coming in over 200 ms; pulsing while the brake is weakened (faster the weaker).
        let started = *STARTED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Instant::now);
        let since = started.elapsed().as_secs_f32();
        let mut opacity = (since / 0.2).min(1.0);
        if strength.is_finite() && strength > 0.0 && strength < 0.999 {
            let rate = 1.0 + 2.0 * (1.0 - strength);
            opacity *= 0.7 + 0.3 * (since * rate * std::f32::consts::TAU).cos().abs();
        }
        let rect = place(screen_w, screen_h, corner, (shown.width as f32, shown.height as f32));
        let Ok(context) = made.device.GetImmediateContext() else { return };
        if let Err(e) = made.quad.draw(&made.device, &context, swapchain, view, rect, opacity, None) {
            log!("brake hud: not drawn: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_go_from_blue_to_red() {
        assert_eq!(disc_colour(20.0), [0x3a, 0x8d, 0xff]);
        assert_eq!(disc_colour(900.0), [0xd7, 0x19, 0x20]);
        let warm = disc_colour(240.0);
        assert!(warm[0] > 0x2e && warm[2] < 0x71, "{warm:?}");
    }

    #[test]
    fn it_paints_its_glass_and_discs() {
        let canvas = paint(Shown { truck: 320, trailer: None, width: 190, height: 100 });
        assert_eq!(canvas.pixels.len(), 190 * 100 * 4);
        // A corner left clear (rounded), the middle of the glass seen.
        assert_eq!(canvas.pixels[3], 0);
        let middle = ((50 * 190 + 95) * 4 + 3) as usize;
        assert!(canvas.pixels[middle] > 150);
        // The truck's disc in its colour (yellow-orange at 320 °C), the trailer's grey.
        let at = |x: u32, y: u32| {
            let i = ((y * 190 + x) * 4) as usize;
            [canvas.pixels[i + 2], canvas.pixels[i + 1], canvas.pixels[i]]
        };
        let truck = at((190.0 * 0.27 + 100.0 * 0.12) as u32, 36);
        assert!(truck[0] > 200 && truck[2] < 60, "{truck:?}");
        let trailer = at((190.0 * 0.73 + 100.0 * 0.12) as u32, 36);
        assert!(trailer[0] > 0x80 && (i32::from(trailer[0]) - i32::from(trailer[2])).abs() < 10, "{trailer:?}");
    }

    /// Writes a picture of it to look at (`cargo test -p westplay-plugin brake_hud_picture -- --ignored`).
    #[test]
    #[ignore]
    fn brake_hud_picture() {
        let canvas = paint(Shown { truck: 320, trailer: Some(455), width: 380, height: 200 });
        let (w, h) = (canvas.width, canvas.height);
        let mut bmp = Vec::new();
        let size = 54 + w * h * 4;
        bmp.extend(b"BM");
        for v in [size, 0, 54, 40] {
            bmp.extend(v.to_le_bytes());
        }
        bmp.extend((w as i32).to_le_bytes());
        bmp.extend((-(h as i32)).to_le_bytes());
        bmp.extend(1u16.to_le_bytes());
        bmp.extend(32u16.to_le_bytes());
        for v in [0u32, w * h * 4, 2835, 2835, 0, 0] {
            bmp.extend(v.to_le_bytes());
        }
        // Over a grey road, as in the game.
        for p in canvas.pixels.chunks(4) {
            let a = f32::from(p[3]) / 255.0;
            for k in 0..3 {
                bmp.push((f32::from(p[k]) + 110.0 * (1.0 - a)).round() as u8);
            }
            bmp.push(255);
        }
        std::fs::write(std::env::temp_dir().join("brake_hud.bmp"), bmp).unwrap();
    }

    #[test]
    fn it_sits_in_its_corner() {
        let [left, top, right, bottom] = place(1920.0, 1080.0, 0, (190.0, 100.0));
        assert!(left < -0.9 && bottom < -0.85 && right > left && top > bottom);
        let [left, top, ..] = place(1920.0, 1080.0, 3, (190.0, 100.0));
        assert!(left > 0.7 && top > 0.9);
    }
}
