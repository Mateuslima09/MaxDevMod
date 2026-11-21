//! Refuelling by hand (owner 2026-11-11): at a pump, the game's own "press Enter to refuel" no longer fills the tank
//! from the cab. The player gets out, walks to the truck's tank and holds Enter (or E): the tank fills while it's
//! held, as much as they want, until they let go or it's full (owner 2026-11-20: "not press and it fills up; hold
//! Enter, put in what you want").
//!
//! What it stands on, all of it the game's own: the SDK's telemetry (the fuel in the tank, the tank's size), the
//! player's `activate` key kept from the game (as `freight` keeps it), and the game's `activate` given through the
//! plugin's input device (`commands`), which is what fills the tank.
//!
//! Without a key pressed (owner 2026-11-11: "the game says turn the engine off to refuel as I get to the icon"):
//! what the game says in its corner is read (`notice::Notices::texts`), and its word of fuel there, with the truck
//! standing, is the pump's green icon: the tank's side offers E. It only offers (the player's own key isn't kept
//! from the game on that word alone: a low-fuel warning says fuel too); the tank filling confirms the pump. What
//! the game said is put in the log, to tell its words in every language.
//!
//! Where the pumps are, the app says (`HostBlock::at_pump`: the played map's pumps, each where the game shows its
//! green icon and fills the tank). At one, the player's `activate` key is kept from the game from the first press,
//! and the tank's side offers E. Where the app doesn't know a pump (a map built before pumps were kept, a pump
//! the map doesn't have), it's known by what the game does there: the player presses `activate` in the cab and the
//! fuel rises; from that moment the key is kept from the game (a fraction of a litre went in). Away from a pump
//! `activate` is never given to the game (it sleeps, it takes jobs).

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VK_TO_VSC, MapVirtualKeyW, VIRTUAL_KEY};

use super::freight::{apart, in_vehicle};
use super::{Placement, State, notice, really_down};
use crate::hints::Action;
use crate::log;
use crate::telemetry::Truck;

/// The fuel rising between two frames (litres): more than the channel's noise, less than a tank changed whole (a
/// game loaded, another truck).
const RISE: std::ops::Range<f32> = 0.005..40.0;
/// The truck standing (m/s).
const STANDING: f32 = 0.5;
/// How far the truck may move from where the pump filled it before the pump is forgotten (metres).
const PUMP_LEFT: f64 = 5.0;
/// The tank taken as full (of its size).
const FULL: f32 = 0.985;
/// The player's `activate` key counts as theirs for this long after it's let go (the fuel's channel lags a frame).
const KEY_LATELY: Duration = Duration::from_millis(400);
/// Filling: given up if no fuel came in this long; done once none has come for this long.
const NO_FUEL_AFTER: Duration = Duration::from_millis(3000);
const DONE_AFTER: Duration = Duration::from_millis(1200);
/// The same notice again no sooner than this.
const NOTICE_AGAIN: Duration = Duration::from_millis(2500);
/// Beside the tank, in the truck's own space (metres): how far out to either side, and from how far behind the
/// driver's head; it ends where the fifth wheel's place begins (`freight::by_fifth_wheel`: owner 2026-11-11, "not
/// where the tank is", the two apart so neither is taken for the other).
const TANK_OUT: std::ops::RangeInclusive<f64> = 0.8..=2.1;
/// (From a metre ahead of the driver's eyes: owner 2026-11-11, standing by the cab's step at the pump, E wasn't
/// offered; some trucks have the tank under the door.)
const TANK_FROM_HEAD: f64 = -1.0;
/// The game's notices are read this often (frames), and not for this long after one of the pump's own (they say
/// fuel too).
const NOTICES_EVERY: u32 = 20;
const OWN_NOTICE_FOR: Duration = Duration::from_secs(8);
/// The game's notices put in the log, at most (each once).
const NOTICES_LOGGED: usize = 80;
/// Fuel, as the game's notices say it at a pump, in its languages (lower case, a part of the word).
/// (Not "fuel" alone: the game's low-fuel warning isn't a pump.)
const FUEL_WORDS: [&str; 13] = [
    "abastec",
    "refuel",
    "repost",
    "tanken",
    "betank",
    "carburant",
    "plein",
    "riforn",
    "tankow",
    "zatank",
    "заправ",
    "yakıt",
    "natank",
];

/// Whether a notice of the game's says fuel (`FUEL_WORDS`).
fn says_fuel(text: &str) -> bool {
    let text = text.to_lowercase();
    // The game's notices come as its own keys (owner's log 2026-11-11: "@@trailer_decoupling@@", "@@attach_load@@"),
    // English whatever the language: one of fuel, or of a gas station, that isn't the low-fuel warning; or a
    // notice's picture of one.
    let key = text.contains("@@") || text.starts_with("/material/");
    // (Its pump's panel keys the fuel's price and amount, no "fuel" in it: `@@diesel_price@@`, `@@litres@@`, owner's
    // log 2026-11-15; it stayed on the screen.)
    let priced = text.contains("_price@@") || text.contains("@@litres@@") || text.contains("@@gallons@@");
    let keyed = key
        && (text.contains("fuel") || text.contains("gas_st") || text.contains("gasst") || priced)
        && !text.contains("low");
    keyed || FUEL_WORDS.iter().any(|word| text.contains(word))
}

/// The game's panel at a pump ("press and hold Enter to fill the tank", its price, litres and total): one of its
/// notices, its keys, saying fuel, long; not its "tank full" nor "stop the engine" (those still hold).
pub(super) fn pump_panel(text: &str) -> bool {
    // Its own keys, whatever else it says (owner 2026-11-20: "the notice up there to press Enter still shows").
    let keyed = text.contains("@@refuel_hold@@") || text.contains("@@gas_station_hint@@");
    text.contains("@@")
        && (keyed || (says_fuel(text) && (text.len() > 60 || text.contains(">Enter<"))))
        && !text.contains("tank_full")
        && !text.contains("stop_engine")
}

/// What the pump's panel says instead, filling by hand (its Enter fills no more).
fn panel_words(language: &str) -> &'static str {
    match language {
        "pt" => "Abasteça a pé: desça e segure Enter ao lado do tanque",
        "es" => "Reposta a pie: baja y mantén Enter junto al depósito",
        _ => "Refuel on foot: get out and hold Enter by the tank",
    }
}

/// The key that fills, on foot: E (as `freight`'s).
const USE_KEY: (u16, u32) = (b'E' as u16, 0x12);
const LITRES_A_GALLON: f32 = 3.785_41;
/// The meter's last reading stays this long after filling.
const METER_AFTER: Duration = Duration::from_secs(6);

/// A notice of the pump's.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Say {
    /// The `activate` key in the cab, at the pump.
    GetOut,
    /// By the tank at a pump: hold Enter (or E) to fill.
    HoldToFill,
    /// E by the tank, the engine running.
    EngineOff,
    /// E by the tank, and no fuel came.
    NoFuel,
    /// E by the tank, the tank already full.
    AlreadyFull,
    Filling,
    /// Stopped by the player: how much went in (litres).
    Filled(f32),
    /// The tank full: how much went in (litres).
    Full(f32),
    /// The fuel filled is owed: to the attendant (the total, the pump's money).
    PayAttendant(f32),
    /// Back in the cab, the fuel still owed.
    StillOwed(f32),
    /// The bill paid (the pump's money).
    Paid(f32),
    /// The truck gone from the pump, the bill unpaid.
    LeftUnpaid,
}

