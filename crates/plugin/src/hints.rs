//! The key hints bar over the game (owner 2026-11-05: "a HUD for whoever doesn't know which key to press"): at
//! the bottom of the screen, the keys for what can be done now. The phone's and the screen's (the cab's
//! "CarPlay") always; getting out once the truck stands on its parking brake; getting back in by the driver's
//! door. Hidden in the app's settings, or by the player's key in the game (`toggle`).
//!
//! The game thread says what's there (`set`, from `character`); the render thread paints it (GDI, into a picture
//! of its own: only when it changes) and draws it as the phone is drawn (`handset::Quad`).

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::RECT;
use windows::Win32::Foundation::{COLORREF, SIZE};
use windows::Win32::Graphics::Direct3D::D3D_SRV_DIMENSION_TEXTURE2D;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_SHADER_RESOURCE, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0,
    D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11Device,
    ID3D11ShaderResourceView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_TYPELESS, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLIP_DEFAULT_PRECIS, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DIB_RGB_COLORS, DeleteDC, DeleteObject, FW_BOLD,
    FW_SEMIBOLD, FillRect, GdiFlush, GetTextExtentPoint32W, HDC, HFONT, HGDIOBJ, OUT_DEFAULT_PRECIS, SelectObject,
    SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyNameTextW, MAPVK_VK_TO_VSC, MapVirtualKeyW};
use windows::core::{Interface, w};

use crate::handset::{Quad, view_format};
use crate::{bridge, log};

/// What a key does now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    GetOut,
    GetIn,
    Phone,
    Screen,
    Flashlight,
    FlashlightOff,
    /// V while walking: first or third person.
    Camera,
    /// Ctrl while walking: crouch.
    Crouch,
    /// The phone out on its Personagem app (owner 2026-11-07).
    Character,
    /// Hiding this bar (owner 2026-11-07: "the key that hides the bar, on the bar").
    HideBar,
    /// E by the clerk, on foot: the cargo's papers to sign (`character::freight`).
    SignNote,
    /// E at the fifth wheel, on foot: the trailer coupled by hand.
    Couple,
}

impl Action {
    /// In the game's language (two letters), English if not one of these.
    fn words(self, language: &str) -> &'static str {
        let i = self as usize;
        let words: [&str; 12] = match language {
            "pt" => [
                "Descer do caminhão",
                "Entrar no caminhão",
                "Celular",
                "Usar a tela",
                "Lanterna",
                "Apagar a lanterna",
                "Câmera",
                "Agachar",
                "Personagem",
                "Esconder a barra",
                "Assinar a nota fiscal",
                "Engatar a carreta",
            ],
            "es" => [
                "Bajar del camión",
                "Subir al camión",
                "Móvil",
                "Usar la pantalla",
                "Linterna",
                "Apagar la linterna",
                "Cámara",
                "Agacharse",
                "Personaje",
                "Ocultar la barra",
                "Firmar la factura",
                "Enganchar el remolque",
            ],
            "de" => [
                "Aussteigen",
                "Einsteigen",
                "Handy",
                "Bildschirm bedienen",
                "Taschenlampe",
                "Taschenlampe aus",
                "Kamera",
                "Ducken",
                "Figur",
                "Leiste ausblenden",
                "Frachtpapiere unterschreiben",
                "Auflieger ankuppeln",
            ],
            "fr" => [
                "Descendre du camion",
                "Monter dans le camion",
                "Téléphone",
                "Utiliser l’écran",
                "Lampe",
                "Éteindre la lampe",
                "Caméra",
                "S’accroupir",
                "Personnage",
                "Masquer la barre",
                "Signer le bon de transport",
                "Atteler la remorque",
            ],
            "it" => [
                "Scendi dal camion",
                "Sali sul camion",
                "Telefono",
                "Usa lo schermo",
                "Torcia",
                "Spegni la torcia",
                "Visuale",
                "Accovacciati",
                "Personaggio",
                "Nascondi la barra",
                "Firma la bolla",
                "Aggancia il rimorchio",
            ],
            "pl" => [
                "Wysiądź",
                "Wsiądź",
                "Telefon",
                "Użyj ekranu",
                "Latarka",
                "Zgaś latarkę",
                "Kamera",
                "Kucnij",
                "Postać",
                "Ukryj pasek",
                "Podpisz list przewozowy",
                "Podepnij naczepę",
            ],
            "ru" => [
                "Выйти",
                "Сесть",
                "Телефон",
                "Экран",
                "Фонарик",
                "Выключить фонарик",
                "Камера",
                "Присесть",
                "Персонаж",
                "Скрыть панель",
                "Подписать накладную",
                "Сцепить прицеп",
            ],
            "tr" => [
                "Tırdan in",
                "Tıra bin",
                "Telefon",
                "Ekranı kullan",
                "Fener",
                "Feneri kapat",
                "Kamera",
                "Çömel",
                "Karakter",
                "Çubuğu gizle",
                "İrsaliyeyi imzala",
                "Dorseyi bağla",
            ],
            "nl" => [
                "Uitstappen",
                "Instappen",
                "Telefoon",
                "Scherm gebruiken",
                "Zaklamp",
                "Zaklamp uit",
                "Camera",
                "Bukken",
                "Personage",
                "Balk verbergen",
                "Vrachtbrief tekenen",
                "Oplegger aankoppelen",
            ],
            "cs" => [
                "Vystoupit",
                "Nastoupit",
                "Telefon",
                "Použít obrazovku",
                "Svítilna",
                "Zhasnout svítilnu",
                "Kamera",
                "Přikrčit se",
                "Postava",
                "Skrýt lištu",
                "Podepsat nákladní list",
                "Připojit návěs",
            ],
            _ => [
                "Get out",
                "Get in",
                "Phone",
                "Use the screen",
                "Flashlight",
                "Flashlight off",
                "Camera",
                "Crouch",
                "Character",
                "Hide the bar",
                "Sign the papers",
                "Couple the trailer",
            ],
        };
        words[i]
    }
}

