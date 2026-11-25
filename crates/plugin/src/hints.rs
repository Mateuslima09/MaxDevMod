//! The contextual interaction HUD over the game: at the bottom centre of the screen, the real configured keys and
//! concise descriptions for the actions available right now. Rendered in the MAXDEV MOD Liquid Glass style (`hud`),
//! with smooth entry/exit transitions and key-press feedback.
//!
//! Hidden in the app's settings, or toggled in the game by the player's key (`toggle`).

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use windows::Win32::Graphics::Direct3D::D3D_SRV_DIMENSION_TEXTURE2D;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_SHADER_RESOURCE, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0,
    D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11Device,
    ID3D11ShaderResourceView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_TYPELESS, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyNameTextW, MAPVK_VK_TO_VSC, MapVirtualKeyW};
use windows::core::Interface;

use crate::handset::view_format;
use crate::hud::{Animator, HudQuad};
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
    /// The phone out on its Personagem app.
    Character,
    /// Hiding this HUD bar.
    HideBar,
    /// E by the clerk, on foot: the cargo's papers to sign (`character::freight`).
    SignNote,
    /// E at the fifth wheel, on foot: the trailer coupled by hand.
    Couple,
    /// E beside the truck's tank, on foot, at a pump: the tank filled by hand (`character::refuel`).
    Refuel,
    /// E while the tank fills.
    StopRefuel,
    /// E at the truck's side, on foot, a trailer coupled: uncoupled by hand.
    Uncouple,
    /// E by the pump's attendant, on foot: the fuel filled by hand paid on the phone (`character::refuel`).
    PayFuel,
    /// E at the trailer's side, by its legs: cranked down or up (`character::freight`, owner 2026-11-19).
    LowerLegs,
    RaiseLegs,
    /// Enter (`activate`) while inside 2-stage Refuelling Mode: hold to pump fuel.
    PumpFuel,
}