impl Say {
    fn icon(self) -> i32 {
        match self {
            Say::Filling | Say::Filled(_) | Say::Full(_) | Say::Paid(_) | Say::HoldToFill => notice::ANNOUNCEMENT,
            _ => notice::ALERT,
        }
    }

    /// In the game's language (two letters), English if not one of these; the fuel in gallons in the American
    /// game.
    fn text(self, language: &str, gallons: bool) -> CString {
        let amount = |litres: f32| {
            let (amount, unit) = match (gallons, language) {
                (true, "pt") => (litres / LITRES_A_GALLON, "gal\u{f5}es"),
                (true, "es") => (litres / LITRES_A_GALLON, "galones"),
                (true, _) => (litres / LITRES_A_GALLON, "gallons"),
                (false, "pt" | "es") => (litres, "litros"),
                (false, _) => (litres, "litres"),
            };
            format!("{} {unit}", amount.round().max(0.0) as u32)
        };
        let money = |total: f32| {
            let sign = crate::fuel_meter::money_sign(language, gallons);
            let number = format!("{total:.2}");
            let number = if matches!(language, "pt" | "es") { number.replace('.', ",") } else { number };
            format!("{sign} {number}")
        };
        let text = match (language, self) {
            ("pt", Say::PayAttendant(total)) => {
                format!("Pague {} ao frentista: ele está na frente do caminhão, aperte E perto dele", money(total))
            }
            ("es", Say::PayAttendant(total)) => {
                format!("Paga {} al empleado: está delante del camión, pulsa E junto a él", money(total))
            }
            (_, Say::PayAttendant(total)) => {
                format!("Pay {} to the attendant: they're in front of the truck, press E by them", money(total))
            }
            ("pt", Say::StillOwed(total)) => format!("Você ainda não pagou o frentista: {}", money(total)),
            ("es", Say::StillOwed(total)) => format!("Aún no pagaste al empleado: {}", money(total)),
            (_, Say::StillOwed(total)) => format!("You haven't paid the attendant yet: {}", money(total)),
            ("pt", Say::HoldToFill) => "Segure Enter (ou E) para abastecer; solte para parar".to_owned(),
            ("es", Say::HoldToFill) => "Mantén Enter (o E) para repostar; suelta para parar".to_owned(),
            (_, Say::HoldToFill) => "Hold Enter (or E) to refuel; let go to stop".to_owned(),
            ("pt", Say::Paid(total)) => format!("Pago: {}", money(total)),
            ("es", Say::Paid(total)) => format!("Pagado: {}", money(total)),
            (_, Say::Paid(total)) => format!("Paid: {}", money(total)),
            ("pt", Say::LeftUnpaid) => "Você saiu sem pagar o abastecimento!".to_owned(),
            ("es", Say::LeftUnpaid) => "¡Te fuiste sin pagar el combustible!".to_owned(),
            (_, Say::LeftUnpaid) => "You left without paying for the fuel!".to_owned(),
            ("pt", Say::GetOut) => "Des\u{e7}a do caminh\u{e3}o e abaste\u{e7}a pelo tanque".to_owned(),
            ("pt", Say::EngineOff) => "Desligue o motor para abastecer".to_owned(),
            ("es", Say::EngineOff) => "Apaga el motor para repostar".to_owned(),
            (_, Say::EngineOff) => "Turn the engine off to refuel".to_owned(),
            ("pt", Say::AlreadyFull) => "O tanque já está cheio".to_owned(),
            ("es", Say::AlreadyFull) => "El depósito ya está lleno".to_owned(),
            (_, Say::AlreadyFull) => "The tank is already full".to_owned(),
            ("pt", Say::NoFuel) => "A bomba n\u{e3}o abasteceu: pare o caminh\u{e3}o no lugar de abastecer".to_owned(),
            ("pt", Say::Filling) => "Abastecendo: solte para parar".to_owned(),
            ("pt", Say::Filled(litres)) => format!("Abastecido: {}", amount(litres)),
            ("pt", Say::Full(litres)) => format!("Tanque cheio: {}", amount(litres)),
            ("es", Say::GetOut) => "Baja del cami\u{f3}n y reposta por el dep\u{f3}sito".to_owned(),
            ("es", Say::NoFuel) => {
                "El surtidor no repost\u{f3}: para el cami\u{f3}n en el lugar de repostar".to_owned()
            }
            ("es", Say::Filling) => "Repostando: suelta para parar".to_owned(),
            ("es", Say::Filled(litres)) => format!("Repostado: {}", amount(litres)),
            ("es", Say::Full(litres)) => format!("Dep\u{f3}sito lleno: {}", amount(litres)),
            (_, Say::GetOut) => "Get out and refuel at the truck's tank".to_owned(),
            (_, Say::NoFuel) => "The pump gave no fuel: stop the truck where it refuels".to_owned(),
            (_, Say::Filling) => "Refuelling: let go to stop".to_owned(),
            (_, Say::Filled(litres)) => format!("Refuelled: {}", amount(litres)),
            (_, Say::Full(litres)) => format!("Tank full: {}", amount(litres)),
        };
        CString::new(text).unwrap_or_default()
    }
}

/// Whether a rise of the fuel between two frames is a pump's (`RISE`).
fn pumped(before: f32, now: f32) -> bool {
    before > 0.0 && RISE.contains(&(now - before))
}

/// Whether the tank is full (its size unknown: never, the pump says when).
fn full(fuel: f32, capacity: f32) -> bool {
    capacity > 0.0 && fuel >= capacity * FULL
}

/// Whether the player (a place in the world) stands beside the truck's tank: either side, behind the cab's doors.
fn by_tank(player: [f64; 3], truck: &Truck) -> bool {
    let [x, z] = in_vehicle(truck.position, truck.heading, player);
    let from = f64::from(truck.head[2]) + TANK_FROM_HEAD;
    let to = super::freight::hook_back(truck) - super::freight::BEFORE_HOOK;
    TANK_OUT.contains(&x.abs()) && z >= from && z < to
}

/// The tank being filled.
struct Filling {
    since: Instant,
    /// The fuel as it started (litres).
    from: f32,
    /// When fuel last came in.
    rose: Option<Instant>,
}

