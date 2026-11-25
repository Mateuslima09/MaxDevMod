//! WestPlay's settings: one store the app keeps on disk, read and changed by the companion window (and, later,
//! the in-cabin Settings app), applied live by the host.
//!
//! Saved as JSON with a version number (`settings.json` in the app's config folder). Loading is forgiving, so
//! the app always starts: unknown fields are ignored, a missing or invalid value takes its default, and an
//! unreadable file is set aside as `settings.json.bad`. Files from older versions are migrated in [`migrate`].

use std::io;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use westplay_shared::{
    CHARACTER_KEY, CHARACTER_PEOPLE, CHARACTER_PERSON, CHARACTER_SENSITIVITY, KEYBIND_COUNT, Keybind, keybind_value,
};

/// Version of the settings file format. Bump it, and teach [`migrate`] the step, when a setting's meaning
/// or shape changes (adding a setting needs neither: it takes its default).
pub const SETTINGS_VERSION: u32 = 2;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    #[serde(deserialize_with = "lenient")]
    pub interaction: InteractionSettings,
    #[serde(deserialize_with = "lenient")]
    pub keybinds: Keybinds,
    #[serde(deserialize_with = "lenient")]
    pub screen: ScreenSettings,
    #[serde(deserialize_with = "lenient")]
    pub look: LookSettings,
    #[serde(deserialize_with = "lenient")]
    pub graphics: GraphicsSettings,
    #[serde(deserialize_with = "lenient")]
    pub sound: SoundSettings,
    #[serde(deserialize_with = "lenient")]
    pub character: CharacterSettings,
    /// Which of the game's virtual mirrors show (the phone's Settings, owner 2026-11-18).
    #[serde(deserialize_with = "lenient")]
    pub virtual_mirrors: VirtualMirrors,
    /// Whether the game sets the suspension back by itself (the phone's Suspensão app, owner 2026-11-18): on, as
    /// the game has it, until the player turns it off. Kept in the games' own options with them closed.
    #[serde(deserialize_with = "lenient")]
    pub suspension_auto_reset: Shown,
    /// How low the front may go (the Suspensão app, owner 2026-11-18): the game's own, lower, the lowest
    /// (`westplay_setup::low_suspension`).
    #[serde(deserialize_with = "lenient")]
    pub suspension_low_front: LowFront,
    /// How firm the dampers are (the Suspensão app, owner 2026-11-18): `westplay_setup::low_suspension::Damping`.
    #[serde(deserialize_with = "lenient")]
    pub suspension_damping: Damping,
    /// How big the mouse's dot is on the truck's screen and the phone (owner 2026-11-19).
    #[serde(deserialize_with = "lenient")]
    pub cursor_size: CursorSize,
    /// The suspension by itself (the Suspensão app, owner 2026-11-19): lowered on the motorway, normal in town
    /// (`auto_suspension`). Off at first.
    #[serde(deserialize_with = "lenient")]
    pub suspension_auto: bool,
    /// Seconds of rough road before automatic suspension raises (1..=5, 0 = default 3 s; owner 2026-11-22).
    #[serde(deserialize_with = "lenient")]
    pub suspension_auto_delay: u8,
    /// Seconds of smooth road before automatic suspension lowers back down (3..=15, 0 = default 7 s; owner 2026-11-22).
    #[serde(deserialize_with = "lenient")]
    pub suspension_auto_return: u8,
    /// Seconds of hard braking before automatic suspension raises a little (1..=5, 0 = default 2 s; owner 2026-11-23).
    #[serde(deserialize_with = "lenient")]
    pub suspension_auto_brake: u8,
    /// Brake fade (owner 2026-11-21, brake_fade.rs): the brakes heat up and, overheated, lose force. On at first.
    #[serde(deserialize_with = "lenient")]
    pub brake_fade: Shown,
    /// How soon it comes on: `light`, `realistic` or `hard`.
    #[serde(deserialize_with = "lenient")]
    pub brake_fade_level: crate::brake_fade::Level,
    /// The brake fade's sounds' volumes, 0 (off) to 100: the iron grinding with the brake gone, the dashboard's
    /// warning (owner 2026-11-21: "an option to turn it off, or louder").
    #[serde(deserialize_with = "lenient")]
    pub brake_squeal_volume: Volume70,
    #[serde(deserialize_with = "lenient")]
    pub brake_warning_volume: Volume70,
    /// The brakes' HUD (plugin brake_hud.rs): shown, its corner, its size, and whether to show it on foot in Modo
    /// Personagem (owner 2026-11-21; off at first so the screen stays clean while walking).
    #[serde(deserialize_with = "lenient")]
    pub brake_hud: Shown,
    #[serde(deserialize_with = "lenient")]
    pub brake_hud_corner: BrakeHudCorner,
    #[serde(deserialize_with = "lenient")]
    pub brake_hud_size: BrakeHudSize,
    #[serde(deserialize_with = "lenient")]
    pub brake_hud_on_foot: bool,
    /// 3D smoke rising from overheated brake hubs (owner 2026-11-23; separate truck and trailer wheels). On at first.
    #[serde(deserialize_with = "lenient")]
    pub brake_smoke: Shown,
    /// At what temperature the dashboard's warning and the phone's "no brakes" come (°C on the realistic level; the
    /// light and hard ones move it with the fade); 0, the usual 360.
    #[serde(deserialize_with = "lenient")]
    pub brake_alarm_c: u16,
    /// Where media audio plays (`cabin` = truck cab speakers + muffled outside/on foot; `device` = directly on the
    /// phone/iPad in clean stereo even in Modo Personagem).
    #[serde(deserialize_with = "lenient")]
    pub audio_output: AudioOutput,
}