/// A key and what it does now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    pub key: String,
    pub action: Action,
}

/// What's there now, and a count of its changes (the picture is painted again on one).
static HINTS: Mutex<(u64, Vec<Hint>)> = Mutex::new((0, Vec::new()));
/// Hidden by the player's key in the game.
static HIDDEN: AtomicBool = AtomicBool::new(false);

/// What the bar shows now (the game thread, each frame).
pub fn set(hints: Vec<Hint>) {
    let mut now = HINTS.lock().unwrap_or_else(|e| e.into_inner());
    if now.1 != hints {
        now.0 += 1;
        now.1 = hints;
    }
}

/// The player's key: the bar hidden, or shown again.
pub fn toggle() {
    let hidden = !HIDDEN.fetch_xor(true, Ordering::AcqRel);
    log!("hints: {}", if hidden { "hidden" } else { "shown" });
}

/// A key's name as the keyboard has it ("F8", "Alt", "4"), the mouse's side buttons by number.
pub fn key_name(vk: u32) -> String {
    match vk {
        0x04 => return "Mouse 3".into(),
        0x05 => return "Mouse 4".into(),
        0x06 => return "Mouse 5".into(),
        0x10 | 0xa0 | 0xa1 => return "Shift".into(),
        0x11 | 0xa2 | 0xa3 => return "Ctrl".into(),
        0x12 | 0xa4 => return "Alt".into(),
        0xa5 => return "Alt Gr".into(),
        _ => {}
    }
    // SAFETY: plain queries, the buffer a local of its length.
    let name = unsafe {
        let scancode = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
        // The keys sharing a scancode with the number pad's (arrows, Insert, Home…) are the extended ones.
        let extended = matches!(vk, 0x21..=0x28 | 0x2d | 0x2e | 0x6f | 0x90);
        let lparam = (scancode << 16 | u32::from(extended) << 24) as i32;
        let mut buffer = [0u16; 32];
        let n = GetKeyNameTextW(lparam, &mut buffer);
        String::from_utf16_lossy(&buffer[..n.max(0) as usize])
    };
    if name.is_empty() { format!("0x{vk:02X}") } else { name }
}

