//! WestPlay host: renders the in-cabin UI in an offscreen WebView2 and feeds it to the game plugin.
//!
//! [`run`] owns one STA thread: it pumps window messages for WebView2, keeps the capture pipeline on the
//! game's GPU, forwards telemetry from the bridge to the page, and beats the host heartbeat.

#[macro_use]
pub mod log;
mod alerts;
mod apple_music;
pub mod browser;
mod camera;
mod capture;
mod cargo_icons;
mod cursor;
pub mod customize;
pub mod fitted;
mod fleet;
mod game_settings;
mod gpu;
pub mod graphics;
mod handover;
mod job_market;
mod lyrics;
pub mod maps;
mod performance;
mod planner;
mod play_log;
mod player;
mod pointer;
mod pumps;
mod remote;
pub mod route;
mod route_change;
pub mod settings;
mod signin;
pub mod speakers;
mod spotify;
pub mod startup;
mod tiktok;
mod truck_commands;
mod auto_suspension;
mod brake_fade;
mod typing;
mod wallet;
mod weather;
mod webview;
mod wheel;
mod youtube;
mod youtube_music;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_MOUSE_EVENT_KIND_LEAVE, COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_NONE,
};
use westplay_shared::{BridgeMapping, JobInfo, Keybind, Telemetry, TruckInfo, screen_layouts};
use windows::Win32::Foundation::LUID;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage};

use capture::{Capture, Published};
use gpu::{Gpu, SharedFrame};
use player::{PartSpeakers, Player, PlayerWatch};
use pointer::Pointer;
use route::{RouteUpdate, RouteWatch};
use settings::SharedSettings;
use weather::{Sky, Weather, WeatherWatch};
use webview::{OffscreenWebView, Stage};
use westplay_mapdata::junctions::Turn;

pub use webview::{Error, wait_for_browsers};
/// Size the cabin page is rendered at: the size of the frames the plugin copies into the screen textures.
pub use westplay_shared::{SCREEN_HEIGHT, SCREEN_WIDTH};

/// How often the page is sent the game's telemetry (ms): 20 times a second, and in reverse as often as the
/// screen shows frames (30: capture.rs MAX_FPS), for the reversing camera to follow the truck smoothly.
const TELEMETRY_MS: u64 = 50;
/// How long after its page is told a paper is shown (the page draws it first: no phone seen before it).
const PAPER_REVEAL: Duration = Duration::from_millis(150);
const TELEMETRY_REVERSING_MS: u64 = 33;

/// Built-in page used when no UI URL is given: shows that frames and telemetry arrive.
pub const TEST_PAGE: &str = include_str!("test_page.html");

#[derive(Clone)]
pub enum Content {
    Url(String),
    Html(String),
    /// A built UI folder, served at `https://westplay.cabin/` through a WebView2 virtual host.
    Folder {
        root: PathBuf,
        page: String,
    },
}

/// Host health, reported about once per second.
#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub webview_version: String,
    /// The GPU frames are handed to the game on (the game's, once it runs).
    pub adapter: Option<String>,
    /// The GPU the cab is drawn on, and the game's (`None` until the game runs).
    pub cab_adapter: Option<String>,
    pub game_adapter: Option<String>,
    /// The PC's GPUs, for the player to choose the cab's (the high-performance ones first).
    pub adapters: Vec<graphics::Adapter>,
    pub frames_per_second: u64,
    pub dropped_total: u64,
    /// The phone page's address, with its key, and it as a QR code (an SVG image), for the companion to show
    /// (remote.rs); `None` when it isn't served.
    pub phone_url: Option<String>,
    pub phone_qr: Option<String>,
    /// The phone in the game's page on the player's phone, as `phone_url` and `phone_qr` the panel's.
    pub handset_url: Option<String>,
    pub handset_qr: Option<String>,
    /// The wheel page (the player's phone as a steering wheel, wheel.rs), over HTTPS, as `phone_url` and `phone_qr`.
    pub wheel_url: Option<String>,
    pub wheel_qr: Option<String>,
    /// The game WestPlay works with (`westplay_setup::game::current`: the one played, switched to as it starts),
    /// the one `game_connected` says of.
    pub game: westplay_setup::game::Kind,
    pub game_connected: bool,
    pub screen_drawing: bool,
    /// Whether the truck the player is in is one WestPlay shows in, as the plugin last saw (`None`: not known yet,
    /// or no game).
    pub truck_supported: Option<bool>,
    /// Whether the game runs as administrator: WestPlay can't read its route, guidance or weather then
    /// ([`is_elevated`]), and the player is told to start it normally.
    pub game_elevated: bool,
    /// Whether WestPlay is open on the truck's screen (otherwise the truck's original screen shows, with
    /// WestPlay's button to come back), as the cabin page last said; `None` until it has.
    pub westplay_open: Option<bool>,
    /// Whether YouTube is signed in, as its page last said (`None`: not known, YouTube not run yet), and
    /// whether Google's sign-in window is open. Nothing else of the account.
    pub youtube_signed_in: Option<bool>,
    pub youtube_signing_in: bool,
    /// The same of Spotify: signed in or not (`None`: not known, Spotify not run yet), its sign-in window open.
    pub spotify_signed_in: Option<bool>,
    pub spotify_signing_in: bool,
    /// The same of Apple Music: signed in or not (`None`: not known, Apple Music not run yet), Apple's sign-in
    /// window open.
    pub apple_music_signed_in: Option<bool>,
    pub apple_music_signing_in: bool,
    /// The same of TikTok (the phone's): signed in or not (`None`: not known, TikTok not run yet), its sign-in
    /// window open.
    pub tiktok_signed_in: Option<bool>,
    pub tiktok_signing_in: bool,
    /// The map of the profile played: ready, being built, waiting for the game to close...
    pub map: maps::MapStatus,
    /// The map the cab shows: its folder in WestPlay's maps, and when it was built (the companion's
    /// onboarding shows it too).
    pub map_folder: Option<String>,
    pub map_built: Option<u64>,
    /// Every profile's map: ready, being built, to build, failed...
    pub profile_maps: Vec<maps::ProfileMap>,
    pub error: Option<String>,
}

/// The companion window's requests to sign YouTube, Spotify or Apple Music in or out (or to know whether it
/// is), from any thread, for the host to carry out as the cab's apps would.
#[derive(Debug, Default)]
pub struct AccountRequests {
    pub youtube: AccountRequest,
    pub spotify: AccountRequest,
    pub apple_music: AccountRequest,
    pub tiktok: AccountRequest,
}

/// One service's request, the latest not carried out yet.
#[derive(Debug, Default)]
pub struct AccountRequest(AtomicU8);

/// What the companion asks of a service's sign-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountAction {
    SignIn,
    SignOut,
    /// Run it (hidden), so whether it's signed in is known (the onboarding's sign-in steps, shown).
    Check,
}

impl AccountRequest {
    const NONE: u8 = 0;
    const SIGN_IN: u8 = 1;
    const SIGN_OUT: u8 = 2;
    const CHECK: u8 = 3;

    /// Asks for it (replacing a request not carried out yet).
    pub fn request(&self, action: AccountAction) {
        let value = match action {
            AccountAction::SignIn => Self::SIGN_IN,
            AccountAction::SignOut => Self::SIGN_OUT,
            AccountAction::Check => Self::CHECK,
        };
        self.0.store(value, Ordering::Release);
    }

    /// The latest request not carried out yet, if any.
    pub fn take(&self) -> Option<AccountAction> {
        match self.0.swap(Self::NONE, Ordering::AcqRel) {
            Self::SIGN_IN => Some(AccountAction::SignIn),
            Self::SIGN_OUT => Some(AccountAction::SignOut),
            Self::CHECK => Some(AccountAction::Check),
            _ => None,
        }
    }
}

/// A request to open or close WestPlay on the truck's screen (the companion window's switch), from any
/// thread, for the host to pass on to the cabin page, which owns that state (its sidebar's truck button and
/// its WestPlay button change it too).
#[derive(Debug, Default)]
pub struct OpenRequest(AtomicU8);

impl OpenRequest {
    const NONE: u8 = 0;
    const CLOSE: u8 = 1;
    const OPEN: u8 = 2;

    /// Asks for WestPlay to be opened or closed (replacing a request not passed on yet).
    pub fn request(&self, open: bool) {
        self.0.store(if open { Self::OPEN } else { Self::CLOSE }, Ordering::Release);
    }

    /// The latest request not passed on yet, if any.
    pub fn take(&self) -> Option<bool> {
        match self.0.swap(Self::NONE, Ordering::AcqRel) {
            Self::OPEN => Some(true),
            Self::CLOSE => Some(false),
            _ => None,
        }
    }
}

pub struct Config {
    pub width: u32,
    pub height: u32,
    pub content: Content,
    /// WebView2 user data (cache, cookies) lives here.
    pub data_dir: PathBuf,
    /// Requests to open or close WestPlay on the truck's screen, from any thread.
    pub open_request: Arc<OpenRequest>,
    /// Signing YouTube or Spotify in or out (or finding out whether it is), from the companion window.
    pub accounts: Arc<AccountRequests>,
    /// Called on the host thread about once per second, and right after WestPlay opens or closes.
    pub on_status: Box<dyn FnMut(&Status) + Send>,
    /// The player's settings; changes are applied as they happen.
    pub settings: Arc<SharedSettings>,
    /// Called when the cabin page changes a setting itself (its Settings' wallpaper), after it's applied: to
    /// keep it and tell the companion.
    pub on_settings: Box<dyn FnMut(&settings::Settings) + Send>,
    /// Bumped whenever the player's wallpapers or icon sets change (`customize`): the page is told again.
    pub customize_changed: Arc<std::sync::atomic::AtomicU64>,
    /// The game WestPlay is set up in (the companion's setup sets it): its map is built for the cab.
    pub game: Arc<maps::SetUpGame>,
}

struct Pipeline {
    gpu: Gpu,
    /// Holds the shared frame texture (and its name) alive.
    capture: Capture,
    /// The phone in the game's frame, as `capture` the screen's (`None`: no phone).
    phone: Option<Capture>,
}

