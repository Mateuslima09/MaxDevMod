//! TikTok in the phone in the game (owner 2026-11-11: "the person can interact with the app as in TikTok"):
//! TikTok's own site in a WebView of its own, on a WebView2 profile of its own, shown under the phone's page
//! where its TikTok app cuts it open, and used with the mouse as a browser's page (its clicks and wheel are the
//! phone's pointer's, passed on while it's over it). Signed in as YouTube and Spotify are: TikTok's own sign-in
//! page in a window on the desktop (signin.rs), on this profile. WestPlay only knows whether it's signed in, and
//! the account's name.
//!
//! The name is for the phone's notifications (owner 2026-11-11: "Mateus followed you", gifts, as on GTA RP's
//! phones, for who streams on TikTok): the player's own LIVE page is opened hidden, in a second WebView on the
//! same profile, and what its room says (tiktok_live.js: follows, gifts, shares, subscriptions) comes out as
//! [`Report::Event`]s. A room not on is looked for again every so often.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::time::{Duration, Instant};

use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_MOUSE_EVENT_KIND, COREWEBVIEW2_MOUSE_EVENT_KIND_LEAVE, COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS,
    COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_NONE,
};
use windows::Win32::Foundation::POINT;

use crate::signin::{self, Confirming, Progress, SignInWindow};
use crate::webview::{self, Layers, OffscreenWebView, Stage};

/// TikTok's For You feed: where it opens.
pub const HOME: &str = "https://www.tiktok.com/foryou";

/// How wide TikTok's page is laid out (its narrowest that still shows a whole video), shrunk evenly into the
/// phone's screen; and its height until the phone says where it shows.
const PAGE_WIDTH: u32 = 420;
const PAGE_HEIGHT: u32 = 760;

/// How long after its page loaded a LIVE not on is looked for again, and how long a LIVE that says nothing is
/// still taken as on.
const LIVE_LOOK_AGAIN: Duration = Duration::from_secs(90);
const LIVE_QUIET: Duration = Duration::from_secs(120);

/// What the phone's TikTok app (and the companion's Accounts) asks.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum Command {
    /// Run it (hidden), so whether it's signed in is known.
    Start,
    SignIn,
    SignOut,
    /// Where it shows under the phone's page, in the phone's pixels.
    Show {
        x: f32,
        y: f32,
        width: u32,
        height: u32,
    },
    Hide,
    /// One of TikTok's pages (the phone's own tabs).
    Go {
        page: Page,
    },
    /// The page before.
    Back,
    /// Its videos' volume a step up (1) or down (-1): the phone's side keys (owner 2026-11-21).
    Volume {
        step: i8,
    },
    /// Whether the player's LIVE is watched for the phone's notifications (the phone's Settings).
    Notifications {
        on: bool,
    },
    /// A made-up follow, to see and hear a notification without a LIVE (the phone's Settings).
    TestNotification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Page {
    ForYou,
    Following,
    Explore,
    Live,
    Profile,
}

impl Page {
    /// Its address; the profile is the account's own (`user`), TikTok's sign-in without one.
    fn address(self, user: Option<&str>) -> String {
        match (self, user) {
            (Self::ForYou, _) => HOME.to_owned(),
            (Self::Following, _) => "https://www.tiktok.com/following".to_owned(),
            (Self::Explore, _) => "https://www.tiktok.com/explore".to_owned(),
            (Self::Live, _) => "https://www.tiktok.com/live".to_owned(),
            (Self::Profile, Some(user)) => format!("https://www.tiktok.com/@{user}"),
            (Self::Profile, None) => "https://www.tiktok.com/profile".to_owned(),
        }
    }
}

/// Whether it's signed in, and the account's name (its @, without it).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub signed_in: bool,
    pub user: Option<String>,
}

/// What happened on the player's LIVE, for a notification.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Event {
    pub kind: Kind,
    /// Who did it, as TikTok shows them.
    pub user: String,
    /// A gift's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// How many of the gift.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Follow,
    Gift,
    Share,
    Subscribe,
}

#[derive(Debug, PartialEq)]
pub enum Report {
    State(State),
    /// The sign-in window opened (`true`) or closed.
    SignInWindow(bool),
    /// The sign-in window closed signed in, and TikTok's page is yet to say it's signed in (`true`), or it has
    /// (or took too long: `false`).
    Confirming(bool),
    /// The player's LIVE is on (its room talks), or no longer.
    Live(bool),
    Event(Event),
    /// A field of TikTok's page took the keyboard (`true`), or let it go.
    Typing(bool),
}