/// The bar's look: its letters' height (of the screen's), its gap to the screen's bottom (of the screen's).
const LETTERS_OF_SCREEN: f32 = 0.0125;
const BOTTOM_GAP_OF_SCREEN: f32 = 0.008;
/// Colours (0xBBGGRR, as GDI has them): the bar, a key's box, a key, an action, the line under it; the bar's
/// opacity.
const BAR: u32 = 0x1e1814;
const KEY_BOX: u32 = 0xeeeeee;
const KEY: u32 = 0x1a1a1a;
const ACTION: u32 = 0xf2f2f2;
const LINE: u32 = 0xff9c40;
const OPACITY: u32 = 226;

/// The picture drawn, for the hints of a change count and a screen height.
struct Painted {
    for_hints: u64,
    for_height: u32,
    format: DXGI_FORMAT,
    size: (u32, u32),
    view: ID3D11ShaderResourceView,
}

struct Bar {
    device: ID3D11Device,
    quad: Quad,
    painted: Option<Painted>,
}

// SAFETY: used only on the game's render thread (Present), the lock only keeping it there.
unsafe impl Send for Bar {}

static BAR_DRAWING: Mutex<Option<Result<Bar, ()>>> = Mutex::new(None);

/// Whether the bar is to be seen: the app open and not hiding it, the player not either, the game not paused
/// (its menus).
fn seen() -> bool {
    let Some(bridge) = bridge::get() else { return false };
    bridge::host_alive()
        && bridge.host.character_hints.load(Ordering::Acquire) != 2
        && !HIDDEN.load(Ordering::Acquire)
        && !crate::telemetry::paused()
}

/// At each of the game's presents: the bar over its picture, when there's something on it.
///
/// # Safety
/// The game's live swap chain, in its `Present`, on its render thread.
pub unsafe fn on_present(swapchain: *mut c_void) {
    if !seen() {
        return;
    }
    let (version, hints) = {
        let now = HINTS.lock().unwrap_or_else(|e| e.into_inner());
        if now.1.is_empty() {
            return;
        }
        (now.0, now.1.clone())
    };
    // SAFETY: the game's live swap chain, on its render thread (the caller's contract).
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let mut bar = BAR_DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(&*bar, Some(Ok(made)) if made.device != device) {
            *bar = None;
        }
        if bar.is_none() {
            *bar = Some(
                Quad::new(&device)
                    .map(|quad| Bar { device: device.clone(), quad, painted: None })
                    .map_err(|e| log!("hints: can't be drawn: {e}")),
            );
        }
        let Some(Ok(made)) = bar.as_mut() else { return };
        if let Err(e) = made.draw(swapchain, version, &hints) {
            log!("hints: not drawn: {e}");
            *bar = Some(Err(()));
        }
    }
}

impl Bar {
    /// # Safety
    /// As [`on_present`]: the game's swap chain, of this drawing's device.
    unsafe fn draw(&mut self, swapchain: &IDXGISwapChain, version: u64, hints: &[Hint]) -> windows::core::Result<()> {
        // SAFETY: the game's live device and back buffer, on its render thread.
        unsafe {
            let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
            let mut back_desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut back_desc);
            drop(back);
            let format = view_format(back_desc.Format);
            let (width, height) = (back_desc.Width, back_desc.Height);
            let fresh = matches!(&self.painted, Some(p) if p.for_hints == version && p.for_height == height && p.format == format);
            if !fresh {
                let letters = ((height as f32 * LETTERS_OF_SCREEN).round() as i32).clamp(10, 40);
                let language = crate::character::language();
                let Some((w, h, pixels)) = paint(hints, letters, language) else { return Ok(()) };
                let view = self.upload(w, h, &pixels, format)?;
                self.painted = Some(Painted { for_hints: version, for_height: height, format, size: (w, h), view });
            }
            let Some(painted) = &self.painted else { return Ok(()) };
            let rect = place_on(width as f32, height as f32, painted.size);
            let context = self.device.GetImmediateContext()?;
            self.quad.draw(&self.device, &context, swapchain, &painted.view, rect)
        }
    }

    /// The painted pixels (BGRA, premultiplied) as a texture's view, read as `format`.
    unsafe fn upload(
        &self,
        w: u32,
        h: u32,
        pixels: &[u8],
        format: DXGI_FORMAT,
    ) -> windows::core::Result<ID3D11ShaderResourceView> {
        // SAFETY: the game's live device; the pixels a local of `w * h * 4` bytes.
        unsafe {
            let mut texture = None;
            self.device.CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: w,
                    Height: h,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_TYPELESS,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                    ..Default::default()
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: pixels.as_ptr().cast(),
                    SysMemPitch: w * 4,
                    SysMemSlicePitch: 0,
                }),
                Some(&mut texture),
            )?;
            let texture = texture.ok_or_else(windows::core::Error::empty)?;
            let mut view = None;
            self.device.CreateShaderResourceView(
                &texture,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: format,
                    ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
                    },
                }),
                Some(&mut view),
            )?;
            view.ok_or_else(windows::core::Error::empty)
        }
    }
}