/// Runs until `stop` is set. Blocks the calling thread, which becomes the WebView2 UI thread.
pub fn run(mut config: Config, stop: Arc<AtomicBool>) -> Result<(), Error> {
    startup::mark("host: started");
    // SAFETY: COM on this thread, once, before anything on it uses COM (WebView2 needs a single-threaded
    // apartment).
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
    let bridge = Arc::new(BridgeMapping::open()?);
    let host = &bridge.bridge().host;
    host.pid.store(std::process::id(), Ordering::Release);
    startup::mark("host: game bridge open");

    let stage = Stage::new(config.width, config.height)?;
    // The GPU the cab is drawn on (graphics.rs): the player's choice, the game's if it's running, or Windows'
    // high-performance one. Every WebView of this run draws on it.
    let mut adapters = graphics::adapters();
    let mut chosen_gpu = config.settings.get().graphics.gpu;
    let cab_gpu = graphics::choose(chosen_gpu.as_deref(), game_adapter(&bridge), None, &adapters);
    webview::draw_on(cab_gpu.map(graphics::browser_argument));
    if let Some(luid) = cab_gpu {
        log!("the cab is drawn on {}", graphics::name_of(luid, &adapters).unwrap_or_default());
    }
    let webview = OffscreenWebView::new(&stage, config.width, config.height, &config.data_dir.join("webview2"))?;
    stage.add(&webview, true)?;
    startup::mark("host: cabin browser (WebView2) ready");
    match &config.content {
        Content::Url(url) => webview.navigate(url)?,
        Content::Html(html) => webview.navigate_to_string(html)?,
        Content::Folder { root, page } => {
            webview.map_folder("westplay.cabin", root)?;
            webview.navigate(&format!("https://westplay.cabin/{page}"))?;
        }
    }
    startup::mark("host: cabin page requested");
    // What WestPlay costs while the game runs, in the log each minute (performance.rs).
    let mut meter = performance::Meter::new(match &config.content {
        Content::Url(url) => performance::host_of(url).unwrap_or_default(),
        Content::Html(_) => String::new(),
        Content::Folder { .. } => "westplay.cabin".to_string(),
    });
    // The player's maps, built from their game (maps.rs), for the cab's Maps app.
    let maps_dir = config.data_dir.join("maps");
    let _ = std::fs::create_dir_all(&maps_dir);
    webview.map_folder_to_any_page("westplay.maps", &maps_dir)?;
    // The player's own wallpapers and icon sets (customize.rs), listed off this thread: deciding whether an
    // image is dark or light decodes it, which mustn't hold the screen up.
    let customize_dir = config.data_dir.join(customize::FOLDER);
    let _ = std::fs::create_dir_all(&customize_dir);
    webview.map_folder_to_any_page("westplay.customize", &customize_dir)?;
    let (customize_list, customize_listed) = std::sync::mpsc::channel::<String>();
    let mut customize_posted: Option<u64> = None;
    let mut customize_message: Option<String> = None;
    let map_watch = maps::MapWatch::start(maps_dir.clone(), config.game.clone());
    // The player's phone as a second screen (remote.rs): told what the cab is, and its playback controls taken.
    let ui_root = match &config.content {
        Content::Folder { root, .. } => Some(root.clone()),
        _ => None,
    };
    let remote = std::rc::Rc::new(remote::Remote::start(ui_root.clone(), maps_dir.clone(), &config.data_dir, bridge.clone()));
    // The phone in the game (owner 2026-11-01), out at its keybind: its own page beside the cab's, on the cab's
    // browser, told what the cab is as the player's phones are, and captured into a frame of its own the plugin
    // draws over the game. Hidden (drawing nothing) while it's put away.
    let handset = match ui_root.as_deref().map(|root| open_handset(&webview, &stage, root, &maps_dir)) {
        Some(Ok(handset)) => Some(std::rc::Rc::new(handset)),
        Some(Err(e)) => {
            log!("[handset] couldn't open: {e}");
            None
        }
        None => None,
    };
    // The phone's picture: its page, and TikTok's under it where its TikTok app cuts it open (tiktok.rs).
    let phone_layers = handset.as_deref().and_then(|handset| {
        let layers = stage.layers(westplay_shared::PHONE_WIDTH, westplay_shared::PHONE_HEIGHT).ok()?;
        layers.add(handset, true).ok()?;
        Some(layers)
    });
    let mut phone_dims = (westplay_shared::PHONE_WIDTH, westplay_shared::PHONE_HEIGHT);
    host.phone_width.store(phone_dims.0, Ordering::Release);
    host.phone_height.store(phone_dims.1, Ordering::Release);
    let mut handset_shown = false;
    // The phone's top showing for a notification while it's put away (owner 2026-11-11: as GTA RP's phones),
    // until then.
    let mut handset_peek: Option<Instant> = None;
    // TikTok in the phone, while its app (or the companion's Accounts) uses it, and what it last said.
    let mut tiktok: Option<tiktok::TikTok> = None;
    let mut tiktok_state: Option<tiktok::State> = None;
    let (mut tiktok_window_open, mut tiktok_confirming) = (false, false);
    // Whether the phone's own page was last told the mouse left it (it's over TikTok).
    let mut handset_left = false;
    // Whether TikTok's page has the keyboard, as the plugin was last told.
    let mut tiktok_typing = false;
    // Whether the in-game phone/iPad's own page (TextField) currently holds the physical keyboard selection.
    let mut handset_typing = false;
    // Where the in-game iPad/phone YouTube app currently has its video hole (`None`: closed/no video).
    let mut handset_youtube_rect: Option<(f32, f32, u32, u32, bool)> = None;
    // Whether the cab page was last told YouTube's picture is in the phone (`youtubeHandset`).
    let mut youtube_on_handset_told: Option<bool> = None;
    // The cargo's paper the player asked for at the clerk (the plugin's `note_wanted`), shown in the phone's frame
    // to sign until it's collected or put down.
    let mut handset_paper: Option<u32> = None;
    // When a paper asked for with the phone away is shown (`PAPER_REVEAL` after the page was told).
    let mut paper_reveal: Option<Instant> = None;
    // The fuel filled by hand, to pay at the pump's attendant (the plugin's `fuel_bill`), shown on the phone's
    // Nupay until it's paid or put down.
    let mut handset_bill: Option<westplay_shared::FuelBill> = None;
    // The truck's commands the phones hold, for the plugin's input device.
    let mut truck_commands = truck_commands::TruckCommands::default();
    let mut auto_suspension = auto_suspension::AutoSuspension::default();
    // Brake fade (brake_fade.rs): only modelled and logged for now (docs/fadiga-freio/PLANO.md, phase B).
    let mut brake_fade = brake_fade::BrakeFade::default();
    let mut brake_fade_logged: Option<Instant> = None;
    // How far the phone was told of the brake fading: 0 not, 1 weak, 2 gone.
    let mut brake_told = 0u8;
    let mut brakes_posted: Option<Instant> = None;
    let mut last_weather: Option<weather::Weather> = None;
    // The player's account, for the phone's Nupay (wallet.rs).
    let mut wallet_game = westplay_setup::game::current();
    let mut wallet = wallet::Wallet::open(&config.data_dir, wallet_game.sdk_id());
    {
        let remote = remote.clone();
        let handset = handset.clone();
        webview.set_mirror(move |json| {
            remote.broadcast(json);
            if let Some(handset) = &handset
                && remote::Remote::mirrored(json)
            {
                let _ = handset.post_json(json);
            }
        });
    }
    let mut map_shown: (Option<String>, Option<u64>) = (None, None);
    let mut map_pumps: Option<Vec<[f32; 2]>> = None;
    let mut status = Status {
        webview_version: webview.version.clone(),
        cab_adapter: cab_gpu.and_then(|luid| graphics::name_of(luid, &adapters)),
        adapters: adapters.clone(),
        phone_url: remote.url.clone(),
        phone_qr: remote.qr.clone(),
        handset_url: remote.handset_url.clone(),
        handset_qr: remote.handset_qr.clone(),
        wheel_url: remote.wheel_url.clone(),
        wheel_qr: remote.wheel_qr.clone(),
        ..Default::default()
    };

    let mut pipeline: Option<Pipeline> = None;
    let mut generation_counter = 0u32;
    let mut last_check = Instant::now() - Duration::from_secs(10);
    let mut last_stats = Instant::now();
    let mut last_telemetry_post = Instant::now();
    // TruckersMP's players around the player (the plugin's, from TruckersMP's SDK), as last sent: the game
    // frame they're from, and when that was last sent.
    let mut nearby_sent: Option<u64> = None;
    let mut last_nearby_post = Instant::now();
    let mut last_telemetry: Option<Telemetry> = None;
    let mut last_truck: Option<TruckInfo> = None;
    let mut last_job: Option<JobInfo> = None;
    let mut last_layout: Option<u32> = None;
    // The game's menu gone while it's still paused (`camera::MenuGone`), and as last told to the page.
    let mut menu = camera::MenuGone::default();
    let mut menu_gone = false;
    // Whether the camera was outside the cab, as last said in the log.
    let mut was_outside = false;
    // The players' tone (the cab's Settings › Equalizer), passed on with the cab ([`settings::SoundSettings`]).
    let mut equalizer = config.settings.get().sound.playing();
    // Where media audio plays (`Cabin` = truck speakers + muffled outside, `Device` = phone/iPad clean stereo).
    let mut audio_output = config.settings.get().audio_output;
    // The music's spectrum wanted by the cab (its Equalizer open): asked of the players with the cab.
    let mut spectrum = false;
    // The page's height (`westplay_shared::page_height`): the frame's until a screen asks for another.
    let mut page_height = config.height;
    let mut pointer = Pointer::new(config.width, config.height);
    // The screen's cursor, drawn into the frames the game takes (cursor.rs), and whether the page was last told the
    // host draws it (`None`: not told since it loaded).
    let mut cursor = cursor::Cursor::new(Instant::now());
    let mut cursor_told: Option<bool> = None;
    // The phone in the game's, as the screen's.
    let mut handset_pointer = Pointer::new(westplay_shared::PHONE_WIDTH, westplay_shared::PHONE_HEIGHT);
    let mut handset_cursor = cursor::Cursor::new(Instant::now());
    // What the player's latest save says (their garages, a map WestPlay doesn't allow), kept to send again
    // when the page reloads.
    let player_watch = PlayerWatch::start();
    // The speakers of the parts of the truck the player is in, for the trucks whose speakers come with them.
    let mut part_speakers: Option<PartSpeakers> = None;
    let mut player_messages: Vec<String> = Vec::new();
    // The game's GPS route, and the truck's progress along it, kept to send again when the page reloads.
    let route_watch = RouteWatch::start(bridge.clone(), map_watch.played.clone());
    // The host's number for the route shown (`route::RouteUpdate::Route`).
    let mut route_number: Option<u32> = None;
    let (mut route_message, mut route_paths_message, mut route_next_message): (
        Option<String>,
        Option<String>,
        Option<String>,
    ) = (None, None, None);
    // The game's weather, kept to send again when the page reloads.
    let weather_watch = WeatherWatch::start(bridge.clone());
    let mut weather_message: Option<String> = None;
    // Whether the game's window shows (as last told the page; sent again to a reloaded page).
    let mut game_shown = true;
    // Whether the game runs (its process, the plugin's pid), as last told the page. As it closes (or crashes)
    // the web players close with it, so none plays on, unseen, with the game gone (2026-10-21).
    let mut game_running = false;
    // Whether the game runs as administrator (`is_elevated`), looked at once for each game (its pid).
    let (mut elevated_pid, mut game_elevated) = (0, false);
    // Whether the game runs under TruckersMP (its client's module loaded in it), as last told the page, and
    // when that was last looked for (every few seconds while the game runs and it isn't found).
    let mut truckersmp = false;
    let mut truckersmp_checked = Instant::now() - TRUCKERSMP_CHECK_EVERY;
    // The page's changes to the game's route, waiting for the plugin's answer.
    let mut route_changes = route_change::RouteChanges::default();
    // Live in-memory Job Market (Bolsa de Fretes), Nupay Bank, and Minha Empresa (Fleet) RPCs to the plugin.
    let mut job_market = job_market::JobMarket::default();
    let mut bank_rpc = wallet::BankRpc::default();
    let mut fleet = fleet::Fleet::default();
    let cargo_icons = cargo_icons::CargoIcons::start();
    // WestPlay's own routes to a place, planned for the page before Go (planner.rs), and the one picked
    // being handed to the game's GPS (handover.rs).
    let mut planner = planner::Planner::start();
    let mut following: Option<Following> = None;
    // The game's route as its map nodes (0 for points inside junctions), for the hand-over.
    let mut route_uids: Option<Vec<u64>> = None;
    // The game's waypoints with the route, the ones WestPlay put in to hold a route it handed over (kept once
    // it follows: they stay in the game's route), and the player's stops among them as last told to the page.
    let mut route_waypoints: Vec<u64> = Vec::new();
    let mut held_vias: Vec<u64> = Vec::new();
    let mut route_stops_message: Option<String> = None;
    // Whether the player's keyboard types into the page's field, as last told to the page.
    let mut typing_state: Option<westplay_shared::TypingState> = None;
    // YouTube's player, while the page uses it, and how it last played (sent again when the page reloads).
    let mut youtube: Option<youtube::YouTube> = None;
    // Whether the game's camera is outside the cab (YouTube's sound muffled then), and the player's camera
    // settings in the game (it turns with the steering).
    let mut camera = camera::Camera::default();
    let mut game_settings = game_settings::GameSettings::new();
    let mut youtube_message: Option<String> = None;
    // Whether YouTube's page is signed in (and the picture for it), as last told: sent again to a reloaded
    // page. Only in memory, never kept.
    let mut youtube_account_message: Option<String> = None;
    // Whether YouTube's video was playing at its last report: a video that stops to buffer is logged.
    let mut youtube_was_playing = false;
    // The picture not over YouTube's page last logged, and its video: logged as it changes, not every report.
    let mut youtube_picture: Option<(Option<String>, String)> = None;
    // Each player's playing, for the log (play_log.rs).
    let mut youtube_log = play_log::PlayLog::new("youtube");
    // A video loading too long: the log says why, as the page reads it.
    let mut youtube_stuck = play_log::Stuck::default();
    let mut spotify_log = play_log::PlayLog::new("spotify");
    let mut apple_music_log = play_log::PlayLog::new("apple music");
    let mut youtube_music_log = play_log::PlayLog::new("youtube music");
    // Spotify's web player, while the page uses it, and what it last said (sent again when the page reloads):
    // signed in or not, and what plays.
    let mut spotify: Option<spotify::Spotify> = None;
    let mut spotify_state: Option<spotify::State> = None;
    let mut spotify_signing_in = false;
    // Each service's sign-in confirming after its window closed signed in (youtube.rs, spotify.rs).
    let (mut youtube_window_open, mut youtube_confirming) = (false, false);
    let mut spotify_confirming = false;
    let mut spotify_playback: Option<String> = None;
    // Apple Music's web player, the same way.
    let mut apple_music: Option<apple_music::AppleMusic> = None;
    let mut apple_music_state: Option<apple_music::State> = None;
    let (mut apple_music_window_open, mut apple_music_confirming) = (false, false);
    let mut apple_music_playback: Option<String> = None;
    // YouTube Music's web player, the same way (on the cabin page's profile, sharing YouTube's sign-in).
    let mut youtube_music: Option<youtube_music::YouTubeMusic> = None;
    let mut suspension_option = SuspensionOption::default();
    let mut youtube_music_state: Option<youtube_music::State> = None;
    let (mut youtube_music_window_open, mut youtube_music_confirming) = (false, false);
    let mut youtube_music_playback: Option<String> = None;
    // The settings generation last applied (`None`: apply them once at startup).
    let mut applied_settings = None;
    // Each keybind's presses the page was told of (`PluginBlock::keybind_press_slot`).
    let mut keybind_presses =
        Keybind::ALL.map(|keybind| bridge.bridge().plugin.keybind_press_slot(keybind).load(Ordering::Acquire));
    let mut last_frames = 0;
    // The startup stages still to log (`startup`), checked every turn until all are reached. The game's
    // screen frame counter as found: a session before this one may have left it counting.
    let mut startup_pending = true;
    let screen_frame_before = bridge.bridge().plugin.screen_frame.load(Ordering::Relaxed);

    let mut browser_gone = false;
    let mut gpu_changed = false;
    let mut game_liveness = Liveness::default();
    // WestPlay's screen out of view (`camera::Unseen`), and as last told to the page.
    let mut unseen = camera::Unseen::default();
    let mut unseen_told: Option<bool> = None;
    // The phone's notifications of the game and MaxDevMod (alerts.rs): what's watched for them, and the groups
    // turned off in its Settings.
    let mut alert_watch = alerts::Watch::default();
    let mut alert_filter = alerts::Filter::default();
    let mut alert_owed = alerts::Owed::default();
    // A notification for the phone (TikTok's, the game's): on its page and the player's phones, the phone's top
    // showing for it while it's put away (not over a paper or a bill in its frame).
    macro_rules! notify_phone {
        ($message:expr) => {{
            let message: &str = $message;
            remote.broadcast(message);
            if let Some(handset) = &handset {
                let _ = handset.post_json(message);
                if !handset_shown && handset_paper.is_none() && handset_bill.is_none() {
                    if handset_peek.is_none() {
                        let _ = handset.show(true);
                        host.phone_shown.store(westplay_shared::PHONE_PEEKS, Ordering::Release);
                    }
                    handset_peek = Some(Instant::now() + PEEK_FOR);
                }
            }
        }};
    }
    while !stop.load(Ordering::Relaxed) {
        pump_messages();
        host.heartbeat.fetch_add(1, Ordering::Release);
        // Set for this turn when the game has just closed: every web player closes, as its WebView failing
        // closes it (below), the page told so.
        let mut players_closing = false;
        // WestPlay's screen out of view (the camera outside the cab, the game's menu): nothing sent to the game,
        // the page told to hold its map and animations still (`camera::Unseen`). The newest frame waits in the
        // capture's mailbox, sent as the screen is drawn again.
        let out_of_view = {
            let plugin = &bridge.bridge().plugin;
            let cab = camera::drawn_frame(
                plugin.screen_frame.load(Ordering::Relaxed),
                plugin.cab_frame.load(Ordering::Relaxed),
            );
            let drawn = camera::drawn(cab, plugin.heartbeat.load(Ordering::Relaxed));
            unseen.update(status.game_connected, drawn, Instant::now())
        };
        if unseen_told != Some(out_of_view) && webview.post_json(&screen_seen_message(!out_of_view)).is_ok() {
            if unseen_told.is_some() {
                log!("screen: {}", if out_of_view { "out of view, held" } else { "in view" });
            }
            unseen_told = Some(out_of_view);
        }
        // A captured frame that came before its turn goes out now that it's due.
        // The cursor drawn over it where the mouse is now, the page squeezed into the frame as it's laid out.
        if let Some(pipeline) = pipeline.as_ref() {
            if !out_of_view {
                let squeeze = config.height as f32 / page_height.max(1) as f32;
                pipeline.capture.flush(cursor.look(Instant::now()), squeeze);
            }
            // The phone in the hand is seen wherever the cab's screen is (on foot too: owner 2026-11-06, out of
            // the truck its cursor didn't show with Alt, the phone's frames held with the cab's screen).
            if let Some(phone) = &pipeline.phone
                && (handset_shown || handset_peek.is_some() || handset_paper.is_some() || handset_bill.is_some())
            {
                phone.flush(handset_cursor.look(Instant::now()), 1.0);
            }
        }
        // The page told whether the host draws the cursor (it draws its own where the host can't).
        let drawn = pipeline.as_ref().is_some_and(|pipeline| pipeline.capture.draws_cursor());
        if cursor_told != Some(drawn) {
            cursor_told = Some(drawn);
            let _ = webview.post_json(&cursor_message(drawn));
        }
        // The page's process failed (a crash, or hung): load it again, and it gets the current state again
        // as it announces itself. The browser's: this WebView is gone, so start over (`BrowserGone`).
        match webview.take_failure() {
            Some(webview::Failure::Page) => {
                startup::mark("host: the cabin page's process failed; reloading it");
                if let Err(e) = webview.reload() {
                    status.error = Some(format!("the cabin page couldn't be reloaded: {e}"));
                    (config.on_status)(&status);
                }
            }
            Some(webview::Failure::Browser) => {
                browser_gone = true;
                break;
            }
            None => {}
        }
        if startup_pending {
            startup_pending = log_startup(&bridge, pipeline.as_ref(), screen_frame_before);
        }

        // WestPlay's frames go to the truck's screen while the host runs (the page, open or closed, is
        // what shows there).
        host.screen_enabled.store(1, Ordering::Release);
        if let Some(open) = config.open_request.take() {
            let _ = webview.post_json(&westplay_message(open));
        }
        // The companion's YouTube, Spotify and Apple Music sign-ins or sign-outs, as the cab's would be (or only
        // running it, to know whether it's signed in).
        if let Some(action) = config.accounts.youtube.take()
            && let Some(player) = start_youtube(&mut youtube, &stage, &config)
        {
            match action {
                AccountAction::SignIn => drop(player.command(&youtube::Command::SignIn)),
                AccountAction::SignOut => {
                    let _ = player.command(&youtube::Command::SignOut);
                    // YouTube Music shares the sign-in: its page again, signed out.
                    if let Some(music) = &youtube_music {
                        let _ = music.reload();
                    }
                }
                AccountAction::Check => {}
            }
        }
        if let Some(action) = config.accounts.spotify.take()
            && let Some(player) = start_spotify(&mut spotify, &stage, &config)
        {
            let _ = player.command(&match action {
                AccountAction::SignIn => spotify::Command::SignIn,
                AccountAction::SignOut => spotify::Command::SignOut,
                AccountAction::Check => spotify::Command::Start,
            });
        }
        if let Some(action) = config.accounts.apple_music.take()
            && let Some(player) = start_apple_music(&mut apple_music, &stage, &config)
        {
            let _ = player.command(&match action {
                AccountAction::SignIn => apple_music::Command::SignIn,
                AccountAction::SignOut => apple_music::Command::SignOut,
                AccountAction::Check => apple_music::Command::Start,
            });
        }
        if let Some(action) = config.accounts.tiktok.take()
            && let Some(app) = start_tiktok(&mut tiktok, &stage, phone_layers.as_ref(), &config)
        {
            let _ = app.command(&match action {
                AccountAction::SignIn => tiktok::Command::SignIn,
                AccountAction::SignOut => tiktok::Command::SignOut,
                AccountAction::Check => tiktok::Command::Start,
            });
        }
        // The cargo's paper asked for at the clerk (the plugin's), or taken back (collected, put down, the player
        // gone): in the phone's frame, large in the middle of the screen, with the mouse; then the phone as it was.
        let paper = Some(bridge.bridge().plugin.note_wanted.load(Ordering::Acquire)).filter(|&kind| kind != 0);
        if paper != handset_paper
            && let Some(handset) = &handset
        {
            handset_paper = paper;
            handset_peek = None;
            let _ = handset.post_json(&paper_message(paper));
            // A paper coming with the phone away: shown a moment later, once the page has drawn it (owner
            // 2026-11-19: the phone flashed up for an instant before the trailer's legs). Going: at once.
            if paper.is_some() && !handset_shown {
                paper_reveal = Some(Instant::now() + PAPER_REVEAL);
            } else {
                paper_reveal = None;
                let _ = handset.show(paper.is_some() || handset_shown);
                let shown = if paper.is_some() { westplay_shared::PHONE_PAPER } else { u32::from(handset_shown) };
                host.phone_shown.store(shown, Ordering::Release);
            }
            if let Some(app) = &tiktok {
                let _ = app.out(handset_shown && paper.is_none());
            }
            log!("[handset] {}", if paper.is_some() { "a paper to sign" } else { "the paper put away" });
        }
        if let (Some(at), Some(handset)) = (paper_reveal, &handset)
            && Instant::now() >= at
        {
            paper_reveal = None;
            if handset_paper.is_some() {
                let _ = handset.show(true);
                host.phone_shown.store(westplay_shared::PHONE_PAPER, Ordering::Release);
            }
        }
        // The fuel's bill asked for at the pump's attendant (the plugin's), or taken back: the phone shows it as the
        // paper is (large, with the mouse), on Nupay; then the phone as it was.
        let bill = bridge.bridge().plugin.fuel_bill.read().filter(|bill| bill.id != 0);
        if bill != handset_bill
            && let Some(handset) = &handset
        {
            handset_bill = bill;
            handset_peek = None;
            let _ = handset.post_json(&bill_message(bill.as_ref()));
            let large = bill.is_some() || handset_paper.is_some();
            let _ = handset.show(large || handset_shown);
            let shown = if large { westplay_shared::PHONE_PAPER } else { u32::from(handset_shown) };
            host.phone_shown.store(shown, Ordering::Release);
            if let Some(app) = &tiktok {
                let _ = app.out(handset_shown && !large);
            }
            log!("[handset] {}", if bill.is_some() { "the fuel's bill to pay" } else { "the fuel's bill put away" });
        }
        // A notification's time on a phone put away is up: away again.
        if handset_peek.is_some_and(|until| Instant::now() >= until) {
            handset_peek = None;
            if !handset_shown && handset_paper.is_none() && handset_bill.is_none() {
                if let Some(handset) = &handset {
                    let _ = handset.show(false);
                }
                host.phone_shown.store(0, Ordering::Release);
            }
        }

        // Checked often so the game is picked up right away when it starts or switches GPU.
        if last_check.elapsed() >= Duration::from_millis(200) {
            last_check = Instant::now();
            // The game's window showing or not (minimised, on another desktop): the page pauses its music
            // meanwhile. Without a game, nothing is away.
            let shown = is_shown(bridge.bridge().plugin.pid.load(Ordering::Relaxed));
            if shown != game_shown {
                game_shown = shown;
                let _ = webview.post_json(&game_shown_message(shown));
            }
            let running = is_running(bridge.bridge().plugin.pid.load(Ordering::Relaxed));
            if running != game_running {
                game_running = running;
                alert_watch.reset();
                if !running {
                    players_closing = true;
                    log!("the game closed: its web players are closed");
                }
                let _ = webview.post_json(&game_message(running));
            }
            let pid = bridge.bridge().plugin.pid.load(Ordering::Relaxed);
            if pid != elevated_pid {
                elevated_pid = pid;
                game_elevated = is_elevated(pid);
                if game_elevated {
                    log!("the game runs as administrator: WestPlay can't read its route, guidance or weather");
                }
            }
            let under_truckersmp = if !running {
                false
            } else if truckersmp || truckersmp_checked.elapsed() < TRUCKERSMP_CHECK_EVERY {
                truckersmp
            } else {
                truckersmp_checked = Instant::now();
                has_module(pid, TRUCKERSMP_MODULES)
            };
            if under_truckersmp != truckersmp {
                truckersmp = under_truckersmp;
                log!("TruckersMP: {}", if truckersmp { "the game runs under it" } else { "not running" });
                let _ = webview.post_json(&truckersmp_message(truckersmp));
            }
            let wanted = game_adapter(&bridge);
            status.game_adapter = wanted.and_then(|luid| graphics::name_of(luid, &adapters));
            // The cab on another GPU than it's drawn on (the game's, or the player's choice): WebView2 is
            // started again on it.
            let cab_wanted = graphics::choose(chosen_gpu.as_deref(), wanted, cab_gpu, &adapters);
            if let (Some(now), Some(next)) = (cab_gpu, cab_wanted)
                && !graphics::same(now, next)
            {
                let name = graphics::name_of(next, &adapters).unwrap_or_default();
                log!("the cab moves to {name}");
                gpu_changed = true;
                break;
            }
            let needs_rebuild = match &pipeline {
                None => true,
                Some(p) => p.capture.broken() || wanted.is_some_and(|luid| !same_luid(luid, p.gpu.luid)),
            };
            if needs_rebuild {
                host.frame_generation.store(0, Ordering::Release);
                host.phone_generation.store(0, Ordering::Release);
                pipeline = None;
                generation_counter = generation_counter.wrapping_add(1);
                let generation = ((std::process::id() & 0x00ff_ffff) << 8) | (generation_counter & 0xff);
                match build_pipeline(&config, &stage, phone_layers.as_ref(), phone_dims, &bridge, wanted, generation) {
                    Ok(built) => {
                        startup::mark(if wanted.is_some() {
                            "host: capturing for the game's GPU"
                        } else {
                            "host: capturing (game not seen yet)"
                        });
                        host.frame_width.store(config.width, Ordering::Relaxed);
                        host.frame_height.store(config.height, Ordering::Relaxed);
                        host.frame_generation.store(generation, Ordering::Release);
                        if built.phone.is_some() {
                            host.phone_generation.store(generation, Ordering::Release);
                        }
                        status.adapter = Some(built.gpu.adapter_name.clone());
                        status.error = None;
                        pipeline = Some(built);
                    }
                    Err(e) => status.error = Some(format!("capture pipeline failed: {e}")),
                }
            }
            if last_stats.elapsed() >= Duration::from_secs(1) {
                last_stats = Instant::now();
                let plugin = &bridge.bridge().plugin;
                let frames = pipeline.as_ref().map_or(0, |p| p.capture.frames.load(Ordering::Relaxed));
                // WestPlay's screen drawn, or the cab's mark in a tablet truck (the tablet itself isn't said there,
                // as it's drawn from outside the cab too): as the camera tells the cab drawn.
                let screen_frame = camera::drawn_frame(
                    plugin.screen_frame.load(Ordering::Relaxed),
                    plugin.cab_frame.load(Ordering::Relaxed),
                );
                let screen_age = plugin.heartbeat.load(Ordering::Relaxed).saturating_sub(screen_frame);
                status.frames_per_second = frames.saturating_sub(last_frames);
                status.dropped_total = pipeline.as_ref().map_or(0, |p| p.capture.dropped.load(Ordering::Relaxed));
                // A game that crashed leaves its pid and counters behind, frozen: alive is its heartbeat
                // moving.
                let alive = game_liveness.update(plugin.heartbeat.load(Ordering::Relaxed), Instant::now());
                status.game_connected = plugin.pid.load(Ordering::Relaxed) != 0 && alive && bridge.matches();
                status.game = westplay_setup::game::current();
                // A plugin of another version (the game still running from before an update): not used.
                if !bridge.matches() && plugin.pid.load(Ordering::Relaxed) != 0 {
                    status.error = Some("the game's WestPlay plugin is another version: restart the game".into());
                }
                status.screen_drawing = status.game_connected && screen_frame != 0 && screen_age < 120;
                status.truck_supported = match plugin.truck_supported.load(Ordering::Relaxed) {
                    _ if !status.game_connected => None,
                    westplay_shared::TRUCK_SUPPORTED => Some(true),
                    westplay_shared::TRUCK_UNSUPPORTED => Some(false),
                    _ => None,
                };
                status.game_elevated = status.game_connected && game_elevated;
                let mut profiles = vec![(performance::Profile::Cab, webview.environment())];
                if let Some(spotify) = &spotify {
                    profiles.push((performance::Profile::Spotify, spotify.view().environment()));
                }
                if let Some(apple_music) = &apple_music {
                    profiles.push((performance::Profile::AppleMusic, apple_music.view().environment()));
                }
                let sample = status.game_connected.then(|| performance::Sample {
                    game_frames: plugin.heartbeat.load(Ordering::Relaxed),
                    in_view: status.screen_drawing,
                    sent: frames,
                    captured: pipeline.as_ref().map_or(0, |p| p.capture.arrived.load(Ordering::Relaxed)),
                    waited: status.dropped_total,
                });
                meter.second(Instant::now(), sample, &webview, &profiles);
                (config.on_status)(&status);
                last_frames = frames;
            }
        }

        let customize_version = config.customize_changed.load(Ordering::Acquire);
        if customize_posted != Some(customize_version) {
            customize_posted = Some(customize_version);
            let (dir, list) = (customize_dir.clone(), customize_list.clone());
            std::thread::spawn(move || {
                let _ = list.send(customize_list_message(&customize::list(&dir)));
            });
        }
        while let Ok(message) = customize_listed.try_recv() {
            let _ = webview.post_json(&message);
            customize_message = Some(message);
        }

        if applied_settings != Some(config.settings.generation()) {
            applied_settings = Some(config.settings.generation());
            // The player may have chosen another GPU: the list again too (an external GPU plugged in).
            let graphics = config.settings.get().graphics;
            if graphics.gpu != chosen_gpu {
                chosen_gpu = graphics.gpu;
                adapters = graphics::adapters();
                status.adapters = adapters.clone();
            }
            let interaction = config.settings.get().interaction;
            pointer.set_sensitivity(interaction.sensitivity.get());
            // Slower in the phone (owner 2026-11-02: "too sensitive"): it's smaller on the screen than the cab's page.
            let phone_scale = if phone_dims.0 > westplay_shared::PHONE_WIDTH {
                HANDSET_SENSITIVITY * 1.35
            } else {
                HANDSET_SENSITIVITY
            };
            handset_pointer.set_sensitivity(interaction.sensitivity.get() * phone_scale);
            host.interaction_key.store(interaction.key.virtual_key(), Ordering::Release);
            let toggle = interaction.mode == settings::InteractionMode::Toggle;
            host.interaction_toggle.store(u32::from(toggle), Ordering::Release);
            for (keybind, value) in Keybind::ALL.into_iter().zip(config.settings.get().keybinds.bridge_values()) {
                host.keybind_slot(keybind).store(value, Ordering::Release);
            }
            let character = config.settings.get().character;
            host.character_sensitivity.store(character.sensitivity.get(), Ordering::Release);
            host.character_enabled.store(u32::from(character.enabled), Ordering::Release);
            host.character_key.store(character.key.get(), Ordering::Release);
            host.character_person.store(character.person.get() + 1, Ordering::Release);
            host.character_hints.store(if character.hints.0 { 1 } else { 2 }, Ordering::Release);
            host.character_hints_key.store(character.hints_key.get(), Ordering::Release);
            let switch = |on: settings::Shown| if on.0 { 1 } else { 2 };
            host.freight_papers.store(switch(character.papers), Ordering::Release);
            host.manual_refuel.store(switch(character.refuel), Ordering::Release);
            host.pay_attendant.store(switch(character.pay_attendant), Ordering::Release);
            host.virtual_mirrors.store(config.settings.get().virtual_mirrors.bridge_value(), Ordering::Release);
            cursor::set_size(config.settings.get().cursor_size.factor());
            let screen = config.settings.get().screen;
            let res_code = match screen.resolution {
                westplay_setup::game_mod::ScreenResolution::GameDefault => westplay_shared::SCREEN_RES_GAME_DEFAULT,
                westplay_setup::game_mod::ScreenResolution::Standard => westplay_shared::SCREEN_RES_STANDARD,
                westplay_setup::game_mod::ScreenResolution::High => westplay_shared::SCREEN_RES_HIGH,
                westplay_setup::game_mod::ScreenResolution::Ultra => westplay_shared::SCREEN_RES_ULTRA,
            };
            host.screen_resolution.store(res_code, Ordering::Release);
            let split_code = match screen.split {
                westplay_setup::game_mod::Split::Off => 0,
                westplay_setup::game_mod::Split::MapLeft => 1,
                westplay_setup::game_mod::Split::MapRight => 2,
            };
            host.screen_split.store(split_code, Ordering::Release);
            // Never on (owner 2026-11-18: with it, the music played on with the ignition cut, and the screen didn't
            // power up again; the game draws no screen without the electrics anyway). The setting is left, unused.
            host.keep_screen_on.store(0, Ordering::Release);
            let latest = config.settings.get();
            audio_output = latest.audio_output;
            let _ = webview.post_json(&settings_message(&latest));
        }
        // The mouse in the phone while it's out (its own pointer and cursor, at its size), else in the cab's screen.
        if let Some(handset) = handset.as_deref().filter(|_| handset_shown || handset_paper.is_some() || handset_bill.is_some()) {
            // Over TikTok (under the phone's page, where its app cuts it open), the mouse is TikTok's page's; the
            // phone's own page is told it left.
            let mouse = handset_pointer.update_with(&bridge.bridge().plugin.pointer, |kind, keys, data, point| {
                if tiktok.as_ref().is_some_and(|app| app.mouse(kind, keys, data, point)) {
                    if !handset_left {
                        handset_left = true;
                        let none = COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_NONE;
                        let _ = handset.send_mouse(COREWEBVIEW2_MOUSE_EVENT_KIND_LEAVE, none, 0, point);
                    }
                } else {
                    handset_left = false;
                    let _ = handset.send_mouse(kind, keys, data, point);
                }
            });
            handset_cursor.follow(mouse, Instant::now());
        } else {
            let mouse = pointer.update(&bridge.bridge().plugin.pointer, &webview);
            cursor.follow(mouse, Instant::now());
        }
        // The mouse's speed while walking, as the player set it in the game (`-`, `=`): into the settings (the app
        // keeps them).
        let walked = bridge.bridge().plugin.character_sensitivity.swap(0, Ordering::AcqRel);
        if walked != 0 {
            let mut changed = config.settings.get();
            changed.character.sensitivity = settings::CharacterSensitivity::new(f64::from(walked));
            config.settings.set(changed);
        }
        // The truck's commands the phones hold, as the plugin gives them to the game.
        // And the suspension by itself, if the player turned it on (auto_suspension.rs).
        let susp_settings = config.settings.get();
        let auto_held = auto_suspension.tick_with_damping(
            susp_settings.suspension_auto,
            susp_settings.suspension_damping,
            Duration::from_secs(u64::from(settings::suspension_auto_delay(&susp_settings))),
            Duration::from_secs(u64::from(settings::suspension_auto_return(&susp_settings))),
            Duration::from_secs(u64::from(settings::suspension_auto_brake(&susp_settings))),
            last_telemetry.as_ref(),
            Instant::now(),
        );
        host.truck_commands.store(truck_commands.held(Instant::now()) | auto_held, Ordering::Release);
        // Brake fade: the discs' temperatures and the brake left, logged every 2 s while the truck moves.
        {
            let job = bridge.bridge().plugin.job.read().filter(|job| job.active && job.cargo_loaded);
            let cargo = job.map_or(0.0, |job| job.cargo_mass);
            let fade_settings = config.settings.get();
            let level = fade_settings.brake_fade_level;
            let fade = brake_fade.tick(level, last_telemetry.as_ref(), cargo, last_weather.as_ref(), Instant::now());
            let strength = if fade_settings.brake_fade.0 { fade.strength } else { 1.0 };
            host.brake_strength.store(strength.to_bits(), Ordering::Release);
            host.brake_temp_truck.store(fade.truck.to_bits(), Ordering::Release);
            host.brake_temp_trailer.store(fade.trailer.unwrap_or(f32::NAN).to_bits(), Ordering::Release);
            host.brake_sound.store(
                0x1_0000
                    | u32::from(brake_fade::grinds(&fade, level) && fade_settings.brake_fade.0) << 17
                    | u32::from(fade_settings.brake_squeal_volume.0)
                    | u32::from(fade_settings.brake_warning_volume.0) << 8,
                Ordering::Release,
            );
            // The alarm's brake left, from its temperature (on the realistic level, the same point of the fade on all).
            let alarm = brake_fade::strength_at(f32::from(settings::brake_alarm_c(&fade_settings)), brake_fade::Level::Realistic);
            host.brake_alarm.store(alarm.to_bits(), Ordering::Release);
            let corner = fade_settings.brake_hud_corner as u32;
            let size = fade_settings.brake_hud_size as u32;
            let level_bits = match level {
                brake_fade::Level::Realistic => 0u32,
                brake_fade::Level::Light => 1,
                brake_fade::Level::Hard => 2,
            };
            host.brake_hud.store(
                0x100
                    | u32::from(fade_settings.brake_hud.0 && fade_settings.brake_fade.0)
                    | corner << 1
                    | size << 3
                    | u32::from(fade_settings.brake_hud_on_foot) << 5
                    | u32::from(fade_settings.brake_smoke.0 && fade_settings.brake_fade.0) << 6
                    | level_bits << 9,
                Ordering::Release,
            );
            // The phone told once as the brake weakens and once as it's gone (owner 2026-11-21), again only once
            // it has all come back.
            if strength >= 0.999 {
                brake_told = 0;
            } else if strength <= westplay_shared::BRAKE_HOT_STRENGTH && brake_told < 1
                || strength <= alarm && brake_told < 2
            {
                let gone = strength <= alarm;
                brake_told = if gone { 2 } else { 1 };
                let alert = alerts::Alert::new(alerts::Group::Brake, "truck", if gone { "brakeGone" } else { "brakeHot" });
                let allowed = alert_filter.allows(&alert);
                log!("[brake fade] the phone told: {} ({})", alert.kind, if allowed { "sent" } else { "its group is off" });
                if allowed {
                    notify_phone!(&alert.message());
                }
            }
            // The phone's Freios app: the discs live, once a second.
            if brakes_posted.is_none_or(|at| at.elapsed() >= Duration::from_secs(1)) {
                brakes_posted = Some(Instant::now());
                let message = serde_json::json!({
                    "type": "brakes",
                    "truck": fade.truck.round(),
                    "trailer": fade.trailer.map(f32::round),
                    "strength": (strength * 100.0).round(),
                });
                let _ = webview.post_json(&message.to_string());
            }
            let moving = last_telemetry.as_ref().is_some_and(|t| t.speed.abs() > 0.5 || t.brake > 0.01);
            if moving && brake_fade_logged.is_none_or(|at| at.elapsed() >= Duration::from_secs(2)) {
                brake_fade_logged = Some(Instant::now());
                log!(
                    "[brake fade] model: truck {:.0} C, trailer {}, strength {:.2}, game {:.0} C",
                    fade.truck,
                    fade.trailer.map_or("-".to_owned(), |t| format!("{t:.0} C")),
                    fade.strength,
                    last_telemetry.as_ref().map_or(0.0, |t| t.brake_temperature)
                );
            }
        }
        // The game played changed (`westplay_setup::game::current`): its own account.
        if westplay_setup::game::current() != wallet_game {
            wallet_game = westplay_setup::game::current();
            wallet = wallet::Wallet::open(&config.data_dir, wallet_game.sdk_id());
            let _ = webview.post_json(&wallet.message());
        }
        // The money the game moved, as it did.
        if let Some(money) = bridge.bridge().plugin.money.read()
            && wallet.take(&money)
        {
            let _ = webview.post_json(&wallet.message());
        }
        // Live in-memory Bank mirror (exact balance, active loans, game loan offers): keeps Nupay synced 1:1.
        if status.game_connected {
            if wallet.poll_live_bank(&bridge.bridge().bank) {
                let _ = webview.post_json(&wallet.message());
            }
        } else if wallet.on_game_stopped() {
            let _ = webview.post_json(&wallet.message());
        }
        // The fuel filled by hand owed to the pump's attendant: told on the phone as it's owed.
        if let Some(owed) = bridge.bridge().plugin.fuel_owed.read()
            && let Some(alert) = alert_owed.check(&owed)
            && alert_filter.allows(&alert)
        {
            notify_phone!(&alert.message());
        }
        // Money in and out as notifications on the phone (a job paid, a fine, a toll, fuel, a repair at the save).
        let fresh = wallet.take_fresh();
        if !fresh.is_empty() {
            for alert in alerts::from_entries(&fresh, wallet.currency()) {
                if alert_filter.allows(&alert) {
                    notify_phone!(&alert.message());
                }
            }
        }
        // The player's keybinds pressed in the game: the page acts on each.
        for keybind in Keybind::ALL {
            let presses = bridge.bridge().plugin.keybind_press_slot(keybind).load(Ordering::Acquire);
            let new = presses.wrapping_sub(keybind_presses[keybind as usize]);
            keybind_presses[keybind as usize] = presses;
            // More at once than a player presses in a moment: another plugin's count (the game restarted).
            if new > MOST_KEYBIND_PRESSES {
                continue;
            }
            for _ in 0..new {
                match (keybind, keybind_message(keybind)) {
                    (_, Some(message)) => {
                        let _ = webview.post_json(&message);
                    }
                    // The phone's is the host's own: out, or put away.
                    // (Not while a paper to sign is in its frame.)
                    (Keybind::Phone | Keybind::Character, None) if handset_paper.is_some() || handset_bill.is_some() => {}
                    (Keybind::Phone, None) => {
                        if let Some(handset) = &handset {
                            // Its key while only its top shows (a notification): out, whole.
                            handset_shown = !handset_shown;
                            handset_peek = None;
                            let _ = handset.show(handset_shown);
                            host.phone_shown.store(u32::from(handset_shown), Ordering::Release);
                            if let Some(app) = &tiktok {
                                let _ = app.out(handset_shown);
                            }
                            if handset_shown
                                && let (Some(player), Some((x, y, w, h, crop)), Some(layers)) =
                                    (&youtube, handset_youtube_rect, &phone_layers)
                            {
                                let _ = player.attach_to_layers(&stage, layers);
                                let _ = player.show_in(phone_dims, x, y, w, h, crop);
                            } else if !handset_shown
                                && let (Some(player), Some(layers)) = (&youtube, &phone_layers)
                                && player.is_on_phone()
                            {
                                // Put away: the video goes back to the cab (its widget places it again), the map too.
                                let _ = player.command(&youtube::Command::Hide);
                                let _ = player.attach_to_stage(&stage, layers);
                            }
                            log!("[handset] {}", if handset_shown { "out" } else { "put away" });
                        }
                    }
                    // The phone out on its Personagem app (owner 2026-11-07: "WestPlay in front, to change the
                    // person, without leaving the game"). The phone's own key puts it away.
                    (Keybind::Character, None) => {
                        if let Some(handset) = &handset {
                            if !handset_shown {
                                handset_shown = true;
                                handset_peek = None;
                                let _ = handset.show(true);
                                host.phone_shown.store(1, Ordering::Release);
                                if let Some(app) = &tiktok {
                                    let _ = app.out(true);
                                }
                            }
                            let _ = handset.post_json(r#"{"type":"openApp","app":"character"}"#);
                            log!("[handset] out on Personagem");
                        }
                    }
                    // Emergency rough-road suspension trigger (owner 2026-11-22): lifts into rough-road mode and
                    // automatically lowers back down once on smooth pavement for the configured return delay.
                    (Keybind::RoughRoad, None) => {
                        auto_suspension.force_rough_road(last_telemetry.as_ref(), Instant::now());
                    }
                    // The key hints bar's: the plugin's own (it hides the bar itself).
                    _ => {}
                }
            }
        }

        while let Ok(update) = route_watch.updates.try_recv() {
            // The ways through a route's junctions come later: only for the route still shown.
            if let RouteUpdate::Paths { id, .. } = &update
                && route_number != Some(*id)
            {
                continue;
            }
            let message = route_message_for(&update);
            match update {
                RouteUpdate::Next(_) => route_next_message = Some(message.clone()),
                RouteUpdate::Paths { .. } => route_paths_message = Some(message.clone()),
                RouteUpdate::Route { id, nodes, waypoints } => {
                    route_number = Some(id);
                    route_uids = Some(nodes.iter().map(|node| node.uid).collect());
                    route_waypoints = waypoints;
                    (route_message, route_paths_message, route_next_message) = (Some(message.clone()), None, None);
                }
                RouteUpdate::None => {
                    route_number = None;
                    route_uids = None;
                    route_waypoints.clear();
                    (route_message, route_paths_message, route_next_message) = (Some(message.clone()), None, None);
                }
            }
            let _ = webview.post_json(&message);
            // The player's stops along it, after the route itself (the page finds them in it).
            if route_message.as_deref() == Some(message.as_str()) {
                let hidden = following.as_ref().map(|follow| follow.handover.handed()).unwrap_or_default();
                let stops = stops_message(
                    route_number,
                    route_uids.as_deref(),
                    &route_waypoints,
                    &[hidden, &held_vias].concat(),
                );
                let _ = webview.post_json(&stops);
                route_stops_message = Some(stops);
            }
        }

        while let Ok(weather) = weather_watch.updates.try_recv() {
            last_weather = weather;
            let message = weather_message_for(weather);
            let _ = webview.post_json(&message);
            weather_message = Some(message);
        }

        while let Ok(update) = map_watch.updates.try_recv() {
            if map_shown != (update.folder.clone(), update.built) {
                map_shown = (update.folder.clone(), update.built);
                let _ = webview.post_json(&maps::map_message(map_shown.0.as_deref(), map_shown.1));
                (status.map_folder, status.map_built) = map_shown.clone();
                // The map's fuel pumps, for refuelling by hand (pumps.rs).
                map_pumps = map_shown.0.as_deref().and_then(|folder| pumps::load(&maps_dir, folder));
                log!(
                    "[pumps] {} of the map shown",
                    map_pumps.as_ref().map_or("none known".to_owned(), |p| p.len().to_string())
                );
                (config.on_status)(&status);
            }
            if status.map != update.status || status.profile_maps != update.profiles {
                status.map = update.status;
                status.profile_maps = update.profiles;
                (config.on_status)(&status);
            }
        }

        while let Ok(player) = player_watch.updates.try_recv() {
            player_messages = player_messages_for(&player);
            let mut wallet_changed = false;
            if let Some(money) = player.money
                && wallet.saved(money)
            {
                wallet_changed = true;
            }
            if wallet.update_save_loans(player.loans.clone(), player.save_path.clone()) {
                wallet_changed = true;
            }
            if wallet_changed {
                let _ = webview.post_json(&wallet.message());
            }
            for message in &player_messages {
                let _ = webview.post_json(message);
            }
        }
        while let Ok(found) = player_watch.speakers.try_recv() {
            part_speakers = Some(found);
        }

        // What the phone in the game's page asks of the host alone (never a player's own phone's to ask): TikTok
        // under it, and a notification still on it.
        let (handset_own, handset_rest) = handset_messages(handset.as_deref());
        for message in handset_own {
            match serde_json::from_str::<HandsetMessage>(&message) {
                Ok(HandsetMessage::Tiktok { command }) => {
                    let idle = matches!(command, tiktok::Command::Hide | tiktok::Command::Notifications { .. });
                    if tiktok.is_none() && idle {
                        // Nothing to hide: TikTok isn't running (the phone says it again when it is).
                    } else if let Some(app) = start_tiktok(&mut tiktok, &stage, phone_layers.as_ref(), &config) {
                        app.set_bounds(phone_dims.0, phone_dims.1);
                        let _ = app.command(&command);
                        let _ = app.out(handset_shown);
                    }
                }
                Ok(HandsetMessage::Typing { selection }) => {
                    handset_typing = selection != 0;
                    bridge.bridge().typing.selection.store(selection, Ordering::Release);
                }
                Ok(HandsetMessage::HandsetYoutube { command }) => {
                    if command == youtube::Command::Close {
                        handset_youtube_rect = None;
                        if let Some(closing) = youtube.take() {
                            closing.close(&stage, phone_layers.as_ref());
                        }
                        youtube_message = None;
                        let _ = webview.post_json(&youtube::state_message(None));
                    } else if youtube.is_none() && matches!(command, youtube::Command::Hide) {
                        handset_youtube_rect = None;
                    } else if let Some(player) = start_youtube(&mut youtube, &stage, &config) {
                        match &command {
                            youtube::Command::Show { x, y, width, height, crop } => {
                                handset_youtube_rect = Some((*x, *y, *width, *height, *crop));
                                if let Some(layers) = &phone_layers {
                                    if let Err(e) = player.attach_to_layers(&stage, layers) {
                                        log!("[youtube] attach_to_layers failed: {e}");
                                    }
                                }
                                if let Err(e) = player.show_in(phone_dims, *x, *y, *width, *height, *crop) {
                                    log!("[youtube] show_in failed: {e}");
                                } else {
                                    log!("[youtube] shown on handset ({width}x{height} at {x},{y})");
                                }
                            }
                            youtube::Command::Hide => {
                                handset_youtube_rect = None;
                                if player.is_on_phone() {
                                    let _ = player.command(&youtube::Command::Hide);
                                    if let Some(layers) = &phone_layers {
                                        let _ = player.attach_to_stage(&stage, layers);
                                    }
                                }
                            }
                            _ => {
                                let _ = player.command(&command);
                                if command == youtube::Command::SignOut
                                    && let Some(music) = &youtube_music
                                {
                                    let _ = music.reload();
                                }
                            }
                        }
                    }
                }
                // The notifications' groups turned off in the phone's Settings, and one of each to try them.
                Ok(HandsetMessage::Alerts { off }) => alert_filter.set_off(&off),
                Ok(HandsetMessage::TestAlerts) => {
                    for alert in alerts::samples(wallet.currency()) {
                        if alert_filter.allows(&alert) {
                            notify_phone!(&alert.message());
                        }
                    }
                }
                Ok(HandsetMessage::PhoneBanner { showing }) => {
                    if handset_peek.is_some() {
                        let left = if showing { PEEK_FOR } else { PEEK_AFTER };
                        handset_peek = Some(Instant::now() + left);
                    }
                }
                // The paper collected, signed, or put down: the plugin takes it from there.
                Ok(HandsetMessage::PaperDone { collected }) => {
                    if let Some(kind) = handset_paper {
                        let done = if collected { kind } else { westplay_shared::NOTE_LEFT };
                        host.note_done.store(done, Ordering::Release);
                    }
                }
                // The fuel's bill paid on Nupay (in the game's money: the wallet's entry), or put down unpaid.
                Ok(HandsetMessage::FuelPaid { id, amount, detail }) => {
                    if handset_bill.is_some_and(|bill| bill.id == id) {
                        wallet.paid("fuel", amount, &detail);
                        let _ = webview.post_json(&wallet.message());
                        host.fuel_paid.store(id, Ordering::Release);
                        // Paid, the phone put away (owner 2026-11-12: "when I pay, the phone stays open"), even
                        // one taken out by its key before: it goes as the plugin takes the bill back.
                        handset_shown = false;
                        log!("[handset] the fuel's bill paid ({amount})");
                    }
                }
                Ok(HandsetMessage::FuelPutDown { id }) => {
                    if handset_bill.is_some_and(|bill| bill.id == id) {
                        host.fuel_paid.store(id | westplay_shared::FUEL_NOT_PAID, Ordering::Release);
                    }
                }
                // The phone's page loaded (anew): the paper again, and TikTok's account.
                Ok(HandsetMessage::Ready) => {
                    if let (Some(handset), Some(_)) = (&handset, handset_paper) {
                        let _ = handset.post_json(&paper_message(handset_paper));
                    }
                    if let (Some(handset), Some(_)) = (&handset, handset_bill) {
                        let _ = handset.post_json(&bill_message(handset_bill.as_ref()));
                    }
                    if let (Some(handset), Some(app)) = (&handset, &tiktok) {
                        let signing_in = tiktok_window_open || tiktok_confirming;
                        let message = tiktok::state_message(tiktok_state.as_ref(), signing_in, app.live());
                        let _ = handset.post_json(&message);
                    }
                }
                Err(_) => {}
            }
        }
        // The suspension's auto reset and profile spring stiffness in the games' options as the player chose (with
        // them closed), and whether auto reset is there yet, told to the pages (`{"type":"suspensionOption","applied":…}`).
        let current_settings = config.settings.get();
        if let Some(applied) =
            suspension_option.check(current_settings.suspension_auto_reset.0, current_settings.suspension_damping.level())
        {
            let message = serde_json::json!({ "type": "suspensionOption", "applied": applied }).to_string();
            let _ = webview.post_json(&message);
            if let Some(handset) = handset.as_deref() {
                let _ = handset.post_json(&message);
            }
            remote.broadcast(&message);
        }
        if wallet.try_apply_pending_loan() {
            let message = wallet.message();
            let _ = webview.post_json(&message);
            if let Some(handset) = handset.as_deref() {
                let _ = handset.post_json(&message);
            }
            remote.broadcast(&message);
        }
        for message in webview.take_page_messages().into_iter().chain(remote.take()).chain(handset_rest) {
            // Adjust Screen's: passed on, the cab's page and the phones each taking what's theirs.
            if remote::Remote::relayed(&message) {
                let _ = webview.post_json(&message);
                continue;
            }
            match parse_page_message(&message) {
                // The page announces itself when it (re)loads; it missed everything sent before, so send the
                // current state again (some of it is otherwise only sent when it changes).
                Some(PageMessage::Ready) => {
                    for slot in &host.mirror_rect {
                        slot.store(0, Ordering::Release);
                    }
                    // The suspension option told the page again.
                    suspension_option = SuspensionOption::default();
                    // No camera on a page loaded anew: the videos' pictures no longer put away for it.
                    if let Some(player) = &youtube {
                        let _ = player.cover(false);
                    }
                    if let Some(music) = &youtube_music {
                        let _ = music.cover(false);
                    }
                    // A page loaded anew asks again if it wants it.
                    spectrum = false;
                    cursor_told = None;
                    startup::mark("page: running (its scripts loaded)");
                    last_telemetry = None;
                    last_truck = None;
                    last_job = None;
                    last_layout = None;
                    typing_state = None;
                    let route = route_message
                        .iter()
                        .chain(&route_stops_message)
                        .chain(&route_paths_message)
                        .chain(&route_next_message);
                    let _ = webview.post_json(&game_shown_message(game_shown));
                    let _ = webview.post_json(&game_message(game_running));
                    let _ = webview.post_json(&truckersmp_message(truckersmp));
                    let _ = webview.post_json(&maps::map_message(map_shown.0.as_deref(), map_shown.1));
                    if spotify.is_some() {
                        let _ = webview.post_json(&spotify::state_message(spotify_state.as_ref(), spotify_signing_in));
                        if let Some(playback) = &spotify_playback {
                            let _ = webview.post_json(playback);
                        }
                    }
                    if apple_music.is_some() {
                        let signing_in = apple_music_window_open || apple_music_confirming;
                        let message = apple_music::state_message(apple_music_state.as_ref(), signing_in);
                        let _ = webview.post_json(&message);
                        if let Some(playback) = &apple_music_playback {
                            let _ = webview.post_json(playback);
                        }
                    }
                    if youtube_music.is_some() {
                        let signing_in = youtube_music_window_open || youtube_music_confirming;
                        let message = youtube_music::state_message(youtube_music_state.as_ref(), signing_in);
                        let _ = webview.post_json(&message);
                        if let Some(playback) = &youtube_music_playback {
                            let _ = webview.post_json(playback);
                        }
                    }
                    let _ = webview.post_json(&settings_message(&config.settings.get()));
                    if let Some(message) = &customize_message {
                        let _ = webview.post_json(message);
                    }
                    let youtube = youtube_message.iter().chain(&youtube_account_message);
                    let all = player_messages.iter().chain(route).chain(&weather_message).chain(youtube);
                    for message in all {
                        let _ = webview.post_json(message);
                    }
                    // The account, as it is now.
                    let _ = webview.post_json(&wallet.message());
                }
                Some(PageMessage::WalletClear) => {
                    wallet.clear_history();
                    log!("wallet: its history cleared (Nupay)");
                    let _ = webview.post_json(&wallet.message());
                }
                Some(PageMessage::WalletLoan { amount, days, rate }) => {
                    wallet.queue_loan(amount, days, rate);
                    let applied = wallet.try_apply_pending_loan();
                    log!("wallet: loan requested ({amount}, {days}d, rate {rate}; applied now: {applied})");
                    let message = wallet.message();
                    let _ = webview.post_json(&message);
                    if let Some(handset) = handset.as_deref() {
                        let _ = handset.post_json(&message);
                    }
                    remote.broadcast(&message);
                }
                Some(PageMessage::WalletLoanCancel) => {
                    wallet.cancel_pending_loan();
                    let message = wallet.message();
                    let _ = webview.post_json(&message);
                    if let Some(handset) = handset.as_deref() {
                        let _ = handset.post_json(&message);
                    }
                    remote.broadcast(&message);
                }
                Some(PageMessage::BankRefresh { id }) => {
                    if let Some(overtaken) = bank_rpc.ask_refresh(&bridge.bridge().bank, id, Instant::now()) {
                        let _ = webview.post_json(&overtaken);
                    }
                }
                Some(PageMessage::BankTakeLoan { id, offer_index }) => {
                    let (amount, detail) = wallet
                        .offer_summary(offer_index as usize)
                        .map(|(a, d, r)| (a, format!("{d}d · {}%", (r * 100.0).round() as i32)))
                        .unwrap_or((0, String::new()));
                    if let Some(overtaken) = bank_rpc.ask_take_loan(
                        &bridge.bridge().bank,
                        id,
                        offer_index,
                        amount,
                        detail,
                        Instant::now(),
                    ) {
                        let _ = webview.post_json(&overtaken);
                    }
                }
                Some(PageMessage::BankRepayLoan { id, loan_index }) => {
                    let (amount, detail) = wallet
                        .loan_summary(loan_index as usize)
                        .map(|(rem, orig)| (rem, format!("Original: {orig}")))
                        .unwrap_or((0, String::new()));
                    if let Some(overtaken) = bank_rpc.ask_repay_loan(
                        &bridge.bridge().bank,
                        id,
                        loan_index,
                        amount,
                        detail,
                        Instant::now(),
                    ) {
                        let _ = webview.post_json(&overtaken);
                    }
                }
                Some(PageMessage::Westplay { open }) => {
                    bridge.bridge().host.screen_handed_over.store(u32::from(!open), Ordering::Release);
                    if status.westplay_open != Some(open) {
                        status.westplay_open = Some(open);
                        (config.on_status)(&status);
                    }
                }
                Some(PageMessage::Startup { stage }) => startup::mark(&format!("page: {stage}")),
                Some(PageMessage::TruckStrip { strip }) => {
                    let mut settings = config.settings.get();
                    if settings.screen.truck_strip != strip {
                        settings.screen.truck_strip = strip;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                    }
                }
                Some(PageMessage::ScreenResolution { resolution }) => {
                    let mut settings = config.settings.get();
                    if settings.screen.resolution != resolution {
                        settings.screen.resolution = resolution;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[screen] live resolution changed from cabin: {resolution:?}");
                    }
                }
                Some(PageMessage::KeepScreenOn { on }) => {
                    let mut settings = config.settings.get();
                    if settings.screen.keep_screen_on != on {
                        settings.screen.keep_screen_on = on;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[screen] keep screen on with truck off: {}", if on { "on" } else { "off" });
                    }
                }
                // The Modo Personagem's switches, from a phone (the game's, or the player's own): the app's settings,
                // the companion's Personagem page the same (owner 2026-11-13).
                Some(
                    message @ (PageMessage::FreightPapers { .. }
                    | PageMessage::ManualRefuel { .. }
                    | PageMessage::PayAttendant { .. }),
                ) => {
                    let mut settings = config.settings.get();
                    let (slot, on, what) = match message {
                        PageMessage::FreightPapers { on } => (&mut settings.character.papers, on, "papers"),
                        PageMessage::ManualRefuel { on } => (&mut settings.character.refuel, on, "refuel"),
                        PageMessage::PayAttendant { on } => (&mut settings.character.pay_attendant, on, "pay"),
                        _ => unreachable!(),
                    };
                    if slot.0 != on {
                        *slot = settings::Shown(on);
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the Modo Personagem's {what}: {}", if on { "on" } else { "off" });
                    }
                }
                // The suspension set back by the game or not, from the Suspensão app: in the games' options at once if
                // they're closed, else once they are (`suspension_option`).
                Some(PageMessage::SuspensionAutoReset { on }) => {
                    let mut settings = config.settings.get();
                    if settings.suspension_auto_reset.0 != on {
                        settings.suspension_auto_reset = settings::Shown(on);
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the suspension set back by the game: {}", if on { "on" } else { "off" });
                    }
                    suspension_option.checked = None;
                }
                // How low the front may go, from the Suspensão app: the archive made again for the next start.
                Some(PageMessage::SuspensionLowFront { which }) => {
                    let mut settings = config.settings.get();
                    if let Some(which) = settings::LowFront::from_word(&which)
                        && settings.suspension_low_front != which
                    {
                        settings.suspension_low_front = which;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the front's lowest: {which:?}");
                    }
                }
                // How firm the dampers & springs are, from the Suspensão app: the archive made again and profiles synced.
                Some(PageMessage::SuspensionDamping { which }) => {
                    let mut settings = config.settings.get();
                    if let Some(which) = settings::Damping::from_word(&which)
                        && settings.suspension_damping != which
                    {
                        settings.suspension_damping = which;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the dampers: {which:?}");
                    }
                    suspension_option.checked = None;
                }
                // The suspension by itself, from the Suspensão app (owner 2026-11-19).
                Some(PageMessage::SuspensionAuto { on }) => {
                    let mut settings = config.settings.get();
                    if settings.suspension_auto != on {
                        settings.suspension_auto = on;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the suspension by itself: {}", if on { "on" } else { "off" });
                    }
                }
                // Rough-road activation delay (1..=5 s), from the Suspensão app (owner 2026-11-22).
                Some(PageMessage::SuspensionAutoDelay { secs }) => {
                    let secs = secs.clamp(1, 5);
                    let mut settings = config.settings.get();
                    if settings::suspension_auto_delay(&settings) != secs {
                        settings.suspension_auto_delay = secs;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the suspension rough-road delay: {secs} s");
                    }
                }
                // Smooth-road return delay (3..=15 s), from the Suspensão app (owner 2026-11-22).
                Some(PageMessage::SuspensionAutoReturn { secs }) => {
                    let secs = secs.clamp(3, 15);
                    let mut settings = config.settings.get();
                    if settings::suspension_auto_return(&settings) != secs {
                        settings.suspension_auto_return = secs;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the suspension smooth-road return delay: {secs} s");
                    }
                }
                // Hard-braking raise delay (1..=5 s), from the Suspensão app (owner 2026-11-23).
                Some(PageMessage::SuspensionAutoBrake { secs }) => {
                    let secs = secs.clamp(1, 5);
                    let mut settings = config.settings.get();
                    if settings::suspension_auto_brake(&settings) != secs {
                        settings.suspension_auto_brake = secs;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the suspension braking raise delay: {secs} s");
                    }
                }
                // Brake fade, on or off and how soon (owner 2026-11-21).
                Some(PageMessage::BrakeFade { on }) => {
                    let mut settings = config.settings.get();
                    if settings.brake_fade.0 != on {
                        settings.brake_fade = settings::Shown(on);
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] brake fade: {}", if on { "on" } else { "off" });
                    }
                }
                Some(PageMessage::BrakeFadeLevel { which }) => {
                    let level = match which.as_str() {
                        "light" => Some(brake_fade::Level::Light),
                        "realistic" => Some(brake_fade::Level::Realistic),
                        "hard" => Some(brake_fade::Level::Hard),
                        _ => None,
                    };
                    let mut settings = config.settings.get();
                    if let Some(level) = level.filter(|&level| level != settings.brake_fade_level) {
                        settings.brake_fade_level = level;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] brake fade: {level:?}");
                    }
                }
                // The brake fade's sounds and HUD (owner 2026-11-21).
                Some(
                    message @ (PageMessage::BrakeSqueal { .. }
                    | PageMessage::BrakeWarning { .. }
                    | PageMessage::BrakeHud { .. }
                    | PageMessage::BrakeHudCorner { .. }
                    | PageMessage::BrakeHudSize { .. }
                    | PageMessage::BrakeHudOnFoot { .. }
                    | PageMessage::BrakeSmoke { .. }
                    | PageMessage::BrakeAlarm { .. }),
                ) => {
                    let mut settings = config.settings.get();
                    let volume = |v: f64| settings::Volume70(v.clamp(0.0, 100.0).round() as u8);
                    match &message {
                        PageMessage::BrakeSqueal { volume: v } => settings.brake_squeal_volume = volume(*v),
                        PageMessage::BrakeWarning { volume: v } => settings.brake_warning_volume = volume(*v),
                        PageMessage::BrakeHud { on } => settings.brake_hud = settings::Shown(*on),
                        PageMessage::BrakeHudOnFoot { on } => settings.brake_hud_on_foot = *on,
                        PageMessage::BrakeSmoke { on } => settings.brake_smoke = settings::Shown(*on),
                        PageMessage::BrakeHudCorner { which } => {
                            if let Ok(corner) = serde_json::from_value(serde_json::Value::String(which.clone())) {
                                settings.brake_hud_corner = corner;
                            }
                        }
                        PageMessage::BrakeHudSize { which } => {
                            if let Ok(size) = serde_json::from_value(serde_json::Value::String(which.clone())) {
                                settings.brake_hud_size = size;
                            }
                        }
                        PageMessage::BrakeAlarm { celsius } => {
                            settings.brake_alarm_c = celsius.clamp(260.0, 395.0).round() as u16;
                        }
                        _ => {}
                    }
                    if settings != config.settings.get() {
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] brakes: {message:?}");
                    }
                }
                // How big the mouse's dot is, from a phone's Settings (owner 2026-11-19).
                Some(PageMessage::CursorSize { which }) => {
                    let mut settings = config.settings.get();
                    if let Some(which) = settings::CursorSize::from_word(&which)
                        && settings.cursor_size != which
                    {
                        settings.cursor_size = which;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the cursor's size: {which:?}");
                    }
                }
                // Where media audio plays: truck cab speakers (`cabin`) or phone/iPad (`device`).
                Some(PageMessage::AudioOutput { which }) => {
                    let mut settings = config.settings.get();
                    if let Some(which) = settings::AudioOutput::from_word(&which)
                        && settings.audio_output != which
                    {
                        settings.audio_output = which;
                        audio_output = which;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] audio output: {which:?}");
                    }
                }
                // Which virtual mirrors show, from a phone's Settings (owner 2026-11-18).
                Some(PageMessage::VirtualMirrors { which }) => {
                    let mut settings = config.settings.get();
                    if let Some(which) = settings::VirtualMirrors::from_word(&which)
                        && settings.virtual_mirrors != which
                    {
                        settings.virtual_mirrors = which;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the virtual mirrors shown: {which:?}");
                    }
                }
                Some(PageMessage::CharacterPerson { person }) => {
                    let mut settings = config.settings.get();
                    let person = settings::CharacterPerson::new(person);
                    if settings.character.person != person {
                        settings.character.person = person;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                        log!("[handset] the person who walks chosen in the phone: {}", person.get());
                    }
                }
                Some(PageMessage::Spectrum { on }) => spectrum = on,
                Some(PageMessage::CameraNudge { action }) => {
                    let press = match action.as_str() {
                        "back" => westplay_shared::CAMERA_NUDGE_BACK,
                        "forward" => westplay_shared::CAMERA_NUDGE_FORWARD,
                        "up" => westplay_shared::CAMERA_NUDGE_UP,
                        "down" => westplay_shared::CAMERA_NUDGE_DOWN,
                        "reset" => westplay_shared::CAMERA_NUDGE_RESET,
                        "tiltUp" => westplay_shared::CAMERA_NUDGE_TILT_UP,
                        "tiltDown" => westplay_shared::CAMERA_NUDGE_TILT_DOWN,
                        "turnLeft" => westplay_shared::CAMERA_NUDGE_TURN_LEFT,
                        "turnRight" => westplay_shared::CAMERA_NUDGE_TURN_RIGHT,
                        "wider" => westplay_shared::CAMERA_NUDGE_WIDER,
                        "narrower" => westplay_shared::CAMERA_NUDGE_NARROWER,
                        "left" => westplay_shared::CAMERA_NUDGE_LEFT,
                        "right" => westplay_shared::CAMERA_NUDGE_RIGHT,
                        "rollLeft" => westplay_shared::CAMERA_NUDGE_ROLL_LEFT,
                        "rollRight" => westplay_shared::CAMERA_NUDGE_ROLL_RIGHT,
                        "backFar" => westplay_shared::CAMERA_NUDGE_BACK_FAR,
                        "forwardFar" => westplay_shared::CAMERA_NUDGE_FORWARD_FAR,
                        _ => 0,
                    };
                    if press != 0 {
                        // A new count each press (the plugin takes each once), the press in the low byte.
                        let count = (host.camera_nudge.load(Ordering::Acquire) >> 8).wrapping_add(1) & 0x00ff_ffff;
                        host.camera_nudge.store((count.max(1) << 8) | press, Ordering::Release);
                    }
                }
                Some(PageMessage::FloatScreen { action }) => {
                    let press = match action.as_str() {
                        "toggle" => westplay_shared::FLOAT_TOGGLE,
                        "forward" => westplay_shared::FLOAT_FORWARD,
                        "back" => westplay_shared::FLOAT_BACK,
                        "up" => westplay_shared::FLOAT_UP,
                        "down" => westplay_shared::FLOAT_DOWN,
                        "left" => westplay_shared::FLOAT_LEFT,
                        "right" => westplay_shared::FLOAT_RIGHT,
                        "turnLeft" => westplay_shared::FLOAT_TURN_LEFT,
                        "turnRight" => westplay_shared::FLOAT_TURN_RIGHT,
                        "tiltUp" => westplay_shared::FLOAT_TILT_UP,
                        "tiltDown" => westplay_shared::FLOAT_TILT_DOWN,
                        "rollLeft" => westplay_shared::FLOAT_ROLL_LEFT,
                        "rollRight" => westplay_shared::FLOAT_ROLL_RIGHT,
                        "reset" => westplay_shared::FLOAT_RESET,
                        "bigger" => westplay_shared::FLOAT_BIGGER,
                        "smaller" => westplay_shared::FLOAT_SMALLER,
                        _ => 0,
                    };
                    if press != 0 {
                        // A new count each press (the plugin takes each once), the press in the low byte.
                        let count = (host.float_screen.load(Ordering::Acquire) >> 8).wrapping_add(1) & 0x00ff_ffff;
                        host.float_screen.store((count.max(1) << 8) | press, Ordering::Release);
                    }
                }
                Some(PageMessage::JobMarketRefresh { id }) => {
                    if let Some(overtaken) = job_market.ask_refresh(&bridge.bridge().job_market, id, Instant::now()) {
                        let _ = webview.post_json(&overtaken);
                    }
                }
                Some(PageMessage::JobMarketCancel { id }) => {
                    if let Some(overtaken) = job_market.ask_cancel(&bridge.bridge().job_market, id, Instant::now()) {
                        let _ = webview.post_json(&overtaken);
                    }
                }
                Some(PageMessage::JobMarketAccept { id, mode, company_ptr, offer_ptr }) => {
                    match job_market.ask_accept(
                        &bridge.bridge().job_market,
                        id,
                        &mode,
                        &company_ptr,
                        &offer_ptr,
                        Instant::now(),
                    ) {
                        Ok(Some(overtaken)) => {
                            let _ = webview.post_json(&overtaken);
                        }
                        Ok(None) => {}
                        Err(immediate) => {
                            let _ = webview.post_json(&immediate);
                        }
                    }
                }
                Some(PageMessage::JobMarketIcon { id, icon, cargo_id }) => {
                    cargo_icons.ask(id, icon, cargo_id, config.game.get());
                }
                Some(PageMessage::FleetRefresh { id }) => {
                    if let Some(overtaken) = fleet.ask_refresh(&bridge.bridge().fleet, id, Instant::now()) {
                        let _ = webview.post_json(&overtaken);
                    }
                }
                Some(PageMessage::FleetDriveTruck { id, truck_ptr }) => {
                    match fleet.ask_drive_truck(&bridge.bridge().fleet, id, &truck_ptr, Instant::now()) {
                        Ok(Some(overtaken)) => {
                            let _ = webview.post_json(&overtaken);
                        }
                        Ok(None) => {}
                        Err(immediate) => {
                            let _ = webview.post_json(&immediate);
                        }
                    }
                }
                Some(PageMessage::FleetUseTrailer { id, trailer_ptr }) => {
                    match fleet.ask_use_trailer(&bridge.bridge().fleet, id, &trailer_ptr, Instant::now()) {
                        Ok(Some(overtaken)) => {
                            let _ = webview.post_json(&overtaken);
                        }
                        Ok(None) => {}
                        Err(immediate) => {
                            let _ = webview.post_json(&immediate);
                        }
                    }
                }
                Some(PageMessage::FleetStoreTrailer { id }) => {
                    if let Some(overtaken) = fleet.ask_store_trailer(&bridge.bridge().fleet, id, Instant::now()) {
                        let _ = webview.post_json(&overtaken);
                    }
                }
                Some(PageMessage::FleetUpgradeGarage { id, garage_ptr }) => {
                    match fleet.ask_upgrade_garage(&bridge.bridge().fleet, id, &garage_ptr, Instant::now()) {
                        Ok(Some(overtaken)) => {
                            let _ = webview.post_json(&overtaken);
                        }
                        Ok(None) => {}
                        Err(immediate) => {
                            let _ = webview.post_json(&immediate);
                        }
                    }
                }
                Some(PageMessage::PhoneDevice { device }) => {
                    let wanted = if device == "ipad" {
                        (westplay_shared::IPAD_WIDTH, westplay_shared::IPAD_HEIGHT)
                    } else {
                        (westplay_shared::PHONE_WIDTH, westplay_shared::PHONE_HEIGHT)
                    };
                    if wanted != phone_dims {
                        phone_dims = wanted;
                        host.phone_width.store(wanted.0, Ordering::Release);
                        host.phone_height.store(wanted.1, Ordering::Release);
                        if let Some(handset) = &handset {
                            let _ = handset.place(0.0, 0.0, wanted.0, wanted.1);
                        }
                        if let Some(layers) = &phone_layers {
                            let _ = layers.resize(wanted.0, wanted.1);
                        }
                        if let Some(tt) = &tiktok {
                            tt.set_bounds(wanted.0, wanted.1);
                        }
                        handset_pointer = Pointer::new(wanted.0, wanted.1);
                        let interaction = config.settings.get().interaction;
                        let phone_scale = if wanted.0 > westplay_shared::PHONE_WIDTH {
                            HANDSET_SENSITIVITY * 1.35
                        } else {
                            HANDSET_SENSITIVITY
                        };
                        handset_pointer.set_sensitivity(interaction.sensitivity.get() * phone_scale);
                        if let (Some(built), Some(layers)) = (pipeline.as_mut(), phone_layers.as_ref()) {
                            host.phone_generation.store(0, Ordering::Release);
                            built.phone = None;
                            generation_counter = generation_counter.wrapping_add(1);
                            let generation =
                                ((std::process::id() & 0x00ff_ffff) << 8) | (generation_counter & 0xff);
                            let name = westplay_shared::phone_texture_name(generation);
                            let restarted = (|| -> Result<Capture, Error> {
                                let shared = Arc::new(SharedFrame::new(&built.gpu, wanted.0, wanted.1, &name)?);
                                Ok(Capture::start(
                                    &built.gpu,
                                    &layers.visual()?,
                                    shared,
                                    bridge.clone(),
                                    Published::Phone,
                                )?)
                            })();
                            match restarted {
                                Ok(cap) => {
                                    built.phone = Some(cap);
                                    host.phone_generation.store(generation, Ordering::Release);
                                    log!(
                                        "[handset] device switched to {device} ({}x{}, gen {generation})",
                                        wanted.0,
                                        wanted.1
                                    );
                                }
                                Err(e) => log!("[handset] failed to resize capture to {device}: {e}"),
                            }
                        }
                    }
                }
                Some(PageMessage::MirrorCamera { x, y, width, height }) => {
                    let rect = mirror_rect(x, y, width, height);
                    for (slot, value) in host.mirror_rect.iter().zip(rect) {
                        slot.store(value, Ordering::Release);
                    }
                    // The videos' pictures lie under the page, where the camera is seen through it: put away while
                    // it shows (owner 2026-11-18: with YouTube playing, only the guide lines), their sound plays on.
                    let covered = rect[2] > 0;
                    if let Some(player) = &youtube
                        && !player.is_on_phone()
                    {
                        let _ = player.cover(covered);
                    }
                    if let Some(music) = &youtube_music {
                        let _ = music.cover(covered);
                    }
                }
                Some(PageMessage::CursorLight { light }) => cursor.set_light(light, Instant::now()),
                Some(PageMessage::TruckCommand { command, down }) => {
                    truck_commands.press(&command, down, Instant::now());
                    // The player's own suspension press: the suspension by itself waits.
                    if down && command.contains("susp") {
                        auto_suspension.manual(Instant::now());
                    }
                }
                Some(PageMessage::VolumeShown { volume }) => {
                    let _ =
                        webview.post_json(&format!(r#"{{"type":"volumeShown","volume":{}}}"#, volume.clamp(0.0, 1.0)));
                }
                Some(PageMessage::Equalizer { equalizer: curve, custom }) => {
                    let sound = settings::SoundSettings { equalizer: curve.clamped(), custom };
                    equalizer = sound.playing();
                    let mut settings = config.settings.get();
                    if settings.sound != sound {
                        settings.sound = sound;
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                    }
                }
                Some(PageMessage::Wallpaper { id }) => {
                    let mut settings = config.settings.get();
                    if settings.look.wallpaper.as_deref() != Some(id.as_str()) && id.len() <= 300 {
                        settings.look.wallpaper = Some(id);
                        config.settings.set(settings.clone());
                        (config.on_settings)(&settings);
                    }
                }
                Some(PageMessage::Youtube { command }) => {
                    if command == youtube::Command::Close {
                        if let Some(closing) = youtube.take() {
                            closing.close(&stage, phone_layers.as_ref());
                        }
                        youtube_message = None;
                        let _ = webview.post_json(&youtube::state_message(None));
                    } else if youtube.is_none() && matches!(command, youtube::Command::Hide) {
                        // Nothing to hide: YouTube isn't running (a page reloading says it anyway).
                    } else if let Some(player) = start_youtube(&mut youtube, &stage, &config) {
                        if player.is_on_phone() && matches!(command, youtube::Command::Hide) {
                            // The in-game iPad/phone currently owns YouTube's visual: never let the cab's initial
                            // or background Hide turn the iPad video invisible!
                        } else if player.is_on_phone()
                            && handset_shown
                            && matches!(command, youtube::Command::Show { .. })
                        {
                            // While the iPad/phone is actively out with YouTube open, keep the video there instead
                            // of letting the cab Dashboard's VideoHole steal it away.
                        } else {
                            if matches!(command, youtube::Command::Show { .. })
                                && player.is_on_phone()
                                && let Some(layers) = &phone_layers
                            {
                                let _ = player.attach_to_stage(&stage, layers);
                            }
                            let _ = player.command(&command);
                            // YouTube Music shares the sign-in: its page again, signed out.
                            if command == youtube::Command::SignOut
                                && let Some(music) = &youtube_music
                            {
                                let _ = music.reload();
                            }
                        }
                    }
                }
                Some(PageMessage::Spotify { command }) => {
                    if spotify.is_none() && matches!(command, spotify::Command::Pause) {
                        // Nothing to pause: Spotify isn't running.
                    } else if let Some(player) = start_spotify(&mut spotify, &stage, &config) {
                        let _ = player.command(&command);
                    }
                }
                Some(PageMessage::AppleMusic { command }) => {
                    if apple_music.is_none() && matches!(command, apple_music::Command::Pause) {
                        // Nothing to pause: Apple Music isn't running.
                    } else if let Some(player) = start_apple_music(&mut apple_music, &stage, &config) {
                        let _ = player.command(&command);
                    }
                }
                Some(PageMessage::YoutubeMusic { command }) => {
                    if youtube_music.is_none() && matches!(command, youtube_music::Command::Pause) {
                        // Nothing to pause: YouTube Music isn't running.
                    } else if let Some(player) = start_youtube_music(&mut youtube_music, &stage, &config) {
                        let _ = player.command(&command);
                        // YouTube shares the sign-in: its page again, signed out.
                        if command == youtube_music::Command::SignOut
                            && let Some(youtube) = &youtube
                        {
                            let _ = youtube.reload();
                        }
                    }
                }
                Some(PageMessage::Typing { selection }) => {
                    if selection != 0 {
                        handset_typing = false;
                    }
                    bridge.bridge().typing.selection.store(selection, Ordering::Release);
                }
                Some(PageMessage::Routes { id, node }) => {
                    let to = u64::from_str_radix(&node, 16).unwrap_or(0);
                    match (&map_shown.0, &last_telemetry) {
                        (Some(folder), Some(t)) => {
                            planner.ask(id, &maps_dir, folder, (t.x as f32, t.z as f32, t.heading), to)
                        }
                        _ => {
                            let _ = webview.post_json(
                                &serde_json::json!({ "type": "routes", "id": id, "routes": [] }).to_string(),
                            );
                        }
                    }
                }
                Some(PageMessage::Route { id, action, node }) => {
                    let node = node.as_deref().and_then(|node| u64::from_str_radix(node, 16).ok()).unwrap_or(0);
                    let block = &bridge.bridge().route;
                    following = None;
                    if let Some(overtaken) = route_changes.ask(block, id, action.into(), node, &[], Instant::now()) {
                        let _ = webview.post_json(&overtaken);
                    }
                }
                Some(PageMessage::Detours { id, nodes }) => {
                    // The trip's end: the last map node of the game's route.
                    let end = route_uids.as_ref().and_then(|uids| uids.iter().rev().find(|&&uid| uid != 0).copied());
                    let stops = nodes.iter().map(|node| u64::from_str_radix(node, 16).unwrap_or(0)).collect();
                    if let (Some(folder), Some(t), Some(end)) = (&map_shown.0, &last_telemetry, end) {
                        planner.ask_detours(id, &maps_dir, folder, (t.x as f32, t.z as f32, t.heading), end, stops);
                    }
                }
                Some(PageMessage::Follow { id, index }) => {
                    let picked = planner.last.as_ref().filter(|last| last.id == id).and_then(|last| {
                        Some((
                            last.routes.get(index)?.clone(),
                            last.roads.get(index)?.clone(),
                            last.positions.get(index)?.clone(),
                        ))
                    });
                    match picked {
                        Some((picked, roads, positions)) if picked.len() > 1 => {
                            let end = picked[picked.len() - 1];
                            let block = &bridge.bridge().route;
                            let _ = route_changes.ask(
                                block,
                                id | FOLLOWING,
                                westplay_shared::RouteAction::Follow,
                                end,
                                &[],
                                Instant::now(),
                            );
                            let handover = handover::Handover::new(picked, roads, &positions);
                            // The last hand-over's waypoints go as this one sets the game's (without them).
                            held_vias.clear();
                            following =
                                Some(Following { id, handover, positions, asked: route_number, answered: None });
                        }
                        _ => {
                            let _ = webview.post_json(&followed_message(id, false, "unexpected"));
                        }
                    }
                }
                None => {}
            }
        }
        if let Some(routes) = planner.answer() {
            let _ = webview.post_json(&routes);
        }
        if let Some(detours) = planner.detours() {
            let _ = webview.post_json(&detours);
        }
        if let Some(answer) = route_changes.answer(&bridge.bridge().route, Instant::now()) {
            let reply: serde_json::Value = serde_json::from_str(&answer).unwrap_or_default();
            let id = reply["id"].as_u64().unwrap_or(0) as u32;
            if id & FOLLOWING == 0 {
                let _ = webview.post_json(&answer);
            } else if let Some(follow) = following.as_mut().filter(|follow| follow.id == id & !FOLLOWING) {
                // Handing a route over: once the game has it, its new route is compared (below).
                match reply["answer"].as_str() {
                    Some("done") => follow.answered = Some((Instant::now(), follow.asked)),
                    other => {
                        let _ = webview.post_json(&followed_message(follow.id, false, other.unwrap_or("noAnswer")));
                        following = None;
                    }
                }
            }
        }
        if let Some(msg) = job_market.poll(
            &bridge.bridge().job_market,
            &maps_dir,
            map_shown.0.as_deref(),
            config.game.get().as_deref(),
            None,
            last_telemetry.map(|t| [t.x, t.z]),
            Instant::now(),
        ) {
            let _ = webview.post_json(&msg);
        }
        if let Some(msg) = bank_rpc.poll(&bridge.bridge().bank, &mut wallet, Instant::now()) {
            let _ = webview.post_json(&wallet.message());
            let _ = webview.post_json(&msg);
        }
        if let Some(msg) = fleet.poll(&bridge.bridge().fleet, &maps_dir, map_shown.0.as_deref(), Instant::now()) {
            if wallet.poll_live_bank(&bridge.bridge().bank) {
                let _ = webview.post_json(&wallet.message());
            }
            let _ = webview.post_json(&msg);
        }
        for icon_msg in cargo_icons.poll_all() {
            let _ = webview.post_json(&icon_msg);
        }
        // The game's route after handing one over: follows the one picked, or is held to it by another
        // waypoint, or the game won't (handover.rs).
        if let Some(follow) = following.as_mut()
            && let Some((since, before)) = follow.answered
        {
            if route_number != before
                && let Some(game) = &route_uids
            {
                let step = follow.handover.read(game);
                let picked = follow.handover.picked();
                let end = picked[picked.len() - 1];
                let block = &bridge.bridge().route;
                match step {
                    handover::Step::Follows => {
                        log!("route handed over: the game follows it ({} waypoints)", follow.handover.handed().len());
                        let _ = webview.post_json(&followed_message(follow.id, true, "done"));
                        // Its waypoints stay in the game's route: never the player's stops.
                        held_vias = follow.handover.handed().to_vec();
                        let stops = stops_message(route_number, route_uids.as_deref(), &route_waypoints, &held_vias);
                        let _ = webview.post_json(&stops);
                        route_stops_message = Some(stops);
                        following = None;
                    }
                    handover::Step::GivesUp => {
                        log!("route handed over: the game takes its own way");
                        held_vias.clear();
                        // Its own way, with none of the waypoints that tried to hold it (owner 2026-10-21: they
                        // were left in the game).
                        if !follow.handover.handed().is_empty() {
                            let _ = route_changes.ask(
                                block,
                                follow.id | FOLLOWING,
                                westplay_shared::RouteAction::Follow,
                                end,
                                &[],
                                Instant::now(),
                            );
                        }
                        let _ = webview.post_json(&followed_message(follow.id, false, "ownWay"));
                        following = None;
                    }
                    step => {
                        match step {
                            handover::Step::Hold { parts, via } => {
                                // Where the game leaves the route picked, and where the waypoint holds it: a route
                                // of ours the game won't take is looked into from these (owner 2026-10-21).
                                let at = |index: usize| follow.positions.get(index).copied().unwrap_or_default();
                                let (left, held) = (at(parts), at(via));
                                log!(
                                    "route handed over: the game leaves the route picked at node {:x} ({:.0}, {:.0}); waypoint at node {:x} ({:.0}, {:.0}), {} in all",
                                    picked[parts],
                                    left[0],
                                    left[1],
                                    picked[via],
                                    held[0],
                                    held[1],
                                    follow.handover.handed().len()
                                );
                            }
                            handover::Step::TryWithout(via) => {
                                log!(
                                    "route handed over: the game follows it; trying without the waypoint at node {via:x}"
                                );
                            }
                            handover::Step::PutBack => {
                                log!(
                                    "route handed over: that waypoint is needed; the ones that held the game handed back"
                                )
                            }
                            handover::Step::Follows | handover::Step::GivesUp => {}
                        }
                        let vias = follow.handover.handed();
                        let _ = route_changes.ask(
                            block,
                            follow.id | FOLLOWING,
                            westplay_shared::RouteAction::Follow,
                            end,
                            vias,
                            Instant::now(),
                        );
                        follow.asked = route_number;
                        follow.answered = None;
                    }
                }
            } else if since.elapsed() >= Duration::from_secs(6) {
                // The game took it, but its route can't be read in a game run as administrator: said so.
                let answer = if game_elevated { "elevated" } else { "noAnswer" };
                let _ = webview.post_json(&followed_message(follow.id, false, answer));
                following = None;
            }
        }

        // How YouTube's player plays, for the page; its WebView failing closes it (the page opens it again).
        if let Some(player) = &youtube {
            if player.failed() || players_closing {
                if players_closing {
                    log!("[youtube] closed with the game");
                } else {
                    log!("[youtube] its browser failed: closed, the page opens it again");
                }
                if let Some(line) = youtube_log.heard(None) {
                    log!("{line}");
                }
                if let Some(failed) = youtube.take() {
                    failed.close(&stage, phone_layers.as_ref());
                }
                youtube_message = None;
                let _ = webview.post_json(&youtube::state_message(None));
            } else {
                // Every answer, and how the player plays now.
                let mut state = None;
                for report in player.take_reports() {
                    match report {
                        youtube::Report::Spectrum(bars) => {
                            let _ = webview.post_json(&speakers::spectrum_message(&bars));
                        }
                        youtube::Report::Answer(answer) => {
                            let _ = webview.post_json(&youtube::answer_message(&answer));
                        }
                        youtube::Report::Account(account) => {
                            let message = youtube::account_message(&account);
                            let _ = webview.post_json(&message);
                            youtube_account_message = Some(message);
                            if status.youtube_signed_in != Some(account.signed_in) {
                                status.youtube_signed_in = Some(account.signed_in);
                                (config.on_status)(&status);
                            }
                        }
                        youtube::Report::SignInWindow(open) => {
                            let _ = webview.post_json(&youtube::sign_in_message(open));
                            youtube_window_open = open;
                            status.youtube_signing_in = youtube_window_open || youtube_confirming;
                            (config.on_status)(&status);
                        }
                        youtube::Report::Confirming(confirming) => {
                            // Signed in in YouTube's window: YouTube Music (the same sign-in) loads again too.
                            if confirming && let Some(music) = &youtube_music {
                                let _ = music.reload();
                            }
                            youtube_confirming = confirming;
                            status.youtube_signing_in = youtube_window_open || youtube_confirming;
                            (config.on_status)(&status);
                        }
                        youtube::Report::State(latest) => {
                            let heard = youtube_heard(&latest);
                            let loading = heard.doing == play_log::Doing::Loading;
                            if let Some(line) = youtube_log.heard(Some(heard)) {
                                log!("{line}");
                            }
                            // Still loading a while after it began: why not yet, as the page reads it.
                            if youtube_stuck.heard(latest.video.as_deref(), loading, Instant::now()) {
                                let video = latest.video.as_deref().unwrap_or("?");
                                let why =
                                    latest.diag.as_ref().map_or("nothing read".to_owned(), |diag| diag.to_string());
                                log!(
                                    "[youtube] {video} still loading after {} s: {why}",
                                    play_log::STUCK_AFTER.as_secs()
                                );
                            }
                            if youtube_was_playing && latest.state == 3 {
                                let video = latest.video.as_deref().unwrap_or("?");
                                log!("[youtube] {video} stopped to buffer at {:.1} s", latest.time);
                            }
                            youtube_was_playing = latest.state == 1;
                            // Playing with its picture not over the whole page (players, 2026-10-27: the next
                            // video small in the top left): what the page has, once a video and look.
                            if let Some(picture) = &latest.picture {
                                let seen = (latest.video.clone(), picture.clone());
                                if youtube_picture.as_ref() != Some(&seen) {
                                    let video = latest.video.as_deref().unwrap_or("?");
                                    log!("[youtube] {video}'s picture isn't over the whole page: {picture}");
                                    youtube_picture = Some(seen);
                                }
                            }
                            state = Some(latest);
                        }
                    }
                }
                if let Some(state) = state {
                    let message = youtube::state_message(Some(&state));
                    let _ = webview.post_json(&message);
                    youtube_message = Some(message);
                }
            }
        }

        // What Spotify's page says (signed in, playing) and its sign-in window, for the page; its WebView
        // failing closes it (the page opens it again).
        if let Some(player) = &spotify {
            if player.failed() || players_closing {
                if players_closing {
                    log!("[spotify] closed with the game");
                } else {
                    log!("[spotify] its browser failed: closed, the page opens it again");
                }
                if let Some(line) = spotify_log.heard(None) {
                    log!("{line}");
                }
                if let Some(failed) = spotify.take() {
                    failed.close(&stage);
                }
                spotify_state = None;
                spotify_playback = None;
                status.spotify_signed_in = None;
                status.spotify_signing_in = false;
                spotify_confirming = false;
                (config.on_status)(&status);
                let _ = webview.post_json(&spotify::state_message(None, false));
                let _ = webview.post_json(&spotify::playback_message(None));
            } else {
                let mut state_changed = false;
                for report in player.take_reports() {
                    match report {
                        spotify::Report::Spectrum(bars) => {
                            let _ = webview.post_json(&speakers::spectrum_message(&bars));
                        }
                        spotify::Report::State(state) => {
                            if status.spotify_signed_in != Some(state.signed_in) {
                                status.spotify_signed_in = Some(state.signed_in);
                                (config.on_status)(&status);
                            }
                            spotify_state = Some(state);
                            state_changed = true;
                        }
                        spotify::Report::SignInWindow(open) => {
                            spotify_signing_in = open;
                            status.spotify_signing_in = spotify_signing_in || spotify_confirming;
                            (config.on_status)(&status);
                            state_changed = true;
                        }
                        spotify::Report::Confirming(confirming) => {
                            spotify_confirming = confirming;
                            status.spotify_signing_in = spotify_signing_in || spotify_confirming;
                            (config.on_status)(&status);
                        }
                        // Answers and what plays go straight on to the page (never logged).
                        spotify::Report::Answer(answer) => {
                            let _ = webview.post_json(&spotify::answer_message(&answer));
                        }
                        spotify::Report::Timing(timing) => log!("{}", timing.line()),
                        spotify::Report::Playback(playback) => {
                            // What plays on the account's other devices (a phone) isn't this player's.
                            let here = playback.as_deref().filter(|playback| playback.here != Some(false));
                            let heard = here.map(|playback| play_log::Heard {
                                doing: if playback.playing {
                                    play_log::Doing::Playing
                                } else {
                                    play_log::Doing::Paused
                                },
                                track: playback.track.as_ref().map(|track| track.uri.as_str()),
                                position: playback.position,
                            });
                            if let Some(line) = spotify_log.heard(heard) {
                                log!("{line}");
                            }
                            let message = spotify::playback_message(playback.as_deref());
                            let _ = webview.post_json(&message);
                            spotify_playback = Some(message);
                        }
                    }
                }
                if state_changed {
                    let _ = webview.post_json(&spotify::state_message(spotify_state.as_ref(), spotify_signing_in));
                }
            }
        }

        // What Apple Music's page says, and its sign-in window, for the page; its WebView failing closes it.
        if let Some(player) = &apple_music {
            if player.failed() || players_closing {
                if players_closing {
                    log!("[apple music] closed with the game");
                } else {
                    log!("[apple music] its browser failed: closed, the page opens it again");
                }
                if let Some(line) = apple_music_log.heard(None) {
                    log!("{line}");
                }
                if let Some(failed) = apple_music.take() {
                    failed.close(&stage);
                }
                apple_music_state = None;
                apple_music_playback = None;
                (apple_music_window_open, apple_music_confirming) = (false, false);
                status.apple_music_signed_in = None;
                status.apple_music_signing_in = false;
                (config.on_status)(&status);
                let _ = webview.post_json(&apple_music::state_message(None, false));
                let _ = webview.post_json(&apple_music::playback_message(None));
            } else {
                let mut state_changed = false;
                for report in player.take_reports() {
                    match report {
                        apple_music::Report::Spectrum(bars) => {
                            let _ = webview.post_json(&speakers::spectrum_message(&bars));
                        }
                        apple_music::Report::State(state) => {
                            if status.apple_music_signed_in != Some(state.signed_in) {
                                status.apple_music_signed_in = Some(state.signed_in);
                                (config.on_status)(&status);
                            }
                            apple_music_state = Some(state);
                            state_changed = true;
                        }
                        apple_music::Report::SignInWindow(open) => {
                            apple_music_window_open = open;
                            state_changed = true;
                        }
                        // The cab shows it signing in until the page here says it's signed in (not signed out
                        // for a moment as it loads again).
                        apple_music::Report::Confirming(confirming) => {
                            apple_music_confirming = confirming;
                            state_changed = true;
                        }
                        // Answers and what plays go straight on to the page (never logged).
                        apple_music::Report::Answer(answer) => {
                            let _ = webview.post_json(&apple_music::answer_message(&answer));
                        }
                        // Times only (never what played): for the log.
                        apple_music::Report::Timing(timing) => log!("{}", timing.line()),
                        apple_music::Report::Playback(playback) => {
                            let heard = playback.as_deref().map(|playback| play_log::Heard {
                                doing: play_log::doing(playback.loading, playback.playing),
                                track: playback.track.as_ref().map(|track| track.id.as_str()),
                                position: playback.position,
                            });
                            if let Some(line) = apple_music_log.heard(heard) {
                                log!("{line}");
                            }
                            let message = apple_music::playback_message(playback.as_deref());
                            let _ = webview.post_json(&message);
                            apple_music_playback = Some(message);
                        }
                    }
                }
                if state_changed {
                    let signing_in = apple_music_window_open || apple_music_confirming;
                    if status.apple_music_signing_in != signing_in {
                        status.apple_music_signing_in = signing_in;
                        (config.on_status)(&status);
                    }
                    let message = apple_music::state_message(apple_music_state.as_ref(), signing_in);
                    let _ = webview.post_json(&message);
                }
            }
        }

        // What YouTube Music's page says, and its sign-in window, for the page; its WebView failing closes it.
        if let Some(player) = &youtube_music {
            if player.failed() || players_closing {
                if players_closing {
                    log!("[youtube music] closed with the game");
                } else {
                    log!("[youtube music] its browser failed: closed, the page opens it again");
                }
                if let Some(line) = youtube_music_log.heard(None) {
                    log!("{line}");
                }
                if let Some(failed) = youtube_music.take() {
                    failed.close(&stage);
                }
                youtube_music_state = None;
                youtube_music_playback = None;
                (youtube_music_window_open, youtube_music_confirming) = (false, false);
                let _ = webview.post_json(&youtube_music::state_message(None, false));
                let _ = webview.post_json(&youtube_music::playback_message(None));
            } else {
                let mut state_changed = false;
                for report in player.take_reports() {
                    match report {
                        youtube_music::Report::Spectrum(bars) => {
                            let _ = webview.post_json(&speakers::spectrum_message(&bars));
                        }
                        youtube_music::Report::State(state) => {
                            // One sign-in for both: the companion hears it as YouTube's.
                            if status.youtube_signed_in != Some(state.signed_in) {
                                status.youtube_signed_in = Some(state.signed_in);
                                (config.on_status)(&status);
                            }
                            youtube_music_state = Some(state);
                            state_changed = true;
                        }
                        youtube_music::Report::SignInWindow(open) => {
                            youtube_music_window_open = open;
                            state_changed = true;
                        }
                        // The cab shows it signing in until the page here says it's signed in; YouTube (the same
                        // sign-in) loads again too.
                        youtube_music::Report::Confirming(confirming) => {
                            if confirming && let Some(youtube) = &youtube {
                                let _ = youtube.reload();
                            }
                            youtube_music_confirming = confirming;
                            state_changed = true;
                        }
                        // Answers and what plays go straight on to the page (never logged).
                        youtube_music::Report::Answer(answer) => {
                            let _ = webview.post_json(&youtube_music::answer_message(&answer));
                        }
                        // Times only (never what played): for the log.
                        youtube_music::Report::Timing(timing) => log!("{}", timing.line()),
                        youtube_music::Report::Playback(playback) => {
                            let heard = playback.as_deref().map(|playback| play_log::Heard {
                                doing: play_log::doing(playback.loading, playback.playing),
                                track: Some(playback.track.video.as_str()),
                                position: playback.position,
                            });
                            if let Some(line) = youtube_music_log.heard(heard) {
                                log!("{line}");
                            }
                            let message = youtube_music::playback_message(playback.as_deref());
                            let _ = webview.post_json(&message);
                            youtube_music_playback = Some(message);
                        }
                    }
                }
                if state_changed {
                    let signing_in = youtube_music_window_open || youtube_music_confirming;
                    let message = youtube_music::state_message(youtube_music_state.as_ref(), signing_in);
                    let _ = webview.post_json(&message);
                }
            }
        }

        // What TikTok's page says, its sign-in window, and the player's LIVE, for the phone and the companion; its
        // WebView failing closes it.
        if let Some(app) = &tiktok {
            if app.failed() || players_closing {
                if players_closing {
                    log!("[tiktok] closed with the game");
                } else {
                    log!("[tiktok] its browser failed: closed, the phone opens it again");
                }
                if let (Some(failed), Some(layers)) = (tiktok.take(), &phone_layers) {
                    failed.close(layers);
                }
                tiktok_state = None;
                (tiktok_window_open, tiktok_confirming) = (false, false);
                status.tiktok_signed_in = None;
                status.tiktok_signing_in = false;
                (config.on_status)(&status);
                if let Some(handset) = &handset {
                    let _ = handset.post_json(&tiktok::state_message(None, false, false));
                }
            } else {
                let mut state_changed = false;
                for report in app.take_reports(&stage) {
                    match report {
                        tiktok::Report::State(state) => {
                            if status.tiktok_signed_in != Some(state.signed_in) {
                                status.tiktok_signed_in = Some(state.signed_in);
                                (config.on_status)(&status);
                                log!("[tiktok] signed {}", if state.signed_in { "in" } else { "out" });
                            }
                            tiktok_state = Some(state);
                            state_changed = true;
                        }
                        tiktok::Report::SignInWindow(open) => {
                            tiktok_window_open = open;
                            state_changed = true;
                        }
                        tiktok::Report::Confirming(confirming) => {
                            tiktok_confirming = confirming;
                            state_changed = true;
                        }
                        tiktok::Report::Live(_) => state_changed = true,
                        tiktok::Report::Typing(_) => {}
                        // A follow, a gift: on the phone, its top showing for it while it's put away. Never
                        // logged (who did what on the player's LIVE is theirs).
                        tiktok::Report::Event(event) => notify_phone!(&tiktok::notification_message(&event)),
                    }
                }
                if state_changed {
                    let signing_in = tiktok_window_open || tiktok_confirming;
                    if status.tiktok_signing_in != signing_in {
                        status.tiktok_signing_in = signing_in;
                        (config.on_status)(&status);
                    }
                    if let Some(handset) = &handset {
                        let message = tiktok::state_message(tiktok_state.as_ref(), signing_in, app.live());
                        let _ = handset.post_json(&message);
                    }
                }
            }
        }

        // What the player typed on their own keyboard, and whether it types (the plugin decides: the truck
        // standing still, a field selected then).
        let typing_block = &bridge.bridge().typing;
        // The cab told whether YouTube's picture is in the phone: its Dashboard then cuts no hole for it (the game's
        // map and the cab's own map stay whole).
        let on_handset = youtube.as_ref().is_some_and(youtube::YouTube::is_on_phone);
        if youtube_on_handset_told != Some(on_handset) {
            youtube_on_handset_told = Some(on_handset);
            let _ = webview.post_json(&serde_json::json!({ "type": "youtubeHandset", "on": on_handset }).to_string());
        }
        if handset_typing && !handset_shown {
            handset_typing = false;
            typing_block.selection.store(0, Ordering::Release);
        }
        let typed: Vec<u32> = std::iter::from_fn(|| typing_block.keys.pop()).collect();
        if let Some(message) = typing::typed_message(&typed, typing::clipboard_text) {
            // Into TikTok's field while it has the keyboard, else the handset's if selected, else the cab's page's.
            match tiktok.as_ref().filter(|app| app.typing()) {
                Some(app) => drop(app.typed(&message)),
                None if handset_typing => {
                    if let Some(handset) = &handset {
                        let _ = handset.post_json(&message);
                    }
                }
                None => drop(webview.post_json(&message)),
            }
        }
        // A field of TikTok's page selected: the plugin's keys type (a number of its own, no page's selection).
        let types = tiktok.as_ref().is_some_and(tiktok::TikTok::typing);
        if types != tiktok_typing {
            tiktok_typing = types;
            typing_block.selection.store(if types { TIKTOK_SELECTION } else { 0 }, Ordering::Release);
        }
        let state = westplay_shared::TypingState::from_code(typing_block.state.load(Ordering::Acquire));
        if typing_state != Some(state) {
            typing_state = Some(state);
            let message = typing::state_message(state);
            let _ = webview.post_json(&message);
            if let Some(handset) = &handset {
                let _ = handset.post_json(&message);
            }
        }

        // YouTube's, Apple Music's, YouTube Music's and Spotify's sound from the truck's own speakers, heard from the
        // driver's head (as last told, every 50 ms); muffled with the camera outside the cab, seen on every pass (as
        // quick as the game's cut).
        if let (Some(truck), Some(telemetry)) = (&last_truck, &last_telemetry) {
            let plugin = &bridge.bridge().plugin;
            // The cab drawn: WestPlay's screen, or the cab's mark (the tablet's left out as the player looks away).
            let cab_drawn = camera::drawn_frame(
                plugin.screen_frame.load(Ordering::Acquire),
                plugin.cab_frame.load(Ordering::Acquire),
            );
            let game_frame = plugin.heartbeat.load(Ordering::Acquire);
            let (electrics, paused) = (telemetry.electric_enabled, telemetry.paused);
            let effective_electrics = electrics;
            // On foot (the plugin's Modo Personagem) or camera physically far from the driver's head (> 1.4 m):
            // outside the cab, even if a floating screen or through-window draw stamped `cab_frame`/`screen_frame`.
            let on_foot = plugin.on_foot.load(Ordering::Acquire) == 1;
            let far_from_head = camera::Camera::is_far_from_head(telemetry);
            let outside =
                camera.outside_or_far(truck.id.as_str(), cab_drawn, game_frame, effective_electrics, paused, far_from_head)
                    || on_foot;
            if outside != was_outside {
                was_outside = outside;
                log!("camera: {} the cab", if outside { "outside" } else { "in" });
            }
            // The game's menu gone while it's still paused: the music back as its still picture shows.
            let gone = menu.update(paused, camera::drawn(cab_drawn, game_frame), outside);
            if gone != menu_gone {
                menu_gone = gone;
                if gone {
                    log!("the game's menu has gone, the game about to run again");
                }
                let _ = webview.post_json(&serde_json::json!({ "type": "menuGone", "gone": gone }).to_string());
            }
            let distance = camera::Camera::distance(telemetry, outside).unwrap_or(0.0) as f32;
            let turn = game_settings.camera().steering_turn(telemetry.steering);
            let head = speakers::Head::of(telemetry).turned(turn);
            // The truck's speakers: its parts' for a truck whose speakers come with them (none until they're
            // found), else its own.
            let truck_speakers = match &part_speakers {
                Some(found) if found.truck == *truck => found.speakers.as_deref(),
                _ if speakers::by_parts(truck.id.as_str()) => None,
                _ => speakers::in_truck(truck.id.as_str()),
            };
            // When audio output is routed to the phone/iPad (`Device`), play clean stereo directly without truck
            // speaker spatialization or outside-cab muffling (so YouTube & music work cleanly in Modo Personagem).
            let (eff_speakers, eff_outside, eff_distance) = if audio_output == settings::AudioOutput::Device {
                (None, false, 0.0)
            } else {
                (truck_speakers, outside, distance)
            };
            if let Some(player) = &youtube {
                let _ = player.cab(eff_speakers, head, eff_outside, eff_distance, equalizer, spectrum);
            }
            if let Some(player) = &apple_music {
                let _ = player.cab(eff_speakers, head, eff_outside, eff_distance, equalizer, spectrum);
            }
            if let Some(player) = &youtube_music {
                let _ = player.cab(eff_speakers, head, eff_outside, eff_distance, equalizer, spectrum);
            }
            if let Some(player) = &spotify {
                let _ = player.cab(eff_speakers, head, eff_outside, eff_distance, equalizer, spectrum);
            }
        }

        // TruckersMP's players around the player, as the plugin last wrote them: to the page as they change (each
        // game frame's), at most 30 times a second; none when the game isn't under TruckersMP.
        if last_nearby_post.elapsed() >= NEARBY_POST_EVERY {
            last_nearby_post = Instant::now();
            let nearby = if truckersmp { bridge.bridge().plugin.nearby.read() } else { None };
            let frame = nearby.as_ref().map(|nearby| nearby.frame).filter(|&frame| frame != 0);
            if frame != nearby_sent {
                nearby_sent = frame;
                let _ = webview.post_json(&nearby_message(nearby.as_ref().filter(|_| frame.is_some())));
            }
        }

        // In reverse, the reversing camera follows the truck: telemetry as often as the screen shows frames. The
        // dashboard's gear as well as the engine's: stopped with an automatic gearbox, the engine's is neutral though
        // reverse is chosen (as the page's `inReverse`, cabin/reversing/shown.ts).
        let reversing =
            last_telemetry.as_ref().is_some_and(|telemetry| telemetry.displayed_gear < 0 || telemetry.gear < 0);
        let telemetry_every = Duration::from_millis(if reversing { TELEMETRY_REVERSING_MS } else { TELEMETRY_MS });
        if last_telemetry_post.elapsed() >= telemetry_every {
            last_telemetry_post = Instant::now();
            let plugin = &bridge.bridge().plugin;
            // Whether the truck stands at a fuel pump, told to the plugin (refuelling by hand).
            if let Some(telemetry) = plugin.telemetry.read() {
                let at = pumps::told(map_pumps.as_deref(), [telemetry.x, telemetry.z]);
                bridge.bridge().host.at_pump.store(at, Ordering::Release);
            }
            // A write the game never finished (it crashed in the middle) reads as nothing: nothing to send. A
            // game that closed leaves its last telemetry behind: not sent (a page reloading would power on).
            if game_running
                && let Some(telemetry) = plugin.telemetry.read()
                && last_telemetry != Some(telemetry)
            {
                last_telemetry = Some(telemetry);
                let message = serde_json::json!({ "type": "telemetry", "data": telemetry });
                let _ = webview.post_json(&message.to_string());
            }
            if let Some(truck) = plugin.truck.read()
                && last_truck != Some(truck)
            {
                if truck != TruckInfo::default() {
                    startup::mark("host: truck sent to the page");
                }
                last_truck = Some(truck);
                player_watch.truck(truck);
                let message = serde_json::json!({ "type": "truck", "data": truck });
                let _ = webview.post_json(&message.to_string());
            }
            if let Some(job) = plugin.job.read()
                && last_job != Some(job)
            {
                last_job = Some(job);
                let message = serde_json::json!({ "type": "job", "data": job });
                let _ = webview.post_json(&message.to_string());
            }
            // The game's news for the phone (alerts.rs): the truck's damage, its fuel, the rest, the job.
            if game_running && let Some(telemetry) = last_telemetry {
                let truck = last_truck.map(|truck| truck.id.as_str().to_owned()).unwrap_or_default();
                for alert in alert_watch.tick(&telemetry, last_job.as_ref(), &truck, Instant::now()) {
                    if alert_filter.allows(&alert) {
                        notify_phone!(&alert.message());
                    }
                }
            }
            let layout = plugin.screen_layout.load(Ordering::Acquire);
            if last_layout != Some(layout) {
                last_layout = Some(layout);
                // The page in the shape the game shows it in on this screen (none drawn yet: as it was), before
                // the page hears of the screen and lays itself out for it.
                if let Some(index) = (layout as usize).checked_sub(1) {
                    let height = page_height_on(index);
                    if height != page_height {
                        page_height = height;
                        log!("the page laid out {}x{height}, shown in the {}-row frame", config.width, config.height);
                        let _ = stage.set_page_height(height);
                        let _ = webview.place(0.0, 0.0, config.width, height);
                        pointer.set_size(config.width, height);
                    }
                }
                let _ = webview.post_json(&screen_message(layout));
            }
        }

        std::thread::sleep(Duration::from_millis(1));
    }

    host.frame_generation.store(0, Ordering::Release);
    host.phone_generation.store(0, Ordering::Release);
    host.phone_shown.store(0, Ordering::Release);
    host.note_done.store(0, Ordering::Release);
    for slot in &host.mirror_rect {
        slot.store(0, Ordering::Release);
    }
    host.fuel_paid.store(0, Ordering::Release);
    host.truck_commands.store(0, Ordering::Release);
    host.wheel_active.store(0, Ordering::Release);
    host.screen_enabled.store(0, Ordering::Release);
    host.pid.store(0, Ordering::Release);
    drop(pipeline);
    if browser_gone {
        return Err(Box::new(BrowserGone));
    }
    if gpu_changed {
        return Err(Box::new(graphics::GpuChanged));
    }
    Ok(())
}

