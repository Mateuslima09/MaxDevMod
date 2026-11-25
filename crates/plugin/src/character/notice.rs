//! The game's own notices in the screen's corner (`INVESTIGACAO.md` §7.9): why the walk key did nothing ("pull
//! the parking brake to get out", "walk up to the driver's door"), as the game says "Stop the vehicle!".
//!
//! The game's function is found by its shape (the same start in ETS2 1.61 and ATS 1.62), and its object
//! through the core object as the game's own calls reach it; nothing found, or the object not as known: the log
//! only, as before.

use std::ffi::{CStr, c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::Threading::GetCurrentProcess;

use super::{pattern, read, rel};
use crate::log;
use crate::navigation::Image;

/// The notice function's start (`0x623da0` in ETS2 1.61, `0x622870` in ATS 1.62).
const NOTIFY: &str = "44 89 4C 24 20 4C 89 44 24 18 48 89 54 24 10 55 57 41 56 48 83 EC 60 80 79 38 00 4D 8B F0 \
                      4C 8B C2 49 63 E9 48 8B F9 0F 84 ?? ?? ?? ?? 80 79 3A 00 75 0A 80 79 3B 00 74 04 32 C0 EB 02 \
                      B0 01 84 C0 0F 85";
/// How the game's calls reach the notices' object: `[[core] + 0x31b0] + 0x30`, then `+ 0xd8` (12 calls so in
/// either game).
const OBJECT_SITE: &str = "48 8B 05 ?? ?? ?? ?? 48 8B 80 B0 31 00 00 48 85 C0 74 ?? 48 8B 48 30";
const HUD: usize = 0x31b0;
const NOTICES: usize = 0x30;
const LIST: usize = 0xd8;
/// In the object: notices on (0: the function does nothing), its list of notices (pointer and count, 0x70 bytes
/// each).
const ON: usize = 0x38;
const ENTRIES: usize = 0x18;
const COUNT: usize = 0x20;
const MOST_ENTRIES: usize = 256;
/// A notice in the list.
const ENTRY: usize = 0x70;

/// Whether a value of the game's could be an address of its memory.
fn is_pointer(value: usize) -> bool {
    (0x1_0000..0x7fff_ffff_0000).contains(&value)
}

/// The text at `at` in the game's memory, if there's one a player could read there: NUL-ended UTF-8, a few
/// letters at least, nothing unprintable, not long.
///
/// # Safety
/// Any address (read guarded).
unsafe fn text_at(at: usize) -> Option<String> {
    let mut bytes = Vec::new();
    // (Up to 512 bytes: the pump's panel, its price, litres and total, is longer than 192: owner 2026-11-11.)
    for chunk in 0..16 {
        // SAFETY: guarded.
        let part = unsafe { read::<[u8; 32]>(at + chunk * 32) }?;
        match part.iter().position(|&byte| byte == 0) {
            Some(end) => {
                bytes.extend_from_slice(&part[..end]);
                return readable(&bytes);
            }
            None => bytes.extend_from_slice(&part),
        }
    }
    None
}

/// Bytes as a text a player could read (`text_at`).
fn readable(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    (letters >= 4 && letters * 2 >= text.chars().count() && !text.chars().any(char::is_control))
        .then(|| text.to_owned())
}

/// `with` as bytes `length` long (spaces after it), if it fits.
fn padded(with: &str, length: usize) -> Option<Vec<u8>> {
    let mut bytes = with.as_bytes().to_vec();
    (bytes.len() <= length).then(|| {
        bytes.resize(length, b' ');
        bytes
    })
}

/// The icons (the game's table, the same in both games).
pub const ALERT: i32 = 1;
pub const ANNOUNCEMENT: i32 = 2;
pub const PARKING_BRAKE: i32 = 6;

/// The notice goes after the game's usual time (1 would keep it up until taken away).
const TIMED: i32 = 0;

/// A notice of WestPlay's, by its number: a new one replaces the last instead of piling up.
const OURS: u32 = 0x5750_0001;

#[repr(C)]
struct Id {
    has: u8,
    id: u32,
}

/// `(object, &text, &"", icon, &material, order ×3, word, listener, &id)`, as the game's calls make it.
type Notify = unsafe extern "system" fn(
    *mut u8,
    *const *const c_char,
    *const *const c_char,
    i32,
    *const *const c_char,
    i32,
    i32,
    i32,
    u32,
    *mut c_void,
    *const Id,
) -> usize;

/// The function and the core object's global, as found.
pub struct Notices {
    notify: Notify,
    core: usize,
}

// SAFETY: the game's code and a global of its executable, used only on its main thread.
unsafe impl Send for Notices {}

/// Finds the notice function in the game's code, and checks the game reaches its object from `core` (the core
/// object's global) as known.
pub fn locate(image: &Image, core: usize) -> Option<Notices> {
    let Some(notify) = image.unique_masked(&pattern(NOTIFY)) else {
        log!("character: not found: the game's notices (the walk key's reasons go to the log only)");
        return None;
    };
    let reached =
        image.all_masked(&pattern(OBJECT_SITE)).into_iter().filter(|&at| rel(image, at + 3) == Some(core)).count();
    if reached == 0 {
        log!("character: not found: the game's notices' object (the walk key's reasons go to the log only)");
        return None;
    }
    // The game's own notices said otherwise as it makes them (`notify_detour`).
    // SAFETY: the game's function, its detour of the same signature (`Notify`), calling on through the original.
    let hooked =
        unsafe { crate::hooks::hook_now(notify, notify_detour as *mut c_void, &NOTIFY_ORIGINAL, "the game's notices") };
    if let Err(e) = hooked {
        log!("character: the game's notices not hooked ({e}): its advice to couple stays its own");
    }
    // SAFETY: the game's code, where its own calls go, with the arguments they give (`Notify`).
    Some(Notices { notify: unsafe { std::mem::transmute::<usize, Notify>(notify) }, core })
}

/// The game's notice function as it was (MinHook's trampoline), once hooked.
static NOTIFY_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// The game's advice to couple from the cab ("press T", `@@attach_load@@`), to uncouple at a delivery
/// (`@@unload_load@@`), and to load the player's own trailer ("press Enter", `@@job_load@@`: owner 2026-11-12, the
/// papers load it), and what each says instead (null: its own).
/// And to unload it (`@@job_unload@@`, as the game names its loading's).
const ADVICE: [&[u8]; 4] = [b"@@attach_load@@", b"@@unload_load@@", b"@@job_load@@", b"@@job_unload@@"];
static ADVICE_WORDS: [AtomicPtr<c_char>; 4] = [const { AtomicPtr::new(std::ptr::null_mut()) }; 4];
static ADVICE_SAID: [AtomicBool; 4] = [const { AtomicBool::new(false) }; 4];

/// What the game's advice to couple and to uncouple from the cab say instead (`None`: its own words): both are
/// done on foot (owner 2026-11-11: "the T stays until I couple", and at a delivery, to uncouple; its words through
/// the mod, or rewritten in its list, never showed: it keeps its text as it made it).
pub fn set_attach_words(
    couple: Option<&'static CStr>,
    uncouple: Option<&'static CStr>,
    load: Option<&'static CStr>,
    unload: Option<&'static CStr>,
) {
    for (slot, words) in ADVICE_WORDS.iter().zip([couple, uncouple, load, unload]) {
        slot.store(words.map_or(std::ptr::null_mut(), |w| w.as_ptr().cast_mut()), Ordering::Release);
    }
}

/// Whether the game's panel at a pump ("press and hold Enter to fill the tank", its price, litres and total) is
/// kept from the screen: while WestPlay's filling by hand is on (`refuel`). Owner 2026-11-12, "the refuelling
/// message is still there": rewritten in the game's list it came back, the game making it again each frame.
static HIDE_PUMP_PANEL: AtomicBool = AtomicBool::new(false);
static PUMP_PANEL_HIDDEN: AtomicBool = AtomicBool::new(false);
/// Whether the panel was let through once (for the log).
static PUMP_PANEL_SHOWN: AtomicBool = AtomicBool::new(false);

/// The pump's price as the game's panel last said it (its bits; 0: not heard), for WestPlay's meter.
static PUMP_PRICE: AtomicU32 = AtomicU32::new(0);
/// Whether `PUMP_PRICE` was heard live at the current pump (reset when leaving the pump).
static PUMP_PRICE_FRESH: AtomicBool = AtomicBool::new(false);

/// Whether the game has already reported a live fuel price at the current pump.
pub fn pump_price_fresh() -> bool {
    PUMP_PRICE_FRESH.load(Ordering::Acquire) && f32::from_bits(PUMP_PRICE.load(Ordering::Acquire)) > 0.0
}

/// Marks the cached pump price as stale when leaving a gas station so the next station probes its live price.
pub fn clear_fresh_pump_price() {
    PUMP_PRICE_FRESH.store(false, Ordering::Release);
}

/// The pump's price for a litre (a gallon in the American game) as the game's panel last said it; not said this
/// time, the last one it said (kept in the game's WestPlay folder): getting out before the game showed its panel,
/// no price was heard, so no bill, no attendant, no price on the meter (owner 2026-11-14).
pub fn pump_price() -> Option<f32> {
    let heard = f32::from_bits(PUMP_PRICE.load(Ordering::Acquire));
    if heard > 0.0 {
        return Some(heard);
    }
    let kept = *KEPT_PRICE.get_or_init(|| {
        std::fs::read_to_string(price_file()).ok().and_then(|text| text.trim().parse::<f32>().ok()).filter(|p| *p > 0.0)
    });
    kept
}

/// The last price heard, read from its file once.
static KEPT_PRICE: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();

fn price_file() -> std::path::PathBuf {
    crate::log::game_dir().join("preco-da-bomba.txt")
}

/// A price heard: kept for next time (written only when it changes).
fn keep_price(price: f32) {
    PUMP_PRICE_FRESH.store(true, Ordering::Release);
    if PUMP_PRICE.swap(price.to_bits(), Ordering::AcqRel) != price.to_bits() {
        let _ = std::fs::write(price_file(), format!("{price}\n"));
    }
}

/// The price in the game's panel at a pump (its line `@@diesel_price@@: @@eurosp@@@@europ@@5@@ds@@46@@euros@@`,
/// the money's signs as its keys, `@@ds@@` its decimal point): the number, if one.
fn price_in(text: &str) -> Option<f32> {
    let line = text.split("<br").find(|line| line.contains("price@@") && !line.contains("total_price"))?;
    let (_, after) = line.split_once("price@@")?;
    let after = after.trim_start_matches(':').replace("@@ds@@", ".");
    let mut kept = String::new();
    let mut parts = after.split("@@");
    // Outside the keys (`@@…@@`): the even parts.
    let mut outside = true;
    for part in parts.by_ref() {
        if outside {
            kept.extend(part.chars().filter(|c| c.is_ascii_digit() || *c == '.'));
        }
        outside = !outside;
    }
    kept.parse::<f32>().ok().filter(|price| price.is_finite() && *price > 0.0)
}

/// Keeps the game's panel at a pump from the screen, or lets it be (`HIDE_PUMP_PANEL`).
pub fn hide_pump_panel(hide: bool) {
    HIDE_PUMP_PANEL.store(hide, Ordering::Release);
}

/// The text at `at` as it lies (up to 512 bytes, NUL-ended), whatever it says (read guarded).
///
/// # Safety
/// Any address (read guarded).
unsafe fn raw_text_at(at: usize) -> Option<String> {
    let mut bytes = Vec::new();
    for chunk in 0..16 {
        // SAFETY: guarded.
        let part = unsafe { read::<[u8; 32]>(at + chunk * 32) }?;
        match part.iter().position(|&byte| byte == 0) {
            Some(end) => {
                bytes.extend_from_slice(&part[..end]);
                return Some(String::from_utf8_lossy(&bytes).into_owned());
            }
            None => bytes.extend_from_slice(&part),
        }
    }
    None
}

/// Which of the game's advices from the cab a notice's text (its first bytes) is, if one.
fn advice_of(start: &[u8]) -> Option<usize> {
    ADVICE.iter().position(|key| start.starts_with(key))
}

/// The game's notice function, hooked: its advice to couple given other words (`set_attach_words`), everything
/// else as it was.
#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn notify_detour(
    object: *mut u8,
    text: *const *const c_char,
    second: *const *const c_char,
    icon: i32,
    material: *const *const c_char,
    a: i32,
    b: i32,
    c: i32,
    word: u32,
    listener: *mut c_void,
    id: *const Id,
) -> usize {
    let mut given = text;
    // The pump's panel: its price kept for WestPlay's meter (`fuel_meter`), whether it's shown or not.
    // SAFETY: reads guarded (the game's argument, its text).
    let all = unsafe { read::<usize>(text as usize) }
        .filter(|&at| is_pointer(at))
        // SAFETY: read guarded.
        .and_then(|at| unsafe { raw_text_at(at) });
    // (Its second line too: the panel may come as one, owner 2026-11-20 "the notice up there to press Enter still
    // shows".)
    // SAFETY: reads guarded (the game's argument, its text).
    let second_words = unsafe { read::<usize>(second as usize) }
        .filter(|&at| is_pointer(at))
        // SAFETY: read guarded.
        .and_then(|at| unsafe { raw_text_at(at) });
    // The price from any of its notices that says it (either line): the "tank full" one too, the only one seen
    // getting out before the panel (owner 2026-11-15: it said R$6,42, the meter showed none, no bill).
    if let Some(price) = all.as_deref().and_then(price_in).or_else(|| second_words.as_deref().and_then(price_in)) {
        keep_price(price);
    }
    let words = all.filter(|words| super::refuel::pump_panel(words)).or_else(|| {
        second_words.filter(|words| super::refuel::pump_panel(words))
    });
    if words.is_some() && !HIDE_PUMP_PANEL.load(Ordering::Acquire) && !PUMP_PANEL_SHOWN.swap(true, Ordering::AcqRel) {
        log!("notice: the game's pump panel shown (filling by hand off here)");
    }
    // The pump's panel, while WestPlay fills by hand: not made at all. The function's own way out when the notices
    // are off or hidden (`INVESTIGACAO.md` §7.9: the stack given back, nothing made, nothing returned the callers
    // read), so the game's pump goes on as when its notices are off.
    if HIDE_PUMP_PANEL.load(Ordering::Acquire) && words.is_some() {
        if !PUMP_PANEL_HIDDEN.swap(true, Ordering::AcqRel) {
            log!("notice: the game's pump panel kept from the screen (filling by hand)");
        }
        return 0;
    }
    // SAFETY: reads guarded (the game's argument, its text's first bytes).
    let advice = unsafe { read::<usize>(text as usize) }
        .filter(|&at| is_pointer(at))
        .and_then(|at| unsafe { read::<[u8; 15]>(at) })
        .and_then(|start| advice_of(&start));
    let instead = advice.map_or(std::ptr::null(), |i| ADVICE_WORDS[i].load(Ordering::Acquire).cast_const());
    if let Some(i) = advice
        && !instead.is_null()
    {
        given = &instead;
        if !ADVICE_SAID[i].swap(true, Ordering::AcqRel) {
            log!("notice: the game's advice to {} said otherwise", ["couple", "uncouple", "load", "unload"][i]);
        }
    }
    // SAFETY: the original function (MinHook's trampoline), with the game's arguments (the text maybe ours, a
    // static string, as `show` gives it).
    unsafe {
        let original = std::mem::transmute::<usize, Notify>(NOTIFY_ORIGINAL.load(Ordering::Acquire));
        original(object, given, second, icon, material, a, b, c, word, listener, id)
    }
}

