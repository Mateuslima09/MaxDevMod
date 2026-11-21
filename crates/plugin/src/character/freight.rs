//! The cargo's papers and coupling by hand (owner 2026-11-11): on a job with the player's own truck, a clerk stands
//! by the trailer, and the player gets out, walks up to them and signs the cargo's nota fiscal (the app shows it:
//! `westplay_shared::PHONE_PAPER`) before the trailer can be coupled (or, with the player's own trailer, the cargo
//! loaded); at the other end, the recipient signs before the cargo can be delivered. The trailer itself is coupled
//! on foot, at the truck's fifth wheel.
//!
//! What it stands on, all of it the game's own, none of it new in its code: the SDK's telemetry (the job, the
//! trailer's place, whether it's coupled, the route left), the keys the plugin keeps from the game (the player's
//! `attach` and `activate` keys, read from their `controls.sii`: the game's own "press T to couple" then does
//! nothing, and a notice says why), the game's `attach` given through the plugin's input device (`commands`), its
//! notices (`notice`) and its people (`person`). The game's own advice to press the key can't be taken away.
//!
//! Only on a job with the player's own truck: one that starts with a loaded trailer already coupled (a quick job)
//! has no papers. What's been signed is kept by the job (`Papers`, a small file), so a game loaded mid-job doesn't
//! ask again.

use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use westplay_shared::{
    JobInfo, NOTE_DELIVERY, NOTE_LEFT, NOTE_LEGS_DOWN, NOTE_LEGS_UP, NOTE_LOCK_PULL, NOTE_LOCK_PUSH, NOTE_PICKUP,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VK_TO_VSC, MapVirtualKeyW, VIRTUAL_KEY};

use super::{Placement, State, heading_of, notice, person, really_down, turn_toward};
use crate::hints::Action;
use crate::log;
use crate::telemetry::{Trailer, Truck};

/// How long a job is watched before it's said whose it is (frames): the trailer's coupling comes a moment after
/// the job does.
const JUDGED_AFTER: u32 = 150;
/// The end of the route: the shipper's or the recipient's yard (metres left on the game's GPS, more than none).
const NEAR: f32 = 300.0;
/// The truck standing (m/s).
const STANDING: f32 = 0.5;
/// How far the truck sees the clerk by a trailer still to couple (metres).
const CLERK_SEEN: f64 = 150.0;
/// How near the player stands to the clerk to sign, and to the fifth wheel to couple (metres along the ground).
const CLERK_REACH: f64 = 2.6;
/// Beside the fifth wheel, where the trailer is coupled and uncoupled by hand (owner 2026-11-11, with pictures:
/// "only by the axle, where it couples, between the truck's wheels; not where the tank is"): either side of the
/// truck, from its side out to an arm's reach (a truck is 1.25 m to its side), from this far in front of the fifth
/// wheel to this far behind it. The tank is in front of it (`refuel`, up to `BEFORE_HOOK`): the two never meet.
/// (Where the truck's fifth wheel isn't told, it's this far behind the driver's head.)
pub(super) const SIDE_OUT: std::ops::RangeInclusive<f64> = 0.8..=2.1;
pub(super) const BEFORE_HOOK: f64 = 0.6;
const PAST_HOOK: f64 = 1.0;
pub(super) const HOOK_FROM_HEAD: f64 = 4.3;

/// How far behind the driver's head the truck's fifth wheel is (metres, in the truck's own space).
pub(super) fn hook_back(truck: &Truck) -> f64 {
    crate::telemetry::truck_hook().map_or(f64::from(truck.head[2]) + HOOK_FROM_HEAD, |hook| f64::from(hook[2]))
}

/// Whether the player (a place in the world) stands beside the truck's fifth wheel, either side.
pub(super) fn by_fifth_wheel(player: [f64; 3], truck: &Truck) -> bool {
    let hook = hook_back(truck);
    let [side, back] = in_vehicle(truck.position, truck.heading, player);
    SIDE_OUT.contains(&side.abs()) && (hook - BEFORE_HOOK..=hook + PAST_HOOK).contains(&back)
}
/// How far the trailer's pin may be from the fifth wheel for the game to couple them (metres).
const COUPLING_REACH: f64 = 1.3;
/// Whether a notice of the game's says it can load the trailer where it stands, with its key (its words as its keys:
/// `@@…load…@@` and Enter); not its advice to couple, to uncouple, nor its "go and load" at the job's start.
fn load_prompt(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("@@")
        && lower.contains("enter")
        && lower.contains("load")
        && !["unload", "attach_load", "waiting", "job_offer"].iter().any(|not| lower.contains(not))
}

/// Whether a notice of the game's says it can unload the trailer where it stands, with its key: `@@…unload…@@` and
/// Enter; not its advice to uncouple (`@@unload_load@@`, T) nor its "unload at the marked place".
fn unload_prompt(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("@@")
        && lower.contains("enter")
        && lower.contains("unload")
        && !["unload_load", "cargo_unloading"].iter().any(|not| lower.contains(not))
}

/// The advice to unload the player's own trailer, in the game's language (two letters): the signature unloads it.
fn unload_cargo_words(language: &str) -> &'static CStr {
    match language {
        "pt" => c"Entregue a nota fiscal ao conferente: a carga é descarregada em seguida",
        "es" => c"Entrega la factura al encargado: la carga se descarga enseguida",
        _ => c"Hand the papers to the clerk: the cargo is unloaded right after",
    }
}

/// How long the game's activate is held to load the trailer once the papers are collected.
const LOAD_HELD: Duration = Duration::from_millis(400);
/// After coupling or uncoupling by hand, how long E at the fifth wheel does nothing.
const AFTER_BY_HAND: Duration = Duration::from_secs(4);
/// How far over the truck's own point the ground is (metres, as bits), learnt (`frame`): none, as the owner's log
/// measured it (2026-11-11: the player's ground and the truck's point both 91.01). A clerk put there from the cab
/// sank where the yard rose by the trailer, and floated with 0.7 m added: the ground right by them is the player's,
/// as they come up to the clerk.
static TRUCK_GROUND: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// How near the player comes to the clerk for the ground under their feet to be the clerk's (metres), and how far
/// they go for it to be measured again as they come back.
/// From this near, then each time they've come closer by a part (`GROUND_CLOSER`): from afar a little off,
/// right by them exact, never followed step by step (owner 2026-11-11: "you have to come very close for the feet
/// to be fixed: make it farther").
const GROUND_NEAR: f64 = 15.0;
const GROUND_CLOSER: f64 = 0.6;
const GROUND_AGAIN: f64 = 25.0;

/// The ground under the truck, as learnt.
fn truck_ground(truck: &Truck) -> f64 {
    truck.position[1] + f64::from_bits(TRUCK_GROUND.load(Ordering::Acquire))
}

/// The advice to load the player's own trailer, in the game's language (two letters): the papers load it.
fn load_words(language: &str) -> &'static CStr {
    match language {
        "pt" => c"Colete a nota fiscal com o conferente: a carga \u{e9} carregada em seguida",
        "es" => c"Recoge la factura con el encargado: la carga se carga enseguida",
        _ => c"Collect the papers from the clerk: the cargo is loaded right after",
    }
}

/// The advice to uncouple at a delivery, in the game's language (two letters).
fn unload_words(language: &str) -> &'static CStr {
    match language {
        "pt" => c"Desengate a carreta a pé, na quinta roda: tecla E",
        "es" => c"Desengancha el remolque a pie, en la quinta rueda: tecla E",
        _ => c"Uncouple the trailer on foot, at the fifth wheel: key E",
    }
}

/// The advice to couple, in the game's language (two letters).
fn attach_words(language: &str) -> &'static CStr {
    match language {
        "pt" => c"Engate a carreta a pé, na quinta roda: tecla E",
        "es" => c"Engancha el remolque a pie, en la quinta rueda: tecla E",
        _ => c"Couple the trailer on foot, at the fifth wheel: key E",
    }
}
/// How far the trailer's pin may be from the fifth wheel for coupling to be offered at all (metres): a trailer
/// backed up to it, a bit off ("not aligned" then), but not one left at a company far away (owner 2026-11-11).
const TRAILER_NEAR: f64 = 8.0;
/// Coupling by hand: how long the player is down at the fifth wheel before the trailer is coupled, how long the
/// game's `attach` is held, and how long after it's seen whether it took.
/// (As long as the person's `work` takes to get up from its work: frame 80, `person::WORK_UP`.)
const COUPLING_TIME: Duration = Duration::from_millis(2640);
/// How long a step by hand waits in the app (the lock, the legs) before the person gets up, nothing done.
const LEVER_FOR: Duration = Duration::from_secs(90);
/// The trailer's legs (owner 2026-11-19: "cranked down and up as in real life"): reached at its side, this far
/// behind its pin (metres, along it) and out from its side as the fifth wheel is (`SIDE_OUT`).
const LEGS_BEHIND_PIN: std::ops::RangeInclusive<f64> = 1.2..=5.0;
const ATTACH_HELD: Duration = Duration::from_millis(350);
const COUPLED_WITHIN: Duration = Duration::from_millis(1800);
/// Uncoupling takes the game longer to tell (owner's log 2026-11-11: still coupled 4 s after, uncoupled by 5).
const UNCOUPLED_WITHIN: Duration = Duration::from_secs(9);
/// The clerk going away: at a walk (m/s), for this long.
pub(super) const CLERK_PACE: f64 = 1.4;
pub(super) const CLERK_LEAVES_FOR: Duration = Duration::from_secs(9);
/// The same notice again no sooner than this.
const NOTICE_AGAIN: Duration = Duration::from_millis(2500);
/// The key that signs and couples, on foot: E (the game's engine key, kept from it while walking).
const USE_KEY: (u16, u32) = (b'E' as u16, 0x12);