/// Where YouTube / music audio is routed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioOutput {
    /// Through the truck's 3D cab speakers (muffled when outside the cab / on foot).
    #[default]
    Cabin,
    /// Directly on the handheld phone/iPad (clear stereo even on foot in Modo Personagem).
    Device,
}

impl AudioOutput {
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "cabin" => Some(Self::Cabin),
            "device" => Some(Self::Device),
            _ => None,
        }
    }
}

/// Rough-road activation delay in seconds (1..=5, default 3 when unset/0).
pub fn suspension_auto_delay(settings: &Settings) -> u8 {
    match settings.suspension_auto_delay {
        0 => 3,
        secs => secs.clamp(1, 5),
    }
}

/// Smooth-road return delay in seconds (3..=15, default 7 when unset/0).
pub fn suspension_auto_return(settings: &Settings) -> u8 {
    match settings.suspension_auto_return {
        0 => 7,
        secs => secs.clamp(3, 15),
    }
}

/// Hard-braking raise delay in seconds (1..=5, default 2 when unset/0).
pub fn suspension_auto_brake(settings: &Settings) -> u8 {
    match settings.suspension_auto_brake {
        0 => 2,
        secs => secs.clamp(1, 5),
    }
}

/// The brake alarm's temperature on the realistic level (`Settings::brake_alarm_c`, 0 the usual).
pub fn brake_alarm_c(settings: &Settings) -> u16 {
    match settings.brake_alarm_c {
        0 => 360,
        c => c.clamp(260, 395),
    }
}

/// A volume, 0..100, 70 at first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "u8", into = "u8")]
pub struct Volume70(pub u8);

impl Default for Volume70 {
    fn default() -> Self {
        Self(70)
    }
}

impl From<u8> for Volume70 {
    fn from(value: u8) -> Self {
        Self(value.min(100))
    }
}

impl From<Volume70> for u8 {
    fn from(value: Volume70) -> Self {
        value.0
    }
}

/// Where the brakes' HUD sits (`westplay_shared::BRAKE_HUD_*`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrakeHudCorner {
    #[default]
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

/// How big the brakes' HUD is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrakeHudSize {
    #[default]
    Small,
    Medium,
    Large,
}

/// How big the mouse's dot is (`Settings::cursor_size`), times the one drawn at first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CursorSize {
    Small,
    #[default]
    Normal,
    Large,
    Larger,
}

impl CursorSize {
    pub fn factor(self) -> f32 {
        match self {
            Self::Small => 0.7,
            Self::Normal => 1.0,
            Self::Large => 1.35,
            Self::Larger => 1.7,
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "small" => Some(Self::Small),
            "normal" => Some(Self::Normal),
            "large" => Some(Self::Large),
            "larger" => Some(Self::Larger),
            _ => None,
        }
    }
}

/// How firm the dampers are (`Settings::suspension_damping`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Damping {
    Soft,
    #[default]
    Normal,
    Firm,
    Firmest,
}

impl Damping {
    pub fn level(self) -> westplay_setup::low_suspension::Damping {
        match self {
            Self::Soft => westplay_setup::low_suspension::Damping::Soft,
            Self::Normal => westplay_setup::low_suspension::Damping::Normal,
            Self::Firm => westplay_setup::low_suspension::Damping::Firm,
            Self::Firmest => westplay_setup::low_suspension::Damping::Firmest,
        }
    }

    /// From a page's word for it.
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "soft" => Some(Self::Soft),
            "normal" => Some(Self::Normal),
            "firm" => Some(Self::Firm),
            "firmest" => Some(Self::Firmest),
            _ => None,
        }
    }
}

/// How low the front may go (`Settings::suspension_low_front`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LowFront {
    #[default]
    Normal,
    Low,
    Lowest,
}

impl LowFront {
    pub fn level(self) -> westplay_setup::low_suspension::Level {
        match self {
            Self::Normal => westplay_setup::low_suspension::Level::Normal,
            Self::Low => westplay_setup::low_suspension::Level::Low,
            Self::Lowest => westplay_setup::low_suspension::Level::Lowest,
        }
    }

    /// From a page's word for it.
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "normal" => Some(Self::Normal),
            "low" => Some(Self::Low),
            "lowest" => Some(Self::Lowest),
            _ => None,
        }
    }
}