impl Notices {
    /// Shows `text` (UTF-8, or a game key `@@…@@`, which it translates) with `icon` in the screen's corner.
    ///
    /// # Safety
    /// On the game's main thread, at the end of a frame.
    pub unsafe fn show(&self, text: &CStr, icon: i32) {
        // SAFETY: every read guarded; the function called as the game's calls call it, the object checked first.
        unsafe {
            let Some(object) = self.object() else {
                log!("character: no notice (the game's notices aren't as known)");
                return;
            };
            let text = text.as_ptr();
            let empty = c"".as_ptr();
            let id = Id { has: 1, id: OURS };
            // The order as "Stop the vehicle!" gives it (2, _, 2; its word 7), but not kept: the second number 1
            // keeps a notice up until the game takes it away (as it does "Stop the vehicle!" once stopped; ours
            // stayed for good, owner 2026-11-05), any other gives it the game's usual time (`INVESTIGACAO.md` §7.9).
            (self.notify)(object, &text, &empty, icon, &empty, 2, TIMED, 2, 7, std::ptr::null_mut(), &id);
        }
    }

    /// The texts of the notices in the game's list now (its own, and WestPlay's), as far as they can be read: each
    /// notice's 0x70 bytes are looked through for texts (a pointer to one, or to one's holder). What a notice is
    /// made of isn't known beyond that, so this only ever tells what the game says, never changes it.
    ///
    /// # Safety
    /// On the game's main thread.
    pub unsafe fn texts(&self) -> Vec<String> {
        let mut texts = Vec::new();
        // SAFETY: every read guarded (`read`: the system's checked copy).
        unsafe {
            let Some(object) = self.object() else { return texts };
            let object = object as usize;
            let (Some(count), Some(entries)) = (read::<usize>(object + COUNT), read::<usize>(object + ENTRIES)) else {
                return texts;
            };
            for entry in (0..count.min(MOST_ENTRIES)).map(|i| entries + i * ENTRY) {
                for slot in (0..ENTRY).step_by(8) {
                    let Some(pointer) = read::<usize>(entry + slot).filter(|&p| is_pointer(p)) else { continue };
                    let text = text_at(pointer)
                        .or_else(|| read::<usize>(pointer).filter(|&p| is_pointer(p)).and_then(|p| text_at(p)));
                    if let Some(text) = text
                        && !texts.contains(&text)
                    {
                        texts.push(text);
                    }
                }
            }
        }
        texts
    }