/// Where TikTok shows in the phone, in the phone's pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Place {
    x: f32,
    y: f32,
    width: u32,
    height: u32,
}

impl Place {
    /// How much smaller than laid out the page shows.
    fn scale(self) -> f32 {
        (self.width as f32 / PAGE_WIDTH as f32).min(1.0)
    }

    /// The phone's `point` in the page's own pixels, if it's over it.
    fn inside(self, point: POINT) -> Option<POINT> {
        let (x, y) = (point.x as f32 - self.x, point.y as f32 - self.y);
        if x < 0.0 || y < 0.0 || x >= self.width as f32 || y >= self.height as f32 {
            return None;
        }
        let scale = self.scale();
        Some(POINT { x: (x / scale) as i32, y: (y / scale) as i32 })
    }
}

pub struct TikTok {
    view: OffscreenWebView,
    sign_in: RefCell<Option<SignInWindow>>,
    /// Once it closed signed in, until the page says it's signed in.
    confirming: Confirming,
    pending: RefCell<Vec<Report>>,
    state: RefCell<Option<State>>,
    /// Where the phone's TikTok app shows it (`None`: not open).
    place: Cell<Option<Place>>,
    /// Whether the phone is out (in the hand, not put away or only peeking).
    out: Cell<bool>,
    /// Whether its videos play, as the page was last told.
    sounding: Cell<bool>,
    /// Whether the mouse was last over it.
    hovered: Cell<bool>,
    /// Whether a field of its page has the keyboard.
    typing: Cell<bool>,
    /// The player's LIVE watched for notifications: wanted, its page, whose it is, when it loaded, when its room
    /// last talked, and whether it's taken as on.
    notifications: Cell<bool>,
    live: RefCell<Option<OffscreenWebView>>,
    live_user: RefCell<Option<String>>,
    live_loaded: Cell<Option<Instant>>,
    live_heard: Cell<Option<Instant>>,
    live_on: Cell<bool>,
}

impl TikTok {
    /// Opens TikTok in a WebView of the phone's picture (`layers`), hidden under the phone's page and quiet until
    /// the phone's TikTok app shows it.
    pub fn start(stage: &Stage, layers: &Layers, user_data: &Path) -> Result<Self, webview::Error> {
        let view = OffscreenWebView::new(stage, PAGE_WIDTH, PAGE_HEIGHT, user_data)?;
        view.log_as("tiktok")?;
        view.show(false)?;
        view.set_muted(true)?;
        // Black until TikTok's page has drawn: never the game's picture through the phone.
        view.set_background(0, 0, 0)?;
        layers.add(&view, false)?;
        view.add_startup_script(include_str!("tiktok.js"))?;
        view.navigate(HOME)?;
        Ok(Self {
            view,
            sign_in: RefCell::new(None),
            confirming: Confirming::default(),
            pending: RefCell::new(Vec::new()),
            state: RefCell::new(None),
            place: Cell::new(None),
            out: Cell::new(false),
            sounding: Cell::new(false),
            hovered: Cell::new(false),
            typing: Cell::new(false),
            notifications: Cell::new(true),
            live: RefCell::new(None),
            live_user: RefCell::new(None),
            live_loaded: Cell::new(None),
            live_heard: Cell::new(None),
            live_on: Cell::new(false),
        })
    }