/// Whether the game's plugin is alive, judged by its heartbeat (its frame count) having moved within
/// [`GAME_TIMEOUT`]. The first value seen proves nothing: a game that's gone may have left it.
#[derive(Default)]
struct Liveness {
    last: Option<(u64, Instant)>,
}

/// A game whose heartbeat stands still this long is taken to be gone (crashed, or stopped responding).
const GAME_TIMEOUT: Duration = Duration::from_secs(3);

impl Liveness {
    fn update(&mut self, heartbeat: u64, now: Instant) -> bool {
        match self.last {
            Some((seen, _)) if seen == heartbeat => {}
            Some(_) => self.last = Some((heartbeat, now)),
            None => self.last = Some((heartbeat, now.checked_sub(GAME_TIMEOUT).unwrap_or(now))),
        }
        self.last.is_some_and(|(_, moved)| now.duration_since(moved) < GAME_TIMEOUT)
    }
}

/// Being the one WestPlay running in this Windows session: two would both drive the game's bridge (taking
/// turns at its typed-key queue, which has room for one reader, and flipping the game between their frames).
/// Held for as long as the app runs; a WestPlay that's killed lets go with its process.
pub struct Instance(windows::Win32::Foundation::HANDLE);

// SAFETY: a handle to a named mutex, only ever closed (once, as it goes), from whichever thread.
unsafe impl Send for Instance {}