    /// Rewrites in place the text of each notice in the game's list that `matches` (one starting with a key of the
    /// game's, `@@attach_load@@`): `with` written over it, padded with spaces to its length (never longer, the game's own
    /// buffer). The game translates a notice's key as it draws it, so its words change as the text does (owner
    /// 2026-11-11: the T to couple stayed, its words given through the mod not taken). How many were rewritten.
    ///
    /// # Safety
    /// On the game's main thread.
    pub unsafe fn reword(&self, matches: impl Fn(&str) -> bool, with: &str) -> usize {
        let mut done = 0;
        // SAFETY: every read guarded; written only over a text just read there, no longer than it.
        unsafe {
            let Some(object) = self.object() else { return 0 };
            let object = object as usize;
            let (Some(count), Some(entries)) = (read::<usize>(object + COUNT), read::<usize>(object + ENTRIES)) else {
                return 0;
            };
            for entry in (0..count.min(MOST_ENTRIES)).map(|i| entries + i * ENTRY) {
                for slot in (0..ENTRY).step_by(8) {
                    let Some(pointer) = read::<usize>(entry + slot).filter(|&p| is_pointer(p)) else { continue };
                    let holder = read::<usize>(pointer).filter(|&p| is_pointer(p));
                    for at in std::iter::once(pointer).chain(holder) {
                        let Some(text) = text_at(at).filter(|text| text != with && matches(text)) else { continue };
                        let Some(bytes) = padded(with, text.len()) else { continue };
                        let mut wrote = 0usize;
                        let ok = WriteProcessMemory(
                            GetCurrentProcess(),
                            at as *const c_void,
                            bytes.as_ptr().cast(),
                            bytes.len(),
                            Some(&mut wrote),
                        );
                        if ok.is_ok() && wrote == bytes.len() {
                            done += 1;
                        }
                        break;
                    }
                }
            }
        }
        done
    }