/// Which of the game's virtual mirrors show on the screen (owner 2026-11-18: "some like to simulate and don't want
/// the virtual mirrors; let them choose both, the right one only, the left one only, or none"). The right one only
/// at first: the driver sits on the left and sees the left glass mirror, the right one is the one wanted virtual.
/// The reversing camera is made from the left one, hidden or not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VirtualMirrors {
    None,
    #[default]
    Right,
    Left,
    Both,
}

impl VirtualMirrors {
    /// Its value on the bridge (`HostBlock::virtual_mirrors`).
    pub fn bridge_value(self) -> u32 {
        match self {
            Self::None => westplay_shared::VIRTUAL_MIRRORS_NONE,
            Self::Right => westplay_shared::VIRTUAL_MIRRORS_RIGHT,
            Self::Left => westplay_shared::VIRTUAL_MIRRORS_LEFT,
            Self::Both => westplay_shared::VIRTUAL_MIRRORS_BOTH,
        }
    }

    /// From a page's word for it (`none`, `right`, `left`, `both`).
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "none" => Some(Self::None),
            "right" => Some(Self::Right),
            "left" => Some(Self::Left),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

/// The Modo Personagem (the companion's Personagem page): walking about on foot by the truck (F8 with it stopped),
/// on by default (owner 2026-11-07: a fresh install "all ready, just get in the game and play"); the mouse's speed
/// while walking, in percent of the game's; the key getting out and back in; and who walks.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CharacterSettings {
    #[serde(deserialize_with = "lenient")]
    pub enabled: bool,
    #[serde(deserialize_with = "lenient")]
    pub sensitivity: CharacterSensitivity,
    #[serde(deserialize_with = "lenient")]
    pub key: CharacterKey,
    #[serde(deserialize_with = "lenient")]
    pub person: CharacterPerson,
    /// The key hints bar over the game (shown at first), and the key showing and hiding it there (none at first).
    #[serde(deserialize_with = "lenient")]
    pub hints: Shown,
    #[serde(deserialize_with = "lenient")]
    pub hints_key: HintsKey,
    /// The cargo's papers and coupling by hand, refuelling by hand, and paying that fuel at the pump's attendant:
    /// each on at first (owner 2026-11-13: "a switch for each, in the app and on the phone", the in-game phone's
    /// Settings the same ones).
    #[serde(deserialize_with = "lenient")]
    pub papers: Shown,
    #[serde(deserialize_with = "lenient")]
    pub refuel: Shown,
    #[serde(deserialize_with = "lenient")]
    pub pay_attendant: Shown,
}

impl Default for CharacterSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: CharacterSensitivity::default(),
            key: CharacterKey::default(),
            person: CharacterPerson::default(),
            hints: Shown::default(),
            hints_key: HintsKey::default(),
            papers: Shown::default(),
            refuel: Shown::default(),
            pay_attendant: Shown::default(),
        }
    }
}

/// Something shown unless the player hid it: shown by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "bool", into = "bool")]
pub struct Shown(pub bool);

impl Default for Shown {
    fn default() -> Self {
        Self(true)
    }
}