impl Instance {
    /// Claims being the one WestPlay, or `None` if another is running.
    pub fn claim() -> Option<Self> {
        Self::claim_named(windows::core::w!(r"Local\WestPlay.Instance"))
    }

    fn claim_named(name: windows::core::PCWSTR) -> Option<Self> {
        use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError};
        // SAFETY: a plain call: the name is a static string, the mutex is the process's own.
        let handle = unsafe { windows::Win32::System::Threading::CreateMutexW(None, false, name) }.ok()?;
        // SAFETY: reads this thread's last error, right after the call it's for.
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            // SAFETY: `handle` was just opened, and nothing else holds it.
            unsafe {
                let _ = CloseHandle(handle);
            }
            return None;
        }
        Some(Self(handle))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: the mutex is this `Instance`'s own, closed once, as it goes.
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// Tells the player WestPlay is already running (a second one was started), in a message box.
pub fn tell_already_running() {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK, MessageBoxW};
    // SAFETY: a message box with static strings, no owner window.
    unsafe {
        MessageBoxW(
            None,
            windows::core::w!("MaxDevMod is already running. Find it in the taskbar or the notification area."),
            windows::core::w!("MaxDevMod"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

/// [`run`] stopped because WebView2's browser process exited, taking the cabin page's WebView with it: run it
/// again, on a new thread (the thread's composition, set up for the WebView, can't be set up twice).
#[derive(Debug)]
pub struct BrowserGone;

impl std::fmt::Display for BrowserGone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the cabin browser (WebView2) stopped")
    }
}

impl std::error::Error for BrowserGone {}

/// Logs the startup stages the host sees by watching: the game connecting, the first frame captured, the
/// game first drawing one on the truck's screen (its frame counter moving on from `screen_frame_before`).
/// Whether any are still to come.
fn log_startup(bridge: &BridgeMapping, pipeline: Option<&Pipeline>, screen_frame_before: u64) -> bool {
    let plugin = &bridge.bridge().plugin;
    let stages = [
        (plugin.pid.load(Ordering::Relaxed) != 0, "game: connected"),
        (pipeline.is_some_and(|p| p.capture.frames.load(Ordering::Relaxed) > 0), "host: first frame captured"),
        (
            plugin.screen_frame.load(Ordering::Relaxed) != screen_frame_before,
            "game: drawing WestPlay on the truck's screen",
        ),
    ];
    for (reached, stage) in stages {
        if reached {
            startup::mark(stage);
        }
    }
    !stages.iter().all(|(reached, _)| *reached)
}

/// Whether the process `game` (its pid; 0: no game) has a window showing: shown, not minimised, not cloaked
/// (on another virtual desktop). In front or not: Lossless Scaling's window is in front of a game it scales,
/// and a window over the game leaves it showing (owner 2026-10-28). Without a game, it counts as showing
/// (nothing to be away from).
fn is_shown(game: u32) -> bool {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsIconic, IsWindowVisible};
    use windows::core::BOOL;

    struct Looking {
        game: u32,
        shown: bool,
    }
    unsafe extern "system" fn each(window: HWND, looking: LPARAM) -> BOOL {
        // SAFETY: `looking` is the `Looking` below, alive through `EnumWindows`; the window is only asked about.
        unsafe {
            let looking = &mut *(looking.0 as *mut Looking);
            let mut owner = 0;
            GetWindowThreadProcessId(window, Some(&mut owner));
            if owner != looking.game || !IsWindowVisible(window).as_bool() || IsIconic(window).as_bool() {
                return BOOL(1);
            }
            let mut cloaked = 0u32;
            let read = DwmGetWindowAttribute(window, DWMWA_CLOAKED, (&raw mut cloaked).cast(), size_of::<u32>() as u32);
            looking.shown = read.is_err() || cloaked == 0;
            // Found one showing: done.
            BOOL(i32::from(!looking.shown))
        }
    }

    if game == 0 {
        return true;
    }
    let mut looking = Looking { game, shown: false };
    // SAFETY: `each` only reads the windows and writes `looking`, which outlives the call. Stopping early makes
    // it return an error: what was found stands.
    let _ = unsafe { EnumWindows(Some(each), LPARAM(&raw mut looking as isize)) };
    looking.shown
}

