//! The pump's meter over the game while the tank is filled on foot (owner 2026-11-12: "a little panel like the ones at
//! the pump, the litres going in and the price, going up as it fills"): the game's own panel at the pump is kept
//! from the screen then (`character::refuel`), so this one says what it said, in WestPlay's look.
//!
//! The game thread says what's there (`set`, from `refuel`): the fuel gone in and the pump's price, read from the
//! game's own panel as it makes it (`character::notice`); the render thread paints it (GDI) when it changes and draws
//! it at the screen's top right, as the key hints bar is drawn (`hints`).

use std::ffi::c_void;
use std::sync::Mutex;

use windows::Win32::Foundation::{COLORREF, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_TEXTURE2D_DESC, ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLIP_DEFAULT_PRECIS, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DIB_RGB_COLORS, DeleteDC, DeleteObject, FW_BOLD,
    FW_SEMIBOLD, FillRect, GdiFlush, GetStockObject, GetTextExtentPoint32W, HDC, HFONT, HGDIOBJ, NULL_PEN,
    OUT_DEFAULT_PRECIS, Polygon, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
};
use windows::core::{Interface, w};

use crate::handset::{Quad, view_format};
use crate::log;

/// What the meter shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    /// The fuel gone in, in the game's unit (litres, or gallons in the American game).
    pub amount: f32,
    /// The pump's price for one of that unit, as the game's panel said it (`None`: not heard).
    pub price: Option<f32>,
    pub gallons: bool,
    /// Done filling (the last of it, held a moment).
    pub done: bool,
    /// Done, and still to pay at the pump's attendant (`character::refuel`'s bill).
    pub owed: bool,
}

/// What's there now (rounded as it's shown: a change to it is a new picture), and a count of its changes.
static NOW: Mutex<(u64, Option<Reading>)> = Mutex::new((0, None));

/// What the meter shows now (`None`: nothing), the game thread.
pub fn set(reading: Option<Reading>) {
    // Rounded as shown, so a picture isn't painted again each frame for what looks the same.
    // (Done, to the hundredth: as the phone's bill says it.)
    let reading = reading.map(|r| {
        let steps = if r.done { 100.0 } else { 10.0 };
        Reading { amount: (r.amount * steps).round() / steps, ..r }
    });
    let mut now = NOW.lock().unwrap_or_else(|e| e.into_inner());
    if now.1 != reading {
        now.0 += 1;
        now.1 = reading;
    }
}

/// Its look, as a fuel pump's face (owner 2026-11-12: "like a real fuel pump's panel, its letters, its numbers"):
/// a light face with the fuel's name on a blue band, and three liquid-crystal windows of seven-segment digits (the
/// unlit segments faint behind, as on a pump): the total to pay, the amount, the price. Its size by the screen's
/// height (`LETTERS_OF_SCREEN`), its gaps to the screen's top and right (of the screen's height).
const LETTERS_OF_SCREEN: f32 = 0.0135;
const TOP_GAP_OF_SCREEN: f32 = 0.12;
/// (At the left: the phone comes out at the right, owner 2026-11-12, "it's over the phone".)
const LEFT_GAP_OF_SCREEN: f32 = 0.025;
/// Colours (0xBBGGRR): the face, its rim, the band and its letters, the labels, a window, its lit and unlit
/// segments; the face's opacity.
const FACE: u32 = 0xe9e6e3;
const RIM: u32 = 0x8c8782;
const BAND: u32 = 0x9c4e1f;
const BAND_TEXT: u32 = 0xffffff;
const LABEL: u32 = 0x3a3530;
const WINDOW: u32 = 0xa9c4b4;
const WINDOW_EDGE: u32 = 0x4a4a46;
const LIT: u32 = 0x1c2018;
const UNLIT: u32 = 0x98b2a2;
const OPACITY: u32 = 248;