impl Action {
    /// Concise action label in the game's language (two letters), English otherwise.
    pub(crate) fn words(self, language: &str) -> &'static str {
        let i = self as usize;
        let words: [&str; 19] = match language {
            "pt" => [
                "Sair do caminhão",
                "Entrar no caminhão",
                "Abrir celular",
                "Usar a tela",
                "Lanterna",
                "Apagar lanterna",
                "Câmera",
                "Agachar",
                "Personagem",
                "Ocultar HUD",
                "Assinar nota fiscal",
                "Engatar carreta",
                "Modo abastecimento",
                "Sair do abastecimento",
                "Desengatar carreta",
                "Pagar combustível",
                "Baixar os pés da carreta",
                "Subir os pés da carreta",
                "Segure para abastecer",
            ],
            "es" => [
                "Bajar del camión",
                "Subir al camión",
                "Abrir móvil",
                "Usar la pantalla",
                "Linterna",
                "Apagar la linterna",
                "Cámara",
                "Agacharse",
                "Personaje",
                "Ocultar HUD",
                "Firmar la factura",
                "Enganchar el remolque",
                "Modo repostaje",
                "Salir del repostaje",
                "Desenganchar el remolque",
                "Pagar el combustible",
                "Bajar las patas del remolque",
                "Subir las patas del remolque",
                "Mantén para repostar",
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
                "HUD ausblenden",
                "Frachtpapiere unterschreiben",
                "Auflieger ankuppeln",
                "Tankmodus",
                "Tankmodus verlassen",
                "Auflieger abkuppeln",
                "Tanken bezahlen",
                "Stützen absenken",
                "Stützen hochkurbeln",
                "Gedrückt halten zum Tanken",
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
                "Masquer le HUD",
                "Signer le bon de transport",
                "Atteler la remorque",
                "Mode ravitaillement",
                "Quitter le ravitaillement",
                "Dételer la remorque",
                "Payer le carburant",
                "Descendre les béquilles",
                "Remonter les béquilles",
                "Maintenir pour faire le plein",
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
                "Nascondi HUD",
                "Firma la bolla",
                "Aggancia il rimorchio",
                "Modalità rifornimento",
                "Esci dal rifornimento",
                "Sgancia il rimorchio",
                "Paga il carburante",
                "Abbassare i piedini",
                "Alzare i piedini",
                "Tieni premuto per fare rifornimento",
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
                "Ukryj HUD",
                "Podpisz list przewozowy",
                "Podepnij naczepę",
                "Tryb tankowania",
                "Zakończ tankowanie",
                "Odepnij naczepę",
                "Zapłać za paliwo",
                "Opuść podpory",
                "Podnieś podpory",
                "Przytrzymaj, aby zatankować",
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
                "Скрыть HUD",
                "Подписать накладную",
                "Сцепить прицеп",
                "Режим заправки",
                "Выйти из заправки",
                "Отцепить прицеп",
                "Оплатить топливо",
                "Опустить опоры",
                "Поднять опоры",
                "Удерживайте для заправки",
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
                "HUD gizle",
                "İrsaliyeyi imzala",
                "Dorseyi bağla",
                "Yakıt modu",
                "Yakıt modundan çık",
                "Dorseyi ayır",
                "Yakıtı öde",
                "Ayakları indir",
                "Ayakları kaldır",
                "Yakıt almak için basılı tut",
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
                "HUD verbergen",
                "Vrachtbrief tekenen",
                "Oplegger aankoppelen",
                "Tankmodus",
                "Tankmodus verlaten",
                "Oplegger afkoppelen",
                "Brandstof betalen",
                "Poten laten zakken",
                "Poten omhoog draaien",
                "Ingedrukt houden om te tanken",
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
                "Skrýt HUD",
                "Podepsat nákladní list",
                "Připojit návěs",
                "Režim tankování",
                "Ukončit tankování",
                "Odpojit návěs",
                "Zaplatit palivo",
                "Spustit podpěry",
                "Zvednout podpěry",
                "Podržte pro tankování",
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
                "Hide HUD",
                "Sign the papers",
                "Couple the trailer",
                "Refuel mode",
                "Exit refuel mode",
                "Uncouple the trailer",
                "Pay for fuel",
                "Lower the trailer's legs",
                "Raise the trailer's legs",
                "Hold to refuel",
            ],
        };
        words[i]
    }
}

/// A real configured shortcut and what it does now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    pub key: String,
    pub action: Action,
    /// Primary virtual-key code for press feedback (`0` if none).
    pub vk: u32,
    /// Optional second virtual-key code when bound as a modifier combination (`Alt + 4`, etc.).
    pub second_vk: u32,
}

impl Hint {
    pub fn single(key: String, vk: u32, action: Action) -> Self {
        Self { key, action, vk, second_vk: 0 }
    }

    pub fn combo(key: String, first_vk: u32, second_vk: u32, action: Action) -> Self {
        Self { key, action, vk: first_vk, second_vk }
    }
}

/// Active hints and a monotonic version counter incremented only when the list actually changes.
static HINTS: Mutex<(u64, Vec<Hint>)> = Mutex::new((0, Vec::new()));
/// Hidden by the player's key in the game.
static HIDDEN: AtomicBool = AtomicBool::new(false);

/// Updates what the HUD shows now (called from the game thread once per frame).
pub fn set(hints: Vec<Hint>) {
    let mut now = HINTS.lock().unwrap_or_else(|e| e.into_inner());
    if now.1 != hints {
        now.0 += 1;
        now.1 = hints;
    }
}

/// Toggles the HUD bar hidden or visible.
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
    // SAFETY: plain Win32 queries with a stack-allocated buffer of exact length.
    let name = unsafe {
        let scancode = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
        let extended = matches!(vk, 0x21..=0x28 | 0x2d | 0x2e | 0x6f | 0x90);
        let lparam = (scancode << 16 | u32::from(extended) << 24) as i32;
        let mut buffer = [0u16; 32];
        let n = GetKeyNameTextW(lparam, &mut buffer);
        String::from_utf16_lossy(&buffer[..n.max(0) as usize])
    };
    if name.is_empty() { format!("0x{vk:02X}") } else { name }
}