/// Whether the process `game` (its pid) runs as administrator (elevated), as Task Manager's "Elevated" says: then
/// Windows keeps WestPlay out of it, and the game's route, guidance and weather can't be read. WestPlay doesn't
/// work with a game run that way (owner 2026-10-28: nothing that reaches into an elevated game); the player is
/// told. A token WestPlay isn't let read counts as elevated: it's kept out all the same.
fn is_elevated(game: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, E_ACCESSDENIED, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
    use windows::Win32::System::Threading::{OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION};
    if game == 0 {
        return false;
    }
    // SAFETY: handles opened here and closed once; `elevation` and `length` are locals the call writes, its size
    // given.
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, game) else { return false };
        let mut token = HANDLE::default();
        let opened = OpenProcessToken(process, TOKEN_QUERY, &mut token);
        let _ = CloseHandle(process);
        if let Err(error) = opened {
            return error.code() == E_ACCESSDENIED;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut length = 0;
        let read = GetTokenInformation(
            token,
            TokenElevation,
            Some((&raw mut elevation).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut length,
        );
        let _ = CloseHandle(token);
        read.is_ok() && elevation.TokenIsElevated != 0
    }
}

/// Whether the process `game` (its pid; 0: no game) is running: the plugin clears its pid as the game closes,
/// and a game that crashed leaves it behind, its process gone.
fn is_running(game: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
    if game == 0 {
        return false;
    }
    // SAFETY: a handle opened here, waited on without waiting (signalled: the process has ended), and closed.
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_SYNCHRONIZE, false, game) else { return false };
        let running = WaitForSingleObject(process, 0) == WAIT_TIMEOUT;
        let _ = CloseHandle(process);
        running
    }
}