pub(super) struct Refuel {
    /// The player's `activate` keys (virtual keys).
    keys: Vec<u32>,
    /// Where the truck stood as the game's pump filled it.
    pump: Option<[f64; 3]>,
    last_fuel: f32,
    /// When the player's `activate` key was last down.
    key_down: Option<Instant>,
    filling: Option<Filling>,
    /// What E does now, on foot (the key hints bar's).
    offer: Option<Action>,
    was_use: AtomicBool,
    was_activate: AtomicBool,
    said: Option<(Say, Instant)>,
    /// The game's notices: frames since they were last read, whether one said fuel then, and those already in
    /// the log.
    notices_in: u32,
    game_says_fuel: bool,
    logged: Vec<String>,
    /// Whether the player stood by the tank last frame (for the log).
    was_by_tank: bool,
    /// Whether the pump's panel was reworded yet (for the log).
    panel_reworded: bool,
    /// The meter's last reading after filling, held until then (`fuel_meter`).
    meter_until: Option<(Instant, f32)>,
    /// The fuel filled, still to pay at the pump's attendant (owner 2026-11-12: "go to the attendant and pay, on
    /// the phone").
    bill: Option<Bill>,
    /// The last bill's number.
    bills: u32,
    /// The attendant walking away once paid (owner 2026-11-12: "they just vanish; a little walk away"): where,
    /// which way, since when.
    leaving: Option<(super::person::Person, [f64; 3], f32, Instant, Instant)>,
    /// When to say the bill is to pay at the attendant: a moment after the filling ends, the game's own "tank
    /// full" said first (owner 2026-11-12: the notice to pay wasn't seen).
    say_pay_at: Option<Instant>,
    /// Whether the player was on foot last frame (back in the cab with the fuel owed: said so).
    was_on_foot: bool,
}

/// The fuel filled by hand, to pay: how much (litres), the pump's price for one of the game's unit and the total
/// (in the money its panel showed), where the truck stood, and the attendant who takes it.
struct Bill {
    id: u32,
    litres: f32,
    price: f32,
    total: f32,
    truck_at: [f64; 3],
    attendant: Option<super::person::Person>,
    attendant_at: [f64; 3],
    attendant_yaw: f32,
    /// Shown on the phone to pay now (`PluginBlock::fuel_bill`).
    asked: bool,
    last: Instant,
}

/// E pays within this of the attendant (metres along the ground; they stand in front of the truck, as the cargo's
/// clerk does: `freight::FRONT_AHEAD`). With no attendant (the game's people not found), E pays within this of where
/// the truck filled.
const PAY_WITHIN: f64 = 1.9;
const PAY_WITHOUT_ATTENDANT: f64 = 8.0;
/// The truck this far from where it filled, the bill unpaid: left without paying (metres).
const BILL_LEFT: f64 = 25.0;
/// The least filled that's a bill (litres).
const BILL_LEAST: f32 = 0.5;
/// The notice to pay, this long after the filling ends.
const SAY_PAY_AFTER: Duration = Duration::from_millis(3500);
/// And again this often while it's owed, the player not by the attendant.
const SAY_PAY_AGAIN: Duration = Duration::from_secs(30);
/// The pump's attendant: one of the game's workers in WestPlay's mod (`westplay_setup::clerk`, its service man
/// first), else the game's own worker or man standing.
const ATTENDANT: [&str; 3] = ["wp_clk3_idle", "work_idle", "m_idle01"];
/// And walking away (the clerk's walk, or the game's man's).
const ATTENDANT_WALKS: [&str; 2] = ["wp_clk3_walk", "n_ca_man_wlk"];

/// The bill as the phone shows it (`PluginBlock::fuel_bill`), or none.
fn ask_bill(bill: Option<&Bill>, gallons: bool) {
    let Some(bridge) = crate::bridge::get() else { return };
    let shown = bill.map_or_else(westplay_shared::FuelBill::default, |bill| westplay_shared::FuelBill {
        id: bill.id,
        gallons: u32::from(gallons),
        amount: if gallons { bill.litres / LITRES_A_GALLON } else { bill.litres },
        price: bill.price,
        total: bill.total,
        sign: westplay_shared::FixedStr::new(crate::fuel_meter::money_sign(notice::language(), gallons)),
    });
    bridge.plugin.fuel_bill.write(&shown);
}

/// What's owed now, for the app (`PluginBlock::fuel_owed`): written only when it changes.
fn owe(bill: Option<&Bill>, gallons: bool) {
    let Some(bridge) = crate::bridge::get() else { return };
    let owed = bill.map_or_else(westplay_shared::FuelBill::default, |bill| westplay_shared::FuelBill {
        id: bill.id,
        gallons: u32::from(gallons),
        amount: if gallons { bill.litres / LITRES_A_GALLON } else { bill.litres },
        price: bill.price,
        total: bill.total,
        sign: westplay_shared::FixedStr::new(crate::fuel_meter::money_sign(notice::language(), gallons)),
    });
    if bridge.plugin.fuel_owed.read() != Some(owed) {
        bridge.plugin.fuel_owed.write(&owed);
    }
}

/// The bill let go (paid, left unpaid, or the plugin done with it): the attendant gone, the phone told.
fn close_bill(state: &mut State) {
    // One walking away, gone too, when the plugin's done with the pump (not once paid: it's just begun).
    close_bill_only(state);
}

/// [`close_bill`], and the attendant walking away gone too (the plugin done with the pump).
fn let_go(state: &mut State) {
    close_bill_only(state);
    if let Some((walker, ..)) = state.refuel.as_mut().and_then(|refuel| refuel.leaving.take())
        && let Some(Some(game)) = &state.game
        && let Some(people) = game.people.as_ref()
    {
        // SAFETY: on the game's main thread, the walker as `show_other` made them.
        unsafe { super::person::hide_other(people, walker) };
    }
}

fn close_bill_only(state: &mut State) {
    let Some(bill) = state.refuel.as_mut().and_then(|refuel| refuel.bill.take()) else { return };
    owe(None, false);
    if bill.asked {
        ask_bill(None, false);
    }
    if let Some(attendant) = bill.attendant
        && let Some(Some(game)) = &state.game
        && let Some(people) = game.people.as_ref()
    {
        // SAFETY: on the game's main thread, the attendant as `show_other` made them.
        unsafe { super::person::hide_other(people, attendant) };
    }
}

/// The player's `activate` keys kept from the game now (0: none), and E while on foot (read by the keyboard hooks,
/// any thread).
static KEPT: [AtomicU32; 2] = [AtomicU32::new(0), AtomicU32::new(0)];
static USE_KEPT: AtomicBool = AtomicBool::new(false);

fn keep(keys: &[u32], kept: bool) {
    for (i, slot) in KEPT.iter().enumerate() {
        slot.store(if kept { keys.get(i).copied().unwrap_or(0) } else { 0 }, Ordering::Release);
    }
}

/// Whether the game is kept from `key` (a virtual key) by the pump.
pub(super) fn takes(key: u32) -> bool {
    key != 0
        && (KEPT.iter().any(|kept| kept.load(Ordering::Acquire) == key)
            || (USE_KEPT.load(Ordering::Acquire) && key == u32::from(super::virtual_key(USE_KEY))))
}

/// The same for DirectInput's keyboard (a scan code).
pub(super) fn takes_scancode(scancode: u32) -> bool {
    if scancode == 0 {
        return false;
    }
    (USE_KEPT.load(Ordering::Acquire) && scancode == USE_KEY.1)
        || KEPT.iter().any(|kept| {
            let key = kept.load(Ordering::Acquire);
            // SAFETY: a plain query, no pointers.
            key != 0 && unsafe { MapVirtualKeyW(key, MAPVK_VK_TO_VSC) } == scancode
        })
}