    pub fn command(&self, command: &Command) -> windows::core::Result<()> {
        match command {
            Command::Start => Ok(()),
            Command::SignIn => {
                self.open_sign_in();
                Ok(())
            }
            Command::SignOut => {
                self.view.delete_cookies()?;
                self.close_live();
                self.view.navigate(HOME)
            }
            Command::Show { x, y, width, height } => {
                // Only a place in the phone, and one worth showing in.
                let (phone_width, phone_height) = (westplay_shared::PHONE_WIDTH, westplay_shared::PHONE_HEIGHT);
                let fits = x.is_finite()
                    && y.is_finite()
                    && *x >= 0.0
                    && *y >= 0.0
                    && (100..=phone_width).contains(width)
                    && (100..=phone_height).contains(height)
                    && *x + *width as f32 <= phone_width as f32
                    && *y + *height as f32 <= phone_height as f32;
                if !fits {
                    return Ok(());
                }
                let place = Place { x: *x, y: *y, width: *width, height: *height };
                if self.place.get() != Some(place) {
                    self.place.set(Some(place));
                    self.view.fit((PAGE_WIDTH, u32::MAX / 4), place.x, place.y, place.width, place.height)?;
                    self.view.show(true)?;
                }
                self.sound()
            }
            Command::Hide => {
                self.place.set(None);
                self.view.show(false)?;
                self.sound()
            }
            Command::Go { page } => {
                let state = self.state.borrow();
                let user = state.as_ref().and_then(|state| state.user.as_deref());
                self.view.navigate(&page.address(user))
            }
            Command::Back => self.view.post_json(r#"{"action":"back"}"#),
            Command::Volume { step } => {
                let step = step.signum();
                self.view.post_json(&serde_json::json!({ "action": "volume", "step": step }).to_string())
            }
            Command::Notifications { on } => {
                self.notifications.set(*on);
                Ok(())
            }
            Command::TestNotification => {
                self.pending.borrow_mut().push(Report::Event(Event {
                    kind: Kind::Follow,
                    user: "Mateus".to_owned(),
                    detail: None,
                    count: None,
                }));
                Ok(())
            }
        }
    }

    /// Whether the phone is out: TikTok plays only in a phone in the hand, open on it.
    pub fn out(&self, out: bool) -> windows::core::Result<()> {
        self.out.set(out);
        self.sound()
    }

    /// Its videos playing (and heard) while it shows in a phone that's out, held paused otherwise.
    fn sound(&self) -> windows::core::Result<()> {
        let wanted = self.out.get() && self.place.get().is_some();
        if wanted != self.sounding.get() {
            self.sounding.set(wanted);
            self.view.set_muted(!wanted)?;
            self.tell_sound()?;
        }
        Ok(())
    }

    fn tell_sound(&self) -> windows::core::Result<()> {
        let on = self.sounding.get();
        self.view.post_json(&serde_json::json!({ "action": "sound", "on": on }).to_string())
    }

    /// A mouse event of the phone's pointer, at `point` in the phone's pixels: TikTok's if it's over it (`true`),
    /// in its page's own pixels. Leaving it, its page is told the mouse left.
    pub fn mouse(
        &self,
        kind: COREWEBVIEW2_MOUSE_EVENT_KIND,
        keys: COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS,
        data: u32,
        point: POINT,
    ) -> bool {
        let inside =
            self.place.get().filter(|_| kind != COREWEBVIEW2_MOUSE_EVENT_KIND_LEAVE).and_then(|p| p.inside(point));
        match inside {
            Some(inside) => {
                self.hovered.set(true);
                let _ = self.view.send_mouse(kind, keys, data, inside);
                true
            }
            None => {
                if self.hovered.replace(false) {
                    let none = COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS_NONE;
                    let _ = self.view.send_mouse(COREWEBVIEW2_MOUSE_EVENT_KIND_LEAVE, none, 0, POINT::default());
                }
                false
            }
        }
    }

    /// Whether a field of its page has the keyboard, in a phone that's out and open on it.
    pub fn typing(&self) -> bool {
        self.typing.get() && self.sounding.get()
    }

    /// What the player typed (`typing::typed_message`), into the field that has the keyboard.
    pub fn typed(&self, message: &str) -> windows::core::Result<()> {
        self.view.post_json(message)
    }

    /// Whether the player's LIVE is taken as on.
    pub fn live(&self) -> bool {
        self.live_on.get()
    }

    /// TikTok's sign-in window, opened (or brought forward if it is).
    fn open_sign_in(&self) {
        let mut sign_in = self.sign_in.borrow_mut();
        if let Some(window) = sign_in.as_ref() {
            window.bring_forward();
            return;
        }
        match SignInWindow::open(self.view.environment(), &signin::TIKTOK) {
            Ok(window) => {
                *sign_in = Some(window);
                self.pending.borrow_mut().push(Report::SignInWindow(true));
            }
            Err(e) => crate::startup::mark(&format!("host: the TikTok sign-in window couldn't open: {e}")),
        }
    }

    /// What TikTok's page and the player's LIVE said since the last call, and the sign-in window opening or
    /// closing (signed in, the page loads again, signed in). Looks after the LIVE's page too: opened once the
    /// account's name is known, looked for again while it isn't on.
    pub fn take_reports(&self, stage: &Stage) -> Vec<Report> {
        let mut reports = std::mem::take(&mut *self.pending.borrow_mut());
        let progress = self.sign_in.borrow().as_ref().map(SignInWindow::progress);
        if let Some(progress @ (Progress::SignedIn | Progress::Closed)) = progress {
            self.sign_in.borrow_mut().take();
            reports.push(Report::SignInWindow(false));
            if progress == Progress::SignedIn {
                let _ = self.view.navigate(HOME);
                self.confirming.start();
                reports.push(Report::Confirming(true));
            }
        }
        for message in self.view.take_page_messages() {
            match read_message(&message) {
                // A page loaded anew: quiet or playing, as it was.
                Some(Message::Ready) => {
                    let _ = self.tell_sound();
                    if self.typing.replace(false) {
                        reports.push(Report::Typing(false));
                    }
                }
                Some(Message::State(state)) => {
                    *self.state.borrow_mut() = Some(state.clone());
                    reports.push(Report::State(state));
                }
                Some(Message::Typing(typing)) => {
                    if self.typing.replace(typing) != typing {
                        reports.push(Report::Typing(typing));
                    }
                }
                _ => {}
            }
        }
        self.watch_live(stage, &mut reports);
        let signed_in = reports.iter().rev().find_map(|report| match report {
            Report::State(state) => Some(state.signed_in),
            _ => None,
        });
        if self.confirming.ended(signed_in) {
            reports.push(Report::Confirming(false));
        }
        reports
    }

    /// The player's LIVE page: open while notifications are wanted and the account's name is known, loaded
    /// again every so often while its room says nothing (not on yet, or over); what its room said, reported.
    fn watch_live(&self, stage: &Stage, reports: &mut Vec<Report>) {
        let user = self
            .state
            .borrow()
            .as_ref()
            .filter(|state| state.signed_in && self.notifications.get())
            .and_then(|state| state.user.clone());
        let Some(user) = user else {
            if self.close_live() {
                reports.push(Report::Live(false));
            }
            return;
        };
        let now = Instant::now();
        if self.live_user.borrow().as_deref() != Some(user.as_str()) {
            if self.close_live() {
                reports.push(Report::Live(false));
            }
            *self.live_user.borrow_mut() = Some(user.clone());
        }
        let address = format!("https://www.tiktok.com/@{user}/live");
        if self.live.borrow().is_none() {
            let opened = (|| -> Result<OffscreenWebView, webview::Error> {
                let live = self.view.beside(stage, PAGE_WIDTH, PAGE_HEIGHT)?;
                live.set_muted(true)?;
                live.add_startup_script(include_str!("tiktok_live.js"))?;
                live.navigate(&address)?;
                Ok(live)
            })();
            match opened {
                Ok(live) => {
                    log!("[tiktok] the account's LIVE is watched for notifications");
                    *self.live.borrow_mut() = Some(live);
                }
                Err(e) => log!("[tiktok] the LIVE's page couldn't open: {e}"),
            }
            // Tried again no sooner than a LIVE not on is looked for.
            self.live_loaded.set(Some(now));
            return;
        }
        let live = self.live.borrow();
        let Some(live) = live.as_ref() else { return };
        for message in live.take_page_messages() {
            match read_message(&message) {
                Some(Message::Live) => {
                    self.live_heard.set(Some(now));
                    if !self.live_on.replace(true) {
                        log!("[tiktok] the LIVE is on");
                        reports.push(Report::Live(true));
                    }
                }
                Some(Message::Event(event)) => {
                    self.live_heard.set(Some(now));
                    reports.push(Report::Event(event));
                }
                _ => {}
            }
        }
        let quiet = self.live_heard.get().is_none_or(|heard| now.duration_since(heard) >= LIVE_QUIET);
        if quiet && self.live_on.replace(false) {
            log!("[tiktok] the LIVE says nothing: taken as over");
            reports.push(Report::Live(false));
        }
        let loaded_long_ago = self.live_loaded.get().is_none_or(|loaded| now.duration_since(loaded) >= LIVE_LOOK_AGAIN);
        if quiet && loaded_long_ago {
            let _ = live.navigate(&address);
            self.live_loaded.set(Some(now));
        }
    }

    /// Closes the LIVE's page; whether a LIVE was taken as on until now.
    fn close_live(&self) -> bool {
        self.live.borrow_mut().take();
        self.live_user.borrow_mut().take();
        self.live_loaded.set(None);
        self.live_heard.set(None);
        self.live_on.replace(false)
    }

    /// Whether the WebView failed since the last call (it's then closed and started again when needed).
    pub fn failed(&self) -> bool {
        self.view.take_failure().is_some()
    }

    /// Takes the WebView out of the phone's picture; it closes as it's dropped.
    pub fn close(self, layers: &Layers) {
        let _ = layers.remove(&self.view);
    }
}

/// A message of TikTok's page (`tiktok.js`) or of the player's LIVE's (`tiktok_live.js`).
#[derive(Debug, PartialEq)]
enum Message {
    Ready,
    State(State),
    Typing(bool),
    /// The LIVE's room talks.
    Live,
    Event(Event),
}

/// The longest name or gift's name told on.
const NAME_MOST: usize = 60;

fn read_message(message: &str) -> Option<Message> {
    let value: serde_json::Value = serde_json::from_str(message).ok()?;
    if value.get("ready").and_then(serde_json::Value::as_bool) == Some(true) {
        return Some(Message::Ready);
    }
    if value.get("live").and_then(serde_json::Value::as_bool) == Some(true) {
        return Some(Message::Live);
    }
    if let Some(typing) = value.get("typing").and_then(serde_json::Value::as_bool) {
        return Some(Message::Typing(typing));
    }
    if let Some(state) = value.get("state") {
        let signed_in = state.get("signedIn").and_then(serde_json::Value::as_bool)?;
        let user = state.get("user").and_then(serde_json::Value::as_str).filter(|user| is_user_name(user));
        return Some(Message::State(State { signed_in, user: user.filter(|_| signed_in).map(str::to_owned) }));
    }
    if let Some(event) = value.get("event") {
        let mut event: Event = serde_json::from_value(event.clone()).ok()?;
        event.user = one_line(&event.user)?;
        event.detail = event.detail.as_deref().and_then(one_line);
        event.count = event.count.filter(|&count| count > 1).map(|count| count.min(9999));
        return Some(Message::Event(event));
    }
    None
}

/// Whether `name` is a TikTok account's name (its @): letters, digits, `.` and `_`, as its addresses take it.
fn is_user_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 40 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
}