/// The money's sign: dollars in the American game, reais in Portuguese (the owner's game shows R$), else euros.
pub fn money_sign(language: &str, gallons: bool) -> &'static str {
    match (gallons, language) {
        (true, _) => "$",
        (false, "pt") => "R$",
        _ => "€",
    }
}

/// One of the pump's windows: its label, the unit beside the digits, the digits' cells (whole, decimals), their
/// value (`None`: not known, dashes), and how tall its digits are (of the letters).
struct Window {
    label: String,
    unit: String,
    cells: (usize, usize),
    value: Option<f32>,
    tall: f32,
}

/// The pump's face: its band's words (the fuel; whether still filling) and its windows.
fn face(reading: &Reading, language: &str) -> (String, String, Vec<Window>) {
    let fuel = match language {
        "es" => "DIÉSEL",
        _ => "DIESEL",
    };
    let state = match (language, reading.done) {
        ("pt", _) if reading.owed => "A PAGAR",
        ("es", _) if reading.owed => "A PAGAR",
        (_, _) if reading.owed => "TO PAY",
        ("pt", false) => "ABASTECENDO",
        ("pt", true) => "FIM",
        ("es", false) => "REPOSTANDO",
        ("es", true) => "FIN",
        (_, false) => "FUELLING",
        (_, true) => "DONE",
    };
    let (total, amount, price) = match (language, reading.gallons) {
        ("pt", false) => ("TOTAL A PAGAR", "LITROS", "PREÇO / LITRO"),
        ("pt", true) => ("TOTAL A PAGAR", "GALÕES", "PREÇO / GALÃO"),
        ("es", false) => ("TOTAL A PAGAR", "LITROS", "PRECIO / LITRO"),
        ("es", true) => ("TOTAL A PAGAR", "GALONES", "PRECIO / GALÓN"),
        (_, false) => ("TOTAL SALE", "LITRES", "PRICE / LITRE"),
        (_, true) => ("TOTAL SALE", "GALLONS", "PRICE / GALLON"),
    };
    let sign = money_sign(language, reading.gallons).to_owned();
    let unit = if reading.gallons { "gal" } else { "L" };
    let windows = vec![
        Window {
            label: total.into(),
            unit: sign.clone(),
            cells: (5, 2),
            value: reading.price.map(|p| reading.amount * p),
            tall: 2.3,
        },
        Window { label: amount.into(), unit: unit.into(), cells: (5, 2), value: Some(reading.amount), tall: 2.3 },
        Window { label: price.into(), unit: sign, cells: (2, 2), value: reading.price, tall: 1.6 },
    ];
    (fuel.into(), state.into(), windows)
}

/// What each of a window's cells shows: a digit, a dash (not known), or nothing lit; and whether a decimal point
/// follows it.
fn cells_of(value: Option<f32>, (whole, decimals): (usize, usize)) -> Vec<(Option<char>, bool)> {
    let count = whole + decimals;
    let digits: Vec<char> = match value {
        Some(value) => {
            let scaled = (f64::from(value.max(0.0)) * 10f64.powi(decimals as i32)).round() as u64;
            let limit = 10u64.pow(count as u32) - 1;
            format!("{:0width$}", scaled.min(limit), width = decimals + 1).chars().collect()
        }
        None => vec!['-'; count],
    };
    let first = count.saturating_sub(digits.len());
    (0..count).map(|i| (i.checked_sub(first).map(|j| digits[j]), decimals > 0 && i + 1 == whole)).collect()
}

/// The seven segments lit for a character (a top, b top right, c bottom right, d bottom, e bottom left, f top
/// left, g middle).
fn segments(c: char) -> [bool; 7] {
    let lit = match c {
        '0' => "abcdef",
        '1' => "bc",
        '2' => "abdeg",
        '3' => "abcdg",
        '4' => "bcfg",
        '5' => "acdfg",
        '6' => "acdefg",
        '7' => "abc",
        '8' => "abcdefg",
        '9' => "abcdfg",
        '-' => "g",
        _ => "",
    };
    std::array::from_fn(|i| lit.contains((b'a' + i as u8) as char))
}