/// What a job's papers are at: whose job it is (`applies`: the player's own truck), and what's been signed. By the
/// job (`key`), kept in a file.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Papers {
    key: String,
    /// Frames the job has been seen, until it's judged.
    seen: u32,
    judged: bool,
    applies: bool,
    /// The nota fiscal collected (or the trailer coupled and loaded without it: nothing more to ask).
    picked: bool,
    /// The recipient's signature collected.
    signed: bool,
}

/// What the game says now.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Facts {
    /// The job (`Papers::key`), `None` without one.
    job: Option<String>,
    cargo_loaded: bool,
    connected: bool,
    /// At the end of the route (`NEAR`): the shipper's or the recipient's yard.
    at_end: bool,
}

/// The paper the player needs now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Need {
    None,
    /// The nota fiscal, before the trailer is coupled.
    Pickup,
    /// The nota fiscal, before the player's own trailer is loaded.
    Load,
    /// The recipient's signature, before the cargo is delivered.
    Delivery,
}

/// The game's workers the clerk may be: the mod's movers of them (`westplay_setup::clerk::CLERKS`), the worker in
/// a vest, the security guard, the service man.
const CLERKS: [&str; 3] = ["wp_clk1", "wp_clk2", "wp_clk3"];

/// The clerk of a job (`key`), the same each time; at its delivery, another.
fn clerk_of(key: &str, delivery: bool) -> &'static str {
    let hash = key.bytes().fold(2_166_136_261_u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(16_777_619));
    CLERKS[(hash as usize + usize::from(delivery)) % CLERKS.len()]
}

/// A job by what the SDK says of it: the same job is the same key in a game loaded again.
fn job_key(job: &JobInfo) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        job.cargo.as_str(),
        job.source_company.as_str(),
        job.source_city.as_str(),
        job.destination_company.as_str(),
        job.destination_city.as_str(),
        job.delivery_time
    )
}

impl Papers {
    /// Takes what the game says now; whether anything to keep changed.
    fn update(&mut self, facts: &Facts) -> bool {
        let before = self.clone();
        match &facts.job {
            None => *self = Self::default(),
            Some(key) => {
                if *key != self.key {
                    *self = Self { key: key.clone(), ..Self::default() };
                }
                self.seen = self.seen.saturating_add(1);
                if !self.judged && self.seen >= JUDGED_AFTER {
                    self.judged = true;
                    // A loaded trailer already coupled as the job starts: a quick job, the company's truck.
                    self.applies = !(facts.connected && facts.cargo_loaded);
                    (self.picked, self.signed) = (!self.applies, !self.applies);
                }
                // Coupled and loaded all the same (another device's key): nothing more to ask of the pick-up.
                if self.judged && !self.picked && facts.connected && facts.cargo_loaded {
                    self.picked = true;
                }
            }
        }
        (&before.key, before.judged, before.applies, before.picked, before.signed)
            != (&self.key, self.judged, self.applies, self.picked, self.signed)
    }

    fn need(&self, facts: &Facts) -> Need {
        if !self.judged || !self.applies || facts.job.is_none() {
            Need::None
        } else if !self.picked {
            if facts.connected { Need::Load } else { Need::Pickup }
        } else if !self.signed && facts.connected && facts.cargo_loaded {
            Need::Delivery
        } else {
            Need::None
        }
    }

    /// As the file keeps it: the job, then what's known of it.
    fn to_text(&self) -> String {
        format!("{}\n{} {} {}\n", self.key, u8::from(self.applies), u8::from(self.picked), u8::from(self.signed))
    }

    /// From the file: a job already judged.
    fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let key = lines.next().filter(|key| !key.is_empty())?.to_owned();
        let mut flags = lines.next()?.split_whitespace().map(|flag| flag == "1");
        let (applies, picked, signed) = (flags.next()?, flags.next()?, flags.next()?);
        Some(Self { key, seen: JUDGED_AFTER, judged: true, applies, picked, signed })
    }
}

/// A place in a vehicle's own space (metres: x to its right, z backwards) in the world, along the ground.
pub(super) fn from_vehicle(position: [f64; 3], heading: f32, [lx, lz]: [f64; 2]) -> [f64; 3] {
    let turn = f64::from(heading) * std::f64::consts::TAU;
    let [x, y, z] = position;
    [x + lx * turn.cos() + lz * turn.sin(), y, z - lx * turn.sin() + lz * turn.cos()]
}

/// A place in the world in a vehicle's own space (x to its right, z backwards).
pub(super) fn in_vehicle(position: [f64; 3], heading: f32, world: [f64; 3]) -> [f64; 2] {
    let turn = f64::from(heading) * std::f64::consts::TAU;
    let (dx, dz) = (world[0] - position[0], world[2] - position[2]);
    [dx * turn.cos() - dz * turn.sin(), dx * turn.sin() + dz * turn.cos()]
}

pub(super) fn apart(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

/// The clerk in front of the truck (owner 2026-11-12, with pictures: beside the trailer they stood in a wall, in a
/// narrow way between a building and a tanker; "in front of the truck there's almost never anything"): this far
/// ahead of the driver's head (metres; past a cab-over's bumper, ~1.5 m ahead of it, and a bonneted truck's,
/// ~2.5 m), this far to the driver's side of its middle; the next spots farther out.
pub(super) const FRONT_AHEAD: f64 = 3.8;
pub(super) const FRONT_SIDE: f64 = 1.0;
/// The truck is at its trailer (coupled, or backed up to it) with its fifth wheel this near the trailer's pin.
const AT_TRAILER: f64 = 8.0;

/// Where the clerk stands, and where they walk off to once done.
struct Spot {
    at: [f64; 3],
    off_to: [f64; 3],
}

/// Where the clerk stands. Beside the trailer the game's own things (a wall, a building, other trailers) were in
/// the way, and the plugin can't see them: the game's physics ray, the one way to, closed the game twice
/// (`docs/personagem/TESTES.md` §9, removed by the owner 2026-11-05). So where they almost never are: the open way
/// the truck came in by. With the truck at the trailer (coupled or backed up to it), in front of the truck on the
/// driver's side; with the trailer standing alone, in front of its nose, out to the side of where the truck backs
/// in. Never in the truck nor on the player: then the other side, then farther out. Done, they walk off ahead and
/// outwards, along that open way.
fn clerk_spot(trailer: &Trailer, truck: &Truck, player: Option<[f64; 3]>) -> Spot {
    let pin = trailer.hook.map_or(0.0, |hook| f64::from(hook[2]));
    let fifth_wheel = from_vehicle(truck.position, truck.heading, [0.0, hook_back(truck)]);
    let at_trailer = trailer.connected
        || apart(fifth_wheel, from_vehicle(trailer.position, trailer.heading, [0.0, pin])) < AT_TRAILER;
    let [head_x, _, head_z] = truck.head.map(f64::from);
    let driver = if head_x > 0.0 { 1.0 } else { -1.0 };
    // In the vehicle's own space (x to its right, z backwards): the spots, nearest first.
    let (position, heading, spots) = if at_trailer {
        let ahead = head_z - FRONT_AHEAD;
        let spots = [
            [driver * FRONT_SIDE, ahead],
            [-driver * FRONT_SIDE, ahead],
            [driver * 2.6, ahead - 1.5],
            [-driver * 2.6, ahead - 1.5],
        ];
        (truck.position, truck.heading, spots)
    } else {
        let spots = [
            [driver * 3.0, pin - 4.0],
            [-driver * 3.0, pin - 4.0],
            [driver * 3.0, pin - 6.5],
            [-driver * 3.0, pin - 6.5],
        ];
        (trailer.position, trailer.heading, spots)
    };
    let free = |at: [f64; 3]| {
        let [x, z] = in_vehicle(truck.position, truck.heading, at);
        let in_truck = x.abs() < 2.4 && (head_z - 3.0..10.0).contains(&z);
        !in_truck && player.is_none_or(|player| apart(player, at) > 1.2)
    };
    let spot = spots.into_iter().find(|&spot| free(from_vehicle(position, heading, spot))).unwrap_or(spots[0]);
    let at = from_vehicle(position, heading, spot);
    let side = if spot[0] < 0.0 { -1.0 } else { 1.0 };
    Spot { at, off_to: from_vehicle(at, heading, [side * 5.0, -10.0]) }
}

/// The keys of the player's that the game couples (`attach`) and confirms (`activate`) with, as virtual keys: from
/// their profile's `controls.sii` (the one last played), T and Enter if it can't be read.
pub(super) struct Keys {
    attach: Vec<u32>,
    pub(super) activate: Vec<u32>,
}

/// The keyboard keys of a control (`mix <name> \`keyboard.t?0 | joy.b1?0\``) in a `controls.sii`.
fn keys_of(controls: &str, name: &str) -> Vec<u32> {
    let start = format!("mix {name} `");
    let Some(mix) = controls.lines().find_map(|line| line.split_once(start.as_str()).map(|(_, rest)| rest)) else {
        return Vec::new();
    };
    let mut keys: Vec<u32> = mix
        .split("keyboard.")
        .skip(1)
        .filter_map(|rest| {
            let key: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            virtual_key_of(&key)
        })
        .collect();
    keys.dedup();
    keys
}

/// A key as the game names it, as a Windows virtual key (the ones a player couples or confirms with).
fn virtual_key_of(name: &str) -> Option<u32> {
    let bytes = name.as_bytes();
    Some(match name {
        "enter" => 0x0d,
        "space" => 0x20,
        "tab" => 0x09,
        "back" | "backspace" => 0x08,
        _ if bytes.len() == 1 && bytes[0].is_ascii_alphanumeric() => u32::from(bytes[0].to_ascii_uppercase()),
        _ if bytes.len() >= 2 && bytes[0] == b'f' => {
            let number: u32 = name[1..].parse().ok().filter(|n| (1..=12).contains(n))?;
            0x70 + number - 1
        }
        _ => return None,
    })
}

impl Keys {
    pub(super) fn read() -> Self {
        let controls = crate::log::game_documents().and_then(|documents| {
            ["profiles", "steam_profiles"]
                .iter()
                .filter_map(|folder| std::fs::read_dir(documents.join(folder)).ok())
                .flatten()
                .filter_map(|profile| {
                    let file = profile.ok()?.path().join("controls.sii");
                    Some((std::fs::metadata(&file).ok()?.modified().ok()?, file))
                })
                .max_by_key(|(modified, _)| *modified)
                .and_then(|(_, file)| std::fs::read_to_string(file).ok())
        });
        let of = |name: &str, default: u32| {
            let keys = controls.as_deref().map(|controls| keys_of(controls, name)).unwrap_or_default();
            if keys.is_empty() { vec![default] } else { keys }
        };
        let keys = Self { attach: of("attach", u32::from(b'T')), activate: of("activate", 0x0d) };
        log!("freight: the player's keys: attach {:x?}, activate {:x?}", keys.attach, keys.activate);
        keys
    }
}

/// The keys kept from the game now (read by the keyboard hooks, any thread): up to two each of the player's
/// `attach` and `activate` keys (0: none), and E while on foot.
static KEPT: [AtomicU32; 4] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];
static USE_KEPT: AtomicBool = AtomicBool::new(false);

