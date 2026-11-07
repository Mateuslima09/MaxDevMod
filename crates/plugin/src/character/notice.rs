//! The game's own notices in the screen's corner (`INVESTIGACAO.md` §7.9): why the walk key did nothing ("pull
//! the parking brake to get out", "walk up to the driver's door"), as the game says "Stop the vehicle!".
//!
//! The game's function is found by its shape (the same start in ETS2 1.61 and ATS 1.62), and its object
//! through the core object as the game's own calls reach it; nothing found, or the object not as known: the log
//! only, as before.

use std::ffi::{CStr, c_char, c_void};

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
    // SAFETY: the game's code, where its own calls go, with the arguments they give (`Notify`).
    Some(Notices { notify: unsafe { std::mem::transmute::<usize, Notify>(notify) }, core })
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
                c"Abra o app WestPlay para descer do caminhão",
                c"Descer do caminhão só funciona no single player",
            ],
            "es" => [
                c"Pon el freno de mano para bajar",
                c"Acércate a la puerta del conductor para subir",
                c"Abre la app WestPlay para bajar del camión",
                c"Bajar del camión solo funciona en un jugador",
            ],
            "de" => [
                c"Zum Aussteigen die Handbremse anziehen",
                c"Zum Einsteigen zur Fahrertür gehen",
                c"Zum Aussteigen die WestPlay-App öffnen",
                c"Aussteigen geht nur im Einzelspieler",
            ],
            "fr" => [
                c"Serrez le frein à main pour descendre",
                c"Approchez-vous de la portière conducteur pour monter",
                c"Ouvrez l'app WestPlay pour descendre du camion",
                c"Descendre du camion ne marche qu'en solo",
            ],
            "it" => [
                c"Tira il freno a mano per scendere",
                c"Avvicinati alla portiera del conducente per salire",
                c"Apri l'app WestPlay per scendere dal camion",
                c"Scendere dal camion funziona solo in giocatore singolo",
            ],
            "pl" => [
                c"Zaciągnij hamulec ręczny, aby wysiąść",
                c"Podejdź do drzwi kierowcy, aby wsiąść",
                c"Otwórz aplikację WestPlay, aby wysiąść",
                c"Wysiadanie działa tylko w trybie jednoosobowym",
            ],
            "ru" => [
                c"Затяните ручник, чтобы выйти",
                c"Подойдите к двери водителя, чтобы сесть",
                c"Откройте приложение WestPlay, чтобы выйти",
                c"Выйти из грузовика можно только в одиночной игре",
            ],
            "tr" => [
                c"İnmek için el frenini çekin",
                c"Binmek için sürücü kapısına yaklaşın",
                c"İnmek için WestPlay uygulamasını açın",
                c"Tırdan inmek yalnızca tek oyunculu modda çalışır",
            ],
            "nl" => [
                c"Trek de handrem aan om uit te stappen",
                c"Loop naar de bestuurdersdeur om in te stappen",
                c"Open de WestPlay-app om uit te stappen",
                c"Uitstappen werkt alleen in singleplayer",
            ],
            "cs" => [
                c"Pro vystoupení zatáhněte ruční brzdu",
                c"Pro nastoupení dojděte ke dveřím řidiče",
                c"Pro vystoupení otevřete aplikaci WestPlay",
                c"Vystoupit z kamionu lze jen ve hře pro jednoho hráče",
            ],
            _ => [
                c"Pull the parking brake to get out",
                c"Walk up to the driver's door to get in",
                c"Open the WestPlay app to get out of the truck",
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

    #[test]
    fn the_shapes_read_as_written() {
        assert_eq!(pattern(NOTIFY).len(), 67);
        assert!(fits(&[0x48, 0x8B, 0x05, 1, 2, 3, 4], &pattern(OBJECT_SITE)[..7]));
    }
}