/// The seven segments of a digit `w` x `h` at (`x`, `y`), `t` thick, as six-cornered shapes, a little slanted as a
/// pump's digits are.
fn segment_shapes(x: f32, y: f32, w: f32, h: f32, t: f32) -> [[(f32, f32); 6]; 7] {
    let gap = t * 0.18;
    let half = t / 2.0;
    let slant = w * 0.12;
    // Slanted: a point's x moved right the higher it is.
    let at = |px: f32, py: f32| (px + slant * (1.0 - (py - y) / h), py);
    let across = |y0: f32| {
        let (x0, x1) = (x + half + gap, x + w - half - gap);
        [
            at(x0, y0),
            at(x0 + half, y0 - half),
            at(x1 - half, y0 - half),
            at(x1, y0),
            at(x1 - half, y0 + half),
            at(x0 + half, y0 + half),
        ]
    };
    let down = |x0: f32, y0: f32, y1: f32| {
        let (y0, y1) = (y0 + gap, y1 - gap);
        [
            at(x0, y0),
            at(x0 + half, y0 + half),
            at(x0 + half, y1 - half),
            at(x0, y1),
            at(x0 - half, y1 - half),
            at(x0 - half, y0 + half),
        ]
    };
    let (top, middle, bottom) = (y + half, y + h / 2.0, y + h - half);
    let (left, right) = (x + half, x + w - half);
    [
        across(top),
        down(right, top, middle),
        down(right, middle, bottom),
        across(bottom),
        down(left, middle, bottom),
        down(left, top, middle),
        across(middle),
    ]
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// The pump's face painted: width, height and pixels (BGRA, premultiplied, top row first), `letters` the labels'
/// height.
fn paint(reading: &Reading, letters: i32, language: &str) -> Option<(u32, u32, Vec<u8>)> {
    let (fuel, state, windows) = face(reading, language);
    let em = letters as f32;
    let pad = (em * 0.9).round() as i32;
    let gap = (em * 0.45).round() as i32;
    let rim = ((em / 6.0).round() as i32).max(2);
    // Each window's digits: their size, and the window's.
    let digit_w = |tall: f32| (em * tall * 0.56).round() as i32;
    let digit_h = |tall: f32| (em * tall).round() as i32;
    let spacing = |tall: f32| (em * tall * 0.2).round() as i32;
    // Room for the decimal point between the whole and the decimals.
    let point_gap = |tall: f32| (em * tall * 0.22).round() as i32;
    let inner = (em * 0.5).round() as i32;
    // SAFETY: GDI on objects made and let go of here; the bitmap's bits read while it's alive, after a flush.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let font = |size: i32, weight: i32| -> HFONT {
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
        let band_font = font((em * 1.15).round() as i32, FW_BOLD.0 as i32);
        let state_font = font((em * 0.75).round() as i32, FW_BOLD.0 as i32);
        let label_font = font((em * 0.78).round() as i32, FW_BOLD.0 as i32);
        let unit_font = font((em * 1.0).round() as i32, FW_SEMIBOLD.0 as i32);
        let measure = |dc: HDC, font: HFONT, text: &[u16]| -> (i32, i32) {
            SelectObject(dc, HGDIOBJ(font.0));
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, text, &mut size);
            (size.cx, size.cy)
        };
        // The layout: the band, then each window under its label, all as wide as the widest.
        let (fuel_w, state_w) = (wide(&fuel), wide(&state));
        let (_, band_text_h) = measure(dc, band_font, &fuel_w);
        let band_h = band_text_h + 2 * gap;
        let unit_w = windows.iter().map(|w| measure(dc, unit_font, &wide(&w.unit)).0).max().unwrap_or(0);
        let digits_w = |w: &Window| {
            let n = (w.cells.0 + w.cells.1) as i32;
            n * digit_w(w.tall) + (n - 1) * spacing(w.tall) + if w.cells.1 > 0 { point_gap(w.tall) } else { 0 }
        };
        let window_w = windows.iter().map(|w| inner * 3 + unit_w + digits_w(w)).max().unwrap_or(0);
        let width = window_w + 2 * pad;
        let (_, label_h) = measure(dc, label_font, &wide("A"));
        let mut y = band_h + gap;
        let mut places = Vec::new();
        for w in &windows {
            let window_h = digit_h(w.tall) + 2 * inner;
            places.push((y, y + label_h + gap / 2, window_h));
            y += label_h + gap / 2 + window_h + gap;
        }
        let height = y + pad - gap;
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
        let before = SelectObject(dc, HGDIOBJ(bitmap.0));
        let before_pen = SelectObject(dc, GetStockObject(NULL_PEN));
        let fill = |rect: RECT, colour: u32| {
            let brush = CreateSolidBrush(COLORREF(colour));
            FillRect(dc, &rect, brush);
            let _ = DeleteObject(HGDIOBJ(brush.0));
        };
        let shape = |points: &[(f32, f32)], colour: u32| {
            let brush = CreateSolidBrush(COLORREF(colour));
            let old = SelectObject(dc, HGDIOBJ(brush.0));
            let points: Vec<POINT> =
                points.iter().map(|&(x, y)| POINT { x: x.round() as i32, y: y.round() as i32 }).collect();
            let _ = Polygon(dc, &points);
            SelectObject(dc, old);
            let _ = DeleteObject(HGDIOBJ(brush.0));
        };
        // The face, its rim, the band with the fuel and how it stands.
        fill(RECT { left: 0, top: 0, right: width, bottom: height }, RIM);
        fill(RECT { left: rim, top: rim, right: width - rim, bottom: height - rim }, FACE);
        fill(RECT { left: rim, top: rim, right: width - rim, bottom: band_h }, BAND);
        SetBkMode(dc, TRANSPARENT);
        SelectObject(dc, HGDIOBJ(band_font.0));
        SetTextColor(dc, COLORREF(BAND_TEXT));
        let _ = TextOutW(dc, pad, (band_h - band_text_h) / 2 + rim / 2, &fuel_w);
        let (sw, sh) = measure(dc, state_font, &state_w);
        SetTextColor(dc, COLORREF(BAND_TEXT));
        let _ = TextOutW(dc, width - pad - sw, (band_h - sh) / 2 + rim / 2, &state_w);
        for (w, &(label_y, window_y, window_h)) in windows.iter().zip(&places) {
            SelectObject(dc, HGDIOBJ(label_font.0));
            SetTextColor(dc, COLORREF(LABEL));
            let _ = TextOutW(dc, pad, label_y, &wide(&w.label));
            let edge = (rim / 2).max(1);
            let window = RECT { left: pad, top: window_y, right: width - pad, bottom: window_y + window_h };
            fill(window, WINDOW_EDGE);
            fill(
                RECT {
                    left: window.left + edge,
                    top: window.top + edge,
                    right: window.right - edge,
                    bottom: window.bottom - edge,
                },
                WINDOW,
            );
            // The unit at the window's left, then the digits to its right edge.
            let unit = wide(&w.unit);
            let (_, uh) = measure(dc, unit_font, &unit);
            SelectObject(dc, HGDIOBJ(unit_font.0));
            SetTextColor(dc, COLORREF(LIT));
            let _ = TextOutW(dc, pad + inner, window_y + window_h - inner - uh, &unit);
            let (dw, dh, t) = (digit_w(w.tall) as f32, digit_h(w.tall) as f32, em * w.tall * 0.13);
            let step = dw + spacing(w.tall) as f32;
            let first_x = (width - pad - inner - digits_w(w)) as f32;
            let top = (window_y + inner) as f32;
            for (i, (shown, point)) in cells_of(w.value, w.cells).into_iter().enumerate() {
                let x = first_x + i as f32 * step + if i >= w.cells.0 { point_gap(w.tall) as f32 } else { 0.0 };
                let lit = segments(shown.unwrap_or(' '));
                for (s, corners) in segment_shapes(x, top, dw, dh, t).iter().enumerate() {
                    shape(corners, if lit[s] { LIT } else { UNLIT });
                }
                if point {
                    let r = t * 0.55;
                    let (px, py) =
                        (x + dw + (spacing(w.tall) + point_gap(w.tall)) as f32 / 2.0 - t * 0.3, top + dh - r);
                    shape(&[(px - r, py - r), (px + r, py - r), (px + r, py + r), (px - r, py + r)], LIT);
                }
            }
        }
        let _ = GdiFlush();
        let mut pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec();
        SelectObject(dc, before_pen);
        SelectObject(dc, before);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        for font in [band_font, state_font, label_font, unit_font] {
            let _ = DeleteObject(HGDIOBJ(font.0));
        }
        let _ = DeleteDC(dc);
        crate::hints::rounded(&mut pixels, width as u32, height as u32, (em * 0.5).max(2.0), OPACITY);
        Some((width as u32, height as u32, pixels))
    }
}