impl From<bool> for Shown {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<Shown> for bool {
    fn from(shown: Shown) -> Self {
        shown.0
    }
}

/// The key showing and hiding the key hints bar: a Windows virtual-key code (1 to 254), or 0 for none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "f64", into = "u32")]
pub struct HintsKey(u32);

impl HintsKey {
    pub fn new(code: f64) -> Self {
        if code.is_finite() && (1.0..=254.0).contains(&code) { Self(code as u32) } else { Self(0) }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl From<f64> for HintsKey {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<HintsKey> for u32 {
    fn from(key: HintsKey) -> Self {
        key.0
    }
}

/// The key getting out of the truck and back in: a Windows virtual-key code (1 to 254), F8 if none or another.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "f64", into = "u32")]
pub struct CharacterKey(u32);

impl CharacterKey {
    pub fn new(code: f64) -> Self {
        if code.is_finite() && (1.0..=254.0).contains(&code) { Self(code as u32) } else { Self::default() }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl From<f64> for CharacterKey {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<CharacterKey> for u32 {
    fn from(key: CharacterKey) -> Self {
        key.0
    }
}

impl Default for CharacterKey {
    fn default() -> Self {
        Self(CHARACTER_KEY)
    }
}

/// Who walks: a place in the bridge's [`CHARACTER_PEOPLE`] (the default one if none or out of it).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "f64", into = "u32")]
pub struct CharacterPerson(u32);

/// People in [`CHARACTER_PEOPLE`] not to be chosen for now (owner 2026-11-19: "turn the stout man off for now, his
/// animation isn't good; we'll polish him later"): his files stay in the mod, the default walks in his place.
pub const PEOPLE_OFF: &[usize] = &[4];

impl CharacterPerson {
    pub fn new(place: f64) -> Self {
        if place.is_finite()
            && place >= 0.0
            && (place as usize) < CHARACTER_PEOPLE.len()
            && !PEOPLE_OFF.contains(&(place as usize))
        {
            Self(place as u32)
        } else {
            Self::default()
        }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl From<f64> for CharacterPerson {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<CharacterPerson> for u32 {
    fn from(person: CharacterPerson) -> Self {
        person.0
    }
}

impl Default for CharacterPerson {
    fn default() -> Self {
        Self(CHARACTER_PERSON as u32)
    }
}

/// The mouse's speed while walking, in percent: always within the bridge's [`CHARACTER_SENSITIVITY`] (corrected
/// on the way in).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "f64", into = "u32")]
pub struct CharacterSensitivity(u32);

impl CharacterSensitivity {
    pub fn new(percent: f64) -> Self {
        let (least, most, _) = CHARACTER_SENSITIVITY;
        if percent.is_finite() { Self((percent.round() as u32).clamp(least, most)) } else { Self::default() }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl From<f64> for CharacterSensitivity {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<CharacterSensitivity> for u32 {
    fn from(sensitivity: CharacterSensitivity) -> Self {
        sensitivity.0
    }
}

impl Default for CharacterSensitivity {
    fn default() -> Self {
        Self(CHARACTER_SENSITIVITY.2)
    }
}

/// The sound the players play: its tone (the cab's Settings › Equalizer), the same in every truck. As iOS's
/// AirPods have it (owner 2026-10-22): Recommended, the speakers as tuned, or Custom, the player's own curve
/// (kept while Recommended plays).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SoundSettings {
    /// The player's own curve.
    #[serde(deserialize_with = "lenient")]
    pub equalizer: crate::speakers::Equalizer,
    /// Custom chosen: the curve plays (otherwise flat).
    #[serde(deserialize_with = "lenient")]
    pub custom: bool,
}

impl SoundSettings {
    /// The tone the players play: the curve if Custom is chosen, flat if Recommended.
    pub fn playing(&self) -> crate::speakers::Equalizer {
        if self.custom { self.equalizer.clamped() } else { crate::speakers::Equalizer::default() }
    }
}

/// Which GPU the cab is drawn on (the companion's Truck Screen): one the player chose, by its id
/// (`graphics::Adapter::id`), or (`None`) the one WestPlay picks. A chosen GPU that's gone (an external one
/// unplugged) leaves WestPlay picking.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GraphicsSettings {
    #[serde(deserialize_with = "lenient")]
    pub gpu: Option<String>,
}

/// How the screen looks: its wallpaper (chosen in the cab's Settings or the companion's Customize) and its
/// icons (the companion's Customize).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LookSettings {
    /// A wallpaper's id: one of WestPlay's (`plain`) or the player's (`custom:…`, `customize`). None:
    /// the cabin page's own default.
    #[serde(deserialize_with = "lenient")]
    pub wallpaper: Option<String>,
    /// One of the player's icon sets, by name. None: WestPlay's own icons.
    #[serde(deserialize_with = "lenient")]
    pub icon_set: Option<String>,
    /// Light or dark icons, or (`auto`) as the screen is.
    #[serde(deserialize_with = "lenient")]
    pub icon_style: IconStyle,
}

/// Which of an icon's looks the screen shows: as the screen is (light or dark), or always one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IconStyle {
    #[default]
    Auto,
    Light,
    Dark,
}

/// Using the truck screen: hold the interaction key (or press it, toggled), and the mouse moves a cursor on
/// the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InteractionSettings {
    #[serde(deserialize_with = "lenient")]
    pub key: KeyCode,
    #[serde(deserialize_with = "lenient")]
    pub mode: InteractionMode,
    #[serde(deserialize_with = "lenient")]
    pub sensitivity: Sensitivity,
}

/// How the interaction key is used: held while using the screen, or pressed once to start and again to stop
/// (owner 2026-10-21: a player's suggestion, a switch in the companion).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InteractionMode {
    #[default]
    Hold,
    Toggle,
}

/// A key of the keyboard or a button of the mouse, as Windows numbers it (its virtual-key code: 0x05 the
/// mouse's back button, 0x31 the 1 key, 0xA4 Left Alt). Every keybind is any of them (owner 2026-10-21); the
/// first versions offered four, by name, which still read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(into = "u32")]
pub struct KeyCode(u32);

impl KeyCode {
    /// Left Alt: free in the game's default controls and comfortable to hold while driving. (Not Right Alt:
    /// on many layouts, Turkish among them, it is AltGr, which also sends a Ctrl press, a gearbox key for
    /// many players.)
    pub const LEFT_ALT: Self = Self(0xa4);

    /// A virtual-key code, if it's one (1 to 254).
    pub fn new(code: u32) -> Option<Self> {
        (1..=0xfe).contains(&code).then_some(Self(code))
    }

    /// The Windows virtual-key code the plugin checks.
    pub fn virtual_key(self) -> u32 {
        self.0
    }

    /// A keyboard key's name as Windows gives it for the player's keyboard ("Right Ctrl", "Num 4", "Q"):
    /// the companion's keybinds. `None` for a mouse button, or a key Windows has no name for.
    pub fn name(code: u32) -> Option<String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyNameTextW, MAPVK_VK_TO_VSC_EX, MapVirtualKeyW};
        // SAFETY: plain queries; the buffer is a local the call writes within its length.
        unsafe {
            // The scan code, 0xE0xx for an extended key (right Ctrl and Alt, the arrows, Insert...).
            let scancode = MapVirtualKeyW(code, MAPVK_VK_TO_VSC_EX);
            if scancode == 0 {
                return None;
            }
            let extended = i32::from(scancode & 0xff00 == 0xe000);
            let lparam = ((scancode & 0xff) as i32) << 16 | extended << 24;
            let mut name = [0u16; 64];
            let length = usize::try_from(GetKeyNameTextW(lparam, &mut name)).ok().filter(|&n| n > 0)?;
            Some(String::from_utf16_lossy(&name[..length]))
        }
    }

    /// The four keys the first versions offered, by the names they saved.
    fn named(name: &str) -> Option<Self> {
        Some(Self(match name {
            "leftAlt" => 0xa4,
            "rightCtrl" => 0xa3,
            "mouseBack" => 0x05,
            "mouseForward" => 0x06,
            _ => return None,
        }))
    }
}

impl Default for KeyCode {
    fn default() -> Self {
        Self::LEFT_ALT
    }
}

impl From<KeyCode> for u32 {
    fn from(key: KeyCode) -> Self {
        key.0
    }
}

impl<'de> Deserialize<'de> for KeyCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let key = match &value {
            serde_json::Value::Number(code) => code.as_u64().and_then(|code| Self::new(u32::try_from(code).ok()?)),
            serde_json::Value::String(name) => Self::named(name),
            _ => None,
        };
        key.ok_or_else(|| serde::de::Error::custom(format!("not a key: {value}")))
    }
}

/// The keys for what WestPlay does from the game (owner 2026-10-21: every keybind customizable in the
/// companion): the playing app's volume up and down a step, and play or pause it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Keybinds {
    pub volume_up: Binding,
    pub volume_down: Binding,
    pub play_pause: Binding,
    /// Takes the phone out in the game, or puts it away (owner 2026-11-01).
    pub phone: Binding,
    /// Takes the phone out on its Personagem app (owner 2026-11-07).
    pub character: Binding,
    /// Hides the key hints bar over the game, or shows it again (owner 2026-11-07).
    pub hints: Binding,
    /// Emergency rough-road suspension trigger (owner 2026-11-22).
    pub rough_road: Binding,
}

impl<'de> Deserialize<'de> for Keybinds {
    /// Each binding read on its own, one that doesn't fit falling back to its own default (as `lenient`, but
    /// a binding has no default of its own: each keybind's is another key).
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let file = serde_json::Value::deserialize(deserializer)?;
        let defaults = Self::default();
        let binding = |name: &str, default: Binding| {
            file.get(name).and_then(|value| Binding::deserialize(value).ok()).unwrap_or(default)
        };
        Ok(Self {
            volume_up: binding("volumeUp", defaults.volume_up),
            volume_down: binding("volumeDown", defaults.volume_down),
            play_pause: binding("playPause", defaults.play_pause),
            phone: binding("phone", defaults.phone),
            character: binding("character", defaults.character),
            hints: binding("hints", defaults.hints),
            rough_road: binding("roughRoad", defaults.rough_road),
        })
    }
}

impl Default for Keybinds {
    /// The interaction key with 1, 2 and 3 (as promised to the players who asked: Alt+1, Alt+2, Alt+3), in a
    /// row as on a stereo (owner 2026-10-21): volume down, play/pause between, volume up; the phone next, with 4;
    /// then the phone's Personagem with 5, the key hints bar with 6 (owner 2026-11-07), and emergency rough-road
    /// suspension with 7 (owner 2026-11-22).
    fn default() -> Self {
        let with_key = |code| Binding { key: KeyCode(code), with_interaction_key: true };
        Self {
            volume_down: with_key(0x31),
            play_pause: with_key(0x32),
            volume_up: with_key(0x33),
            phone: with_key(0x34),
            character: with_key(0x35),
            hints: with_key(0x36),
            rough_road: with_key(0x37),
        }
    }
}

impl Keybinds {
    /// Each keybind as the bridge carries it ([`westplay_shared::keybind_value`]), in [`Keybind::ALL`]'s order.
    pub fn bridge_values(&self) -> [u32; KEYBIND_COUNT] {
        Keybind::ALL.map(|keybind| {
            let binding = match keybind {
                Keybind::VolumeUp => self.volume_up,
                Keybind::VolumeDown => self.volume_down,
                Keybind::PlayPause => self.play_pause,
                Keybind::Phone => self.phone,
                Keybind::Character => self.character,
                Keybind::Hints => self.hints,
                Keybind::RoughRoad => self.rough_road,
            };
            keybind_value(binding.key.virtual_key(), binding.with_interaction_key)
        })
    }
}

/// A key for one of WestPlay's actions, pressed alone or while holding the interaction key (while using the
/// screen, toggled).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub key: KeyCode,
    pub with_interaction_key: bool,
}