/// Cached GPU texture for a given hint version and resolution.
struct Painted {
    for_hints: u64,
    for_screen: (u32, u32),
    format: DXGI_FORMAT,
    size: (u32, u32),
    capsules_uv: Vec<[f32; 4]>,
    view: ID3D11ShaderResourceView,
}

struct Bar {
    device: ID3D11Device,
    quad: HudQuad,
    painted: Option<Painted>,
    animator: Animator,
    /// Last non-empty hint list, retained so exit fade-out draws smoothly without blinking.
    last_hints: (u64, Vec<Hint>),
}

// SAFETY: accessed exclusively on the game's D3D11 render thread inside `Present`.
unsafe impl Send for Bar {}

static BAR_DRAWING: Mutex<Option<Result<Bar, ()>>> = Mutex::new(None);

/// Whether the HUD wants to be visible right now.
fn seen() -> bool {
    let Some(bridge) = bridge::get() else { return false };
    bridge::host_alive()
        && bridge.host.character_hints.load(Ordering::Acquire) != 2
        && !HIDDEN.load(Ordering::Acquire)
        && !crate::telemetry::paused()
}

/// At each of the game's `Present` calls: updates transitions and draws the HUD when visible or fading out.
///
/// # Safety
/// `swapchain` is the game's live DXGI swap chain on its render thread.
pub unsafe fn on_present(swapchain: *mut c_void) {
    let want_seen = seen();
    let (version, current_hints) = {
        let now = HINTS.lock().unwrap_or_else(|e| e.into_inner());
        (now.0, now.1.clone())
    };
    let want_visible = want_seen && !current_hints.is_empty();

    // SAFETY: `swapchain` is the game's live swap chain on its render thread (caller's contract).
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let mut bar = BAR_DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        if !want_visible && bar.as_ref().is_none_or(|res| res.as_ref().is_ok_and(|b| b.painted.is_none())) {
            return;
        }
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        if matches!(&*bar, Some(Ok(made)) if made.device != device) {
            *bar = None;
        }
        if bar.is_none() {
            if !want_visible {
                return;
            }
            *bar = Some(
                HudQuad::new(&device)
                    .map(|quad| Bar {
                        device: device.clone(),
                        quad,
                        painted: None,
                        animator: Animator::new(),
                        last_hints: (0, Vec::new()),
                    })
                    .map_err(|e| log!("hints: can't be drawn: {e}")),
            );
        }
        let Some(Ok(made)) = bar.as_mut() else { return };
        if let Err(e) = made.draw(swapchain, want_visible, version, current_hints) {
            log!("hints: not drawn: {e}");
            *bar = Some(Err(()));
        }
    }
}

impl Bar {
    /// # Safety
    /// `swapchain` belongs to `self.device` on the game's render thread.
    unsafe fn draw(
        &mut self,
        swapchain: &IDXGISwapChain,
        want_visible: bool,
        version: u64,
        current_hints: Vec<Hint>,
    ) -> windows::core::Result<()> {
        if !current_hints.is_empty() {
            self.last_hints = (version, current_hints);
        }
        let now = Instant::now();
        let Some((opacity, slide, pulse_slot)) = self.animator.step(want_visible, &self.last_hints.1, now) else {
            return Ok(());
        };
        if self.last_hints.1.is_empty() {
            return Ok(());
        }

        // SAFETY: live device and backbuffer on the render thread.
        unsafe {
            let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
            let mut back_desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut back_desc);
            drop(back);
            let format = view_format(back_desc.Format);
            let (width, height) = (back_desc.Width, back_desc.Height);
            let target_version = self.last_hints.0;
            let fresh = matches!(
                &self.painted,
                Some(p) if p.for_hints == target_version && p.for_screen == (width, height) && p.format == format
            );
            if !fresh {
                let language = crate::character::language();
                let Some(hud) = crate::hud::paint(&self.last_hints.1, width, height, language) else {
                    return Ok(());
                };
                let view = upload(&self.device, hud.width, hud.height, &hud.pixels, format)?;
                self.painted = Some(Painted {
                    for_hints: target_version,
                    for_screen: (width, height),
                    format,
                    size: (hud.width, hud.height),
                    capsules_uv: hud.capsules_uv,
                    view,
                });
            }
            let Some(painted) = &self.painted else { return Ok(()) };
            let quad_size = self.animator.smooth_quad_size(painted.size, now);
            let rect = crate::hud::place_on(width as f32, height as f32, quad_size, slide);
            let active = pulse_slot.and_then(|(idx, strength)| painted.capsules_uv.get(idx).copied().map(|uv| (uv, strength)));
            let context = self.device.GetImmediateContext()?;
            self.quad.draw(&self.device, &context, swapchain, &painted.view, rect, opacity, active)
        }
    }
}