/// The picture drawn, for a change count and a screen height.
struct Painted {
    for_reading: u64,
    for_height: u32,
    format: DXGI_FORMAT,
    size: (u32, u32),
    view: ID3D11ShaderResourceView,
}

struct Meter {
    device: ID3D11Device,
    quad: Quad,
    painted: Option<Painted>,
}

// SAFETY: used only on the game's render thread (Present), the lock only keeping it there.
unsafe impl Send for Meter {}

static DRAWING: Mutex<Option<Result<Meter, ()>>> = Mutex::new(None);

/// Where the meter goes on a `width` x `height` screen (clip space: left, top, right, bottom): at the top right.
fn place_on(width: f32, height: f32, (w, h): (u32, u32)) -> [f32; 4] {
    let left = (height * LEFT_GAP_OF_SCREEN).round();
    let right = left + w as f32;
    let top = (height * TOP_GAP_OF_SCREEN).round();
    let bottom = top + h as f32;
    [left / width * 2.0 - 1.0, 1.0 - top / height * 2.0, right / width * 2.0 - 1.0, 1.0 - bottom / height * 2.0]
}

/// At each of the game's presents: the meter over its picture, while there's a reading.
///
/// # Safety
/// The game's live swap chain, in its `Present`, on its render thread.
pub unsafe fn on_present(swapchain: *mut c_void) {
    // Not over the phone, nor a paper or bill on the screen in its frame.
    if crate::telemetry::paused()
        || !crate::bridge::host_alive()
        || crate::bridge::phone_out()
        || crate::bridge::paper_out()
    {
        return;
    }
    let (version, reading) = {
        let now = NOW.lock().unwrap_or_else(|e| e.into_inner());
        let Some(reading) = now.1 else { return };
        (now.0, reading)
    };
    // SAFETY: the game's live swap chain, on its render thread (the caller's contract).
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let mut meter = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(&*meter, Some(Ok(made)) if made.device != device) {
            *meter = None;
        }
        if meter.is_none() {
            *meter = Some(
                Quad::new(&device)
                    .map(|quad| Meter { device: device.clone(), quad, painted: None })
                    .map_err(|e| log!("fuel meter: can't be drawn: {e}")),
            );
        }
        let Some(Ok(made)) = meter.as_mut() else { return };
        if let Err(e) = made.draw(swapchain, version, &reading) {
            log!("fuel meter: not drawn: {e}");
            *meter = Some(Err(()));
        }
    }
}

