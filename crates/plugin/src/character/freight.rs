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

use westplay_shared::{CHARACTER_PEOPLE, JobInfo, NOTE_DELIVERY, NOTE_LEFT, NOTE_PICKUP};
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
const HOOK_REACH: f64 = 2.4;
/// How far the trailer's pin may be from the fifth wheel for the game to couple them (metres).
const COUPLING_REACH: f64 = 1.3;
/// Coupling by hand: how long the player is down at the fifth wheel before the trailer is coupled, how long the
/// game's `attach` is held, and how long after it's seen whether it took.
const COUPLING_TIME: Duration = Duration::from_millis(1800);
const ATTACH_HELD: Duration = Duration::from_millis(350);
const COUPLED_WITHIN: Duration = Duration::from_millis(1800);
/// The clerk going away: at a walk (m/s), for this long.
const CLERK_PACE: f64 = 1.4;
const CLERK_LEAVES_FOR: Duration = Duration::from_secs(9);
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
fn from_vehicle(position: [f64; 3], heading: f32, [lx, lz]: [f64; 2]) -> [f64; 3] {
    let turn = f64::from(heading) * std::f64::consts::TAU;
    let [x, y, z] = position;
    [x + lx * turn.cos() + lz * turn.sin(), y, z - lx * turn.sin() + lz * turn.cos()]
}

/// A place in the world in a vehicle's own space (x to its right, z backwards).
fn in_vehicle(position: [f64; 3], heading: f32, world: [f64; 3]) -> [f64; 2] {
    let turn = f64::from(heading) * std::f64::consts::TAU;
    let (dx, dz) = (world[0] - position[0], world[2] - position[2]);
    [dx * turn.cos() - dz * turn.sin(), dx * turn.sin() + dz * turn.cos()]
}