/// How WestPlay sits on the truck's screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenSettings {
    #[serde(deserialize_with = "lenient")]
    pub truck_strip: TruckStrip,
    /// The split screen (the companion's Truck Screen): the game's map squeezed into one half of its GPS screens,
    /// from the game's next start (WestPlay's mod is made again for it), or off, the game's own.
    #[serde(deserialize_with = "lenient")]
    pub split: westplay_setup::game_mod::Split,
    /// Render resolution tier (`gps_drawable_size`) for in-cabin GPS screens in `westplay.scs`.
    #[serde(deserialize_with = "lenient")]
    pub resolution: westplay_setup::game_mod::ScreenResolution,
    /// Keep the in-cabin screen powered on even when the truck's electrics/engine are off.
    #[serde(deserialize_with = "lenient")]
    pub keep_screen_on: bool,
}

/// The truck's own strip beside WestPlay (the Scania Smart Dash's, the DAF's status bar): kept where the
/// truck has one, or WestPlay takes the whole screen. The cabin page applies it (`oemMode` there), and its
/// Settings can change it (Appearance › Vehicle Bar), as the companion's Controls can.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TruckStrip {
    #[default]
    Auto,
    Off,
}

/// How fast the cursor moves: a multiplier on its whole speed curve. Always within [`Self::MIN`] and
/// [`Self::MAX`]: values from a file or the companion window are corrected on the way in.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "f32", into = "f32")]
pub struct Sensitivity(f32);