    /// The notices' object, if it's there, on, and its list as the function walks it.
    unsafe fn object(&self) -> Option<*mut u8> {
        // SAFETY: reads guarded.
        unsafe {
            let core = read::<usize>(self.core).filter(|&c| c != 0)?;
            let hud = read::<usize>(core + HUD).filter(|&h| h != 0)?;
            let notices = read::<usize>(hud + NOTICES).filter(|&n| n != 0)?;
            let object = notices + LIST;
            let count = read::<usize>(object + COUNT)?;
            let entries = read::<usize>(object + ENTRIES)?;
            let list_fits = count == 0 || (count <= MOST_ENTRIES && read::<u8>(entries).is_some());
            (read::<u8>(object + ON)? != 0 && list_fits).then_some(object as *mut u8)
        }
    }
}

/// Why the walk key did nothing, as a notice: the player's own words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    ParkingBrake,
    Moving,
    FarFromDoor,
    AppClosed,
    Multiplayer,
}

impl Why {
    pub fn icon(self) -> i32 {
        match self {
            Why::ParkingBrake => PARKING_BRAKE,
            Why::FarFromDoor => ANNOUNCEMENT,
            Why::Moving | Why::AppClosed | Why::Multiplayer => ALERT,
        }
    }

    /// In the game's language (its `g_lang`, two letters), English if not one of these.
    pub fn text(self, language: &str) -> &'static CStr {
        // The game's own words where it has them ("Stop the vehicle!", in every language).
        if self == Why::Moving {
            return c"@@stop_vehicle_short@@";
        }
        let i = match self {
            Why::ParkingBrake => 0,
            Why::FarFromDoor => 1,
            Why::AppClosed => 2,
            _ => 3,
        };
        let words: [&CStr; 4] = match language {
            "pt" => [
                c"Puxe o freio de mão para descer",
                c"Chegue perto da porta do motorista para entrar",
                c"Abra o app MaxDevMod para descer do caminhão",
                c"Descer do caminhão só funciona no single player",
            ],
            "es" => [
                c"Pon el freno de mano para bajar",
                c"Acércate a la puerta del conductor para subir",
                c"Abre la app MaxDevMod para bajar del camión",
                c"Bajar del camión solo funciona en un jugador",
            ],
            "de" => [
                c"Zum Aussteigen die Handbremse anziehen",
                c"Zum Einsteigen zur Fahrertür gehen",
                c"Zum Aussteigen die MaxDevMod-App öffnen",
                c"Aussteigen geht nur im Einzelspieler",
            ],
            "fr" => [
                c"Serrez le frein à main pour descendre",
                c"Approchez-vous de la portière conducteur pour monter",
                c"Ouvrez l'app MaxDevMod pour descendre du camion",
                c"Descendre du camion ne marche qu'en solo",
            ],
            "it" => [
                c"Tira il freno a mano per scendere",
                c"Avvicinati alla portiera del conducente per salire",
                c"Apri l'app MaxDevMod per scendere dal camion",
                c"Scendere dal camion funziona solo in giocatore singolo",
            ],
            "pl" => [
                c"Zaciągnij hamulec ręczny, aby wysiąść",
                c"Podejdź do drzwi kierowcy, aby wsiąść",
                c"Otwórz aplikację MaxDevMod, aby wysiąść",
                c"Wysiadanie działa tylko w trybie jednoosobowym",
            ],
            "ru" => [
                c"Затяните ручник, чтобы выйти",
                c"Подойдите к двери водителя, чтобы сесть",
                c"Откройте приложение MaxDevMod, чтобы выйти",
                c"Выйти из грузовика можно только в одиночной игре",
            ],
            "tr" => [
                c"İnmek için el frenini çekin",
                c"Binmek için sürücü kapısına yaklaşın",
                c"İnmek için MaxDevMod uygulamasını açın",
                c"Tırdan inmek yalnızca tek oyunculu modda çalışır",
            ],
            "nl" => [
                c"Trek de handrem aan om uit te stappen",
                c"Loop naar de bestuurdersdeur om in te stappen",
                c"Open de MaxDevMod-app om uit te stappen",
                c"Uitstappen werkt alleen in singleplayer",
            ],
            "cs" => [
                c"Pro vystoupení zatáhněte ruční brzdu",
                c"Pro nastoupení dojděte ke dveřím řidiče",
                c"Pro vystoupení otevřete aplikaci MaxDevMod",
                c"Vystoupit z kamionu lze jen ve hře pro jednoho hráče",
            ],
            _ => [
                c"Pull the parking brake to get out",
                c"Walk up to the driver's door to get in",
                c"Open the MaxDevMod app to get out of the truck",
                c"Getting out of the truck works in single player only",
            ],
        };
        words[i]
    }
}