fn keep(slots: std::ops::Range<usize>, keys: &[u32], kept: bool) {
    for (i, slot) in slots.enumerate() {
        KEPT[slot].store(if kept { keys.get(i).copied().unwrap_or(0) } else { 0 }, Ordering::Release);
    }
}

fn keep_none() {
    keep(0..4, &[], false);
    USE_KEPT.store(false, Ordering::Release);
}

/// Whether the game is kept from `key` (a virtual key) by the cargo's papers.
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

/// Asks the app to show a step by hand (`NOTE_LOCK_PULL`…), its answer not yet given.
fn minigame(bridge: &westplay_shared::Bridge, kind: u32) {
    bridge.host.note_done.store(0, Ordering::Release);
    bridge.plugin.note_wanted.store(kind, Ordering::Release);
}

/// A notice of the papers', in the player's own words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Say {
    /// The attach key in the cab, the nota fiscal not collected.
    NoteToCouple,
    /// The attach key in the cab: coupling is on foot.
    CoupleOnFoot,
    NoteToLoad,
    SignatureToDeliver,
    NoteCollected,
    DeliverySigned,
    /// At the fifth wheel, the trailer's pin not over it.
    NotAligned,
    Coupled,
    /// The attach key in the cab, a trailer coupled: uncoupling is on foot.
    UncoupleOnFoot,
    Uncoupled,
    /// The papers collected, the player's own trailer not where the game loads it: it's loaded there by itself.
    LoadAtPlace,
    /// The coupling's steps by hand (owner 2026-11-19): the legs before the lock, the lock before the legs, and done.
    LowerLegsFirst,
    LockedRaiseLegs,
    LockFirst,
    LegsDown,
    LegsUp,
}

impl Say {
    fn icon(self) -> i32 {
        match self {
            Say::NoteCollected
            | Say::DeliverySigned
            | Say::Coupled
            | Say::Uncoupled
            | Say::LockedRaiseLegs
            | Say::LegsDown
            | Say::LegsUp => notice::ANNOUNCEMENT,
            _ => notice::ALERT,
        }
    }

    /// In the game's language (two letters), English if not one of these.
    fn text(self, language: &str) -> &'static CStr {
        let words: [&CStr; 16] = match language {
            "pt" => [
                c"Colete a nota fiscal com o conferente para engatar a carreta",
                c"Des\xc3\xa7a do caminh\xc3\xa3o e engate a carreta na quinta roda",
                c"Colete a nota fiscal com o conferente para carregar",
                c"Pegue a assinatura do destinat\xc3\xa1rio para entregar a carga",
                c"Nota fiscal coletada: engate liberado",
                c"Canhoto assinado: entrega liberada",
                c"Encoste a quinta roda sob o pino da carreta para engatar",
                c"Carreta engatada",
                c"Des\xc3\xa7a do caminh\xc3\xa3o e desengate a carreta pela lateral",
                c"Carreta desengatada",
                c"Estacione a carreta na \xc3\xa1rea de carregamento: ela \xc3\xa9 carregada sozinha",
                c"Abaixe os p\xc3\xa9s da carreta antes de desengatar (manivela ao lado dela)",
                c"Quinta roda travada: agora suba os p\xc3\xa9s da carreta",
                c"Encaixe a carreta e trave a quinta roda antes de subir os p\xc3\xa9s",
                c"P\xc3\xa9s da carreta abaixados",
                c"P\xc3\xa9s da carreta levantados",
            ],
            "es" => [
                c"Recoge la factura con el encargado para enganchar el remolque",
                c"Baja del cami\xc3\xb3n y engancha el remolque en la quinta rueda",
                c"Recoge la factura con el encargado para cargar",
                c"Consigue la firma del destinatario para entregar la carga",
                c"Factura recogida: enganche permitido",
                c"Albar\xc3\xa1n firmado: entrega permitida",
                c"Coloca la quinta rueda bajo el perno del remolque para enganchar",
                c"Remolque enganchado",
                c"Baja del cami\xc3\xb3n y desengancha el remolque por el lateral",
                c"Remolque desenganchado",
                c"Aparca el remolque en la zona de carga: se carga solo",
                c"Baja las patas del remolque antes de desenganchar",
                c"Quinta rueda bloqueada: ahora sube las patas del remolque",
                c"Engancha y bloquea la quinta rueda antes de subir las patas",
                c"Patas del remolque bajadas",
                c"Patas del remolque subidas",
            ],
            _ => [
                c"Collect the cargo's papers from the clerk to couple the trailer",
                c"Get out and couple the trailer at the fifth wheel",
                c"Collect the cargo's papers from the clerk to load",
                c"Get the recipient's signature to deliver the cargo",
                c"Papers collected: you may couple the trailer",
                c"Delivery signed: you may deliver",
                c"Back the fifth wheel under the trailer's pin to couple",
                c"Trailer coupled",
                c"Get out and uncouple the trailer at the truck's side",
                c"Trailer uncoupled",
                c"Park the trailer in the loading area: it's loaded by itself",
                c"Lower the trailer's legs before uncoupling (the crank at its side)",
                c"Fifth wheel locked: now raise the trailer's legs",
                c"Back under and lock the fifth wheel before raising the legs",
                c"Trailer legs lowered",
                c"Trailer legs raised",
            ],
        };
        words[self as usize]
    }
}

/// The clerk by the trailer: standing where they wait, or walking away.
struct Clerk {
    person: person::Person,
    at: [f64; 3],
    yaw: f32,
    /// Where they walk off to once done (`clerk_spot`).
    off_to: [f64; 3],
    /// Since when they walk away (their person the walking one then).
    leaving: Option<Instant>,
    /// How far the player was as the clerk was last put on their ground (metres; none: infinite).
    grounded_at: f64,
}

/// Coupling by hand, as far as it's got.
enum Coupling {
    /// A step by hand in the app since (the lock pulled or pushed, the legs cranked: its `NOTE_…` kind), answered
    /// done or put down.
    Minigame(u32, Instant),
    /// Down at the fifth wheel since.
    Kneeling(Instant),
    /// The game's `attach` held since.
    Attaching(Instant),
    /// Let go since: whether it took is seen after a moment.
    Waiting(Instant),
}

pub(super) struct Freight {
    /// Frames since the game's notices were last looked through, and whether its advice to couple was reworded
    /// yet (for the log).
    notices_in: u32,
    reworded: bool,
    papers: Papers,
    keys: Keys,
    clerk: Option<Clerk>,
    /// No clerk can be shown in this game (its people not found): the papers are waived, said once.
    no_clerk: bool,
    /// The job whose trailer was seen coupled once its nota fiscal was collected: the clerk, who waited to see it
    /// coupled, doesn't come back to it.
    coupled: String,
    /// The paper the app is showing (`PluginBlock::note_wanted`).
    paper: Option<u32>,
    coupling: Option<Coupling>,
    /// The trailer's legs up (owner 2026-11-19: lowered before uncoupling, raised after locking the fifth wheel to
    /// couple): up while the game has it coupled, down once it doesn't, as cranked in between; and the trailer's
    /// coupling as last seen.
    legs_up: bool,
    was_connected: Option<bool>,
    /// The fifth wheel's lock pushed home under the trailer's pin, the game not told yet: it couples as the legs are
    /// raised (owner 2026-11-19: "only then does the trailer couple, by itself").
    locked: bool,
    /// Whether the trailer was coupled as that began: it's being uncoupled.
    uncoupling: bool,
    /// The route's last distance not 0 (metres).
    last_route: f32,
    /// When coupling or uncoupling by hand was last done (`AFTER_BY_HAND`).
    done_by_hand: Option<Instant>,
    /// Since when the game's activate is held to load the trailer, the papers collected.
    loading: Option<Instant>,
    /// The papers collected, the trailer still to load (`load_prompt`), and whether the game said it could last
    /// time its notices were read.
    load_armed: bool,
    load_prompted: bool,
    /// The game said it's the unloading place (`Facts::at_end`), the delivery signed with the cargo still on, and
    /// whether the game said it could unload there last time its notices were read.
    at_unload: bool,
    unload_armed: bool,
    unload_prompted: bool,
    /// Which of the game's workers the clerk is (`CLERKS`).
    clerk_name: Option<&'static str>,
    /// What E does now, on foot (the key hints bar's).
    offer: Option<Action>,
    was_use: AtomicBool,
    was_attach: AtomicBool,
    was_activate: AtomicBool,
    said: Option<(Say, Instant)>,
    last: Instant,
}