impl Sensitivity {
    pub const MIN: f32 = 0.5;
    pub const MAX: f32 = 3.0;

    /// Out-of-range values are clamped; anything that isn't a number is the default.
    pub fn new(value: f32) -> Self {
        if value.is_finite() { Self(value.clamp(Self::MIN, Self::MAX)) } else { Self::default() }
    }

    pub fn get(self) -> f32 {
        self.0
    }
}

impl From<f32> for Sensitivity {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<Sensitivity> for f32 {
    fn from(sensitivity: Sensitivity) -> Self {
        sensitivity.0
    }
}

impl Default for Sensitivity {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Deserializes a field, falling back to its default when the stored value doesn't fit (a typo, or a value
/// a newer version of WestPlay wrote), instead of failing the whole file.
fn lenient<'de, D: Deserializer<'de>, T: DeserializeOwned + Default>(deserializer: D) -> Result<T, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).unwrap_or_default())
}

/// The file's content: the version, then the settings.
#[derive(Serialize)]
struct FileContent<'a> {
    version: u32,
    #[serde(flatten)]
    settings: &'a Settings,
}

/// Brings settings written by an older version up to [`SETTINGS_VERSION`].
fn migrate(mut file: serde_json::Value, version: u32) -> serde_json::Value {
    // 2: the interaction key a virtual-key code (any key), not one of four names.
    if version < 2
        && let Some(key) = file.pointer_mut("/interaction/key")
        && let Some(named) = key.as_str().and_then(KeyCode::named)
    {
        *key = serde_json::Value::from(named.virtual_key());
    }
    file
}

/// Reads settings from the JSON of a settings file.
pub fn parse(json: &str) -> Result<Settings, serde_json::Error> {
    let file: serde_json::Value = serde_json::from_str(json)?;
    let version = file.get("version").and_then(serde_json::Value::as_u64).unwrap_or(0) as u32;
    serde_json::from_value(migrate(file, version))
}

pub fn to_json(settings: &Settings) -> String {
    let content = FileContent { version: SETTINGS_VERSION, settings };
    serde_json::to_string_pretty(&content).expect("settings always serialize")
}

/// The settings file on disk.
pub struct SettingsFile {
    path: PathBuf,
}

impl SettingsFile {
    pub fn in_dir(dir: &Path) -> Self {
        Self { path: dir.join("settings.json") }
    }

    /// The saved settings, or the defaults when there are none. A file that can't be read is set aside.
    pub fn load(&self) -> Settings {
        let json = match std::fs::read_to_string(&self.path) {
            Ok(json) => json,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Settings::default(),
            Err(e) => {
                log!("cannot read {}: {e}", self.path.display());
                return Settings::default();
            }
        };
        parse(&json).unwrap_or_else(|e| {
            let aside = self.path.with_extension("json.bad");
            log!("{} is not valid ({e}); moved to {}", self.path.display(), aside.display());
            let _ = std::fs::rename(&self.path, aside);
            Settings::default()
        })
    }

    /// Writes the settings; a crash midway leaves the previous file intact.
    pub fn save(&self, settings: &Settings) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let partial = self.path.with_extension("json.partial");
        std::fs::write(&partial, to_json(settings))?;
        std::fs::rename(partial, &self.path)
    }
}

/// The current settings, shared between the app (which changes them) and the host thread (which applies
/// them). The generation tells the host something changed.
#[derive(Default)]
pub struct SharedSettings {
    current: RwLock<Settings>,
    generation: AtomicU64,
}

impl SharedSettings {
    pub fn new(settings: Settings) -> Self {
        Self { current: RwLock::new(settings), generation: AtomicU64::new(1) }
    }