impl Meter {
    /// # Safety
    /// As [`on_present`]: the game's swap chain, of this drawing's device.
    unsafe fn draw(
        &mut self,
        swapchain: &IDXGISwapChain,
        version: u64,
        reading: &Reading,
    ) -> windows::core::Result<()> {
        // SAFETY: the game's live device and back buffer, on its render thread.
        unsafe {
            let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut desc);
            drop(back);
            let format = view_format(desc.Format);
            let (width, height) = (desc.Width, desc.Height);
            let fresh = matches!(&self.painted, Some(p) if p.for_reading == version && p.for_height == height && p.format == format);
            if !fresh {
                let letters = ((height as f32 * LETTERS_OF_SCREEN).round() as i32).clamp(10, 48);
                let language = crate::character::language();
                let Some((w, h, pixels)) = paint(reading, letters, language) else { return Ok(()) };
                let view = crate::hints::upload(&self.device, w, h, &pixels, format)?;
                self.painted = Some(Painted { for_reading: version, for_height: height, format, size: (w, h), view });
            }
            let Some(painted) = &self.painted else { return Ok(()) };
            let rect = place_on(width as f32, height as f32, painted.size);
            let context = self.device.GetImmediateContext()?;
            self.quad.draw(&self.device, &context, swapchain, &painted.view, rect)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pump_shows_total_amount_and_price() {
        let reading = Reading { amount: 120.0, price: Some(5.46), gallons: false, done: false, owed: false };
        let (fuel, state, windows) = face(&reading, "pt");
        assert_eq!((fuel.as_str(), state.as_str()), ("DIESEL", "ABASTECENDO"));
        assert_eq!(windows[0].label, "TOTAL A PAGAR");
        assert!((windows[0].value.unwrap() - 655.2).abs() < 0.01);
        assert_eq!(windows[1].unit, "L");
        assert_eq!(windows[2].unit, "R$");
        let unknown = Reading { price: None, ..reading };
        assert_eq!(face(&unknown, "pt").2[0].value, None);
    }

    #[test]
    fn the_digits_fill_their_cells_from_the_right() {
        let shown: String = cells_of(Some(87.4), (5, 2))
            .iter()
            .map(|(c, point)| format!("{}{}", c.unwrap_or(' '), if *point { "," } else { "" }))
            .collect();
        assert_eq!(shown, "   87,40");
        let shown: String = cells_of(Some(0.0), (2, 2)).iter().map(|(c, _)| c.unwrap_or(' ')).collect();
        assert_eq!(shown, " 000");
        assert!(cells_of(None, (2, 2)).iter().all(|(c, _)| *c == Some('-')));
        assert_eq!(segments('8'), [true; 7]);
        assert_eq!(segments('1'), [false, true, true, false, false, false, false]);
    }

    /// A look at the meter (\`WESTPLAY_METER\` a .bmp file to write).
    #[test]
    #[ignore]
    fn write_a_meter() {
        let reading = Reading { amount: 87.4, price: Some(5.46), gallons: false, done: false, owed: false };
        let (w, h, pixels) = paint(&reading, (1080.0 * LETTERS_OF_SCREEN).round() as i32, "pt").unwrap();
        let mut out = Vec::new();
        let size = 54 + pixels.len() as u32;
        out.extend(b"BM");
        for v in [size, 0, 54, 40, w, (-(h as i32)) as u32] {
            out.extend(v.to_le_bytes());
        }
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        for v in [0u32, pixels.len() as u32, 2835, 2835, 0, 0] {
            out.extend(v.to_le_bytes());
        }
        out.extend(&pixels);
        std::fs::write(std::env::var("WESTPLAY_METER").unwrap(), out).unwrap();
    }

    #[test]
    fn the_meter_is_painted_and_placed_top_right() {
        let reading = Reading { amount: 12.3, price: Some(5.46), gallons: false, done: false, owed: false };
        let (w, h, pixels) = paint(&reading, 18, "pt").unwrap();
        assert_eq!(pixels.len(), (w * h * 4) as usize);
        assert!(w > 18 * 5 && h > 18 * 5);
        let [left, top, right, bottom] = place_on(1920.0, 1080.0, (w, h));
        assert!(left < -0.9 && right < 0.0 && top < 1.0 && bottom < top);
    }
}