/// TruckersMP's client modules (ETS2's, ATS's): one loaded in the game, it runs under TruckersMP.
const TRUCKERSMP_MODULES: &[&str] = &["core_ets2mp.dll", "core_atsmp.dll"];
/// How often a running game is looked at for TruckersMP until it's found.
const TRUCKERSMP_CHECK_EVERY: Duration = Duration::from_secs(5);

/// Whether the process `pid` has one of the modules `names` loaded (names compared ignoring case). Only the
/// game's list of modules is looked at, nothing in them.
fn has_module(pid: u32, names: &[&str]) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, MODULEENTRY32W, Module32FirstW, Module32NextW, TH32CS_SNAPMODULE,
    };
    if pid == 0 {
        return false;
    }
    // SAFETY: the snapshot is closed once after use; `entry` is a local with its size set, as the calls require.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, pid) else { return false };
        let mut entry = MODULEENTRY32W { dwSize: size_of::<MODULEENTRY32W>() as u32, ..Default::default() };
        let mut found = false;
        let mut more = Module32FirstW(snapshot, &mut entry).is_ok();
        while more && !found {
            let length = entry.szModule.iter().position(|&c| c == 0).unwrap_or(entry.szModule.len());
            let module = String::from_utf16_lossy(&entry.szModule[..length]);
            found = names.iter().any(|name| module.eq_ignore_ascii_case(name));
            more = Module32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
        found
    }
}

/// How often the players around the player under TruckersMP may go to the page.
const NEARBY_POST_EVERY: Duration = Duration::from_millis(33);

/// `{"type":"nearby","server":…,"local":…,"players":[[id, x, z, heading], …]}`: TruckersMP's players around
/// the player (`server` its game server's id, `local` the player's own number; `null` for none: not under
/// TruckersMP, or its SDK not answering).
fn nearby_message(nearby: Option<&westplay_shared::Nearby>) -> String {
    match nearby {
        None => serde_json::json!({ "type": "nearby", "server": null, "local": null, "players": [] }).to_string(),
        Some(nearby) => serde_json::json!({
            "type": "nearby",
            "server": nearby.server,
            "local": nearby.local,
            "players": nearby
                .players()
                .iter()
                .map(|player| serde_json::json!([player.id, player.x, player.z, player.heading]))
                .collect::<Vec<_>>(),
        })
        .to_string(),
    }
}

/// `{"type":"cursor","host":…}`: whether the host draws the screen's cursor (`cursor`), or the page draws its own.
fn cursor_message(host: bool) -> String {
    serde_json::json!({ "type": "cursor", "host": host }).to_string()
}

/// `{"type":"truckersmp","on":…}`: whether the game runs under TruckersMP (the page shows its other players).
fn truckersmp_message(on: bool) -> String {
    serde_json::json!({ "type": "truckersmp", "on": on }).to_string()
}

/// `{"type":"game","running":…}`: whether the game runs. As it closes, the page powers off (as with the
/// truck's electrics off) and forgets the game's last telemetry, so the next game starts it afresh.
fn game_message(running: bool) -> String {
    serde_json::json!({ "type": "game", "running": running }).to_string()
}

/// `{"type":"routeStops","id":…,"stops":[…]}`: the player's stops along the route `id`, by the index of
/// their map node in it (ascending): the game's waypoints (`waypoints`, map node uids) but those WestPlay put
/// in to hold a route it handed over (`hidden`), and but the destination (off a job, the game keeps it as the
/// last waypoint: seen 2026-10-21, at the route's last node). A waypoint not on the route is left out.
fn stops_message(id: Option<u32>, route: Option<&[u64]>, waypoints: &[u64], hidden: &[u64]) -> String {
    let mut stops: Vec<usize> = waypoints
        .iter()
        .filter(|uid| !hidden.contains(uid))
        .filter_map(|uid| route?.iter().position(|node| node == uid))
        .filter(|&at| route.is_some_and(|route| at + 1 < route.len()))
        .collect();
    stops.sort_unstable();
    stops.dedup();
    serde_json::json!({ "type": "routeStops", "id": id, "stops": stops }).to_string()
}

/// The phone in the game's pointer speed, of the screen's.
const HANDSET_SENSITIVITY: f32 = 0.45;

/// How long the phone's top shows for a notification while it's put away, each notification anew (its page says
/// when none is left on it), and how long after the last left it (its way out).
const PEEK_FOR: Duration = Duration::from_secs(6);
const PEEK_AFTER: Duration = Duration::from_millis(400);

/// The plugin's number for a field of TikTok's page selected (`TypingBlock::selection`): none of a page's own.
const TIKTOK_SELECTION: u32 = 0x7157_0001;

/// What the phone in the game's page asks of the host alone (ui/src/shared/host.ts `PageMessage`).
#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum HandsetMessage {
    /// The page (re)loaded (the cab's `PageMessage::Ready` too: it's passed on as well).
    Ready,
    /// A command for TikTok under the phone's page (`tiktok`).
    Tiktok { command: tiktok::Command },
    /// A text field on the in-game phone/iPad selected for physical keyboard typing.
    Typing { selection: u32 },
    /// A command for YouTube running in the in-game iPad/phone (shares YouTube's ad-free engine).
    HandsetYoutube { command: youtube::Command },
    /// Whether a notification is on the phone's screen: its top stays in view meanwhile.
    PhoneBanner { showing: bool },
    /// The notifications' groups turned off in the phone's Settings (alerts.rs `Group`'s names).
    Alerts { off: Vec<String> },
    /// One notification of each group, to see and hear them.
    TestAlerts,
    /// The cargo's paper collected (signed), or put down (`collected` false).
    PaperDone { collected: bool },
    /// The fuel's bill (its `id`) paid on Nupay: `amount` in the game's money (negative), about `detail`.
    FuelPaid { id: u32, amount: i64, detail: String },
    /// The fuel's bill put down without paying.
    FuelPutDown { id: u32 },
}

/// `{"type":"paper","stage":"pickup"}`: the cargo's paper for the phone's page to show in place of the phone
/// (`pickup`: the nota fiscal to sign; `delivery`: the recipient's signature), `null` for none.
fn paper_message(paper: Option<u32>) -> String {
    let stage = paper.map(|kind| match kind {
        westplay_shared::NOTE_DELIVERY => "delivery",
        // The coupling's steps by hand (owner 2026-11-19): the fifth wheel's lock, the trailer's legs.
        westplay_shared::NOTE_LOCK_PULL => "lockPull",
        westplay_shared::NOTE_LOCK_PUSH => "lockPush",
        westplay_shared::NOTE_LEGS_UP => "legsUp",
        westplay_shared::NOTE_LEGS_DOWN => "legsDown",
        _ => "pickup",
    });
    serde_json::json!({ "type": "paper", "stage": stage }).to_string()
}

/// `{"type":"fuelBill","bill":{…}}`: the fuel filled by hand for the phone's page to show on Nupay, to pay (`null`:
/// none).
fn bill_message(bill: Option<&westplay_shared::FuelBill>) -> String {
    let bill = bill.map(|bill| {
        serde_json::json!({
            "id": bill.id,
            "gallons": bill.gallons == 1,
            "amount": bill.amount,
            "price": bill.price,
            "total": bill.total,
            "sign": bill.sign.as_str(),
        })
    });
    serde_json::json!({ "type": "fuelBill", "bill": bill }).to_string()
}

/// TikTok, started as the phone's app first shows it (on a WebView2 profile of its own: signing out of TikTok
/// forgets nothing else), under the phone's page; `None` if it couldn't start, or there's no phone.
fn start_tiktok<'a>(
    tiktok: &'a mut Option<tiktok::TikTok>,
    stage: &Stage,
    layers: Option<&webview::Layers>,
    config: &Config,
) -> Option<&'a tiktok::TikTok> {
    if tiktok.is_none() {
        let user_data = config.data_dir.join("webview2-tiktok");
        match tiktok::TikTok::start(stage, layers?, &user_data) {
            Ok(started) => {
                log!("[tiktok] opened");
                *tiktok = Some(started)
            }
            Err(e) => startup::mark(&format!("host: TikTok couldn't start: {e}")),
        }
    }
    tiktok.as_ref()
}