impl Freight {
    fn new() -> Self {
        let papers = std::fs::read_to_string(papers_file()).ok().and_then(|text| Papers::from_text(&text));
        Self {
            notices_in: 0,
            reworded: false,
            papers: papers.unwrap_or_default(),
            keys: Keys::read(),
            clerk: None,
            no_clerk: false,
            coupled: String::new(),
            paper: None,
            coupling: None,
            legs_up: false,
            was_connected: None,
            locked: false,
            uncoupling: false,
            last_route: f32::MAX,
            done_by_hand: None,
            loading: None,
            load_armed: false,
            load_prompted: false,
            at_unload: false,
            unload_armed: false,
            unload_prompted: false,
            clerk_name: None,
            offer: None,
            was_use: AtomicBool::new(false),
            was_attach: AtomicBool::new(false),
            was_activate: AtomicBool::new(false),
            said: None,
            last: Instant::now(),
        }
    }
}

/// Where a game's papers are kept (Documents\WestPlay, by the game).
fn papers_file() -> std::path::PathBuf {
    let game = crate::log::game_dir().file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    crate::log::documents_dir().join("WestPlay").join(format!("freight-{game}.txt"))
}

/// What E does now on foot, for the key hints bar.
pub(super) fn offer(state: &State) -> Option<Action> {
    state.freight.as_ref().and_then(|freight| freight.offer)
}

fn on() -> bool {
    super::enabled()
        && crate::bridge::get().is_some_and(|bridge| bridge.host.freight_papers.load(Ordering::Acquire) == 1)
        && !crate::truckersmp::loaded()
}

/// Tells the player in the game's corner (its notices found), and in the log; the same again only after a while.
fn say(state: &mut State, what: Say) {
    let Some(freight) = state.freight.as_mut() else { return };
    if freight.said.is_some_and(|(said, at)| said == what && at.elapsed() < NOTICE_AGAIN) {
        return;
    }
    freight.said = Some((what, Instant::now()));
    log!("freight: notice {what:?}");
    // The game's code found first (its notices among it), on the game's main thread at the end of a frame.
    // SAFETY: the code is the game's, read as it lies.
    state.game.get_or_insert_with(|| unsafe { super::locate() });
    if let Some(Some(game)) = &state.game
        && let Some(notices) = game.notices.as_ref()
    {
        // SAFETY: on the game's main thread, at the end of a frame (`frame`'s contract).
        unsafe { notices.show(what.text(notice::language()), what.icon()) };
    }
}

/// Lets everything of the papers go: the keys back to the game, the clerk gone, no paper asked.
fn rest(state: &mut State) {
    keep_none();
    super::HELD_STILL.store(false, Ordering::Release);
    super::KNEELING.store(false, Ordering::Release);
    super::WORKING.store(false, Ordering::Release);
    crate::commands::hold(crate::commands::ATTACH, false);
    if let Some(bridge) = crate::bridge::get() {
        bridge.plugin.note_wanted.store(0, Ordering::Release);
    }
    let Some(freight) = state.freight.as_mut() else { return };
    (freight.paper, freight.coupling, freight.offer) = (None, None, None);
    if freight.loading.take().is_some() {
        crate::commands::hold(crate::commands::ACTIVATE, false);
        crate::commands::hold(crate::commands::ENGINE_OFF, false);
    }
    if let Some(clerk) = freight.clerk.take()
        && let Some(Some(game)) = &state.game
        && let Some(people) = game.people.as_ref()
    {
        // SAFETY: on the game's main thread, the clerk as `show_other` made them.
        unsafe { person::hide_other(people, clerk.person) };
    }
}

/// When the plugin is unloaded, or walking's objects go: the clerk with them.
pub(super) fn shutdown(state: &mut State) {
    rest(state);
}