fn apart(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

/// Where the clerk stands: beside the trailer, by its front (the driver's side first), where nothing the plugin
/// knows of is in the way: not in the truck (wherever it's parked, coupled or not) nor on the player. The game's
/// own things around (other trailers, walls) aren't the plugin's to see.
fn clerk_spot(trailer: &Trailer, truck: &Truck, player: Option<[f64; 3]>) -> [f64; 3] {
    let front = trailer.hook.map_or(0.0, |hook| f64::from(hook[2]));
    let spots = [[-3.0, 3.5], [3.0, 3.5], [-3.0, 8.0], [3.0, 8.0], [-3.4, 12.0], [3.4, 12.0], [0.0, 17.0]];
    let world = |spot: [f64; 2]| from_vehicle(trailer.position, trailer.heading, [spot[0], front + spot[1]]);
    let free = |at: [f64; 3]| {
        let [x, z] = in_vehicle(truck.position, truck.heading, at);
        let in_truck = x.abs() < 2.4 && (-4.0..10.0).contains(&z);
        !in_truck && player.is_none_or(|player| apart(player, at) > 1.2)
    };
    spots.iter().map(|&spot| world(spot)).find(|&at| free(at)).unwrap_or_else(|| world(spots[0]))
}

/// The keys of the player's that the game couples (`attach`) and confirms (`activate`) with, as virtual keys: from
/// their profile's `controls.sii` (the one last played), T and Enter if it can't be read.
struct Keys {
    attach: Vec<u32>,
    activate: Vec<u32>,
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
    fn read() -> Self {
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
}

impl Say {
    fn icon(self) -> i32 {
        match self {
            Say::NoteCollected | Say::DeliverySigned | Say::Coupled => notice::ANNOUNCEMENT,
            _ => notice::ALERT,
        }
    }

    /// In the game's language (two letters), English if not one of these.
    fn text(self, language: &str) -> &'static CStr {
        let words: [&CStr; 8] = match language {
            "pt" => [
                c"Colete a nota fiscal com o conferente para engatar a carreta",
                c"Des\xc3\xa7a do caminh\xc3\xa3o e engate a carreta na quinta roda",
                c"Colete a nota fiscal com o conferente para carregar",
                c"Pegue a assinatura do destinat\xc3\xa1rio para entregar a carga",
                c"Nota fiscal coletada: engate liberado",
                c"Canhoto assinado: entrega liberada",
                c"Encoste a quinta roda sob o pino da carreta para engatar",
                c"Carreta engatada",
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
    /// Since when they walk away (their person the walking one then).
    leaving: Option<Instant>,
}

/// Coupling by hand, as far as it's got.
enum Coupling {
    /// Down at the fifth wheel since.
    Kneeling(Instant),
    /// The game's `attach` held since.
    Attaching(Instant),
    /// Let go since: whether it took is seen after a moment.
    Waiting(Instant),
}

pub(super) struct Freight {
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
            papers: papers.unwrap_or_default(),
            keys: Keys::read(),
            clerk: None,
            no_clerk: false,
            coupled: String::new(),
            paper: None,
            coupling: None,
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
    crate::commands::hold(crate::commands::ATTACH, false);
    if let Some(bridge) = crate::bridge::get() {
        bridge.plugin.note_wanted.store(0, Ordering::Release);
    }
    let Some(freight) = state.freight.as_mut() else { return };
    (freight.paper, freight.coupling, freight.offer) = (None, None, None);
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
        if state.freight.is_some() {
            rest(state);
        }
        return;
    }
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
    let facts = Facts {
        job: job.as_ref().map(job_key),
        cargo_loaded: job.as_ref().is_some_and(|job| job.cargo_loaded),
        connected: trailer.connected,
        at_end: navigation > 0.0 && navigation < NEAR,
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
    if wanted && !freight.no_clerk && freight.clerk.as_ref().is_none_or(|clerk| clerk.leaving.is_some()) {
        // SAFETY: the code is the game's, read as it lies.
        state.game.get_or_insert_with(|| unsafe { super::locate() });
        let people = state.game.as_ref().and_then(|game| game.as_ref()).and_then(|game| game.people.as_ref());
        let at = clerk_spot(&trailer, &truck, player);
        let yaw = heading_of(truck.position[0] - at[0], truck.position[2] - at[2]);
        let chosen = super::chosen_person();
        let own = CHARACTER_PEOPLE[if chosen == 2 { 0 } else { 2 }].trim_end_matches("_walk");
        let names = [format!("{own}_idle"), "m_idle01".to_owned(), "idle_event01".to_owned()];
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
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
                freight.clerk = Some(Clerk { person, at, yaw, leaving: None });
            }
            other => {
                log!("freight: no clerk ({}): the papers are waived", other.and_then(Result::err).unwrap_or("no people"));
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
            let own = CHARACTER_PEOPLE[if super::chosen_person() == 2 { 0 } else { 2 }];
            // Away from the trailer, the way they face from it.
            let away = in_vehicle(trailer.position, trailer.heading, clerk.at);
            let side = if away[0] < 0.0 { -1.0 } else { 1.0 };
            let to = from_vehicle(clerk.at, trailer.heading, [side * 10.0, 4.0]);
            clerk.yaw = heading_of(to[0] - clerk.at[0], to[2] - clerk.at[2]);
            let placement = Placement::at(clerk.at, clerk.yaw / std::f32::consts::TAU);
            // SAFETY: on the game's main thread, `people` as `locate` found it.
            match unsafe { person::show_other(people, &placement, &[own, westplay_shared::CHARACTER_GAME_PERSON]) } {
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
    keep(0..2, &freight.keys.attach.clone(), !trailer.connected);
    keep(2..4, &freight.keys.activate.clone(), blocks_activate);
    USE_KEPT.store(on_foot.is_some(), Ordering::Release);
    let shown = crate::d3d::game_shown();
    let down = |keys: &[u32]| shown && keys.iter().any(|&key| really_down(VIRTUAL_KEY(key as u16)));
    let attach_pressed = crate::input::newly_down(&freight.was_attach, down(&freight.keys.attach));
    let activate_pressed = crate::input::newly_down(&freight.was_activate, down(&freight.keys.activate));
    let use_down = shown && really_down(VIRTUAL_KEY(super::virtual_key(USE_KEY)));
    let use_pressed = crate::input::newly_down(&freight.was_use, use_down) && on_foot.is_some();
    let mut to_say = None;
    if attach_pressed && !trailer.connected && freight.coupling.is_none() {
        to_say = Some(if need == Need::Pickup { Say::NoteToCouple } else { Say::CoupleOnFoot });
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
                } else {
                    freight.papers.signed = true;
                    to_say = Some(Say::DeliverySigned);
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
    let pin = trailer.hook.filter(|_| trailer.placed).map(|pin| {
        from_vehicle(trailer.position, trailer.heading, [f64::from(pin[0]), f64::from(pin[2])])
    });
    freight.offer = None;
    if let Some(player) = player.filter(|_| freight.paper.is_none() && freight.coupling.is_none()) {
        let by_clerk = need != Need::None
            && freight.clerk.as_ref().is_some_and(|clerk| clerk.leaving.is_none() && apart(player, clerk.at) < CLERK_REACH);
        let by_hook = !trailer.connected && hook.is_some_and(|hook| apart(player, hook) < HOOK_REACH);
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
        } else if by_hook {
            freight.offer = Some(Action::Couple);
            if use_pressed {
                let aligned = match (hook, pin) {
                    (Some(hook), Some(pin)) => apart(hook, pin) < COUPLING_REACH,
                    // Where either is isn't known: the game says whether it couples.
                    _ => true,
                };
                if need == Need::Pickup {
                    to_say = Some(Say::NoteToCouple);
                } else if !aligned {
                    to_say = Some(Say::NotAligned);
                } else {
                    freight.coupling = Some(Coupling::Kneeling(Instant::now()));
                    super::HELD_STILL.store(true, Ordering::Release);
                    super::KNEELING.store(true, Ordering::Release);
                    log!("freight: coupling by hand");
                }
            }
        }
    }
    // Coupling by hand: down at the fifth wheel, the game's own `attach`, then whether it took.
    match freight.coupling {
        Some(Coupling::Kneeling(since)) if since.elapsed() >= COUPLING_TIME => {
            crate::commands::hold(crate::commands::ATTACH, true);
            // The lock's clunk (the door's sound: the plugin has no other).
            super::steps::door(super::steps::Door::Shut, 0.0);
            freight.coupling = Some(Coupling::Attaching(Instant::now()));
        }
        Some(Coupling::Attaching(since)) if since.elapsed() >= ATTACH_HELD => {
            crate::commands::hold(crate::commands::ATTACH, false);
            super::HELD_STILL.store(false, Ordering::Release);
            super::KNEELING.store(false, Ordering::Release);
            freight.coupling = Some(Coupling::Waiting(Instant::now()));
        }
        Some(Coupling::Waiting(since)) if trailer.connected || since.elapsed() >= COUPLED_WITHIN => {
            freight.coupling = None;
            log!("freight: coupled by hand: {}", trailer.connected);
            to_say = Some(if trailer.connected { Say::Coupled } else { Say::NotAligned });
        }
        Some(_) if on_foot.is_none() => {
            crate::commands::hold(crate::commands::ATTACH, false);
            super::HELD_STILL.store(false, Ordering::Release);
            super::KNEELING.store(false, Ordering::Release);
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
        assert_eq!((kept.key.as_str(), kept.judged, kept.applies, kept.picked, kept.signed), ("a job", true, true, true, false));
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
        Truck { position, heading, speed: 0.0, parking_brake: true, multiplayer: false, head: [-0.7, 1.5, -1.0] }
    }

    /// The clerk stands beside the trailer's front on the driver's side; with the truck parked there, on the other.
    #[test]
    fn the_clerk_stands_clear_of_the_truck() {
        let trailer = Trailer { connected: false, placed: true, position: [100.0, 0.0, 200.0], heading: 0.0, hook: None };
        let far = truck_at([300.0, 0.0, 300.0], 0.0);
        let spot = clerk_spot(&trailer, &far, None);
        assert_eq!(in_vehicle(trailer.position, 0.0, spot).map(|v| v.round()), [-3.0, 4.0]);
        // The truck standing right on that spot: the other side.
        let beside = truck_at([97.0, 0.0, 201.0], 0.0);
        let spot = clerk_spot(&trailer, &beside, None);
        assert!(in_vehicle(trailer.position, 0.0, spot)[0] > 0.0);
        // Nor on the player.
        let spot = clerk_spot(&trailer, &far, Some([97.0, 0.0, 203.5]));
        assert!(apart(spot, [97.0, 0.0, 203.5]) > 1.2);
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
        for say in [Say::NoteToCouple, Say::Coupled, Say::NotAligned, Say::DeliverySigned] {
            for language in ["pt", "es", "en", "ja"] {
                assert!(!say.text(language).is_empty());
            }
        }
        assert_eq!(Say::Coupled.text("pt"), c"Carreta engatada");
    }
}