/// The most presses of a keybind the page is told of at once.
const MOST_KEYBIND_PRESSES: u32 = 10;

/// `{"type":"keybind","action":"volumeUp"|"volumeDown"|"playPause"}`: the player pressed a keybind in the game
/// (`None`: the phone's, which the host acts on itself).
fn keybind_message(keybind: Keybind) -> Option<String> {
    let action = match keybind {
        Keybind::VolumeUp => "volumeUp",
        Keybind::VolumeDown => "volumeDown",
        Keybind::PlayPause => "playPause",
        Keybind::Phone | Keybind::Character | Keybind::Hints | Keybind::RoughRoad => return None,
    };
    Some(serde_json::json!({ "type": "keybind", "action": action }).to_string())
}

/// The phone in the game's page (ui/handset.html), beside the cab's on its browser, with the cab's folders.
fn open_handset(cab: &OffscreenWebView, stage: &Stage, root: &Path, maps: &Path) -> Result<OffscreenWebView, Error> {
    let handset = cab.beside(stage, westplay_shared::PHONE_WIDTH, westplay_shared::PHONE_HEIGHT)?;
    handset.map_folder("westplay.cabin", root)?;
    handset.map_folder_to_any_page("westplay.maps", maps)?;
    handset.navigate("https://westplay.cabin/handset.html")?;
    handset.show(false)?;
    Ok(handset)
}

/// The page's height on the layout at `index` (`westplay_shared::page_height`); a mod screen's slot's, as setup
/// found its screen's shape (`westplay_shared::MOD_SCREENS_FILE`, beside the mod).
fn page_height_on(index: usize) -> u32 {
    let slot = westplay_shared::screen_layouts()
        .get(index)
        .map(|layout| layout.id)
        .filter(|id| westplay_shared::is_mod_slot(id));
    let told = slot.and_then(|id| {
        let file = westplay_setup::places::westplay_documents()?.join(westplay_shared::MOD_SCREENS_FILE);
        westplay_shared::mod_page_height(&std::fs::read_to_string(file).ok()?, id)
    });
    told.unwrap_or_else(|| westplay_shared::page_height(index))
}

/// What the phone in the game's page posted since last taken: what a player's phone may send (remote.rs).
///
/// Apart (first), what it asks of the host alone ([`HandsetMessage`]): its `ready` is both.
fn handset_messages(handset: Option<&OffscreenWebView>) -> (Vec<String>, Vec<String>) {
    let Some(handset) = handset else { return (Vec::new(), Vec::new()) };
    let (mut own, mut rest) = (Vec::new(), Vec::new());
    for message in handset.take_page_messages() {
        if serde_json::from_str::<HandsetMessage>(&message).is_ok() {
            own.push(message.clone());
        }
        if remote::allowed(&message) {
            rest.push(message);
        }
    }
    (own, rest)
}

/// `{"type":"screenSeen","seen":…}`: whether WestPlay's screen is in the game's view (`camera::Unseen`).
fn screen_seen_message(seen: bool) -> String {
    serde_json::json!({ "type": "screenSeen", "seen": seen }).to_string()
}

/// `{"type":"gameShown","shown":…}`: whether the game's window shows (`is_shown`).
fn game_shown_message(shown: bool) -> String {
    serde_json::json!({ "type": "gameShown", "shown": shown }).to_string()
}

/// What YouTube's player does, for the log (play_log.rs), from its state (-1 unstarted, 0 ended, 1 playing,
/// 2 paused, 3 buffering, 5 cued).
fn youtube_heard(state: &youtube::State) -> play_log::Heard<'_> {
    let doing = match state.state {
        1 => play_log::Doing::Playing,
        3 | -1 => play_log::Doing::Loading,
        _ => play_log::Doing::Paused,
    };
    play_log::Heard { doing, track: state.video.as_deref(), position: state.time }
}

/// YouTube, started as it's first needed (on the profile the cabin page's WebView uses), or `None` if it
/// couldn't start.
fn start_youtube<'a>(
    youtube: &'a mut Option<youtube::YouTube>,
    stage: &Stage,
    config: &Config,
) -> Option<&'a youtube::YouTube> {
    if youtube.is_none() {
        let user_data = config.data_dir.join("webview2");
        match youtube::YouTube::start(stage, config.width, config.height, &user_data) {
            Ok(started) => {
                log!("[youtube] opened");
                *youtube = Some(started)
            }
            Err(e) => startup::mark(&format!("host: YouTube couldn't start: {e}")),
        }
    }
    youtube.as_ref()
}

/// Apple Music's web player, started as the page first shows it (on a WebView2 profile of its own), or `None`
/// if it couldn't start.
fn start_apple_music<'a>(
    apple_music: &'a mut Option<apple_music::AppleMusic>,
    stage: &Stage,
    config: &Config,
) -> Option<&'a apple_music::AppleMusic> {
    if apple_music.is_none() {
        let user_data = config.data_dir.join("webview2-apple-music");
        match apple_music::AppleMusic::start(stage, config.width, config.height, &user_data) {
            Ok(started) => {
                log!("[apple music] opened");
                *apple_music = Some(started)
            }
            Err(e) => startup::mark(&format!("host: Apple Music couldn't start: {e}")),
        }
    }
    apple_music.as_ref()
}

/// YouTube Music's web player, started as the page first shows it (on the cabin page's profile, as YouTube's:
/// they share the sign-in), or `None` if it couldn't start.
fn start_youtube_music<'a>(
    youtube_music: &'a mut Option<youtube_music::YouTubeMusic>,
    stage: &Stage,
    config: &Config,
) -> Option<&'a youtube_music::YouTubeMusic> {
    if youtube_music.is_none() {
        let user_data = config.data_dir.join("webview2");
        match youtube_music::YouTubeMusic::start(stage, config.width, config.height, &user_data) {
            Ok(started) => {
                log!("[youtube music] opened");
                *youtube_music = Some(started)
            }
            Err(e) => startup::mark(&format!("host: YouTube Music couldn't start: {e}")),
        }
    }
    youtube_music.as_ref()
}

/// Spotify's web player, started as the page first shows it (on a WebView2 profile of its own: signing out
/// of Spotify forgets nothing else), or `None` if it couldn't start.
fn start_spotify<'a>(
    spotify: &'a mut Option<spotify::Spotify>,
    stage: &Stage,
    config: &Config,
) -> Option<&'a spotify::Spotify> {
    if spotify.is_none() {
        let user_data = config.data_dir.join("webview2-spotify");
        match spotify::Spotify::start(stage, config.width, config.height, &user_data) {
            Ok(started) => {
                log!("[spotify] opened");
                *spotify = Some(started)
            }
            Err(e) => startup::mark(&format!("host: Spotify couldn't start: {e}")),
        }
    }
    spotify.as_ref()
}

fn build_pipeline(
    config: &Config,
    stage: &Stage,
    phone: Option<&webview::Layers>,
    phone_dims: (u32, u32),
    bridge: &Arc<BridgeMapping>,
    luid: Option<LUID>,
    generation: u32,
) -> Result<Pipeline, Error> {
    let gpu = Gpu::new(luid)?;
    let name = westplay_shared::frame_texture_name(generation);
    let shared = Arc::new(SharedFrame::new(&gpu, config.width, config.height, &name)?);
    let capture = Capture::start(&gpu, &stage.visual()?, shared, bridge.clone(), Published::Screen)?;
    // The phone's frame, the cab's without it if it can't be had.
    let phone = phone.and_then(|layers| {
        let started = (|| -> Result<Capture, Error> {
            let (width, height) = phone_dims;
            let name = westplay_shared::phone_texture_name(generation);
            let shared = Arc::new(SharedFrame::new(&gpu, width, height, &name)?);
            Ok(Capture::start(&gpu, &layers.visual()?, shared, bridge.clone(), Published::Phone)?)
        })();
        started.map_err(|e| log!("[handset] no frame: {e}")).ok()
    });
    Ok(Pipeline { gpu, capture, phone })
}

/// The GPU the game renders on, once the plugin has published it.
fn game_adapter(bridge: &BridgeMapping) -> Option<LUID> {
    let plugin = &bridge.bridge().plugin;
    if plugin.pid.load(Ordering::Acquire) == 0 {
        return None;
    }
    let luid = LUID {
        LowPart: plugin.adapter_luid_low.load(Ordering::Relaxed),
        HighPart: plugin.adapter_luid_high.load(Ordering::Relaxed),
    };
    (luid.LowPart != 0 || luid.HighPart != 0).then_some(luid)
}

/// `{"type":"screen","layout":"scania-2025"}`: which kind of screen the game draws WestPlay on, from the
/// bridge's `screen_layout` (`null` until a screen has been drawn).
fn screen_message(layout: u32) -> String {
    let id = (layout as usize).checked_sub(1).and_then(|index| screen_layouts().get(index)).map(|l| l.id);
    serde_json::json!({ "type": "screen", "layout": id }).to_string()
}

/// `{"type":"settings","truckStrip":"auto","wallpaper":"plain","iconSet":null,"iconStyle":"auto","equalizer":…}`: the
/// settings the cabin page applies itself.
fn settings_message(settings: &settings::Settings) -> String {
    serde_json::json!({
        "type": "settings",
        "truckStrip": settings.screen.truck_strip,
        "split": settings.screen.split,
        "screenResolution": settings.screen.resolution,
        "keepScreenOn": false,
        "wallpaper": settings.look.wallpaper,
        "iconSet": settings.look.icon_set,
        "iconStyle": settings.look.icon_style,
        "equalizer": settings.sound.equalizer.clamped(),
        "equalizerCustom": settings.sound.custom,
        "characterPerson": settings.character.person.get(),
        "freightPapers": settings.character.papers.0,
        "manualRefuel": settings.character.refuel.0,
        "payAttendant": settings.character.pay_attendant.0,
        "virtualMirrors": settings.virtual_mirrors,
        "suspensionAutoReset": settings.suspension_auto_reset.0,
        "suspensionLowFront": settings.suspension_low_front,
        "suspensionDamping": settings.suspension_damping,
        "cursorSize": settings.cursor_size,
        "suspensionAuto": settings.suspension_auto,
        "suspensionAutoDelay": settings::suspension_auto_delay(settings),
        "suspensionAutoReturn": settings::suspension_auto_return(settings),
        "suspensionAutoBrake": settings::suspension_auto_brake(settings),
        "brakeFade": settings.brake_fade.0,
        "brakeFadeLevel": settings.brake_fade_level,
        "brakeSquealVolume": settings.brake_squeal_volume,
        "brakeWarningVolume": settings.brake_warning_volume,
        "brakeHud": settings.brake_hud.0,
        "brakeHudCorner": settings.brake_hud_corner,
        "brakeHudSize": settings.brake_hud_size,
        "brakeHudOnFoot": settings.brake_hud_on_foot,
        "brakeSmoke": settings.brake_smoke.0,
        "brakeAlarm": settings::brake_alarm_c(&settings),
        "audioOutput": settings.audio_output,
    })
    .to_string()
}

/// `{"type":"customize","wallpapers":[…],"iconSets":[…]}`: the player's own wallpapers and icon sets
/// (`customize::Customizations`), their files at `https://westplay.customize/`.
fn customize_list_message(customizations: &customize::Customizations) -> String {
    let mut message = serde_json::to_value(customizations).unwrap_or_default();
    message["type"] = "customize".into();
    message.to_string()
}

/// Messages from the cabin page (`PageMessage` in ui/src/shared/host.ts).
#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum PageMessage {
    /// Posted when the page (re)loads.
    Ready,
    /// A startup stage the page reached (the boot screens shown), for the startup log (`startup`).
    Startup { stage: String },
    /// Whether WestPlay is open on the truck's screen: posted as the page loads and whenever it changes.
    Westplay { open: bool },
    /// The wallpaper chosen in the cab's Settings (an id: `wallpapers.ts`), to keep with the settings.
    Wallpaper { id: String },
    /// The truck's information bar beside WestPlay, kept or not, chosen in the cab's Settings (Appearance).
    TruckStrip { strip: settings::TruckStrip },
    /// Live screen sharpness tier chosen in the cab's Settings (Adjust Screen), applied immediately on the next frame.
    ScreenResolution { resolution: westplay_setup::game_mod::ScreenResolution },
    /// Keep the in-cabin screen powered on even when the truck's ignition/electrics are off.
    KeepScreenOn { on: bool },
    /// The players' tone, set in the cab's Settings (Equalizer): the player's curve (bass, mid and treble, dB
    /// each), and whether it plays (Custom) or the speakers as tuned (Recommended).
    Equalizer {
        #[serde(flatten)]
        equalizer: speakers::Equalizer,
        #[serde(default)]
        custom: bool,
    },
    /// The music's spectrum wanted (the cab's Equalizer open) or not: the players measure it, and it's passed
    /// on to the page as they say it.
    Spectrum { on: bool },
    /// Whether what's under the cursor is light (it's drawn dark there: `cursor`).
    CursorLight { light: bool },
    /// One of the truck's commands pressed or let go in the phone (truck_commands.rs).
    TruckCommand { command: String, down: bool },
    /// A volume the player's phone set (remote.rs, 0 to 1): the cab shows it, as for a keybind.
    VolumeShown { volume: f64 },
    /// A command for YouTube's player (`youtube`): what to play, where it shows, and so on.
    Youtube { command: youtube::Command },
    /// A command for Spotify's web player (`spotify`): its sign-in, a question, its controls.
    Spotify { command: spotify::Command },
    /// A command for Apple Music's web player (`appleMusic`): its sign-in, a question, its controls.
    AppleMusic { command: apple_music::Command },
    /// A command for YouTube Music's web player (`youtubeMusic`): its sign-in, a question, its controls.
    YoutubeMusic { command: youtube_music::Command },
    /// The text field selected on the page, a number that changes with every selection (0: none), for the
    /// player's own keyboard to type into (`typing`).
    Typing { selection: u32 },
    /// A change to the game's route (Maps' search): at a map node (its uid in hex) to go or add a stop; `id`
    /// comes back with the game's answer (`route_change`).
    Route { id: u32, action: PageRouteAction, node: Option<String> },
    /// The routes to a map node (its uid in hex) from where the truck is, to offer before Go (`planner`);
    /// `id` comes back with them.
    Routes { id: u32, node: String },
    /// Go along the route at `index` of those offered for request `id`: handed to the game's GPS
    /// (`handover`); `{"type":"followed"}` comes back once the game follows it, or won't.
    Follow { id: u32, index: usize },
    /// What stopping at each of these map nodes (uids in hex) would add to the trip the game's route makes
    /// (`planner`); `{"type":"detours"}` comes back with `id`.
    Detours { id: u32, nodes: Vec<String> },
    /// Nupay's history let go, its balance kept (`wallet`): the account comes back without it.
    WalletClear,
    /// Queue a new bank loan in Nupay (offline fallback when the game isn't running).
    WalletLoan { amount: i64, days: i32, rate: f32 },
    WalletLoanCancel,
    /// Live in-memory Nupay Bank RPCs (zero restart).
    BankRefresh { id: u32 },
    #[serde(rename_all = "camelCase")]
    BankTakeLoan { id: u32, offer_index: u32 },
    #[serde(rename_all = "camelCase")]
    BankRepayLoan { id: u32, loan_index: u32 },
    /// Live in-memory Company / Fleet & Garage RPCs (zero restart).
    FleetRefresh { id: u32 },
    #[serde(rename_all = "camelCase")]
    FleetDriveTruck { id: u32, truck_ptr: String },
    #[serde(rename_all = "camelCase")]
    FleetUseTrailer { id: u32, trailer_ptr: String },
    FleetStoreTrailer { id: u32 },
    #[serde(rename_all = "camelCase")]
    FleetUpgradeGarage { id: u32, garage_ptr: String },
    /// Who walks in the Modo Personagem, chosen in the phone's Personagem app (a place in
    /// `westplay_shared::CHARACTER_PEOPLE`): kept with the settings, for the next time out of the truck.
    CharacterPerson { person: f64 },
    /// The Modo Personagem's switches (a phone's Settings): the cargo's papers and coupling by hand, refuelling by
    /// hand, paying at the pump's attendant.
    FreightPapers { on: bool },
    ManualRefuel { on: bool },
    PayAttendant { on: bool },
    /// Which of the game's virtual mirrors show (a phone's Settings): `none`, `right`, `left` or `both`.
    VirtualMirrors { which: String },
    /// Whether the game sets the suspension back by itself (the phone's Suspensão app).
    SuspensionAutoReset { on: bool },
    /// How low the front may go (the Suspensão app): `normal`, `low` or `lowest`.
    SuspensionLowFront { which: String },
    /// How firm the dampers are (the Suspensão app): `soft`, `normal`, `firm` or `firmest`.
    SuspensionDamping { which: String },
    CursorSize { which: String },
    /// Where media audio plays (`cabin` or `device`).
    AudioOutput { which: String },
    SuspensionAuto { on: bool },
    SuspensionAutoDelay { secs: u8 },
    SuspensionAutoReturn { secs: u8 },
    SuspensionAutoBrake { secs: u8 },
    /// Brake fade (owner 2026-11-21): on or off, and how soon (`light`, `realistic`, `hard`).
    BrakeFade { on: bool },
    BrakeFadeLevel { which: String },
    /// The brake fade's sounds' volumes (0..100; 0 off) and the brakes' HUD (shown, `bottomLeft`… and `small`…).
    BrakeSqueal { volume: f64 },
    BrakeWarning { volume: f64 },
    BrakeHud { on: bool },
    BrakeHudCorner { which: String },
    BrakeHudSize { which: String },
    BrakeHudOnFoot { on: bool },
    BrakeSmoke { on: bool },
    /// When the brake alarm comes (°C on the realistic level).
    BrakeAlarm { celsius: f64 },
    /// Where the reversing camera shows the game's rear-view mirrors (the page see-through there), in the frame's
    /// pixels; a width of 0 while it doesn't (`HostBlock::mirror_rect`).
    MirrorCamera { x: f64, y: f64, width: f64, height: f64 },
    /// The reversing camera moved on its screen (`back`, `forward`, `up`, `down`, `reset`): `HostBlock::camera_nudge`.
    CameraNudge { action: String },
    /// MaxDevMod's floating screen in the cab, from the phone's Tela app (`toggle`, `forward`, `back`, `up`, `down`,
    /// `left`, `right`, `turnLeft`, `turnRight`, `tiltUp`, `tiltDown`, `rollLeft`, `rollRight`, `reset`):
    /// `HostBlock::float_screen`.
    FloatScreen { action: String },
    /// Live in-memory Job Market (Bolsa de Fretes): snapshot offers from the game's live economy.
    JobMarketRefresh { id: u32 },
    /// Live in-memory Job Market: accept a chosen offer (`freight` | `cargo` | `quick`) directly on the main thread.
    #[serde(rename_all = "camelCase")]
    JobMarketAccept { id: u32, mode: String, company_ptr: String, offer_ptr: String },
    /// Live in-memory Job Market: cancel the player's current job (the game's own cancel, with its penalty).
    JobMarketCancel { id: u32 },
    /// On-demand cargo icon image lookup from game/DLC/mod `.scs` archives.
    #[serde(rename_all = "camelCase")]
    JobMarketIcon { id: u32, icon: String, cargo_id: String },
    /// Live handheld form-factor switch (`phone` = iPhone 340x720, `ipad` = vertical iPad 560x760).
    PhoneDevice { device: String },
}

/// The page's rectangle for the mirrors as the bridge has it (whole pixels, within the frame): none (all 0) if it's
/// empty.
fn mirror_rect(x: f64, y: f64, width: f64, height: f64) -> [u32; 4] {
    let (most_x, most_y) = (f64::from(westplay_shared::SCREEN_WIDTH), f64::from(westplay_shared::SCREEN_HEIGHT));
    let left = x.clamp(0.0, most_x).round();
    let top = y.clamp(0.0, most_y).round();
    let right = (x + width).clamp(0.0, most_x).round();
    let bottom = (y + height).clamp(0.0, most_y).round();
    if !(right - left >= 8.0 && bottom - top >= 8.0) {
        return [0; 4];
    }
    [left as u32, top as u32, (right - left) as u32, (bottom - top) as u32]
}

/// A route being handed to the game's GPS: the page's request, the hand-over (the route picked and the
/// waypoints holding the game to it), where each of the route's nodes is, the game's route number as the last
/// change was asked, and, once the game took it, when and that route number (its next route is compared).
struct Following {
    id: u32,
    handover: handover::Handover,
    positions: Vec<[f32; 2]>,
    /// The game's route number as the last change was asked, not as it answered: the game may plan the new route
    /// before its answer is read (2026-11-02, a short route in ETS2: the new route came first, and the hand-over
    /// waited for another until it said the game didn't answer).
    asked: Option<u32>,
    answered: Option<(Instant, Option<u32>)>,
}

/// Marks a route request as the hand-over's own (its answer isn't the page's).
const FOLLOWING: u32 = 1 << 31;

/// `{"type":"followed","id":4,"follows":true,"answer":"done"}`: whether the game's GPS follows the route the
/// player picked (`answer`: "ownWay" when it takes another way however held, else the game's answer).
fn followed_message(id: u32, follows: bool, answer: &str) -> String {
    serde_json::json!({ "type": "followed", "id": id, "follows": follows, "answer": answer }).to_string()
}

/// What the page asks of the game's route.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum PageRouteAction {
    AddStop,
    Go,
    End,
}

impl From<PageRouteAction> for westplay_shared::RouteAction {
    fn from(action: PageRouteAction) -> Self {
        match action {
            PageRouteAction::AddStop => Self::AddStop,
            PageRouteAction::Go => Self::Go,
            PageRouteAction::End => Self::End,
        }
    }
}

/// `{"type":"westplay","open":true}`: open (or close) WestPlay on the truck's screen.
fn westplay_message(open: bool) -> String {
    serde_json::json!({ "type": "westplay", "open": open }).to_string()
}

/// The suspension's auto reset as the games' options have it, looked at now and then: set there as the player chose
/// once the games are closed (they write their options back as they close).
#[derive(Default)]
struct SuspensionOption {
    /// When it was last looked at (`None`: never, or the choice just changed), and whether it was there then.
    checked: Option<Instant>,
    applied: Option<bool>,
}

impl SuspensionOption {
    const EVERY: Duration = Duration::from_secs(5);

    /// Looked at again (every few seconds, or at once after a change): set if it can be; whether auto reset is there
    /// now, if that changed. Also syncs profile spring stiffness (`g_suspension_stiffness` /
    /// `g_cabin_suspension_stiffness`) when a non-Normal damping preset is active (owner 2026-11-22).
    fn check(&mut self, wanted: bool, damping: westplay_setup::low_suspension::Damping) -> Option<bool> {
        if self.checked.is_some_and(|at| at.elapsed() < Self::EVERY) {
            return None;
        }
        self.checked = Some(Instant::now());
        let mut now = westplay_setup::suspension::auto_reset().unwrap_or(true) == wanted;
        if !now && westplay_setup::suspension::set_auto_reset(wanted) {
            log!("[suspension] the game's auto reset set {} in its options", if wanted { "on" } else { "off" });
            now = westplay_setup::suspension::auto_reset().unwrap_or(true) == wanted;
        }
        if !westplay_setup::suspension::stiffness_applied(damping)
            && westplay_setup::suspension::set_stiffness(damping)
        {
            log!("[suspension] the profiles' spring stiffness set for {damping:?}");
        }
        (self.applied != Some(now)).then(|| {
            self.applied = Some(now);
            now
        })
    }
}