/// Once a frame, at its end: the job's papers, the clerk, the keys kept from the game, and what E does on foot.
///
/// # Safety
/// On the game's main thread, at the end of a frame.
pub(super) unsafe fn frame(state: &mut State) {
    if !on() {
        notice::set_attach_words(None, None, None, None);
        if state.freight.is_some() {
            rest(state);
        }
        return;
    }
    // The game's advices from the cab, said otherwise as the game makes them (`notice::set_attach_words`): from the
    // first frame, the game's notices hooked (`super::locate`) and the words given, menu or not (owner 2026-11-12:
    // a game loaded at the unloading place showed its own words, made before they were given).
    let language = notice::language();
    notice::set_attach_words(
        Some(attach_words(language)),
        Some(unload_words(language)),
        Some(load_words(language)),
        Some(unload_cargo_words(language)),
    );
    // SAFETY: the code is the game's, read as it lies.
    state.game.get_or_insert_with(|| unsafe { super::locate() });
    let truck = crate::telemetry::truck();
    if truck.multiplayer || crate::telemetry::paused() {
        // The game's menu: nothing kept from it, the paper put down.
        if state.freight.as_ref().is_some_and(|freight| freight.paper.is_some() || freight.coupling.is_some()) {
            rest(state);
        }
        keep_none();
        return;
    }
    let freight = state.freight.get_or_insert_with(Freight::new);
    let dt = freight.last.elapsed().as_secs_f32().clamp(0.0, 0.1);
    freight.last = Instant::now();
    let trailer = crate::telemetry::trailer();
    let job = crate::telemetry::job();
    let navigation = crate::telemetry::navigation_distance();
    // The route's last distance not 0 (0: on its end, or no route at all; `Facts::at_end`).
    if navigation > 0.0 {
        freight.last_route = navigation;
    }
    let facts = Facts {
        job: job.as_ref().map(job_key),
        cargo_loaded: job.as_ref().is_some_and(|job| job.cargo_loaded),
        connected: trailer.connected,
        // (Its distance 0 too, as it was last near: parked on the spot the route ends at; owner's log 2026-11-11,
        // the clerk at the delivery walked away and came back as it went to 0 and back.)
        // Before loading, 0 is the shipper's yard: a job taken there with the player's own trailer has no route
        // yet (owner 2026-11-12: "the clerk doesn't show and Enter loads").
        at_end: job.is_some()
            && (if navigation > 0.0 {
                navigation
            } else if !job.as_ref().is_some_and(|job| job.cargo_loaded) {
                0.0
            } else {
                freight.last_route
            }) < NEAR
            // Or the game said it's the unloading place (its own words: `frame`'s notices), whatever the route says
            // (owner 2026-11-12: unloading with the player's trailer, 3 km left on the route, no clerk).
            || freight.at_unload,
    };
    let standing = truck.speed.abs() < STANDING;
    if freight.papers.update(&facts) {
        let papers = freight.papers.clone();
        log!(
            "freight: the job's papers: {} (own truck: {}, picked up: {}, signed: {})",
            if papers.key.is_empty() { "no job" } else { "a job" },
            papers.applies,
            papers.picked,
            papers.signed
        );
        // A game loaded mid-job finds them (a few bytes, as they change).
        if papers.judged || papers.key.is_empty() {
            let _ = std::fs::write(papers_file(), papers.to_text());
        }
    }
    let mut need = freight.papers.need(&facts);
    // Where the player is: on foot, their eyes; else the truck.
    let on_foot = state.walking.as_ref().filter(|walking| walking.climb.is_none() && walking.descend.is_none());
    // SAFETY: the walking camera is held (one reference); read guarded.
    let player =
        on_foot.and_then(|walking| unsafe { super::read::<Placement>(walking.camera as usize + super::PLACEMENT) });
    let player = player.map(|placement| placement.world());
    // The ground where the player stands, on foot (their eyes less how far under them their feet are): the clerk
    // stands on it (owner 2026-11-11: "the feet inside the ground", at the trailer's own height, which isn't its
    // wheels' ground on every trailer).
    let physics = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.physics.as_ref());
    // SAFETY: the walking camera is held (one reference); `physics` as `locate` found it.
    let ground = on_foot.zip(player).map(|(walking, eyes)| eyes[1] - unsafe { super::feet_under(walking, physics) });
    // How far over the truck's own point the ground is, learnt as the player stands by it (for a clerk put there
    // from the cab: owner 2026-11-11, "in the ground to the waist" at the truck's point).
    if let (Some(ground), Some(player)) = (ground, player)
        && truck.speed.abs() < STANDING
        && apart(player, truck.position) < 12.0
    {
        let over = ground - truck.position[1];
        if (-0.5..2.0).contains(&over) {
            TRUCK_GROUND.store(over.to_bits(), Ordering::Release);
        }
    }
    // The game's own advice to couple from the cab ("press T"), reworded where it stands in its list: the words
    // given through the mod aren't taken (owner 2026-11-11, "the T still shows").
    freight.notices_in += 1;
    if freight.notices_in >= 10 {
        freight.notices_in = 0;
        // SAFETY: the code is the game's, read as it lies.
        state.game.get_or_insert_with(|| unsafe { super::locate() });
        if let Some(notices) = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.notices.as_ref())
        {
            // SAFETY: on the game's main thread (`frame`'s contract).
            let reworded = unsafe {
                notices.reword(
                    |text| text.starts_with("@@attach_load@@"),
                    attach_words(notice::language()).to_str().unwrap_or_default(),
                )
            };
            if reworded > 0 && !freight.reworded {
                log!("freight: the game's advice to couple with T reworded ({reworded})");
                freight.reworded = true;
            }
            // The papers collected, the trailer not loaded where it stood: loaded as soon as the game says it can
            // load it there (the trailer in its loading place: owner 2026-11-12, "once I collect the papers").
            // SAFETY: on the game's main thread (`frame`'s contract).
            let texts = if facts.job.is_some() { unsafe { notices.texts() } } else { Vec::new() };
            // The unloading place, as the game says it: there the recipient signs, then it unloads by itself.
            let ours_unload = unload_cargo_words(notice::language()).to_str().unwrap_or_default();
            let unload_here = texts.iter().find(|text| unload_prompt(text) || text.trim_end() == ours_unload);
            if facts.cargo_loaded
                && !freight.at_unload
                && let Some(text) = unload_here.or_else(|| texts.iter().find(|text| text.contains("@@cargo_unloading")))
            {
                log!("freight: the game says it's the unloading place ({text:?})");
                freight.at_unload = true;
            }
            if freight.unload_armed {
                if let Some(text) = unload_here
                    && !freight.unload_prompted
                    && freight.loading.is_none()
                    && crate::commands::registered()
                {
                    crate::commands::hold(crate::commands::ACTIVATE, true);
                    freight.loading = Some(Instant::now());
                    log!("freight: the game can unload the trailer here ({text:?}): unloading");
                }
            }
            freight.unload_prompted = unload_here.is_some();
            if freight.load_armed {
                // (Its own words, or ours given it as it's made: \`notice::set_attach_words\`.)
                let ours = load_words(notice::language()).to_str().unwrap_or_default();
                let prompt = texts.iter().find(|text| load_prompt(text) || text.trim_end() == ours);
                if let Some(text) = prompt
                    && !freight.load_prompted
                    && freight.loading.is_none()
                    && crate::commands::registered()
                {
                    crate::commands::hold(crate::commands::ACTIVATE, true);
                    freight.loading = Some(Instant::now());
                    log!("freight: the game can load the trailer here ({text:?}): loading");
                }
                freight.load_prompted = prompt.is_some();
            }
        }
    }
    if facts.cargo_loaded || !facts.connected || facts.job.is_none() {
        freight.load_armed = false;
    }
    if !facts.cargo_loaded || facts.job.is_none() {
        (freight.at_unload, freight.unload_armed) = (false, false);
    }

    // The clerk: by the trailer while a paper is needed there (at the end of a route, once the truck stands; they
    // stay as it's moved about the yard), and, the nota fiscal collected, until the trailer is coupled.
    let papers = &freight.papers;
    if papers.judged && papers.applies && papers.picked && trailer.connected && freight.coupled != papers.key {
        freight.coupled = papers.key.clone();
    }
    let waits_to_see_it_coupled = papers.judged
        && papers.applies
        && papers.picked
        && !papers.signed
        && !trailer.connected
        && freight.coupled != papers.key;
    let stands = freight.clerk.as_ref().is_some_and(|clerk| clerk.leaving.is_none());
    let wanted = trailer.placed
        && match need {
            Need::None => waits_to_see_it_coupled && apart(truck.position, trailer.position) < CLERK_SEEN,
            Need::Pickup => apart(truck.position, trailer.position) < CLERK_SEEN,
            Need::Load | Need::Delivery => facts.at_end && (standing || stands),
        };
    // Only once the player is out of the truck (owner 2026-11-11: "show them only when I get out"): the ground
    // under their feet known then, the clerk not put on a guess of it seen from the cab; staying after, the player
    // back in or not.
    if wanted
        && ground.is_some()
        && !freight.no_clerk
        && freight.clerk.as_ref().is_none_or(|clerk| clerk.leaving.is_some())
    {
        // SAFETY: the code is the game's, read as it lies.
        state.game.get_or_insert_with(|| unsafe { super::locate() });
        let people = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.people.as_ref());
        let Spot { at, off_to } = clerk_spot(&trailer, &truck, player);
        // On the ground: the player's, on foot near; else the truck's (its own point is on its wheels' ground).
        let at = [at[0], ground.unwrap_or_else(|| truck_ground(&truck)), at[2]];
        log!(
            "freight: the clerk's spot {:?} from the truck, {:?} from the trailer",
            in_vehicle(truck.position, truck.heading, at).map(|v| (v * 10.0).round() / 10.0),
            in_vehicle(trailer.position, trailer.heading, at).map(|v| (v * 10.0).round() / 10.0)
        );
        let yaw = heading_of(truck.position[0] - at[0], truck.position[2] - at[2]);
        // One of the game's own workers (owner 2026-11-11: "not my person: the game's workers"), the same for a
        // job, another at its delivery; the game's worker group or its man standing where the mod's aren't.
        let clerk = clerk_of(&freight.papers.key, need == Need::Delivery);
        let names = [format!("{clerk}_idle"), "work_idle".to_owned(), "m_idle01".to_owned()];
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        freight.clerk_name = Some(clerk);
        // One clerk at a time: the one walking away goes at once.
        if let (Some(leaving), Some(people)) = (freight.clerk.take(), people) {
            // SAFETY: on the game's main thread, the clerk as `show_other` made them.
            unsafe { person::hide_other(people, leaving.person) };
        }
        // SAFETY: on the game's main thread, `people` as `locate` found it.
        let shown = people.map(|people| unsafe {
            person::show_other(people, &Placement::at(at, yaw / std::f32::consts::TAU), &names)
        });
        match shown {
            Some(Ok(person)) => {
                log!("freight: the clerk stands by the trailer");
                let grounded_at = player.filter(|_| ground.is_some()).map_or(f64::INFINITY, |p| apart(p, at));
                freight.clerk = Some(Clerk { person, at, yaw, off_to, leaving: None, grounded_at });
            }
            other => {
                log!(
                    "freight: no clerk ({}): the papers are waived",
                    other.and_then(Result::err).unwrap_or("no people")
                );
                freight.no_clerk = true;
            }
        }
    }
    // No clerk to sign with: nothing asked of the player (coupling by hand still is).
    if freight.no_clerk && need != Need::None {
        match need {
            Need::Pickup | Need::Load => freight.papers.picked = true,
            _ => freight.papers.signed = true,
        }
        need = Need::None;
    }
    // Standing, turning to the player as they come; no longer needed, walking away, then gone.
    let mut gone = false;
    if let Some(clerk) = freight.clerk.as_mut()
        && let Some(people) = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.people.as_ref())
    {
        if clerk.leaving.is_none() && !wanted {
            let walker = format!("{}_walk", freight.clerk_name.unwrap_or(CLERKS[0]));
            // Off along the open way they stood by, ahead and outwards (`clerk_spot`).
            let to = clerk.off_to;
            clerk.yaw = heading_of(to[0] - clerk.at[0], to[2] - clerk.at[2]);
            let placement = Placement::at(clerk.at, clerk.yaw / std::f32::consts::TAU);
            // SAFETY: on the game's main thread, `people` as `locate` found it.
            match unsafe { person::show_other(people, &placement, &[walker.as_str(), "n_ca_man_wlk"]) } {
                Ok(walker) => {
                    let standing = std::mem::replace(&mut clerk.person, walker);
                    // SAFETY: the standing one as `show_other` made them; the walker is the one drawn from here.
                    unsafe {
                        person::hide_other(people, standing);
                        person::place_other(people, &mut clerk.person, &placement, 0.0);
                    }
                    clerk.leaving = Some(Instant::now());
                    log!("freight: the clerk walks away");
                }
                Err(_) => gone = true,
            }
        } else if let Some(since) = clerk.leaving {
            let ahead = super::ahead_of(clerk.yaw, CLERK_PACE * f64::from(dt));
            clerk.at = [clerk.at[0] + ahead[0], clerk.at[1], clerk.at[2] + ahead[1]];
            let placement = Placement::at(clerk.at, clerk.yaw / std::f32::consts::TAU);
            // SAFETY: on the game's main thread, the clerk as `show_other` made them.
            unsafe { person::place_other(people, &mut clerk.person, &placement, dt) };
            gone = since.elapsed() >= CLERK_LEAVES_FOR;
        } else {
            // On the player's ground, once, as they come near on foot (the yard's flat: theirs is the clerk's). Not
            // followed after: the eyes go up and down with the steps, and so did the clerk ("shaking": owner
            // 2026-11-11).
            // (Right by them: owner 2026-11-11, measured 30 m off, on other ground, they floated or sank.)
            if player.is_none_or(|player| apart(player, clerk.at) > GROUND_AGAIN) {
                clerk.grounded_at = f64::INFINITY;
            }
            if let (Some(ground), Some(player)) = (ground, player)
                && let away = apart(player, clerk.at)
                && away < GROUND_NEAR
                && away < clerk.grounded_at * GROUND_CLOSER
            {
                if (ground - clerk.at[1]).abs() > 0.03 {
                    log!(
                        "freight: the clerk put on the player's ground {away:.1} m away ({ground:.2}; was {:.2}, the truck's point {:.2}, the trailer's {:.2})",
                        clerk.at[1],
                        truck.position[1],
                        trailer.position[1]
                    );
                    clerk.at[1] = ground;
                }
                clerk.grounded_at = away;
            }
            if let Some(player) = player.filter(|&player| apart(player, clerk.at) < 8.0) {
                let to = heading_of(player[0] - clerk.at[0], player[2] - clerk.at[2]);
                clerk.yaw = turn_toward(clerk.yaw, to, 3.0 * dt);
            }
            let placement = Placement::at(clerk.at, clerk.yaw / std::f32::consts::TAU);
            // SAFETY: on the game's main thread, the clerk as `show_other` made them.
            unsafe { person::place_other(people, &mut clerk.person, &placement, dt) };
        }
    }
    if gone
        && let Some(clerk) = freight.clerk.take()
        && let Some(people) = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.people.as_ref())
    {
        // SAFETY: on the game's main thread, the clerk as `show_other` made them.
        unsafe { person::hide_other(people, clerk.person) };
    }

    // The keys kept from the game: coupling is by hand (the attach key does nothing while no trailer is coupled),
    // loading and delivering wait for their paper at the end of the route; E is the plugin's on foot.
    let blocks_activate = matches!(need, Need::Load | Need::Delivery) && facts.at_end;
    // (Only while the game takes the plugin's own `attach`: without its device, the key stays the player's.)
    let by_hand = crate::commands::registered();
    keep(0..2, &freight.keys.attach.clone(), by_hand);
    keep(2..4, &freight.keys.activate.clone(), blocks_activate);
    USE_KEPT.store(on_foot.is_some(), Ordering::Release);
    let shown = crate::d3d::game_shown();
    let down = |keys: &[u32]| shown && keys.iter().any(|&key| really_down(VIRTUAL_KEY(key as u16)));
    let attach_pressed = crate::input::newly_down(&freight.was_attach, down(&freight.keys.attach));
    let activate_pressed = crate::input::newly_down(&freight.was_activate, down(&freight.keys.activate));
    let use_down = shown && really_down(VIRTUAL_KEY(super::virtual_key(USE_KEY)));
    let use_pressed = crate::input::newly_down(&freight.was_use, use_down) && on_foot.is_some();
    let mut to_say = None;
    if attach_pressed && by_hand && freight.coupling.is_none() {
        to_say = Some(if trailer.connected {
            Say::UncoupleOnFoot
        } else if need == Need::Pickup {
            Say::NoteToCouple
        } else {
            Say::CoupleOnFoot
        });
    }
    if activate_pressed && blocks_activate {
        to_say = Some(if need == Need::Load { Say::NoteToLoad } else { Say::SignatureToDeliver });
    }

    // The paper the app shows: collected (or put down) as it says.
    let bridge = crate::bridge::get();
    if let (Some(kind), Some(bridge)) = (freight.paper, bridge) {
        let done = bridge.host.note_done.swap(0, Ordering::AcqRel);
        if done != 0 || on_foot.is_none() {
            freight.paper = None;
            bridge.plugin.note_wanted.store(0, Ordering::Release);
            super::HELD_STILL.store(false, Ordering::Release);
            if done == kind && done != NOTE_LEFT {
                if kind == NOTE_PICKUP {
                    freight.papers.picked = true;
                    to_say = Some(Say::NoteCollected);
                    // The player's own trailer, coupled, not loaded: loaded at once, as the game's Enter would
                    // (owner 2026-11-12: "I sign, collect, and it's loaded right after").
                    if facts.connected && !facts.cargo_loaded && crate::commands::registered() {
                        // The engine off first: the game loads only with it off (owner 2026-11-12: the trailer on
                        // its green place, the engine running, nothing loaded); its "load" comes then (`load_prompt`).
                        if crate::telemetry::engine_on() {
                            crate::commands::hold(crate::commands::ENGINE_OFF, true);
                            log!("freight: the engine switched off to load");
                        }
                        crate::commands::hold(crate::commands::ACTIVATE, true);
                        freight.loading = Some(Instant::now());
                        freight.load_armed = true;
                        log!("freight: the papers collected: the trailer loaded (the game's activate given)");
                    }
                } else {
                    freight.papers.signed = true;
                    to_say = Some(Say::DeliverySigned);
                    // Unloaded once the game says it can unload there (the player's own trailer: the game's Enter;
                    // a trailer of the company's is delivered as it's uncoupled): now if it says so already.
                    if facts.connected && facts.cargo_loaded {
                        freight.unload_armed = true;
                        // The engine off, as for loading: the game unloads only with it off (its "unload" then).
                        if crate::telemetry::engine_on() && crate::commands::registered() && freight.loading.is_none() {
                            crate::commands::hold(crate::commands::ENGINE_OFF, true);
                            freight.loading = Some(Instant::now());
                            log!("freight: the engine switched off to unload");
                        }
                        if freight.unload_prompted && crate::commands::registered() {
                            crate::commands::hold(crate::commands::ACTIVATE, true);
                            freight.loading = Some(Instant::now());
                            log!("freight: the delivery signed: the trailer unloaded (the game's activate given)");
                        }
                    }
                }
                let _ = std::fs::write(papers_file(), freight.papers.to_text());
            }
        }
    }

    // On foot: E by the clerk signs, E at the fifth wheel couples.
    let hook = crate::telemetry::truck_hook().map(|hook| {
        let [x, y, z] = from_vehicle(truck.position, truck.heading, [f64::from(hook[0]), f64::from(hook[2])]);
        [x, y + f64::from(hook[1]), z]
    });
    let pin = trailer
        .hook
        .filter(|_| trailer.placed)
        .map(|pin| from_vehicle(trailer.position, trailer.heading, [f64::from(pin[0]), f64::from(pin[2])]));
    if freight.was_connected != Some(trailer.connected) {
        freight.was_connected = Some(trailer.connected);
        freight.legs_up = trailer.connected;
        freight.locked = false;
    }
    if freight.locked && truck.speed.abs() >= STANDING {
        freight.locked = false;
        log!("freight: the truck moved off: the fifth wheel's lock opened again");
    }
    // The steps by hand need the app to show them; without it, coupling and uncoupling as before.
    let by_steps = bridge.filter(|_| crate::bridge::host_alive());
    // At the trailer's side, by its legs.
    let by_legs = |player: [f64; 3]| {
        trailer.placed
            && trailer.hook.is_some_and(|pin| {
                let [side, back] = in_vehicle(trailer.position, trailer.heading, player);
                SIDE_OUT.contains(&side.abs()) && LEGS_BEHIND_PIN.contains(&(back - f64::from(pin[2])))
            })
    };
    freight.offer = None;
    if let Some(player) = player.filter(|_| freight.paper.is_none() && freight.coupling.is_none()) {
        let by_clerk = need != Need::None
            && freight
                .clerk
                .as_ref()
                .is_some_and(|clerk| clerk.leaving.is_none() && apart(player, clerk.at) < CLERK_REACH);
        // At the truck's side, from behind the cab to the fifth wheel: coupled or uncoupled there. At a pump the
        // same place fills the tank (`refuel`), which comes first while it can.
        let by_hook = by_hand && by_fifth_wheel(player, &truck) && !super::refuel::fills(player, &truck);
        if by_hook && use_pressed {
            // Where they stood, to tell the place's size from the owner's tests (the truck's own space, metres).
            let [side, back] = in_vehicle(truck.position, truck.heading, player);
            log!("freight: E at {side:.2} m to the side, {:.2} m from the fifth wheel", back - hook_back(&truck));
        }
        if by_clerk {
            freight.offer = Some(Action::SignNote);
            if use_pressed && let Some(bridge) = bridge {
                let kind = if need == Need::Delivery { NOTE_DELIVERY } else { NOTE_PICKUP };
                bridge.host.note_done.store(0, Ordering::Release);
                bridge.plugin.note_wanted.store(kind, Ordering::Release);
                freight.paper = Some(kind);
                super::HELD_STILL.store(true, Ordering::Release);
                log!("freight: the paper asked of the clerk ({kind})");
            }
        } else if by_hook && freight.done_by_hand.is_some_and(|at| at.elapsed() < AFTER_BY_HAND) {
            // Just coupled or uncoupled by hand: E there does nothing a while (owner 2026-11-11: uncoupled, then a
            // second E as the game finished it coupled the trailer again, "the animation twice").
            if use_pressed {
                log!("freight: E at the fifth wheel just after coupling or uncoupling: nothing done");
            }
        } else if by_hook {
            // Coupling only offered with a trailer there to couple (its pin near the fifth wheel).
            let near = match (hook, pin) {
                (Some(hook), Some(pin)) => apart(hook, pin) < TRAILER_NEAR,
                (None, Some(_)) => apart(truck.position, trailer.position) < 20.0,
                _ => false,
            };
            freight.offer = if trailer.connected {
                Some(Action::Uncouple)
            } else if near {
                Some(Action::Couple)
            } else {
                None
            };
            if use_pressed && trailer.connected && need == Need::Delivery && facts.at_end {
                // At a delivery, the recipient signs first: uncoupled, the game delivers (owner 2026-11-11: "it
                // delivers without my handing in the papers").
                to_say = Some(Say::SignatureToDeliver);
            } else if use_pressed && trailer.connected && by_steps.is_some() && freight.legs_up {
                // The legs down first (owner 2026-11-19), at the trailer's side.
                to_say = Some(Say::LowerLegsFirst);
            } else if use_pressed && trailer.connected {
                // The lock pulled in the app first, if it's there to show it.
                if let Some(bridge) = by_steps {
                    minigame(bridge, NOTE_LOCK_PULL);
                    freight.coupling = Some(Coupling::Minigame(NOTE_LOCK_PULL, Instant::now()));
                } else {
                    freight.coupling = Some(Coupling::Kneeling(Instant::now()));
                }
                freight.uncoupling = true;
                super::HELD_STILL.store(true, Ordering::Release);
                super::KNEELING.store(true, Ordering::Release);
                super::WORKING.store(true, Ordering::Release);
                log!("freight: uncoupling by hand");
            } else if use_pressed && near {
                let aligned = match (hook, pin) {
                    (Some(hook), Some(pin)) => apart(hook, pin) < COUPLING_REACH,
                    // Where either is isn't known: the game says whether it couples.
                    _ => true,
                };
                if need == Need::Pickup {
                    to_say = Some(Say::NoteToCouple);
                } else if !aligned {
                    to_say = Some(Say::NotAligned);
                } else if freight.locked {
                    to_say = Some(Say::LockedRaiseLegs);
                } else if let Some(bridge) = by_steps {
                    minigame(bridge, NOTE_LOCK_PUSH);
                    freight.coupling = Some(Coupling::Minigame(NOTE_LOCK_PUSH, Instant::now()));
                    freight.uncoupling = false;
                    super::HELD_STILL.store(true, Ordering::Release);
                    super::KNEELING.store(true, Ordering::Release);
                    super::WORKING.store(true, Ordering::Release);
                    log!("freight: coupling by hand: the fifth wheel's lock shown to push");
                } else {
                    freight.coupling = Some(Coupling::Kneeling(Instant::now()));
                    freight.uncoupling = false;
                    super::HELD_STILL.store(true, Ordering::Release);
                    super::KNEELING.store(true, Ordering::Release);
                    super::WORKING.store(true, Ordering::Release);
                    log!("freight: coupling by hand");
                }
            }
        } else if let Some(bridge) = by_steps.filter(|_| by_legs(player)) {
            // The trailer's legs, cranked: down or up while coupled; up once the lock is pushed (it couples then).
            let raise = if trailer.connected { !freight.legs_up } else { freight.locked };
            freight.offer = if raise { Some(Action::RaiseLegs) } else if trailer.connected { Some(Action::LowerLegs) } else { None };
            if use_pressed && !trailer.connected && !freight.locked {
                to_say = Some(Say::LockFirst);
            } else if use_pressed {
                let kind = if raise { NOTE_LEGS_UP } else { NOTE_LEGS_DOWN };
                minigame(bridge, kind);
                freight.coupling = Some(Coupling::Minigame(kind, Instant::now()));
                freight.uncoupling = false;
                // Standing still at the crank, no kneeling (owner 2026-11-19: "crouching makes no sense there; maybe a
                // turning animation one day").
                super::HELD_STILL.store(true, Ordering::Release);
                log!("freight: the trailer's legs to crank {}", if raise { "up" } else { "down" });
            }
        }
    }
    // Loading after the papers: the game's activate let go again a moment after.
    if freight.loading.is_some_and(|at| at.elapsed() >= LOAD_HELD) {
        crate::commands::hold(crate::commands::ACTIVATE, false);
        crate::commands::hold(crate::commands::ENGINE_OFF, false);
        freight.loading = None;
        log!("freight: loading asked of the game (the cargo loaded: {})", facts.cargo_loaded);
        // Not there yet: where to take it (the loading armed: `load_prompt`). The engine still stopping: the game
        // says it can load as it stops.
        if freight.load_armed && !facts.cargo_loaded && to_say.is_none() && !crate::telemetry::engine_on() {
            to_say = Some(Say::LoadAtPlace);
        }
    }
    // Coupling by hand: down at the fifth wheel, the game's own `attach`, then whether it took.
    match freight.coupling {
        // A step by hand answered in the app: the lock pulled (uncoupled now), pushed (locked: the legs next), the
        // legs cranked (raised after the lock: coupled now); put down (or too long), up and nothing done.
        Some(Coupling::Minigame(kind, since)) if on_foot.is_some() => {
            let done = bridge.map_or(0, |bridge| bridge.host.note_done.swap(0, Ordering::AcqRel));
            let given_up = done == NOTE_LEFT || since.elapsed() >= LEVER_FOR || !crate::bridge::host_alive();
            if (done == kind || given_up)
                && let Some(bridge) = bridge
            {
                bridge.plugin.note_wanted.store(0, Ordering::Release);
            }
            let up = || {
                super::HELD_STILL.store(false, Ordering::Release);
                super::KNEELING.store(false, Ordering::Release);
                super::WORKING.store(false, Ordering::Release);
            };
            let attach = |freight: &mut Freight, uncoupling: bool| {
                super::WORKING.store(false, Ordering::Release);
                crate::commands::hold(crate::commands::ATTACH, true);
                super::steps::door(super::steps::Door::Shut, 0.0);
                freight.uncoupling = uncoupling;
                freight.coupling = Some(Coupling::Attaching(Instant::now()));
            };
            if done == kind {
                match kind {
                    NOTE_LOCK_PULL => {
                        attach(freight, true);
                        log!("freight: the fifth wheel's lock pulled: uncoupling");
                    }
                    NOTE_LOCK_PUSH => {
                        up();
                        freight.locked = true;
                        freight.coupling = None;
                        super::steps::door(super::steps::Door::Shut, 0.0);
                        to_say = Some(Say::LockedRaiseLegs);
                        log!("freight: the fifth wheel's lock pushed: the legs to raise next");
                    }
                    NOTE_LEGS_UP if !trailer.connected && freight.locked => {
                        freight.locked = false;
                        freight.legs_up = true;
                        attach(freight, false);
                        log!("freight: the legs raised: coupling");
                    }
                    _ => {
                        up();
                        freight.legs_up = kind == NOTE_LEGS_UP;
                        freight.coupling = None;
                        to_say = Some(if freight.legs_up { Say::LegsUp } else { Say::LegsDown });
                        log!("freight: the trailer's legs {}", if freight.legs_up { "up" } else { "down" });
                    }
                }
            } else if given_up {
                up();
                freight.coupling = None;
                log!("freight: the step by hand ({kind}) put down: nothing done");
            }
        }
        Some(Coupling::Kneeling(since)) if since.elapsed() >= COUPLING_TIME => {
            // Getting up from the work as the trailer is coupled (the person's `work` plays on to its end).
            super::WORKING.store(false, Ordering::Release);
            crate::commands::hold(crate::commands::ATTACH, true);
            // The lock's clunk (the door's sound: the plugin has no other).
            super::steps::door(super::steps::Door::Shut, 0.0);
            freight.coupling = Some(Coupling::Attaching(Instant::now()));
        }
        Some(Coupling::Attaching(since)) if since.elapsed() >= ATTACH_HELD => {
            crate::commands::hold(crate::commands::ATTACH, false);
            super::HELD_STILL.store(false, Ordering::Release);
            super::KNEELING.store(false, Ordering::Release);
            super::WORKING.store(false, Ordering::Release);
            freight.coupling = Some(Coupling::Waiting(Instant::now()));
        }
        // (Uncoupling is done once the trailer is no longer coupled; coupling, once it is.)
        Some(Coupling::Waiting(since))
            if trailer.connected != freight.uncoupling
                || since.elapsed() >= if freight.uncoupling { UNCOUPLED_WITHIN } else { COUPLED_WITHIN } =>
        {
            freight.coupling = None;
            freight.done_by_hand = Some(Instant::now());
            log!("freight: by hand (uncoupling: {}): the trailer coupled: {}", freight.uncoupling, trailer.connected);
            to_say = match (freight.uncoupling, trailer.connected) {
                (false, true) => Some(Say::Coupled),
                (false, false) => Some(Say::NotAligned),
                (true, false) => Some(Say::Uncoupled),
                // Still coupled: the game says why itself (the truck moving, the trailer's legs).
                (true, true) => None,
            };
        }
        Some(_) if on_foot.is_none() => {
            if let Some(bridge) = bridge
                && (NOTE_LOCK_PULL..=NOTE_LEGS_DOWN).contains(&bridge.plugin.note_wanted.load(Ordering::Acquire))
            {
                bridge.plugin.note_wanted.store(0, Ordering::Release);
            }
            crate::commands::hold(crate::commands::ATTACH, false);
            super::HELD_STILL.store(false, Ordering::Release);
            super::KNEELING.store(false, Ordering::Release);
            super::WORKING.store(false, Ordering::Release);
            freight.coupling = None;
        }
        _ => {}
    }
    if let Some(what) = to_say {
        say(state, what);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(job: bool, cargo_loaded: bool, connected: bool) -> Facts {
        Facts { job: job.then(|| "a job".to_owned()), cargo_loaded, connected, at_end: false }
    }

    fn judged(papers: &mut Papers, facts: &Facts) {
        for _ in 0..JUDGED_AFTER {
            papers.update(facts);
        }
    }

    /// A trailer to fetch with the player's own truck: the nota fiscal before it's coupled, the recipient's
    /// signature before it's delivered, each once.
    #[test]
    fn a_job_with_the_players_truck_needs_its_papers() {
        let mut papers = Papers::default();
        let waiting = facts(true, true, false);
        papers.update(&waiting);
        assert_eq!(papers.need(&waiting), Need::None, "not before the job is judged");
        judged(&mut papers, &waiting);
        assert_eq!(papers.need(&waiting), Need::Pickup);
        papers.picked = true;
        assert_eq!(papers.need(&waiting), Need::None, "collected: the trailer may be coupled");
        let hauling = facts(true, true, true);
        papers.update(&hauling);
        assert_eq!(papers.need(&hauling), Need::Delivery);
        papers.signed = true;
        assert_eq!(papers.need(&hauling), Need::None);
        // Delivered: the next job starts anew.
        papers.update(&facts(false, false, false));
        assert_eq!(papers, Papers::default());
    }

    /// A quick job (the company's truck, its loaded trailer coupled as it starts) has no papers.
    #[test]
    fn a_quick_job_has_no_papers() {
        let mut papers = Papers::default();
        let quick = facts(true, true, true);
        judged(&mut papers, &quick);
        assert!(!papers.applies);
        assert_eq!(papers.need(&quick), Need::None);
    }

    /// The player's own trailer, coupled and empty: the nota fiscal before it's loaded.
    #[test]
    fn the_players_own_trailer_needs_the_note_to_load() {
        let mut papers = Papers::default();
        let empty = facts(true, false, true);
        judged(&mut papers, &empty);
        assert_eq!(papers.need(&empty), Need::Load);
        // Loaded all the same: nothing more to ask of the pick-up; the delivery still is signed for.
        let loaded = facts(true, true, true);
        papers.update(&loaded);
        assert_eq!(papers.need(&loaded), Need::Delivery);
    }

    #[test]
    fn the_papers_are_kept_by_the_job() {
        let mut papers = Papers::default();
        let waiting = facts(true, true, false);
        judged(&mut papers, &waiting);
        papers.picked = true;
        let kept = Papers::from_text(&papers.to_text()).unwrap();
        assert_eq!(
            (kept.key.as_str(), kept.judged, kept.applies, kept.picked, kept.signed),
            ("a job", true, true, true, false)
        );
        // The same job in a game loaded again: not asked again; another job: anew.
        let mut again = kept.clone();
        assert!(!again.update(&waiting));
        assert_eq!(again.need(&waiting), Need::None);
        let other = Facts { job: Some("another".into()), ..waiting };
        again.update(&other);
        assert!(!again.judged);
        assert_eq!(Papers::from_text(""), None);
    }

    #[test]
    fn reads_the_players_keys_from_their_controls() {
        let controls = "config_lines[351]: \"mix attach `keyboard.t?0 || long_press(joy.pov1_left?0) | semantical.attach?0`\"\n\
                        config_lines[344]: \"mix activate `keyboard.enter?0 | joy.b1?0 | semantical.activate?0`\"\n\
                        config_lines[9]: \"mix other `keyboard.f7?0 | keyboard.lshift?0`\"";
        assert_eq!(keys_of(controls, "attach"), [u32::from(b'T')]);
        assert_eq!(keys_of(controls, "activate"), [0x0d]);
        assert_eq!(keys_of(controls, "other"), [0x76]);
        assert_eq!(keys_of(controls, "missing"), Vec::<u32>::new());
    }

    fn truck_at(position: [f64; 3], heading: f32) -> Truck {
        Truck {
            position,
            heading,
            speed: 0.0,
            pitch: 0.0,
            roll: 0.0,
            parking_brake: true,
            multiplayer: false,
            head: [-0.7, 1.5, -1.0],
        }
    }

    /// A trailer standing alone: the clerk in front of its nose, out to the driver's side of where the truck backs
    /// in; with the truck parked there, the other side; never on the player.
    #[test]
    fn by_a_lone_trailer_the_clerk_stands_before_its_nose() {
        let trailer =
            Trailer { connected: false, placed: true, position: [100.0, 0.0, 200.0], heading: 0.0, pitch: 0.0, roll: 0.0, hook: None };
        let far = truck_at([300.0, 0.0, 300.0], 0.0);
        let spot = clerk_spot(&trailer, &far, None);
        assert_eq!(in_vehicle(trailer.position, 0.0, spot.at).map(|v| v.round()), [-3.0, -4.0]);
        // Off ahead and outwards.
        let off = in_vehicle(trailer.position, 0.0, spot.off_to);
        assert!(off[0] < -3.0 && off[1] < -4.0);
        // The truck standing right on that spot (not at the trailer): the other side.
        let there = truck_at([97.0, 0.0, 188.0], 0.0);
        let spot = clerk_spot(&trailer, &there, None);
        assert!(in_vehicle(trailer.position, 0.0, spot.at)[0] > 0.0);
        // Nor on the player.
        let spot = clerk_spot(&trailer, &far, Some([97.0, 0.0, 196.0]));
        assert!(apart(spot.at, [97.0, 0.0, 196.0]) > 1.2);
    }

    /// The truck at its trailer (backed up to it, or coupled): the clerk in front of the truck, on the driver's
    /// side, clear of it (owner 2026-11-12: beside the trailer, a wall); on the player, the other side.
    #[test]
    fn with_the_truck_at_the_trailer_the_clerk_stands_in_front_of_it() {
        let trailer =
            Trailer { connected: false, placed: true, position: [100.0, 0.0, 200.0], heading: 0.25, pitch: 0.0, roll: 0.0, hook: None };
        // Turned a quarter, its fifth wheel under the pin: the truck ahead of the trailer.
        let truck =
            truck_at(from_vehicle([100.0, 0.0, 200.0], 0.25, [0.0, -hook_back(&truck_at([0.0; 3], 0.0))]), 0.25);
        let spot = clerk_spot(&trailer, &truck, None);
        let [x, z] = in_vehicle(truck.position, truck.heading, spot.at);
        assert!((x - -FRONT_SIDE).abs() < 0.01, "the driver's side: {x}");
        assert!((z - (f64::from(truck.head[2]) - FRONT_AHEAD)).abs() < 0.01, "ahead of the head: {z}");
        assert!(apart(spot.at, truck.position) > 3.0);
        let off = in_vehicle(truck.position, truck.heading, spot.off_to);
        assert!(off[0] < x && off[1] < z, "off ahead and outwards: {off:?}");
        // Coupled, however far the SDK's points: the same.
        let coupled = Trailer { connected: true, position: [0.0; 3], ..trailer };
        assert!(apart(clerk_spot(&coupled, &truck, None).at, spot.at) < 0.01);
        // The player on that spot: the other side.
        let other = clerk_spot(&trailer, &truck, Some(spot.at));
        assert!(in_vehicle(truck.position, truck.heading, other.at)[0] > 0.0);
    }

    /// The trailer is coupled as the person gets up from the work, not before nor long after.
    #[test]
    fn couples_as_the_person_gets_up() {
        assert!((COUPLING_TIME.as_secs_f32() - person::WORK_UP).abs() < 0.05);
    }

    /// Beside the fifth wheel only, either side, by the truck: not by the tank in front of it, not far out.
    #[test]
    fn the_trailer_is_coupled_by_the_fifth_wheel_only() {
        let truck = truck_at([100.0, 0.0, 200.0], 0.0);
        let hook = hook_back(&truck);
        // Heading 0: the truck's right is the world's +x, its back the world's +z.
        let at = |x: f64, back: f64| [100.0 + x, 0.0, 200.0 + back];
        assert!(by_fifth_wheel(at(-1.6, hook), &truck), "the driver's side");
        assert!(by_fifth_wheel(at(1.6, hook + 0.5), &truck), "the other side, by the axle behind");
        assert!(!by_fifth_wheel(at(-1.6, hook - 1.5), &truck), "by the tank");
        assert!(!by_fifth_wheel(at(-3.0, hook), &truck), "a step out");
        assert!(!by_fifth_wheel(at(0.0, hook), &truck), "under the truck");
        assert!(!by_fifth_wheel(at(-1.6, hook + 3.0), &truck), "by the trailer's legs");
    }

    /// The same worker for a job each time; another at its delivery; the mod's names.
    #[test]
    fn a_job_has_its_clerk() {
        assert_eq!(clerk_of("a job", false), clerk_of("a job", false));
        assert_ne!(clerk_of("a job", false), clerk_of("a job", true));
        assert_eq!(CLERKS.to_vec(), westplay_setup_clerks());
    }

    #[test]
    fn hears_the_game_say_it_can_unload_here() {
        assert!(unload_prompt("@@job_unload@@ <color value=@@clr_sel@@>Enter<color value=@@clr_txt@@>"));
        assert!(!unload_prompt("@@unload_load@@ <color value=@@clr_sel@@>T<color value=@@clr_txt@@>"));
        assert!(!unload_prompt("@@cargo_unloading1@@<br><color value=@@clr_txt@@>@@cargo_unloading2@@"));
        assert!(!unload_prompt("@@job_load@@ <color value=@@clr_sel@@>Enter<color value=@@clr_txt@@>"));
    }

    #[test]
    fn hears_the_game_say_it_can_load_here() {
        assert!(load_prompt("@@cargo_load@@ <color value=@@clr_sel@@>Enter<color value=@@clr_txt@@>"));
        assert!(!load_prompt("@@cargo_load_waiting1@@<br><color value=@@clr_txt@@>@@cargo_load_waiting2@@"));
        assert!(!load_prompt("@@attach_load@@ <color value=@@clr_sel@@>T<color value=@@clr_txt@@>"));
        assert!(!load_prompt("@@unload_load@@ <color value=@@clr_sel@@>Enter<color value=@@clr_txt@@>"));
        assert!(!load_prompt("@@job_offer@@ <color value=@@clr_sel@@>Enter<color value=@@clr_txt@@>"));
    }

    fn westplay_setup_clerks() -> Vec<&'static str> {
        // As `westplay_setup::clerk::CLERKS` names them (the plugin doesn't link the setup crate).
        vec!["wp_clk1", "wp_clk2", "wp_clk3"]
    }

    #[test]
    fn a_place_in_a_vehicle_comes_back_the_same() {
        let (position, heading) = ([10.0, 0.0, -20.0], 0.3);
        let world = from_vehicle(position, heading, [2.0, 5.0]);
        let [x, z] = in_vehicle(position, heading, world);
        assert!((x - 2.0).abs() < 1e-9 && (z - 5.0).abs() < 1e-9);
    }

    #[test]
    fn every_notice_has_its_words() {
        for say in
            [Say::NoteToCouple, Say::Coupled, Say::NotAligned, Say::DeliverySigned, Say::UncoupleOnFoot, Say::Uncoupled]
        {
            for language in ["pt", "es", "en", "ja"] {
                assert!(!say.text(language).is_empty());
            }
        }
        assert_eq!(Say::Coupled.text("pt"), c"Carreta engatada");
        assert_eq!(Say::Uncoupled.text("pt"), c"Carreta desengatada");
    }
}