/// Painted pixels (BGRA, premultiplied) as a texture's view on `device`, read as `format` (used by the HUD and
/// the pump's meter in `fuel_meter`).
///
/// # Safety
/// The game's live device, on its render thread.
pub(crate) unsafe fn upload(
    device: &ID3D11Device,
    w: u32,
    h: u32,
    pixels: &[u8],
    format: DXGI_FORMAT,
) -> windows::core::Result<ID3D11ShaderResourceView> {
    // SAFETY: the game's live device; `pixels` holds `w * h * 4` bytes.
    unsafe {
        let mut texture = None;
        device.CreateTexture2D(
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
            Some(&D3D11_SUBRESOURCE_DATA { pSysMem: pixels.as_ptr().cast(), SysMemPitch: w * 4, SysMemSlicePitch: 0 }),
            Some(&mut texture),
        )?;
        let texture = texture.ok_or_else(windows::core::Error::empty)?;
        let mut view = None;
        device.CreateShaderResourceView(
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

/// Pixels painted by GDI given an `opacity` (GDI leaves none), premultiplied, their corners rounded by `radius`
/// (shared with `fuel_meter`).
pub(crate) fn rounded(pixels: &mut [u8], width: u32, height: u32, radius: f32, opacity: u32) {
    for (i, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (x, y) = ((i as u32 % width) as f32 + 0.5, (i as u32 / width) as f32 + 0.5);
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
        let alpha = (opacity as f32 * (1.0 - outside)).round() as u32;
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
        let one = vec![Hint::single("F8".into(), 0x77, Action::GetOut)];
        let two = vec![one[0].clone(), Hint::combo("Alt + 4".into(), 0xa4, 0x34, Action::Phone)];
        let p1 = crate::hud::paint_at_size(&one, 22, "pt").unwrap();
        let p2 = crate::hud::paint_at_size(&two, 22, "pt").unwrap();
        assert!(p2.width > p1.width && p1.height == p2.height && p1.height > 22);
        assert_eq!(p1.pixels.len(), (p1.width * p1.height * 4) as usize);
        // Premultiplied invariant holds everywhere, and outer top-left halo corner is transparent.
        assert!(p1.pixels.chunks_exact(4).all(|px| px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3]));
        assert!(p1.pixels[3] < 20);
    }

    #[test]
    fn the_bar_stands_in_the_middle_above_the_bottom() {
        let [left, top, right, bottom] = crate::hud::place_on(1920.0, 1080.0, (400.0, 48.0), 0.0);
        assert!((left + right).abs() < 0.01);
        assert!(bottom > -1.0 && top > bottom && top < 0.0);
    }

    #[test]
    fn keys_are_named_as_the_keyboard_has_them() {
        assert_eq!(key_name(0x77), "F8");
        assert_eq!(key_name(0xa4), "Alt");
        assert_eq!(key_name(0x05), "Mouse 4");
        assert_eq!(Action::GetOut.words("pt"), "Sair do caminhão");
        assert_eq!(Action::Phone.words("xx"), "Phone");
    }
}