/// Whether the tank can be filled where the truck stands, as the last frame saw: on a pump's green icon (the
/// app's word, or the game's own pump having filled it at the player's key), the tank not full.
static CAN_FILL: AtomicBool = AtomicBool::new(false);

/// Whether E at `player`'s place fills the tank now: beside it, at a pump. Only there; anywhere else that place
/// is the trailer's (`freight` couples and uncouples there).
pub(super) fn fills(player: [f64; 3], truck: &Truck) -> bool {
    CAN_FILL.load(Ordering::Acquire) && by_tank(player, truck)
}

/// While the tank is filled on foot: the heading toward the truck from where the player stands (radians, as
/// `super::heading_of`), and how far ahead of their eyes the nozzle's grip goes (metres); NaN when not filling.
static NOZZLE_YAW: AtomicU32 = AtomicU32::new(0x7fc0_0000);
static NOZZLE_AHEAD: AtomicU32 = AtomicU32::new(0);
/// The tank's side, out from the truck's middle (metres): the nozzle's grip held this far out, its spout in the
/// tank (owner 2026-11-12: "not the tank's mouth itself, more or less that way").
const GRIP_OUT: f64 = 1.6;
/// How far ahead of the eyes the grip may go (metres): the arm's reach.
const GRIP_AHEAD: std::ops::RangeInclusive<f64> = 0.32..=0.6;

/// While the tank is filled on foot: where the nozzle goes (`NOZZLE_YAW`), for the first-person view
/// (`super::nozzle_frame`).
pub(super) fn nozzle() -> Option<(f32, f64)> {
    let yaw = f32::from_bits(NOZZLE_YAW.load(Ordering::Acquire));
    (!yaw.is_nan()).then(|| (yaw, f64::from(f32::from_bits(NOZZLE_AHEAD.load(Ordering::Acquire)))))
}

fn set_nozzle(at: Option<(f32, f64)>) {
    let (yaw, ahead) = at.unwrap_or((f32::NAN, 0.0));
    NOZZLE_AHEAD.store((ahead as f32).to_bits(), Ordering::Release);
    NOZZLE_YAW.store(yaw.to_bits(), Ordering::Release);
}

/// Toward the truck from `player` (a place in the world), square to it (the way to its tank), and how far ahead
/// of them the nozzle's grip goes (`GRIP_OUT` from the truck's middle, within reach).
fn toward_tank(player: [f64; 3], truck: &Truck) -> (f32, f64) {
    let [side, _] = in_vehicle(truck.position, truck.heading, player);
    // The truck's own side in the world (`in_vehicle`'s first axis), toward its middle.
    let turn = f64::from(truck.heading) * std::f64::consts::TAU;
    let inward = -side.signum();
    let yaw = super::heading_of(turn.cos() * inward, -turn.sin() * inward);
    (yaw, (side.abs() - GRIP_OUT).clamp(*GRIP_AHEAD.start(), *GRIP_AHEAD.end()))
}

/// What E does now on foot, for the key hints bar.
pub(super) fn offer(state: &State) -> Option<Action> {
    state.refuel.as_ref().and_then(|refuel| refuel.offer)
}

fn on() -> bool {
    super::enabled()
        && crate::bridge::get().is_some_and(|bridge| bridge.host.manual_refuel.load(Ordering::Acquire) == 1)
        && !crate::truckersmp::loaded()
}

/// Lets everything of the pump go: the keys back to the game, the game's `activate` let go.
fn rest(state: &mut State) {
    keep(&[], false);
    USE_KEPT.store(false, Ordering::Release);
    CAN_FILL.store(false, Ordering::Release);
    notice::hide_pump_panel(false);
    crate::commands::hold(crate::commands::ACTIVATE, false);
    set_nozzle(None);
    crate::fuel_meter::set(None);
    if let Some(refuel) = state.refuel.as_mut() {
        if refuel.filling.take().is_some() {
            super::HELD_STILL.store(false, Ordering::Release);
        }
        refuel.offer = None;
    }
}

/// When the plugin is unloaded.
pub(super) fn shutdown(state: &mut State) {
    let_go(state);
    rest(state);
}