/// Where the bar goes on a `width` x `height` screen (clip space: left, top, right, bottom): in the middle,
/// above the bottom, at its own size in pixels.
fn place_on(width: f32, height: f32, (w, h): (u32, u32)) -> [f32; 4] {
    let left = ((width - w as f32) / 2.0).round();
    let bottom = (height - height * BOTTOM_GAP_OF_SCREEN).round();
    let top = bottom - h as f32;
    let right = left + w as f32;
    [left / width * 2.0 - 1.0, 1.0 - top / height * 2.0, right / width * 2.0 - 1.0, 1.0 - bottom / height * 2.0]
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// The bar painted: its width, height and pixels (BGRA, premultiplied, top row first), letters `letters` pixels
/// high.
fn paint(hints: &[Hint], letters: i32, language: &str) -> Option<(u32, u32, Vec<u8>)> {
    let em = letters as f32;
    let pad_x = (em * 0.9).round() as i32;
    let pad_y = (em * 0.35).round() as i32;
    let key_pad = (em * 0.4).round() as i32;
    let after_key = (em * 0.45).round() as i32;
    let between = (em * 1.2).round() as i32;
    let line = ((em / 9.0).round() as i32).max(1);
    // SAFETY: GDI on objects made and let go of here; the bitmap's bits read while it's alive, after a flush.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let font = |weight: i32| -> HFONT {
            CreateFontW(
                -letters,
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
        let (action_font, key_font) = (font(FW_SEMIBOLD.0 as i32), font(FW_BOLD.0 as i32));
        let measure = |dc: HDC, font: HFONT, text: &[u16]| -> (i32, i32) {
            SelectObject(dc, HGDIOBJ(font.0));
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, text, &mut size);
            (size.cx, size.cy)
        };
        // The layout: a key's box, its action, the next.
        let items: Vec<(Vec<u16>, Vec<u16>)> =
            hints.iter().map(|hint| (wide(&hint.key), wide(hint.action.words(language)))).collect();
        let mut tall = 0;
        let mut x = pad_x;
        let mut places = Vec::new();
        for (key, action) in &items {
            let (kw, kh) = measure(dc, key_font, key);
            let (aw, ah) = measure(dc, action_font, action);
            tall = tall.max(kh).max(ah);
            let box_w = (kw + 2 * key_pad).max(kh + key_pad);
            places.push((x, box_w, kw, aw));
            x += box_w + after_key + aw + between;
        }
        let width = x - between + pad_x;
        let box_h = tall + (em * 0.2).round() as i32;
        let height = box_h + 2 * pad_y + line;
        if width <= 0 || height <= 0 {
            let _ = DeleteDC(dc);
            return None;
        }
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
        let fill = |rect: RECT, colour: u32| {
            let brush = CreateSolidBrush(COLORREF(colour));
            FillRect(dc, &rect, brush);
            let _ = DeleteObject(HGDIOBJ(brush.0));
        };
        fill(RECT { left: 0, top: 0, right: width, bottom: height }, BAR);
        fill(RECT { left: 0, top: height - line, right: width, bottom: height }, LINE);
        SetBkMode(dc, TRANSPARENT);
        for ((key, action), &(x, box_w, kw, _)) in items.iter().zip(&places) {
            fill(RECT { left: x, top: pad_y, right: x + box_w, bottom: pad_y + box_h }, KEY_BOX);
            SelectObject(dc, HGDIOBJ(key_font.0));
            SetTextColor(dc, COLORREF(KEY));
            let _ = TextOutW(dc, x + (box_w - kw) / 2, pad_y + (box_h - tall) / 2, key);
            SelectObject(dc, HGDIOBJ(action_font.0));
            SetTextColor(dc, COLORREF(ACTION));
            let _ = TextOutW(dc, x + box_w + after_key, pad_y + (box_h - tall) / 2, action);
        }
        let _ = GdiFlush();
        let mut pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec();
        SelectObject(dc, before);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteObject(HGDIOBJ(action_font.0));
        let _ = DeleteObject(HGDIOBJ(key_font.0));
        let _ = DeleteDC(dc);
        let radius = (em * 0.35).max(2.0);
        opaque_rounded(&mut pixels, width as u32, height as u32, radius);
        Some((width as u32, height as u32, pixels))
    }
}