/// A name as it's told on: one line, nothing unprintable, not long; `None` for nothing left.
fn one_line(name: &str) -> Option<String> {
    let line: String = name.chars().filter(|c| !c.is_control()).take(NAME_MOST).collect();
    let line = line.trim();
    (!line.is_empty()).then(|| line.to_owned())
}

/// `{"type":"tiktok","signedIn":true,"user":"mateus","signingIn":false,"live":false}`: what the phone's TikTok
/// app shows of the account (`signedIn` `null`: not known yet).
pub fn state_message(state: Option<&State>, signing_in: bool, live: bool) -> String {
    serde_json::json!({
        "type": "tiktok",
        "signedIn": state.map(|state| state.signed_in),
        "user": state.and_then(|state| state.user.as_deref()),
        "signingIn": signing_in,
        "live": live,
    })
    .to_string()
}

/// `{"type":"phoneNotification","app":"tiktok","kind":"follow","user":"Mateus"}`: a notification for the phone
/// to show and sound (a gift's with its name and count).
pub fn notification_message(event: &Event) -> String {
    let mut message = serde_json::to_value(event).unwrap_or_default();
    message["type"] = "phoneNotification".into();
    message["app"] = "tiktok".into();
    message.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_phones_commands() {
        let read = |json: &str| serde_json::from_str::<Command>(json).ok();
        assert_eq!(
            read(r#"{"action":"show","x":7,"y":57,"width":326,"height":592}"#),
            Some(Command::Show { x: 7.0, y: 57.0, width: 326, height: 592 })
        );
        assert_eq!(read(r#"{"action":"go","page":"forYou"}"#), Some(Command::Go { page: Page::ForYou }));
        assert_eq!(read(r#"{"action":"notifications","on":false}"#), Some(Command::Notifications { on: false }));
        assert_eq!(read(r#"{"action":"testNotification"}"#), Some(Command::TestNotification));
        assert_eq!(read(r#"{"action":"volume","step":-1}"#), Some(Command::Volume { step: -1 }));
        assert_eq!(read(r#"{"action":"go","page":"https://evil.example/"}"#), None);
    }

    #[test]
    fn goes_to_tiktoks_own_pages_only() {
        for page in [Page::ForYou, Page::Following, Page::Explore, Page::Live, Page::Profile] {
            assert!(page.address(Some("mateus")).starts_with("https://www.tiktok.com/"));
            assert!(page.address(None).starts_with("https://www.tiktok.com/"));
        }
        assert_eq!(Page::Profile.address(Some("mateus.mtb_l2")), "https://www.tiktok.com/@mateus.mtb_l2");
    }

    /// The phone's pointer over TikTok is in its page's own pixels: the page is laid out wider than it shows.
    #[test]
    fn passes_the_mouse_in_the_pages_own_pixels() {
        let place = Place { x: 7.0, y: 57.0, width: 210, height: 380 };
        assert_eq!(place.inside(POINT { x: 7, y: 57 }), Some(POINT { x: 0, y: 0 }));
        assert_eq!(place.inside(POINT { x: 112, y: 247 }), Some(POINT { x: 210, y: 380 }));
        assert_eq!(place.inside(POINT { x: 6, y: 100 }), None);
        assert_eq!(place.inside(POINT { x: 217, y: 100 }), None);
        assert_eq!(place.inside(POINT { x: 100, y: 437 }), None);
    }

    #[test]
    fn hears_whether_its_signed_in_and_as_whom() {
        assert_eq!(
            read_message(r#"{"state":{"signedIn":true,"user":"mateus.mtb"}}"#),
            Some(Message::State(State { signed_in: true, user: Some("mateus.mtb".into()) }))
        );
        assert_eq!(
            read_message(r#"{"state":{"signedIn":false,"user":null}}"#),
            Some(Message::State(State { signed_in: false, user: None }))
        );
        // A name that isn't an account's is no name: no address is made of it.
        assert_eq!(
            read_message(r#"{"state":{"signedIn":true,"user":"x/../live?y"}}"#),
            Some(Message::State(State { signed_in: true, user: None }))
        );
        assert_eq!(read_message(r#"{"typing":true}"#), Some(Message::Typing(true)));
        assert_eq!(read_message(r#"{"ready":true}"#), Some(Message::Ready));
        assert_eq!(read_message("not json"), None);
    }

    #[test]
    fn tells_a_lives_follows_and_gifts_as_notifications() {
        let Some(Message::Event(follow)) = read_message(r#"{"event":{"kind":"follow","user":" Mateus\n"}}"#) else {
            panic!("a follow")
        };
        assert_eq!(follow, Event { kind: Kind::Follow, user: "Mateus".into(), detail: None, count: None });
        assert_eq!(
            notification_message(&follow),
            r#"{"app":"tiktok","kind":"follow","type":"phoneNotification","user":"Mateus"}"#
        );
        let gift = r#"{"event":{"kind":"gift","user":"Ana","detail":"Rose","count":5}}"#;
        let Some(Message::Event(gift)) = read_message(gift) else { panic!("a gift") };
        assert_eq!(gift.detail.as_deref(), Some("Rose"));
        assert_eq!(gift.count, Some(5));
        // No one, or a kind the phone doesn't tell: nothing.
        assert_eq!(read_message(r#"{"event":{"kind":"follow","user":"  "}}"#), None);
        assert_eq!(read_message(r#"{"event":{"kind":"like","user":"Ana"}}"#), None);
        assert_eq!(read_message(r#"{"live":true}"#), Some(Message::Live));
    }

    #[test]
    fn says_the_account_to_the_phone() {
        let state = State { signed_in: true, user: Some("mateus".into()) };
        assert_eq!(
            state_message(Some(&state), false, true),
            r#"{"live":true,"signedIn":true,"signingIn":false,"type":"tiktok","user":"mateus"}"#
        );
        assert_eq!(
            state_message(None, true, false),
            r#"{"live":false,"signedIn":null,"signingIn":true,"type":"tiktok","user":null}"#
        );
    }
}