/// Once a frame, at its end, after `freight`'s: the pump known by what the game does, the `activate` key kept from
/// it there, and the tank filled on foot.
///
/// # Safety
/// On the game's main thread, at the end of a frame.
pub(super) unsafe fn frame(state: &mut State) {
    if !on() {
        if state.refuel.is_some() {
            let_go(state);
            rest(state);
            state.refuel = None;
        }
        return;
    }
    let truck = crate::telemetry::truck();
    let fuel = crate::telemetry::fuel();
    if truck.multiplayer || crate::telemetry::paused() {
        // The game's menu: nothing kept from it, the pump let go.
        rest(state);
        if let Some(refuel) = state.refuel.as_mut() {
            refuel.last_fuel = fuel;
        }
        return;
    }
    // The cargo's papers come first by the truck (the fifth wheel is beside the tank).
    let freights = super::freight::offer(state).is_some();
    let on_foot = state.walking.as_ref().filter(|walking| walking.climb.is_none() && walking.descend.is_none());
    // SAFETY: the walking camera is held (one reference); read guarded.
    let player =
        on_foot.and_then(|walking| unsafe { super::read::<Placement>(walking.camera as usize + super::PLACEMENT) });
    let player = player.map(|placement| placement.world());
    let refuel = state.refuel.get_or_insert_with(|| Refuel {
        keys: super::freight::Keys::read().activate,
        pump: None,
        last_fuel: fuel,
        key_down: None,
        filling: None,
        offer: None,
        was_use: AtomicBool::new(false),
        was_activate: AtomicBool::new(false),
        said: None,
        notices_in: 0,
        game_says_fuel: false,
        logged: Vec::new(),
        was_by_tank: false,
        panel_reworded: false,
        meter_until: None,
        bill: None,
        bills: 0,
        leaving: None,
        say_pay_at: None,
        was_on_foot: false,
    });
    // The game's panel at a pump kept from the screen while filling by hand is on (owner 2026-11-12: "the
    // refuelling message is still there"): not where the plugin's device isn't, the game's Enter then the only way.
    notice::hide_pump_panel(crate::commands::registered());
    let capacity = crate::telemetry::fuel_capacity();
    let rising = pumped(refuel.last_fuel, fuel);
    refuel.last_fuel = fuel;
    let is_full = full(fuel, capacity);
    let standing = truck.speed.abs() < STANDING;
    let shown = crate::d3d::game_shown();
    let activate_down = shown && refuel.keys.iter().any(|&key| really_down(VIRTUAL_KEY(key as u16)));
    if activate_down {
        refuel.key_down = Some(Instant::now());
    }
    let activate_pressed = crate::input::newly_down(&refuel.was_activate, activate_down);
    let use_down = shown && really_down(VIRTUAL_KEY(super::virtual_key(USE_KEY)));
    let use_pressed = crate::input::newly_down(&refuel.was_use, use_down) && on_foot.is_some();
    // Filling by hand: held, Enter (the player's `activate`, kept from the game at the pump) or E, as long as they
    // want (owner 2026-11-20).
    let hold_down = on_foot.is_some() && (use_down || activate_down);
    let hold_pressed = use_pressed || (activate_pressed && on_foot.is_some());
    let mut to_say = None;

    // The pump: known once the player's own key fills the tank in the cab; forgotten as the truck leaves it.
    if refuel.pump.is_some_and(|pump| apart(pump, truck.position) > PUMP_LEFT) {
        log!("refuel: the pump left behind");
        refuel.pump = None;
    }
    let by_key = refuel.key_down.is_some_and(|at| at.elapsed() < KEY_LATELY);
    if refuel.pump.is_none() && refuel.filling.is_none() && rising && by_key && standing {
        log!("refuel: a pump here (the game filled the tank at the player's key)");
        refuel.pump = Some(truck.position);
        to_say = Some(Say::GetOut);
    }
    // The app's word: the truck on a pump's green icon (1), on none (2), or the map's pumps not known (0).
    let told = crate::bridge::get().map_or(0, |bridge| bridge.host.at_pump.load(Ordering::Acquire));
    // The game's own word, where the app has none: what it says in its corner, the truck standing (not while the
    // pump's own notices are up).
    refuel.notices_in += 1;
    // (Heard even where the app says no pump: the game's own word is the surer one, owner 2026-11-11.)
    if !standing {
        refuel.game_says_fuel = false;
    } else if refuel.notices_in >= NOTICES_EVERY {
        refuel.notices_in = 0;
        // SAFETY: the code is the game's, read as it lies.
        state.game.get_or_insert_with(|| unsafe { super::locate() });
        let notices = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.notices.as_ref());
        // SAFETY: on the game's main thread (`frame`'s contract).
        let texts = notices.map(|notices| unsafe { notices.texts() }).unwrap_or_default();
        let own_up = refuel.said.is_some_and(|(_, at)| at.elapsed() < OWN_NOTICE_FOR) || refuel.filling.is_some();
        for text in &texts {
            if refuel.logged.len() < NOTICES_LOGGED && !refuel.logged.contains(text) {
                log!("refuel: the game says {text:?}{}", if says_fuel(text) { " (fuel)" } else { "" });
                refuel.logged.push(text.clone());
            }
        }
        // (While the pump's own notices are up, as it was: they'd be heard as the game's.)
        let says = if own_up { refuel.game_says_fuel } else { texts.iter().any(|text| says_fuel(text)) };
        if says != refuel.game_says_fuel {
            log!("refuel: the game {} of fuel here", if says { "speaks" } else { "no longer speaks" });
            refuel.game_says_fuel = says;
        }
    }
    // The game's big panel at the pump ("press and hold Enter"), its words the plugin's while filling by hand is
    // on and not filling (owner 2026-11-11: "can that one go too?"); each frame, the game writing it again.
    if standing && refuel.filling.is_none() {
        // SAFETY: the code is the game's, read as it lies.
        state.game.get_or_insert_with(|| unsafe { super::locate() });
        if let Some(notices) = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.notices.as_ref())
        {
            // SAFETY: on the game's main thread (`frame`'s contract).
            let reworded = unsafe { notices.reword(pump_panel, panel_words(notice::language())) };
            if reworded > 0 && !refuel.panel_reworded {
                log!("refuel: the pump's panel reworded ({reworded})");
                refuel.panel_reworded = true;
            }
        }
    }
    // Only on the pump's green icon (owner 2026-11-11): where the app knows the map's pumps, at one of them; where
    // it doesn't, where the game's own pump filled the tank at the player's key. Not without the plugin's device:
    // the player's key is then the only way to fill it.
    // The game having filled it here is the surest word of all, whatever the app says.
    let at_pump = crate::commands::registered() && ((told == 1 && standing) || refuel.pump.is_some());
    let blocked = at_pump && !is_full;
    // Offered at the tank at a pump, or where the game speaks of fuel (its key not kept from it on that alone).
    // (Full too: offered all the same, E then says so; owner 2026-11-11, the option missing at a pump with the
    // tank full looked broken.)
    let offered = crate::commands::registered() && standing && (at_pump || refuel.game_says_fuel);
    CAN_FILL.store(offered, Ordering::Release);
    keep(&refuel.keys.clone(), blocked);
    USE_KEPT.store(on_foot.is_some(), Ordering::Release);
    if activate_pressed && blocked && refuel.filling.is_none() && on_foot.is_none() && to_say.is_none() {
        to_say = Some(Say::GetOut);
    }

    // On foot, beside the tank: E fills, E again stops.
    refuel.offer = None;
    let at_tank = !freights && standing && player.is_some_and(|player| by_tank(player, &truck));
    // Coming by the tank or leaving it, at a pump: in the log, with the fuel (to tell why E is offered or not).
    let by_it = player.is_some_and(|player| by_tank(player, &truck));
    // Come by the tank at a pump, the tank not full: how to fill, in the game's corner (the game's own panel hidden).
    if by_it && !refuel.was_by_tank && offered && !is_full && refuel.filling.is_none() && !freights {
        to_say = Some(Say::HoldToFill);
    }
    if by_it != refuel.was_by_tank && (offered || at_pump || refuel.game_says_fuel) {
        log!(
            "refuel: by the tank {by_it} (offered {offered}, papers first {freights}, full {is_full}: {fuel:.1} of {capacity:.0} l, engine {})",
            crate::telemetry::engine_on()
        );
    }
    refuel.was_by_tank = by_it;
    // E at a pump, on foot, not by the tank: where they stood (the truck's own space), to set the place by.
    if hold_pressed
        && offered
        && !at_tank
        && refuel.filling.is_none()
        && let Some(player) = player
    {
        let [side, back] = in_vehicle(truck.position, truck.heading, player);
        log!(
            "refuel: E at a pump, not by the tank: {side:.2} m to the side, {:.2} m behind the driver's eyes (papers first: {freights})",
            back - f64::from(truck.head[2])
        );
    }
    let mut ended = None;
    let mut filled_now = None;
    if refuel.filling.is_some() {
        refuel.offer = Some(Action::StopRefuel);
        // Let go: stopped, what went in is what they wanted.
        if !hold_down {
            ended = Some(false);
        }
    } else if at_tank {
        if offered {
            refuel.offer = Some(Action::Refuel);
            if hold_pressed && is_full {
                to_say = Some(Say::AlreadyFull);
            } else if hold_pressed && crate::telemetry::engine_on() {
                // The game fills only a truck switched off, and says so itself; here too.
                to_say = Some(Say::EngineOff);
            } else if hold_pressed {
                refuel.filling = Some(Filling { since: Instant::now(), from: fuel, rose: None });
                super::HELD_STILL.store(true, Ordering::Release);
                crate::commands::hold(crate::commands::ACTIVATE, true);
                // The nozzle in the tank (the door's sound: the plugin has no other).
                super::steps::door(super::steps::Door::Shut, 0.0);
                log!("refuel: filling the tank by hand, from {fuel:.1} l of {capacity:.0}");
                // The nozzle in the hand, at the tank (first person: `super::nozzle_frame`).
                set_nozzle(player.map(|player| toward_tank(player, &truck)));
                to_say = Some(Say::Filling);
            }
        }
    }
    // Filling: held where they stand each frame (`freight` may let go of it), until the pump stops giving.
    if let Some(filling) = refuel.filling.as_mut() {
        super::HELD_STILL.store(true, Ordering::Release);
        if rising {
            filling.rose = Some(Instant::now());
            // The pump, for sure: the player's own key is the plugin's here from now on.
            if refuel.pump.is_none() {
                log!("refuel: a pump here (it filled the tank)");
                refuel.pump = Some(truck.position);
            }
        }
        match filling.rose {
            _ if on_foot.is_none() => ended = Some(false),
            None if filling.since.elapsed() >= NO_FUEL_AFTER => ended = Some(false),
            Some(rose) if rose.elapsed() >= DONE_AFTER => ended = Some(true),
            _ => {}
        }
    }
    if let Some(by_pump) = ended
        && let Some(filling) = refuel.filling.take()
    {
        crate::commands::hold(crate::commands::ACTIVATE, false);
        super::HELD_STILL.store(false, Ordering::Release);
        set_nozzle(None);
        let litres = (fuel - filling.from).max(0.0);
        refuel.meter_until = Some((Instant::now() + METER_AFTER, litres));
        if filling.rose.is_some() {
            filled_now = Some(litres);
        }
        log!("refuel: filled {litres:.1} l (the pump stopped: {by_pump}, full: {is_full})");
        to_say = Some(match filling.rose {
            // Let go before the pump began: nothing in, how it goes said again.
            None if filling.since.elapsed() < NO_FUEL_AFTER => Say::HoldToFill,
            None => Say::NoFuel,
            Some(_) if by_pump || is_full => {
                super::steps::door(super::steps::Door::Shut, 0.0);
                if is_full { Say::Full(litres) } else { Say::Filled(litres) }
            }
            Some(_) => Say::Filled(litres),
        });
    }

    let gallons = crate::log::game_dir().to_string_lossy().contains("American");
    let unit = |litres: f32| if gallons { litres / LITRES_A_GALLON } else { litres };

    // The bill (owner 2026-11-12: "not charged at once: go to the attendant and pay, on the phone"): the fuel filled
    // is owed, an attendant stands by the truck, E by them shows it on the phone's Nupay to pay. The game took the
    // money as it filled; this is WestPlay's own way of paying it (Nupay's history, the meter, the notices).
    let pay_attendant =
        crate::bridge::get().is_none_or(|bridge| bridge.host.pay_attendant.load(Ordering::Acquire) != 2);
    if let Some(filled) = filled_now
        && pay_attendant
        && filled >= BILL_LEAST
        && let Some(price) = notice::pump_price()
    {
        refuel.bills += 1;
        let id = refuel.bills;
        let (litres, attendant) = match refuel.bill.take() {
            // Filled again before paying: one bill, the attendant who's there.
            Some(open) => (open.litres + filled, Some((open.attendant, open.attendant_at, open.attendant_yaw))),
            None => (filled, None),
        };
        let total = unit(litres) * price;
        // Where they stand: out from where the player filled, a little ahead along the truck, facing them.
        let physics = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.physics.as_ref());
        let spot = on_foot.zip(player).map(|(walking, eyes)| {
            // SAFETY: the walking camera is held; `physics` as `locate` found it.
            let ground = eyes[1] - unsafe { super::feet_under(walking, physics) };
            // In front of the truck, on the pump's side (the player's), as the cargo's clerk stands (owner
            // 2026-11-12: by the tank, E was the tank's and the clerk's both; "in front of the truck it's safer").
            let [side, _] = in_vehicle(truck.position, truck.heading, eyes);
            let head_z = f64::from(truck.head[2]);
            let spot = [side.signum() * super::freight::FRONT_SIDE, head_z - super::freight::FRONT_AHEAD];
            let [x, _, z] = super::freight::from_vehicle(truck.position, truck.heading, spot);
            let at = [x, ground, z];
            (at, super::heading_of(eyes[0] - at[0], eyes[2] - at[2]))
        });
        let (attendant, attendant_at, attendant_yaw) = match (attendant, spot) {
            (Some(kept), _) => kept,
            (None, Some((at, yaw))) => {
                let people = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.people.as_ref());
                // SAFETY: on the game's main thread, `people` as `locate` found it.
                let shown = people.map(|people| unsafe {
                    super::person::show_other(people, &Placement::at(at, yaw / std::f32::consts::TAU), &ATTENDANT)
                });
                match shown {
                    Some(Ok(person)) => (Some(person), at, yaw),
                    other => {
                        log!(
                            "refuel: no attendant ({}): E pays by the pump",
                            other.and_then(Result::err).unwrap_or("no people")
                        );
                        (None, at, yaw)
                    }
                }
            }
            (None, None) => (None, truck.position, 0.0),
        };
        log!("refuel: the bill {id}: {litres:.1} l at {price:.2}, {total:.2} to pay");
        refuel.bill = Some(Bill {
            id,
            litres,
            price,
            total,
            truck_at: truck.position,
            attendant,
            attendant_at,
            attendant_yaw,
            asked: false,
            last: Instant::now(),
        });
        refuel.say_pay_at = Some(Instant::now() + SAY_PAY_AFTER);
    }
    if refuel.say_pay_at.is_some_and(|at| Instant::now() >= at) {
        refuel.say_pay_at = None;
        if let Some(bill) = refuel.bill.as_ref().filter(|bill| !bill.asked) {
            if to_say.is_none() && refuel.filling.is_none() {
                to_say = Some(Say::PayAttendant(bill.total));
            }
            // Said again while it's owed (owner 2026-11-20: "a notice to pay").
            refuel.say_pay_at = Some(Instant::now() + SAY_PAY_AGAIN);
        }
    }
    // Back in the cab with it owed: said so (once each time).
    let on_foot_now = on_foot.is_some();
    if refuel.was_on_foot && !on_foot_now && to_say.is_none()
        && let Some(bill) = refuel.bill.as_ref()
    {
        to_say = Some(Say::StillOwed(bill.total));
    }
    refuel.was_on_foot = on_foot_now;
    // What's owed, for the app (the phone's notification).
    owe(refuel.bill.as_ref(), gallons);
    // The phone's answer: paid, or put down unpaid.
    let answer = crate::bridge::get().map_or(0, |bridge| bridge.host.fuel_paid.swap(0, Ordering::AcqRel));
    let mut paid = None;
    if let Some(bill) = refuel.bill.as_mut()
        && answer != 0
    {
        if answer == bill.id {
            paid = Some(bill.total);
        } else if answer == bill.id | westplay_shared::FUEL_NOT_PAID {
            bill.asked = false;
            ask_bill(None, gallons);
            log!("refuel: the bill put down unpaid");
        }
    }
    // Left the pump without paying: the truck gone from where it filled.
    let left = refuel.bill.as_ref().is_some_and(|bill| apart(bill.truck_at, truck.position) > BILL_LEFT);
    // Paying: by the attendant, on foot, not filling.
    if let Some(bill) = refuel.bill.as_mut()
        && paid.is_none()
        && !left
        && refuel.filling.is_none()
    {
        if bill.asked && on_foot.is_none() {
            // Back in the cab with the phone showing it: put away, still owed.
            bill.asked = false;
            ask_bill(None, gallons);
        }
        let near = player.is_some_and(|player| {
            let within = if bill.attendant.is_some() { PAY_WITHIN } else { PAY_WITHOUT_ATTENDANT };
            let from = if bill.attendant.is_some() { bill.attendant_at } else { bill.truck_at };
            apart(player, from) < within
        });
        if near && !bill.asked && !at_tank {
            refuel.offer = Some(Action::PayFuel);
            if use_pressed {
                bill.asked = true;
                ask_bill(Some(bill), gallons);
                log!("refuel: the bill {} shown on the phone to pay", bill.id);
            }
        }
        // The attendant standing, turned to the player.
        let dt = bill.last.elapsed().as_secs_f32().min(0.1);
        bill.last = Instant::now();
        if let Some(attendant) = bill.attendant.as_mut()
            && let Some(Some(game)) = &state.game
            && let Some(people) = game.people.as_ref()
        {
            if let Some(player) = player {
                let to = super::heading_of(player[0] - bill.attendant_at[0], player[2] - bill.attendant_at[2]);
                bill.attendant_yaw = super::turn_toward(bill.attendant_yaw, to, 3.0 * dt);
            }
            let placement = Placement::at(bill.attendant_at, bill.attendant_yaw / std::f32::consts::TAU);
            // SAFETY: on the game's main thread, the attendant as `show_other` made them.
            unsafe { super::person::place_other(people, attendant, &placement, dt) };
        }
    }
    if let Some(total) = paid {
        log!("refuel: the bill paid ({total:.2})");
        to_say = Some(Say::Paid(total));
    } else if left {
        log!("refuel: left the pump without paying");
        to_say = Some(Say::LeftUnpaid);
    }
    // Paid: the attendant walks off, ahead and outwards from the truck, along the way it came in by.
    if paid.is_some()
        && let Some(refuel) = state.refuel.as_mut()
        && let Some(bill) = refuel.bill.as_mut()
        && let Some(standing) = bill.attendant.take()
        && let Some(Some(game)) = &state.game
        && let Some(people) = game.people.as_ref()
    {
        let [side, _] = in_vehicle(truck.position, truck.heading, bill.attendant_at);
        let off_to = super::freight::from_vehicle(bill.attendant_at, truck.heading, [side.signum() * 5.0, -10.0]);
        let yaw = super::heading_of(off_to[0] - bill.attendant_at[0], off_to[2] - bill.attendant_at[2]);
        let placement = Placement::at(bill.attendant_at, yaw / std::f32::consts::TAU);
        // SAFETY: on the game's main thread, `people` as `locate` found it; the standing one as `show_other` made.
        unsafe {
            match super::person::show_other(people, &placement, &ATTENDANT_WALKS) {
                Ok(walker) => {
                    super::person::hide_other(people, standing);
                    if let Some((before, ..)) = refuel.leaving.take() {
                        super::person::hide_other(people, before);
                    }
                    refuel.leaving = Some((walker, bill.attendant_at, yaw, Instant::now(), Instant::now()));
                    log!("refuel: the attendant walks away");
                }
                Err(_) => super::person::hide_other(people, standing),
            }
        }
    }
    if paid.is_some() || left {
        close_bill(state);
    }
    // The attendant walking off, then gone.
    if let Some(refuel) = state.refuel.as_mut()
        && let Some((walker, at, yaw, since, last)) = refuel.leaving.as_mut()
        && let Some(Some(game)) = &state.game
        && let Some(people) = game.people.as_ref()
    {
        let dt = last.elapsed().as_secs_f32().min(0.1);
        *last = Instant::now();
        let ahead = super::ahead_of(*yaw, super::freight::CLERK_PACE * f64::from(dt));
        *at = [at[0] + ahead[0], at[1], at[2] + ahead[1]];
        let placement = Placement::at(*at, *yaw / std::f32::consts::TAU);
        // SAFETY: on the game's main thread, the walker as `show_other` made them.
        unsafe { super::person::place_other(people, walker, &placement, dt) };
        if since.elapsed() >= super::freight::CLERK_LEAVES_FOR
            && let Some((walker, ..)) = refuel.leaving.take()
        {
            // SAFETY: as above.
            unsafe { super::person::hide_other(people, walker) };
        }
    }
    let Some(refuel) = state.refuel.as_mut() else { return };

    // The pump's meter (owner 2026-11-12: "the litres and the price, going up as it fills"): while filling, the
    // last of it a moment after, and while it's owed.
    let reading = match (&refuel.filling, refuel.meter_until, &refuel.bill) {
        (Some(filling), _, _) => Some(((fuel - filling.from).max(0.0), false, false)),
        (None, _, Some(bill)) => Some((bill.litres, true, true)),
        (None, Some((until, litres)), None) if Instant::now() < until => Some((litres, true, false)),
        _ => None,
    };
    crate::fuel_meter::set(reading.map(|(litres, done, owed)| crate::fuel_meter::Reading {
        amount: unit(litres),
        price: refuel.bill.as_ref().map(|bill| bill.price).or_else(notice::pump_price),
        gallons,
        done,
        owed,
    }));
    // Told in the game's corner (its notices found), and in the log; the same again only after a while.
    let Some(what) = to_say else { return };
    let again = |(said, at): (Say, Instant)| {
        std::mem::discriminant(&said) == std::mem::discriminant(&what) && at.elapsed() < NOTICE_AGAIN
    };
    if refuel.said.is_some_and(again) {
        return;
    }
    refuel.said = Some((what, Instant::now()));
    log!("refuel: notice {what:?}");
    // SAFETY: the code is the game's, read as it lies.
    state.game.get_or_insert_with(|| unsafe { super::locate() });
    if let Some(Some(game)) = &state.game
        && let Some(notices) = game.notices.as_ref()
    {
        let gallons = crate::log::game_dir().to_string_lossy().contains("American");
        // SAFETY: on the game's main thread, at the end of a frame (`frame`'s contract).
        unsafe { notices.show(&what.text(notice::language(), gallons), what.icon()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn truck() -> Truck {
        Truck {
            position: [100.0, 0.0, 200.0],
            heading: 0.0,
            speed: 0.0,
            pitch: 0.0,
            roll: 0.0,
            parking_brake: true,
            multiplayer: false,
            head: [-0.7, 1.5, -1.0],
        }
    }

    /// A pump's rise is a little each frame: not the channel's noise, not a tank changed whole.
    #[test]
    fn a_pump_fills_a_little_each_frame() {
        assert!(pumped(100.0, 100.5));
        assert!(!pumped(100.0, 100.001), "noise");
        assert!(!pumped(100.0, 99.9), "burning");
        assert!(!pumped(100.0, 600.0), "another truck, a game loaded");
        assert!(!pumped(0.0, 300.0), "no truck before");
    }

    /// Filling, the view turns square to the truck, toward it, from either side; the nozzle within reach.
    #[test]
    fn the_nozzle_goes_toward_the_tank() {
        let truck = truck();
        // Heading 0: the truck's right is the world's +x; a heading of θ looks along (-sin θ, -cos θ).
        let (yaw, ahead) = toward_tank([100.0 + 2.0, 0.0, 201.0], &truck);
        let look = [-f64::from(yaw).sin(), -f64::from(yaw).cos()];
        assert!(look[0] < -0.99 && look[1].abs() < 0.01, "from its right, looking left: {look:?}");
        assert!((ahead - 0.4).abs() < 1e-6);
        let (yaw, ahead) = toward_tank([100.0 - 1.2, 0.0, 201.0], &truck);
        let look = [-f64::from(yaw).sin(), -f64::from(yaw).cos()];
        assert!(look[0] > 0.99, "from its left, looking right: {look:?}");
        assert_eq!(ahead, *GRIP_AHEAD.start(), "close by: the least reach");
        assert_eq!(nozzle(), None, "not filling");
    }

    #[test]
    fn the_tank_is_full_near_its_size() {
        assert!(full(595.0, 600.0));
        assert!(!full(500.0, 600.0));
        assert!(!full(500.0, 0.0), "its size unknown: the pump says when");
    }

    /// Beside the tank: either side of the truck, behind the cab's doors; not in front, not far, not under it.
    #[test]
    fn the_tank_is_beside_the_truck_behind_the_cab() {
        let truck = truck();
        // Heading 0: the truck's right is the world's +x, its back the world's +z.
        let at = |x: f64, z: f64| [100.0 + x, 0.0, 200.0 + z];
        assert!(by_tank(at(-1.8, 1.0), &truck), "the driver's side");
        assert!(by_tank(at(1.8, 1.0), &truck), "the other side");
        // Not by the fifth wheel: that's coupling's.
        let hook = super::super::freight::hook_back(&truck);
        assert!(!by_tank(at(-1.6, hook), &truck), "by the fifth wheel");
        assert!(!super::super::freight::by_fifth_wheel(at(-1.8, 1.0), &truck), "the tank isn't the fifth wheel's");
        assert!(by_tank(at(1.6, -1.5), &truck), "by the cab's step, under the door");
        assert!(!by_tank(at(-1.8, -3.0), &truck), "in front of the cab");
        assert!(!by_tank(at(-6.0, 1.0), &truck), "far to the side");
        assert!(!by_tank(at(-2.6, 1.0), &truck), "a step away from it");
        assert!(!by_tank(at(0.0, 1.0), &truck), "under the truck");
        assert!(!by_tank(at(-1.8, 9.0), &truck), "behind the truck");
        assert!(!by_tank(at(-1.8, -3.0), &truck), "in front of the cab, again");
    }

    /// The game's own words at a pump, in its languages; not its other notices.
    #[test]
    fn hears_the_game_speak_of_fuel() {
        assert!(says_fuel("Desligue o motor para abastecer"));
        assert!(says_fuel("Pressione Enter para abastecer"));
        assert!(says_fuel("Turn off the engine to refuel"));
        assert!(says_fuel("Motor ausschalten, um zu tanken"));
        assert!(!says_fuel("Pare o veículo!"));
        assert!(!says_fuel("Low fuel"));
        assert!(says_fuel("@@fuel_engine_off@@"));
        assert!(says_fuel("/material/ui/hud/fuel.mat"));
        assert!(!says_fuel("@@low_fuel@@"));
        assert!(!says_fuel("@@trailer_decoupling@@"));
        assert!(!says_fuel("Carreta engatada"));
    }

    /// The pump's panel, not the game's other words of fuel.
    #[test]
    fn knows_the_pumps_panel() {
        assert!(pump_panel(
            "@@refuel_hold@@ <color value=@@clr_sel@@>Enter<color value=@@clr_txt@@><br>@@fuel_price@@ 6,42"
        ));
        // As the game made it (owner's log 2026-11-12, ETS2 1.61, Portuguese).
        assert!(pump_panel(
            "<color value=@@clr_sel@@>@@eurosp@@@@europ@@47@@ds@@6@@unit_mega@@ @@euros@@<br offset=10><color \
             value=@@clr_txt@@>@@gas_station_hint@@ <color value=@@clr_sel@@><color value=@@clr_sel@@>Enter<color \
             value=@@clr_txt@@><br offset=10><color value=@@clr_txt@@>@@diesel_price@@: \
             @@eurosp@@@@europ@@5@@ds@@46@@euros@@<br>@@litres@@: 0.00<br><color value=@@clr_sel@@>@@total_price@@: \
             @@eurosp@@@@europ@@0@@euros@@"
        ));
        assert!(!pump_panel("@@tank_full@@"));
        assert!(!pump_panel(
            "@@stop_engine@@ <color value=@@clr_sel@@><color value=@@clr_sel@@>E<color value=@@clr_txt@@>"
        ));
        assert!(!pump_panel("Abastecendo: aperte E para parar"));
        // Short, by its own keys (owner 2026-11-20: still shown).
        assert!(pump_panel("@@refuel_hold@@ <color value=@@clr_sel@@>Enter<color value=@@clr_txt@@>"));
        assert!(pump_panel("@@gas_station_hint@@ <color value=@@clr_sel@@>Enter"));
        assert!(!pump_panel("@@sleep_hint@@ <color value=@@clr_sel@@>Enter<color value=@@clr_txt@@>"));
        assert!(!pump_panel("@@attach_load@@ <color value=@@clr_sel@@>T<color value=@@clr_txt@@>"));
    }

    #[test]
    fn every_notice_has_its_words() {
        for say in [
            Say::GetOut,
            Say::EngineOff,
            Say::NoFuel,
            Say::AlreadyFull,
            Say::Filling,
            Say::Filled(10.0),
            Say::Full(10.0),
            Say::PayAttendant(130.0),
            Say::StillOwed(130.0),
            Say::HoldToFill,
            Say::Paid(477.2),
            Say::LeftUnpaid,
        ] {
            for language in ["pt", "es", "en", "ja"] {
                assert!(!say.text(language, false).is_empty());
            }
        }
        assert_eq!(Say::Full(249.6).text("pt", false).to_str(), Ok("Tanque cheio: 250 litros"));
        assert_eq!(Say::Filled(378.541).text("en", true).to_str(), Ok("Refuelled: 100 gallons"));
    }
}

#[cfg(test)]
mod pump_panel_tests {
    use super::*;

    #[test]
    fn the_pumps_priced_panel_is_known() {
        // The game's words on 2026-11-15, the engine off at the pump.
        let panel = "<color value=@@clr_txt@@>@@diesel_price@@: @@eurosp@@@@europ@@6@@ds@@42@@euros@@<br>@@litres@@: 0.00<br><color value=@@clr_sel@@>@@total_price@@: @@eurosp@@@@europ@@0@@euros@@<br offset=10><color value=@@clr_sel@@>Saldo da conta: @@eurosp@@@@europ@@46@@ds@@3@@unit_mega@@ @@euros@@<br offset=10><color value=@@clr_txt@@>@@refuel_hold@@";
        assert!(pump_panel(panel));
        let full = "<color value=@@clr_txt@@>@@diesel_price@@: @@eurosp@@@@europ@@6@@ds@@42@@euros@@<br>@@litres@@: 20.34<br>@@tank_full@@";
        assert!(!pump_panel(full));
    }
}