/// The bar's pixels given their opacity (GDI leaves none), premultiplied, its corners rounded by `radius`.
fn opaque_rounded(pixels: &mut [u8], width: u32, height: u32, radius: f32) {
    for (i, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (x, y) = ((i as u32 % width) as f32 + 0.5, (i as u32 / width) as f32 + 0.5);
        // Outside a corner's circle: none of it.
        let cx = if x < radius {
            radius
        } else if x > width as f32 - radius {
            width as f32 - radius
        } else {
            x
        };
        let cy = if y < radius {
            radius
        } else if y > height as f32 - radius {
            height as f32 - radius
        } else {
            y
        };
        let outside = ((x - cx).hypot(y - cy) - radius + 0.5).clamp(0.0, 1.0);
        let alpha = (OPACITY as f32 * (1.0 - outside)).round() as u32;
        for c in &mut pixel[..3] {
            *c = (u32::from(*c) * alpha / 255) as u8;
        }
        pixel[3] = alpha as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_is_painted_wider_for_more_hints() {
        let one = vec![Hint { key: "F8".into(), action: Action::GetOut }];
        let two = vec![one[0].clone(), Hint { key: "Alt + 4".into(), action: Action::Phone }];
        let (w1, h1, p1) = paint(&one, 22, "pt").unwrap();
        let (w2, h2, _) = paint(&two, 22, "pt").unwrap();
        assert!(w2 > w1 && h1 == h2 && h1 > 22);
        assert_eq!(p1.len(), (w1 * h1 * 4) as usize);
        // Opaque in the middle (premultiplied: no colour brighter than its opacity), clear at a corner.
        let middle = ((h1 / 2 * w1 + w1 / 2) * 4) as usize;
        assert_eq!(u32::from(p1[middle + 3]), OPACITY);
        assert!(p1[..3].iter().all(|&c| c <= p1[3]) && p1[3] < 40);
    }

    /// A look at the bar (`WESTPLAY_BAR` a file: its width, height and pixels).
    #[test]
    #[ignore]
    fn write_a_bar() {
        let hints = [
            Hint { key: "F8".into(), action: Action::GetOut },
            Hint { key: "Alt".into(), action: Action::Screen },
            Hint { key: "Alt + 4".into(), action: Action::Phone },
        ];
        let (w, h, pixels) = paint(&hints, (1080.0 * LETTERS_OF_SCREEN).round() as i32, "pt").unwrap();
        let mut out = format!(
            "{w} {h}
"
        )
        .into_bytes();
        out.extend(pixels);
        std::fs::write(std::env::var("WESTPLAY_BAR").unwrap(), out).unwrap();
    }

    #[test]
    fn the_bar_stands_in_the_middle_above_the_bottom() {
        let [left, top, right, bottom] = place_on(1920.0, 1080.0, (400, 40));
        assert!((left + right).abs() < 0.01);
        assert!(bottom > -1.0 && top > bottom && top < 0.0);
    }

    #[test]
    fn keys_are_named_as_the_keyboard_has_them() {
        assert_eq!(key_name(0x77), "F8");
        assert_eq!(key_name(0xa4), "Alt");
        assert_eq!(key_name(0x05), "Mouse 4");
        assert_eq!(Action::GetOut.words("pt"), "Descer do caminhão");
        assert_eq!(Action::Phone.words("xx"), "Phone");
    }
}