    pub fn get(&self) -> Settings {
        self.current.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn set(&self, settings: Settings) {
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = settings;
        self.generation.fetch_add(1, Ordering::AcqRel);
    }

    /// Changes whenever the settings do.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modo_personagem_is_on_at_first_and_its_mouse_kept_in_range() {
        let settings = parse(r#"{"version": 2}"#).unwrap();
        assert!(settings.character.enabled);
        let off = parse(r#"{"version": 2, "character": {"enabled": false}}"#).unwrap();
        assert!(!off.character.enabled);
        assert_eq!(settings.character.sensitivity.get(), 50);
        assert_eq!((settings.character.key.get(), settings.character.person.get()), (0x77, 0));
        let chosen = parse(r#"{"version": 2, "character": {"key": 117, "person": 1}}"#).unwrap();
        assert_eq!((chosen.character.key.get(), chosen.character.person.get()), (117, 1));
        let wrong = parse(r#"{"version": 2, "character": {"key": 999, "person": 6}}"#).unwrap();
        assert_eq!((wrong.character.key.get(), wrong.character.person.get()), (0x77, 0));
        let set = parse(r#"{"version": 2, "character": {"enabled": true, "sensitivity": 999}}"#).unwrap();
        assert!(set.character.enabled);
        assert_eq!(set.character.sensitivity.get(), 200);
        assert_eq!(
            parse(r#"{"version": 2, "character": {"sensitivity": "x"}}"#).unwrap().character.sensitivity.get(),
            50
        );
        let back = parse(&to_json(&set)).unwrap();
        assert_eq!(back.character, set.character);
    }

    #[test]
    fn keys_have_windows_names_and_mouse_buttons_none() {
        // The names are the keyboard layout's, so only that there's one.
        assert!(KeyCode::name(0x31).is_some_and(|name| !name.is_empty()));
        assert!(KeyCode::name(0xa3).is_some());
        assert_eq!(KeyCode::name(0x05), None);
    }

    #[test]
    fn round_trips_through_the_file_format() {
        let settings = Settings {
            interaction: InteractionSettings {
                key: KeyCode::new(0x05).unwrap(),
                mode: InteractionMode::Toggle,
                sensitivity: Sensitivity::new(1.6),
            },
            keybinds: Keybinds {
                volume_up: Binding { key: KeyCode::new(0x6b).unwrap(), with_interaction_key: false },
                ..Keybinds::default()
            },
            screen: ScreenSettings { truck_strip: TruckStrip::Off, ..Default::default() },
            look: LookSettings {
                wallpaper: Some("custom:Dusk".into()),
                icon_set: Some("Retro".into()),
                icon_style: IconStyle::Dark,
            },
            graphics: GraphicsSettings { gpu: Some("10de:2702:00000000:161".into()) },
            sound: SoundSettings {
                equalizer: crate::speakers::Equalizer { bass: -3, mid: 1, treble: 2 },
                custom: true,
            },
            character: CharacterSettings {
                enabled: true,
                sensitivity: CharacterSensitivity::new(70.0),
                key: CharacterKey::new(0x75 as f64),
                person: CharacterPerson::new(9.0),
                hints: Shown(false),
                hints_key: HintsKey::new(0x79 as f64),
                papers: Shown(false),
                refuel: Shown(true),
                pay_attendant: Shown(false),
            },
            ..Settings::default()
        };
        let json = to_json(&settings);
        assert!(json.contains("\"version\": 2"), "{json}");
        assert!(json.contains("\"key\": 5"), "{json}");
        assert!(json.contains("\"mode\": \"toggle\""), "{json}");
        assert!(json.contains("\"truckStrip\": \"off\""), "{json}");
        assert!(json.contains("\"payAttendant\": false"), "{json}");
        assert_eq!(parse(&json).unwrap(), settings);
    }

    #[test]
    fn missing_settings_take_their_defaults() {
        assert_eq!(parse(r#"{"version": 1}"#).unwrap(), Settings::default());
        // The key hints bar: shown, with no key, unless chosen.
        assert_eq!(Settings::default().character.hints, Shown(true));
        assert_eq!(Settings::default().character.hints_key.get(), 0);
        assert_eq!(HintsKey::new(300.0).get(), 0);
        let partial = parse(r#"{"version": 1, "interaction": {"sensitivity": 2.0}}"#).unwrap();
        assert_eq!(partial.interaction.key, KeyCode::LEFT_ALT);
        assert_eq!(partial.interaction.mode, InteractionMode::Hold);
        assert_eq!(partial.interaction.sensitivity.get(), 2.0);
        // From before the keybinds: the interaction key with 1 (down), 2 (play/pause) and 3 (up).
        assert_eq!(partial.keybinds, Keybinds::default());
        let with_key = |code| Binding { key: KeyCode::new(code).unwrap(), with_interaction_key: true };
        assert_eq!(
            [partial.keybinds.volume_down, partial.keybinds.play_pause, partial.keybinds.volume_up],
            [with_key(0x31), with_key(0x32), with_key(0x33)]
        );
        // Files from before the truck strip was a setting keep it on Auto.
        assert_eq!(partial.screen.truck_strip, TruckStrip::Auto);
        // And from before the GPU was: WestPlay picks it.
        assert_eq!(partial.graphics.gpu, None);
    }

    #[test]
    fn a_bad_value_falls_back_to_its_default_without_losing_the_others() {
        let settings = parse(r#"{"version": 1, "interaction": {"key": "capsLock", "sensitivity": 2.0}}"#).unwrap();
        assert_eq!(settings.interaction.key, KeyCode::LEFT_ALT);
        assert_eq!(settings.interaction.sensitivity.get(), 2.0);
        let settings = parse(r#"{"interaction": {"key": "rightCtrl", "sensitivity": "fast"}}"#).unwrap();
        assert_eq!(settings.interaction.key.virtual_key(), 0xa3);
        assert_eq!(settings.interaction.sensitivity, Sensitivity::default());
        let settings = parse(r#"{"screen": {"truckStrip": "sideways"}, "interaction": {"key": "rightCtrl"}}"#).unwrap();
        assert_eq!(settings.screen.truck_strip, TruckStrip::Auto);
        assert_eq!(settings.interaction.key.virtual_key(), 0xa3);
        // A key no keyboard has, and a binding missing its key: their defaults.
        let settings =
            parse(r#"{"version": 2, "interaction": {"key": 999}, "keybinds": {"volumeUp": {"key": 300}}}"#).unwrap();
        assert_eq!(settings.interaction.key, KeyCode::LEFT_ALT);
        assert_eq!(settings.keybinds.volume_up, Keybinds::default().volume_up);
        // Each broken binding its own default, never another keybind's key.
        let settings = parse(
            r#"{"version": 2, "keybinds": {"volumeDown": {"key": 300}, "playPause": "f9", "volumeUp": {"key": 107, "withInteractionKey": false}}}"#,
        )
        .unwrap();
        let defaults = Keybinds::default();
        assert_eq!(settings.keybinds.volume_down, defaults.volume_down);
        assert_eq!(settings.keybinds.play_pause, defaults.play_pause);
        assert_eq!(
            settings.keybinds.volume_up,
            Binding { key: KeyCode::new(107).unwrap(), with_interaction_key: false }
        );
        // Not an object at all: every default.
        assert_eq!(parse(r#"{"version": 2, "keybinds": 4}"#).unwrap().keybinds, defaults);
    }

    #[test]
    fn the_first_versions_keys_by_name_become_their_codes() {
        for (name, code) in [("leftAlt", 0xa4), ("rightCtrl", 0xa3), ("mouseBack", 0x05), ("mouseForward", 0x06)] {
            let json = format!(r#"{{"version": 1, "interaction": {{"key": "{name}"}}}}"#);
            assert_eq!(parse(&json).unwrap().interaction.key.virtual_key(), code, "{name}");
        }
        // Any key now, by its code.
        let settings = parse(r#"{"version": 2, "interaction": {"key": 192, "mode": "toggle"}}"#).unwrap();
        assert_eq!(settings.interaction.key.virtual_key(), 192);
        assert_eq!(settings.interaction.mode, InteractionMode::Toggle);
    }

    #[test]
    fn sensitivity_stays_in_range() {
        assert_eq!(Sensitivity::new(10.0).get(), Sensitivity::MAX);
        assert_eq!(Sensitivity::new(0.0).get(), Sensitivity::MIN);
        assert_eq!(Sensitivity::new(f32::NAN), Sensitivity::default());
        let stored = parse(r#"{"interaction": {"sensitivity": 99}}"#).unwrap();
        assert_eq!(stored.interaction.sensitivity.get(), Sensitivity::MAX);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        assert_eq!(parse(r#"{"version": 9, "futureThing": [1, 2]}"#).unwrap(), Settings::default());
    }

    #[test]
    fn an_unreadable_file_is_set_aside() {
        let dir = std::env::temp_dir().join(format!("westplay-settings-test-{}", std::process::id()));
        let file = SettingsFile::in_dir(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), "{ not json").unwrap();
        assert_eq!(file.load(), Settings::default());
        assert!(dir.join("settings.json.bad").exists());

        let settings = Settings {
            interaction: InteractionSettings {
                key: KeyCode::new(0xa3).unwrap(),
                sensitivity: Sensitivity::new(2.0),
                ..InteractionSettings::default()
            },
            ..Settings::default()
        };
        file.save(&settings).unwrap();
        assert_eq!(file.load(), settings);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn shared_settings_count_changes() {
        let shared = SharedSettings::new(Settings::default());
        let before = shared.generation();
        let mut changed = Settings::default();
        changed.interaction.key = KeyCode::new(0x06).unwrap();
        shared.set(changed.clone());
        assert_ne!(shared.generation(), before);
        assert_eq!(shared.get(), changed);
    }
}

#[cfg(test)]
mod people_off_tests {
    use super::*;

    #[test]
    fn the_stout_man_is_off_for_now() {
        assert_eq!(CharacterPerson::new(3.0).get(), 3);
        assert_eq!(CharacterPerson::new(4.0), CharacterPerson::default());
        assert_eq!(CursorSize::from_word("larger").map(CursorSize::factor), Some(1.7));
    }
}