fn parse_page_message(message: &str) -> Option<PageMessage> {
    serde_json::from_str(message).ok()
}

/// What the player's latest save says, for the page: `{"type":"garages","owned":["calais",...],
/// "headquarters":"calais"}` (the cities whose garages they own, and their headquarters', `null` if none),
/// `{"type":"mapBlock","hvt":true,"language":"tr_tr"}` (they play on a map by HVT, which WestPlay
/// doesn't allow, and the game's language to say so in), and `{"type":"timeZones","on":true}` (their
/// profile has the game's time zones on: the clock is the time where the truck is).
fn player_messages_for(player: &Player) -> Vec<String> {
    let garages = &player.garages;
    vec![
        serde_json::json!({ "type": "garages", "owned": garages.owned, "headquarters": garages.headquarters })
            .to_string(),
        serde_json::json!({ "type": "mapBlock", "hvt": player.hvt_map, "language": player.language }).to_string(),
        serde_json::json!({ "type": "timeZones", "on": player.time_zones }).to_string(),
    ]
}

/// The game's weather for the page:
/// `{"type":"weather","weather":{"now":"clear","coming":"rain","changeIn":102,"wetness":0}}` (the weather
/// there is and the one it's turning to, game minutes to its next change and how wet the roads are in
/// percent, each `null` when not read), or `{"type":"weather","weather":null}` when it can't be read.
fn weather_message_for(weather: Option<Weather>) -> String {
    let sky = |sky: Sky| match sky {
        Sky::Clear => "clear",
        Sky::Rain => "rain",
    };
    let weather = weather.map(|w| {
        serde_json::json!({ "now": sky(w.now), "coming": sky(w.coming), "changeIn": w.change_in, "wetness": w.wetness })
    });
    serde_json::json!({ "type": "weather", "weather": weather }).to_string()
}

/// The game's route for the page: `{"type":"route","id":5,"nodes":[[x,z,forward x,forward z,distance
/// left],...]}` (its map nodes in order, metres; `"nodes":null` without a route), `{"type":"routePaths",
/// "id":5,"paths":[[2,[[x,z],...]],...],"maneuvers":[[2,"roundabout",-90.0,3],...],"shifts":[[3,6.5],...]}` (the ways through its
/// junctions and the turns there, each by the node it starts at: the turn, its angle, a roundabout's exit)
/// and `{"type":"routeNext","next":3}` (the next node ahead of the truck).
fn route_message_for(update: &RouteUpdate) -> String {
    match update {
        RouteUpdate::Route { id, nodes, .. } => {
            let nodes: Vec<[f32; 5]> = nodes
                .iter()
                .map(|n| [n.position[0], n.position[1], n.forward[0], n.forward[1], n.distance_left])
                .collect();
            serde_json::json!({ "type": "route", "id": id, "nodes": nodes }).to_string()
        }
        RouteUpdate::Paths { id, paths, maneuvers, shifts } => {
            let maneuvers: Vec<serde_json::Value> = maneuvers
                .iter()
                .map(|(span, maneuver)| {
                    let exit = match maneuver.turn {
                        Turn::Roundabout(exit) => Some(exit),
                        _ => None,
                    };
                    serde_json::json!([span, maneuver.turn.name(), maneuver.angle.round(), exit])
                })
                .collect();
            let shifts: Vec<(usize, f32)> =
                shifts.iter().map(|&(node, shift)| (node, (shift * 100.0).round() / 100.0)).collect();
            serde_json::json!({ "type": "routePaths", "id": id, "paths": paths, "maneuvers": maneuvers, "shifts": shifts })
                .to_string()
        }
        RouteUpdate::Next(next) => serde_json::json!({ "type": "routeNext", "next": next }).to_string(),
        RouteUpdate::None => serde_json::json!({ "type": "route", "id": null, "nodes": null }).to_string(),
    }
}

fn same_luid(a: LUID, b: LUID) -> bool {
    a.LowPart == b.LowPart && a.HighPart == b.HighPart
}

fn pump_messages() {
    // SAFETY: this thread's own messages, each passed on as Windows hands it; `msg` is a local.
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PageMessage, PageRouteAction, parse_page_message, screen_message, settings_message};

    #[test]
    fn a_game_run_normally_isnt_elevated() {
        use windows::Win32::System::Threading::GetCurrentProcessId;
        assert!(!super::is_elevated(0));
        // This test, run as the player runs WestPlay (not as administrator): not elevated.
        // SAFETY: a plain query.
        let this = unsafe { GetCurrentProcessId() };
        assert!(!super::is_elevated(this));
    }

    /// Every process on this PC, elevated or not, for a look by eye (`--ignored --nocapture`): the system's own
    /// services elevated, the player's own programs not.
    #[test]
    #[ignore]
    fn lists_which_processes_are_elevated() {
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
        };
        // SAFETY: a snapshot walked with an entry of its size set, as the calls require.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
            let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
            while more {
                let length = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(0);
                let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
                println!("{:>6} {:<5} {name}", entry.th32ProcessID, super::is_elevated(entry.th32ProcessID));
                more = Process32NextW(snapshot, &mut entry).is_ok();
            }
        }
    }

    #[test]
    fn the_game_shows_while_its_window_does_in_front_or_not() {
        use windows::Win32::System::Threading::GetCurrentProcessId;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, SW_HIDE, SW_MINIMIZE, SW_SHOWNOACTIVATE, ShowWindow, WINDOW_EX_STYLE,
            WS_OVERLAPPEDWINDOW,
        };
        use windows::core::w;
        // SAFETY: a plain query.
        let this = unsafe { GetCurrentProcessId() };
        // No game: nothing to be away from.
        assert!(super::is_shown(0));
        // No window of its own yet.
        assert!(!super::is_shown(this));
        // SAFETY: a window of this test's own, shown, minimised, hidden and destroyed here.
        unsafe {
            let window = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("game"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                200,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            // Shown without being brought to the front: it shows.
            let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
            assert!(super::is_shown(this));
            let _ = ShowWindow(window, SW_MINIMIZE);
            assert!(!super::is_shown(this));
            let _ = ShowWindow(window, SW_HIDE);
            assert!(!super::is_shown(this));
            let _ = DestroyWindow(window);
        }
    }

    #[test]
    fn the_players_stops_are_the_waypoints_but_westplays_and_the_destination() {
        let route = [10, 11, 12, 13, 14, 15];
        // The game's: a stop at 13, WestPlay's own at 11, another stop at 12, the destination at 15, one off
        // the route.
        let message = super::stops_message(Some(7), Some(&route), &[13, 11, 12, 15, 99], &[11]);
        assert_eq!(message, r#"{"id":7,"stops":[2,3],"type":"routeStops"}"#);
        // No route: none.
        assert_eq!(super::stops_message(None, None, &[13], &[]), r#"{"id":null,"stops":[],"type":"routeStops"}"#);
    }

    #[test]
    fn reads_the_pages_messages() {
        assert_eq!(parse_page_message(r#"{"type":"ready"}"#), Some(PageMessage::Ready));
        assert_eq!(
            parse_page_message(r#"{"type":"cameraNudge","action":"back"}"#),
            Some(PageMessage::CameraNudge { action: "back".into() })
        );
        assert_eq!(
            parse_page_message(r#"{"type":"floatScreen","action":"toggle"}"#),
            Some(PageMessage::FloatScreen { action: "toggle".into() })
        );
        assert_eq!(
            parse_page_message(r#"{"type":"mirrorCamera","x":0,"y":0,"width":900.5,"height":720}"#),
            Some(PageMessage::MirrorCamera { x: 0.0, y: 0.0, width: 900.5, height: 720.0 })
        );
        assert_eq!(super::mirror_rect(10.4, -3.0, 900.0, 800.0), [10, 0, 900, 720]);
        assert_eq!(super::mirror_rect(0.0, 0.0, 0.0, 0.0), [0; 4]);
        assert_eq!(parse_page_message(r#"{"type":"walletClear"}"#), Some(PageMessage::WalletClear));
        assert_eq!(
            parse_page_message(r#"{"type":"virtualMirrors","which":"left"}"#),
            Some(PageMessage::VirtualMirrors { which: "left".to_owned() })
        );
        // The PC's own music apps aren't followed (removed 2026-10-17): their controls are no message.
        assert_eq!(parse_page_message(r#"{"type":"media","source":"AppleMusic.exe","action":"playPause"}"#), None);
        assert_eq!(
            parse_page_message(r#"{"type":"startup","stage":"power splash"}"#),
            Some(PageMessage::Startup { stage: "power splash".into() })
        );
        assert_eq!(
            parse_page_message(r#"{"type":"westplay","open":false}"#),
            Some(PageMessage::Westplay { open: false })
        );
        assert_eq!(parse_page_message(r#"{"type":"westplay"}"#), None);
        assert_eq!(
            parse_page_message(r#"{"type":"wallpaper","id":"custom:Dusk"}"#),
            Some(PageMessage::Wallpaper { id: "custom:Dusk".into() })
        );
        assert_eq!(
            parse_page_message(r#"{"type":"truckStrip","strip":"off"}"#),
            Some(PageMessage::TruckStrip { strip: crate::settings::TruckStrip::Off })
        );
        assert_eq!(parse_page_message(r#"{"type":"truckStrip","strip":"sideways"}"#), None);
        assert_eq!(
            parse_page_message(r#"{"type":"equalizer","bass":-3,"mid":0,"treble":2,"custom":true}"#),
            Some(PageMessage::Equalizer {
                equalizer: crate::speakers::Equalizer { bass: -3, mid: 0, treble: 2 },
                custom: true
            })
        );
        // A band not said is flat; one past a byte is no message (one past the range is kept to it as it's set).
        assert_eq!(
            parse_page_message(r#"{"type":"equalizer","bass":4}"#),
            Some(PageMessage::Equalizer {
                equalizer: crate::speakers::Equalizer { bass: 4, mid: 0, treble: 0 },
                custom: false
            })
        );
        assert_eq!(parse_page_message(r#"{"type":"equalizer","bass":400}"#), None);
        assert_eq!(parse_page_message(r#"{"type":"spectrum","on":true}"#), Some(PageMessage::Spectrum { on: true }));
        assert_eq!(
            parse_page_message(r#"{"type":"cursorLight","light":true}"#),
            Some(PageMessage::CursorLight { light: true })
        );
        assert_eq!(
            parse_page_message(r#"{"type":"route","id":3,"action":"addStop","node":"478ef4c4cd160002"}"#),
            Some(PageMessage::Route { id: 3, action: PageRouteAction::AddStop, node: Some("478ef4c4cd160002".into()) })
        );
        assert_eq!(
            parse_page_message(r#"{"type":"route","id":4,"action":"end"}"#),
            Some(PageMessage::Route { id: 4, action: PageRouteAction::End, node: None })
        );
        assert_eq!(parse_page_message(r#"{"type":"route","id":5,"action":"teleport"}"#), None);
        assert_eq!(
            parse_page_message(r#"{"type":"typing","selection":3}"#),
            Some(PageMessage::Typing { selection: 3 })
        );
        assert_eq!(
            parse_page_message(r#"{"type":"youtube","command":{"action":"pause"}}"#),
            Some(PageMessage::Youtube { command: crate::youtube::Command::Pause })
        );
        assert_eq!(parse_page_message(r#"{"type":"other"}"#), None);
        assert_eq!(parse_page_message("not json"), None);
    }

    #[test]
    fn only_one_westplay_claims_the_session() {
        // A name of the test's own (a WestPlay may be running on this PC).
        let name = windows::core::w!(r"Local\WestPlay.Instance.Test");
        let first = super::Instance::claim_named(name).expect("the first claims it");
        assert!(super::Instance::claim_named(name).is_none(), "a second must not");
        drop(first);
        assert!(super::Instance::claim_named(name).is_some(), "once let go, it can be claimed again");
    }

    #[test]
    fn tells_the_page_truckersmps_players_around_the_player() {
        use westplay_shared::{Nearby, NearbyPlayer};
        let mut nearby = Nearby { frame: 9, server: 4, local: 113, count: 1, ..Nearby::default() };
        nearby.players[0] = NearbyPlayer { id: 7, heading: 0.25, x: -6175.5, z: 2139.0 };
        assert_eq!(
            super::nearby_message(Some(&nearby)),
            r#"{"local":113,"players":[[7,-6175.5,2139.0,0.25]],"server":4,"type":"nearby"}"#
        );
        assert_eq!(super::nearby_message(None), r#"{"local":null,"players":[],"server":null,"type":"nearby"}"#);
    }

    #[test]
    fn finds_a_module_a_process_has_loaded() {
        use super::has_module;
        let me = std::process::id();
        assert!(has_module(me, &["NTDLL.dll"]));
        assert!(!has_module(me, &["core_ets2mp.dll"]));
        assert!(!has_module(0, &["ntdll.dll"]));
    }

    #[test]
    fn a_game_runs_while_its_process_does() {
        use super::is_running;
        assert!(!is_running(0));
        assert!(is_running(std::process::id()));
        // A game that crashed: its pid left behind, its process ended.
        let mut ended = std::process::Command::new("cmd").args(["/c", "exit"]).spawn().unwrap();
        let pid = ended.id();
        ended.wait().unwrap();
        assert!(!is_running(pid));
    }

    #[test]
    fn a_game_is_alive_while_its_heartbeat_moves() {
        use std::time::{Duration, Instant};
        let start = Instant::now() + Duration::from_secs(60);
        let mut liveness = super::Liveness::default();
        // A heartbeat already there proves nothing (a crashed game leaves it behind).
        assert!(!liveness.update(500, start));
        assert!(!liveness.update(500, start + Duration::from_secs(1)));
        // Moving: alive.
        assert!(liveness.update(560, start + Duration::from_secs(2)));
        assert!(liveness.update(560, start + Duration::from_secs(4)));
        // Standing still past the timeout (the game crashed): gone.
        assert!(!liveness.update(560, start + Duration::from_secs(2) + super::GAME_TIMEOUT));
        // Moving again (a new game): alive.
        assert!(liveness.update(3, start + Duration::from_secs(20)));
    }

    #[test]
    fn passes_the_latest_open_request_on_once() {
        let request = super::OpenRequest::default();
        assert_eq!(request.take(), None);
        request.request(false);
        request.request(true);
        assert_eq!(request.take(), Some(true));
        assert_eq!(request.take(), None);
        request.request(false);
        assert_eq!(request.take(), Some(false));
        assert_eq!(super::westplay_message(true), r#"{"open":true,"type":"westplay"}"#);
    }

    #[test]
    fn passes_on_the_companions_sign_ins_once() {
        use super::{AccountAction, AccountRequests};
        let requests = AccountRequests::default();
        assert_eq!(requests.youtube.take(), None);
        requests.youtube.request(AccountAction::SignOut);
        requests.youtube.request(AccountAction::SignIn);
        requests.spotify.request(AccountAction::Check);
        assert_eq!(requests.youtube.take(), Some(AccountAction::SignIn));
        assert_eq!(requests.youtube.take(), None);
        assert_eq!(requests.spotify.take(), Some(AccountAction::Check));
        assert_eq!(requests.spotify.take(), None);
        // The companion hears whether YouTube is signed in, and nothing more of the account.
        let status = super::Status { youtube_signed_in: Some(true), youtube_signing_in: false, ..Default::default() };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!((json["youtubeSignedIn"].as_bool(), json["youtubeSigningIn"].as_bool()), (Some(true), Some(false)));
        let status = super::Status { spotify_signed_in: Some(false), spotify_signing_in: true, ..Default::default() };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!((json["spotifySignedIn"].as_bool(), json["spotifySigningIn"].as_bool()), (Some(false), Some(true)));
        let status =
            super::Status { apple_music_signed_in: Some(true), apple_music_signing_in: true, ..Default::default() };
        let json = serde_json::to_value(&status).unwrap();
        let apple_music = (json["appleMusicSignedIn"].as_bool(), json["appleMusicSigningIn"].as_bool());
        assert_eq!(apple_music, (Some(true), Some(true)));
    }

    #[test]
    fn reports_whether_westplay_is_open_to_the_companion() {
        let status = super::Status { westplay_open: Some(false), ..Default::default() };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["westplayOpen"], serde_json::json!(false));
        let unknown = serde_json::to_value(super::Status::default()).unwrap();
        assert_eq!(unknown["westplayOpen"], serde_json::Value::Null);
    }

    #[test]
    fn tells_the_page_the_truck_strip_wallpaper_and_icons() {
        let mut settings = crate::settings::Settings::default();
        let message: serde_json::Value = serde_json::from_str(&settings_message(&settings)).unwrap();
        assert_eq!(
            message,
            serde_json::json!({
                "type": "settings",
                "truckStrip": "auto",
                "split": "off",
                "screenResolution": "high",
                "keepScreenOn": false,
                "wallpaper": null,
                "iconSet": null,
                "iconStyle": "auto",
                "equalizer": { "bass": 0, "mid": 0, "treble": 0 },
                "equalizerCustom": false,
                "characterPerson": 0,
                "freightPapers": true,
                "manualRefuel": true,
                "payAttendant": true,
                "virtualMirrors": "right",
                "suspensionAutoReset": true,
                "suspensionLowFront": "normal",
                "suspensionDamping": "normal",
                "cursorSize": "normal",
                "suspensionAuto": false,
                "suspensionAutoDelay": 3,
                "suspensionAutoReturn": 7,
                "suspensionAutoBrake": 2,
                "brakeFade": true,
                "brakeFadeLevel": "realistic",
                "brakeSquealVolume": 70,
                "brakeWarningVolume": 70,
                "brakeHud": true,
                "brakeHudCorner": "bottomLeft",
                "brakeHudSize": "small",
                "brakeHudOnFoot": false,
                "brakeSmoke": true,
                "brakeAlarm": 360,
                "audioOutput": "cabin",
            })
        );
        settings.screen.truck_strip = crate::settings::TruckStrip::Off;
        settings.sound.equalizer = crate::speakers::Equalizer { bass: -9, mid: 2, treble: 6 };
        settings.look.wallpaper = Some("custom:Dusk".into());
        settings.look.icon_set = Some("Retro".into());
        let message: serde_json::Value = serde_json::from_str(&settings_message(&settings)).unwrap();
        assert_eq!(message["truckStrip"], "off");
        assert_eq!(message["wallpaper"], "custom:Dusk");
        assert_eq!(message["iconSet"], "Retro");
        // Within the range, whatever the file says.
        assert_eq!(message["equalizer"], serde_json::json!({ "bass": -6, "mid": 2, "treble": 6 }));
        // The curve is kept whichever is chosen; what plays is it, with Custom only.
        assert_eq!(settings.sound.playing(), crate::speakers::Equalizer::default());
        settings.sound.custom = true;
        assert_eq!(settings.sound.playing(), crate::speakers::Equalizer { bass: -6, mid: 2, treble: 6 });
    }

    #[test]
    fn tells_the_page_the_players_wallpapers_and_icon_sets() {
        let customizations = crate::customize::Customizations {
            wallpapers: vec![crate::customize::Wallpaper {
                id: "custom:Dusk".into(),
                name: "Dusk".into(),
                dark: Some("wallpapers/Dusk.dark.jpg".into()),
                light: None,
            }],
            icon_sets: vec![crate::customize::IconSet {
                name: "Retro".into(),
                icons: [(
                    "youtubeMusic",
                    crate::customize::Icon { light: None, dark: Some("icons/Retro/youtube-music.dark.png".into()) },
                )]
                .into(),
            }],
        };
        let message: serde_json::Value = serde_json::from_str(&super::customize_list_message(&customizations)).unwrap();
        assert_eq!(
            message,
            serde_json::json!({
                "type": "customize",
                "wallpapers": [{ "id": "custom:Dusk", "name": "Dusk", "dark": "wallpapers/Dusk.dark.jpg", "light": null }],
                "iconSets": [{ "name": "Retro", "icons": { "youtubeMusic": { "light": null, "dark": "icons/Retro/youtube-music.dark.png" } } }],
            })
        );
    }

    #[test]
    fn player_messages_list_the_owned_garages_and_a_blocked_map() {
        let player = crate::player::Player {
            garages: westplay_mapdata::save::Garages {
                owned: ["calais".to_owned(), "dover".to_owned()].into(),
                headquarters: Some("calais".into()),
            },
            hvt_map: true,
            language: Some("tr_tr".into()),
            time_zones: true,
            money: None,
            loans: Vec::new(),
            save_path: None,
        };
        assert_eq!(
            super::player_messages_for(&player),
            [
                r#"{"headquarters":"calais","owned":["calais","dover"],"type":"garages"}"#,
                r#"{"hvt":true,"language":"tr_tr","type":"mapBlock"}"#,
                r#"{"on":true,"type":"timeZones"}"#
            ]
        );
    }

    #[test]
    fn weather_messages_say_the_weather_there_is_and_coming() {
        use crate::weather::{Sky, Weather};
        let turning = Weather { change_in: Some(102), wetness: Some(0), ..Weather::sky(Sky::Clear, Sky::Rain) };
        let message: serde_json::Value = serde_json::from_str(&super::weather_message_for(Some(turning))).unwrap();
        assert_eq!(
            message,
            serde_json::json!({
                "type": "weather",
                "weather": { "now": "clear", "coming": "rain", "changeIn": 102, "wetness": 0 }
            })
        );
        let unknown: serde_json::Value =
            serde_json::from_str(&super::weather_message_for(Some(Weather::sky(Sky::Rain, Sky::Rain)))).unwrap();
        assert_eq!(unknown["weather"]["changeIn"], serde_json::Value::Null);
        assert_eq!(super::weather_message_for(None), r#"{"type":"weather","weather":null}"#);
    }

    #[test]
    fn route_messages_carry_the_nodes_and_the_progress() {
        use crate::route::{RouteNode, RouteUpdate};
        use westplay_mapdata::junctions::{Maneuver, Turn};
        let node = RouteNode { position: [1.0, 2.0], forward: [0.0, -1.0], uid: 7, distance_left: 1500.0 };
        assert_eq!(
            super::route_message_for(&RouteUpdate::Route { id: 5, nodes: vec![node], waypoints: vec![] }),
            r#"{"id":5,"nodes":[[1.0,2.0,0.0,-1.0,1500.0]],"type":"route"}"#
        );
        assert_eq!(super::route_message_for(&RouteUpdate::Next(3)), r#"{"next":3,"type":"routeNext"}"#);
        assert_eq!(
            super::route_message_for(&RouteUpdate::Paths {
                id: 5,
                paths: vec![(2, vec![[1.0, 2.0], [3.0, 4.0]])],
                maneuvers: vec![
                    (2, Maneuver { turn: Turn::Roundabout(3), angle: -89.6 }),
                    (7, Maneuver { turn: Turn::ExitRight, angle: 17.2 }),
                ],
                shifts: vec![(3, 6.5)],
            }),
            r#"{"id":5,"maneuvers":[[2,"roundabout",-90.0,3],[7,"exit-right",17.0,null]],"paths":[[2,[[1.0,2.0],[3.0,4.0]]]],"shifts":[[3,6.5]],"type":"routePaths"}"#
        );
        assert_eq!(super::route_message_for(&RouteUpdate::None), r#"{"id":null,"nodes":null,"type":"route"}"#);
    }

    #[test]
    fn screen_message_names_the_layout() {
        assert_eq!(screen_message(0), r#"{"layout":null,"type":"screen"}"#);
        assert_eq!(screen_message(1), r#"{"layout":"generic","type":"screen"}"#);
        assert_eq!(screen_message(999), r#"{"layout":null,"type":"screen"}"#);
    }
}