/// The game's language, two letters (`pt` of `pt_br`), from its options (`uset g_lang "pt_br"`; before it's
/// chosen, `g_lang_init`).
pub fn language_of(config: &str) -> String {
    let value = |name: &str| {
        config.lines().find_map(|line| {
            let mut words = line.split_whitespace();
            (words.next() == Some("uset") && words.next() == Some(name))
                .then(|| words.next().map(|v| v.trim_matches('"').to_owned()))
                .flatten()
                .filter(|v| !v.is_empty())
        })
    };
    let language = value("g_lang").or_else(|| value("g_lang_init")).unwrap_or_default();
    language.split('_').next().unwrap_or_default().to_ascii_lowercase()
}

/// The game's language, read once from its `config.cfg` in its Documents folder.
pub fn language() -> &'static str {
    static LANGUAGE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    LANGUAGE.get_or_init(|| {
        crate::log::game_documents()
            .and_then(|dir| std::fs::read_to_string(dir.join("config.cfg")).ok())
            .map(|config| language_of(&config))
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::fits;

    #[test]
    fn the_language_is_the_games_chosen_one() {
        assert_eq!(language_of("uset g_lang_init \"en_us\"\nuset g_lang \"pt_br\"\n"), "pt");
        assert_eq!(language_of("uset g_lang \"\"\r\nuset g_lang_init \"de_de\"\r\n"), "de");
        assert_eq!(language_of("uset g_fps 1"), "");
    }

    #[test]
    fn the_notices_are_in_the_games_language_or_english() {
        assert_eq!(Why::ParkingBrake.text("pt"), c"Puxe o freio de mão para descer");
        assert_eq!(Why::FarFromDoor.text("ja"), c"Walk up to the driver's door to get in");
        assert_eq!(Why::Moving.text("pt"), c"@@stop_vehicle_short@@");
        assert_eq!(Why::ParkingBrake.icon(), PARKING_BRAKE);
    }

    /// What the game's notices are searched for: words a player reads, not the bytes around them.
    #[test]
    fn tells_a_text_from_other_bytes() {
        assert_eq!(
            readable("Desligue o motor para abastecer".as_bytes()).as_deref(),
            Some("Desligue o motor para abastecer")
        );
        assert_eq!(readable(b"ab"), None, "too short");
        assert_eq!(readable(&[0xff, 0xfe, b'a', b'b', b'c', b'd']), None, "not text");
        assert_eq!(readable(b"\x01\x02abcd"), None, "unprintable");
        assert_eq!(readable(b"1234567890ab"), None, "numbers, not words");
    }

    #[test]
    fn knows_the_games_advice_to_couple() {
        assert_eq!(advice_of(b"@@attach_load@@"), Some(0));
        assert_eq!(advice_of(b"@@unload_load@@"), Some(1));
        assert_eq!(advice_of(b"@@job_load@@ <col"), Some(2));
        assert_eq!(advice_of(b"@@trailer_coupl"), None);
    }

    /// A text written over another: as long as it, spaces after; never longer.
    #[test]
    fn reads_the_pumps_price_from_its_panel() {
        let panel = "<color value=@@clr_sel@@>@@eurosp@@@@europ@@47@@ds@@6@@unit_mega@@ @@euros@@<br offset=10><color \
                     value=@@clr_txt@@>@@gas_station_hint@@ <color value=@@clr_sel@@>Enter<br offset=10><color \
                     value=@@clr_txt@@>@@diesel_price@@: @@eurosp@@@@europ@@5@@ds@@46@@euros@@<br>@@litres@@: 0.00<br>\
                     <color value=@@clr_sel@@>@@total_price@@: @@eurosp@@@@europ@@0@@euros@@";
        assert_eq!(price_in(panel), Some(5.46));
        assert_eq!(price_in("@@tank_full@@"), None);
    }

    #[test]
    fn a_text_is_written_over_another_no_longer_than_it() {
        assert_eq!(padded("abc", 6).as_deref(), Some(&b"abc   "[..]));
        assert_eq!(padded("abc", 3).as_deref(), Some(&b"abc"[..]));
        assert_eq!(padded("abcd", 3), None);
    }

    #[test]
    fn the_shapes_read_as_written() {
        assert_eq!(pattern(NOTIFY).len(), 67);
        assert!(fits(&[0x48, 0x8B, 0x05, 1, 2, 3, 4], &pattern(OBJECT_SITE)[..7]));
    }
}

#[cfg(test)]
mod tank_full_price_tests {
    use super::*;

    #[test]
    fn the_tank_full_panel_tells_the_price() {
        // The game's words on 2026-11-15, the tank full.
        let text = "<color value=@@clr_txt@@>@@diesel_price@@: @@eurosp@@@@europ@@6@@ds@@42@@euros@@<br>@@litres@@: 20.34<br><color value=@@clr_sel@@>@@total_price@@: @@eurosp@@@@europ@@130@@euros@@<br offset=10><color value=@@clr_txt@@>@@tank_full@@ <color value=@@clr_sel@@>";
        assert_eq!(price_in(text), Some(6.42));
    }
}
