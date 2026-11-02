//! The player's phone as a second screen (owner 2026-11-01: "control it from my phone too, on both at once"):
//! a small web server on the PC's own network, at [`PORT`], serving the phone page (`phone.html`, from the
//! cabin's own UI folder) and the player's maps, and a WebSocket the phone page talks to the host on as the cab
//! page does through WebView2's (the phone page puts a stand-in for `chrome.webview` over it).
//!
//! The phone hears what the cab hears ([`Remote::broadcast`], every message the host posts to the cab page),
//! less what is the cab's alone (its screen, cursor, keyboard and keybinds: [`CAB_ONLY`]). It's the cab's panel
//! in the player's hand (owner 2026-11-01: "what I change on the phone changes on the truck's panel"), so it may
//! say what the cab's pages say to use it ([`Remote::take`]): that it's ready (the host sends the state again,
//! as for the cab page loading), the music apps' questions and controls ([`COMMANDS`]), the Maps app's route
//! requests, and a volume it set, for the cab to show. Not signing in or out, nor YouTube's videos (the phone has
//! no YouTube app), nor placing a player on the cab's screen. Its questions' ids are its own (`firstRequestId` in
//! ui/src/shared/host.ts), so each page takes only the answers to its own.
//!
//! Only on the PC's network, and only with the key in the phone page's address (the companion's QR code): the
//! WebSocket is refused without it. The key is kept in WestPlay's data folder, so a phone that saved the page
//! goes on working. The UI and the maps are served to anyone on the network who asks: neither is the player's
//! own data.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tungstenite::Message;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};

/// The port the phone page is served at: fixed, so the address the phone saved stays the same.
pub const PORT: u16 = 47800;

/// Host messages the phone isn't sent: the cab's screen, its cursor, its keyboard and the player's keybinds (the
/// cab page acts on those; the phone acting too would act twice), the companion's switch, and the Equalizer's
/// spectrum.
const CAB_ONLY: &[&str] = &["screen", "cursor", "typed", "typingState", "keybind", "westplay", "spectrum"];

/// The music players' commands the phone may give: their apps' questions and controls, their libraries kept.
const MUSIC_PLAYERS: &[&str] = &["spotify", "appleMusic", "youtubeMusic"];
const COMMANDS: &[&str] = &[
    "start", "ask", "play", "pause", "next", "previous", "jump", "seek", "volume", "shuffle", "repeat", "save",
    "favorite", "add", "like", "queue",
];
/// YouTube's, whose videos the phone doesn't show: what plays, paused or played on, and its volume.
const YOUTUBE: &[&str] = &["play", "pause", "seek", "volume"];
/// The Maps app's route requests, the volume the phone set (for the cab to show), the truck's commands, and Adjust
/// Screen's changes.
const PAGE_MESSAGES: &[&str] =
    &["ready", "route", "routes", "follow", "detours", "volumeShown", "truckCommand", "screenFit"];
/// Messages the host only passes on to the pages, the cab's and the phones' (mirrored): Adjust Screen's, a phone's
/// change for the cab's page, and the cab's fit for the phones (owner 2026-11-02: the screen fitted from the phone
/// when it's too small to reach Settings on).
const RELAYED: &[&str] = &["screenFit", "screenFitShown"];

/// Messages from the phones not taken yet, at most this many (a phone sending more than the host takes loses
/// the oldest).
const MOST_WAITING: usize = 64;

/// The phone server, while WestPlay runs.
pub struct Remote {
    phones: Arc<Mutex<Vec<Sender<Arc<str>>>>>,
    incoming: Arc<Mutex<VecDeque<String>>>,
    /// The phone page's address, with its key (`None`: the server couldn't start, or the PC has no network).
    pub url: Option<String>,
    /// The address as a QR code, an SVG image.
    pub qr: Option<String>,
    /// The phone in the game's page on the player's phone (owner 2026-11-01: "a QR code of the phone alone"), its
    /// address and QR code as the panel's.
    pub handset_url: Option<String>,
    pub handset_qr: Option<String>,
}

impl Remote {
    /// Starts serving: the phone page from `ui` (the cabin's built UI folder; `None`: none, so no page) and the
    /// maps from `maps`, the key kept in `data_dir`. A port in use or no network leaves it off (logged).
    pub fn start(ui: Option<PathBuf>, maps: PathBuf, data_dir: &Path) -> Self {
        let phones = Arc::new(Mutex::new(Vec::new()));
        let incoming = Arc::new(Mutex::new(VecDeque::new()));
        let off =
            |phones, incoming| Self { phones, incoming, url: None, qr: None, handset_url: None, handset_qr: None };
        let key = key(data_dir);
        let listener = match TcpListener::bind(("0.0.0.0", PORT)) {
            Ok(listener) => listener,
            Err(error) => {
                log!("[phone] couldn't serve at port {PORT}: {error}");
                return off(phones, incoming);
            }
        };
        let served = Served { ui, maps, key: key.clone(), phones: phones.clone(), incoming: incoming.clone() };
        let started = std::thread::Builder::new().name("westplay-phone".into()).spawn(move || {
            for stream in listener.incoming().flatten() {
                let served = served.clone();
                let _ = std::thread::Builder::new().name("westplay-phone-client".into()).spawn(move || {
                    if let Err(error) = served.answer(stream) {
                        log!("[phone] {error}");
                    }
                });
            }
        });
        if let Err(error) = started {
            log!("[phone] couldn't start serving: {error}");
            return off(phones, incoming);
        }
        let Some(address) = local_address() else {
            log!("[phone] serving at port {PORT}, but the PC's network address isn't known");
            return off(phones, incoming);
        };
        let url = format!("http://{address}:{PORT}/phone.html#{key}");
        log!("[phone] serving at http://{address}:{PORT}/phone.html");
        let handset_url = format!("http://{address}:{PORT}/handset.html#{key}");
        Self {
            phones,
            incoming,
            qr: qr_of(&url),
            url: Some(url),
            handset_qr: qr_of(&handset_url),
            handset_url: Some(handset_url),
        }
    }

    /// Whether a page's message is only passed on to the pages ([`RELAYED`]).
    pub fn relayed(json: &str) -> bool {
        RELAYED.contains(&message_type(json).as_str())
    }

    /// Whether a host message (JSON) is for the phones too: all but the cab's alone ([`CAB_ONLY`]).
    pub fn mirrored(json: &str) -> bool {
        !CAB_ONLY.contains(&message_type(json).as_str())
    }

    /// Sends a host message (JSON) to every phone, unless it's the cab's alone ([`CAB_ONLY`]).
    pub fn broadcast(&self, json: &str) {
        let Ok(mut phones) = self.phones.lock() else { return };
        if phones.is_empty() || CAB_ONLY.contains(&message_type(json).as_str()) {
            return;
        }
        let message: Arc<str> = Arc::from(json);
        phones.retain(|phone| phone.send(message.clone()).is_ok());
    }

    /// The messages the phones sent since last taken, those they may send ([`allowed`]) only.
    pub fn take(&self) -> Vec<String> {
        self.incoming.lock().map(|mut incoming| incoming.drain(..).collect()).unwrap_or_default()
    }
}

/// What a client thread needs: where the files are, the key, and the phones' queues.
#[derive(Clone)]
struct Served {
    ui: Option<PathBuf>,
    maps: PathBuf,
    key: String,
    phones: Arc<Mutex<Vec<Sender<Arc<str>>>>>,
    incoming: Arc<Mutex<VecDeque<String>>>,
}

impl Served {
    /// Answers one connection: a WebSocket (`/ws`), or a file.
    fn answer(&self, stream: TcpStream) -> Result<(), String> {
        stream.set_read_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
        let mut head = [0u8; 512];
        let peeked = stream.peek(&mut head).map_err(|e| e.to_string())?;
        let first = String::from_utf8_lossy(&head[..peeked]);
        let path = first.split_whitespace().nth(1).unwrap_or("/").to_owned();
        if path.starts_with("/ws") { self.phone(stream) } else { self.file(stream) }
    }

    /// A phone's WebSocket: refused without the key; then the host's messages out to it, and its own in.
    fn phone(&self, stream: TcpStream) -> Result<(), String> {
        let key = self.key.clone();
        let check = move |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
            let query = request.uri().query().unwrap_or_default();
            if query.split('&').any(|pair| pair == format!("k={key}")) {
                Ok(response)
            } else {
                let mut refused = ErrorResponse::new(Some("wrong key".into()));
                *refused.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
                Err(refused)
            }
        };
        let mut socket = tungstenite::accept_hdr(stream, check).map_err(|e| format!("a phone refused: {e}"))?;
        let peer = socket.get_ref().peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
        log!("[phone] {peer} connected");
        socket.get_ref().set_read_timeout(Some(Duration::from_millis(20))).map_err(|e| e.to_string())?;
        let (to_phone, outgoing): (Sender<Arc<str>>, Receiver<Arc<str>>) = channel();
        self.phones.lock().map_err(|_| "poisoned")?.push(to_phone);
        let result = loop {
            match outgoing.try_recv() {
                Ok(message) => {
                    if let Err(error) = socket.send(Message::text(message.to_string())) {
                        break Err(error);
                    }
                    continue;
                }
                Err(TryRecvError::Disconnected) => break Ok(()),
                Err(TryRecvError::Empty) => {}
            }
            match socket.read() {
                // How the page fares on the phone (handset/diagnostics.ts), into the log: a phone stalling can't
                // be watched from here otherwise.
                Ok(Message::Text(text)) if text.as_str().starts_with(r#"{"type":"diag""#) => {
                    let said: String = text.as_str().chars().take(600).collect();
                    log!("[phone] {peer} {said}");
                }
                Ok(Message::Text(text)) if allowed(text.as_str()) => {
                    if let Ok(mut incoming) = self.incoming.lock() {
                        if incoming.len() >= MOST_WAITING {
                            incoming.pop_front();
                        }
                        incoming.push_back(text.as_str().to_owned());
                    }
                }
                Ok(Message::Close(_)) => break Ok(()),
                Ok(_) => {}
                Err(tungstenite::Error::Io(error))
                    if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(error) => break Err(error),
            }
        };
        log!("[phone] {peer} gone{}", result.err().map(|e| format!(": {e}")).unwrap_or_default());
        Ok(())
    }

    /// A file: the phone page and its UI (`/`, `/phone.html`, `/assets/…`), or a map's (`/maps/…`).
    fn file(&self, mut stream: TcpStream) -> Result<(), String> {
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        // The rest of the request's head, read and dropped.
        let mut header = String::new();
        while reader.read_line(&mut header).map_err(|e| e.to_string())? > 2 {
            header.clear();
        }
        let mut parts = line.split_whitespace();
        let (method, target) = (parts.next().unwrap_or_default(), parts.next().unwrap_or("/"));
        let path = decode(target.split(['?', '#']).next().unwrap_or("/"));
        let found = if method != "GET" {
            None
        } else if let Some(rest) = path.strip_prefix("/maps/") {
            inside(&self.maps, rest)
        } else if let Some(ui) = &self.ui {
            let rest = path.trim_start_matches('/');
            inside(ui, if rest.is_empty() { "phone.html" } else { rest })
        } else {
            None
        };
        let body = found.and_then(|file| std::fs::read(&file).ok().map(|bytes| (file, bytes)));
        let head = match &body {
            Some((file, bytes)) => format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n\
                 Access-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                content_type(file),
                bytes.len()
            ),
            None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
        };
        stream.write_all(head.as_bytes()).map_err(|e| e.to_string())?;
        if let Some((_, bytes)) = body {
            stream.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// An address as a QR code, an SVG image, for the companion to show.
fn qr_of(url: &str) -> Option<String> {
    qrcode::QrCode::new(url.as_bytes()).ok().map(|code| {
        code.render::<qrcode::render::svg::Color>()
            .min_dimensions(240, 240)
            .dark_color(qrcode::render::svg::Color("#000000"))
            .light_color(qrcode::render::svg::Color("#ffffff"))
            .build()
    })
}

/// Whether a phone may send this ([`PAGE_MESSAGES`], [`COMMANDS`], [`YOUTUBE`]).
pub fn allowed(json: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else { return false };
    let kind = value.get("type").and_then(|kind| kind.as_str()).unwrap_or_default();
    if PAGE_MESSAGES.contains(&kind) {
        return true;
    }
    let action = value.pointer("/command/action").and_then(|action| action.as_str()).unwrap_or_default();
    (MUSIC_PLAYERS.contains(&kind) && COMMANDS.contains(&action)) || (kind == "youtube" && YOUTUBE.contains(&action))
}

/// A host message's `type`, read without parsing the rest (`""`: none found).
fn message_type(json: &str) -> String {
    let Some(at) = json.find("\"type\"") else { return String::new() };
    let rest = json[at + 6..].trim_start().trim_start_matches(':').trim_start();
    rest.strip_prefix('"').and_then(|rest| rest.split('"').next()).unwrap_or_default().to_owned()
}

/// `relative` under `root`, if it stays there (no `..`, no drive or root of its own) and is a file.
fn inside(root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    if !relative.components().all(|part| matches!(part, Component::Normal(_))) {
        return None;
    }
    let path = root.join(relative);
    path.is_file().then_some(path)
}

/// A URL path with its `%xx` escapes undone (invalid ones kept as they are).
fn decode(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |at: usize| bytes.get(at).and_then(|b| (*b as char).to_digit(16));
        match (bytes[i], hex(i + 1), hex(i + 2)) {
            (b'%', Some(high), Some(low)) => {
                out.push((high * 16 + low) as u8);
                i += 3;
            }
            (byte, _, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "pbf" => "application/x-protobuf",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// The phone page's key: kept in `data_dir` (made once, at random), so a phone's saved address goes on working.
fn key(data_dir: &Path) -> String {
    let file = data_dir.join("phone_key.txt");
    if let Ok(kept) = std::fs::read_to_string(&file)
        && kept.trim().len() >= 16
        && kept.trim().bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return kept.trim().to_owned();
    }
    use std::hash::{BuildHasher, Hasher};
    let random = |n: u64| {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u64(n ^ u64::from(std::process::id()));
        hasher.finish()
    };
    let made = format!("{:016x}{:016x}", random(1), random(2));
    let _ = std::fs::write(&file, &made);
    made
}

/// The PC's address on its network: the one Windows would send from to the internet (nothing is sent).
fn local_address() -> Option<std::net::IpAddr> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("8.8.8.8", 80)).ok()?;
    socket.local_addr().ok().map(|address| address.ip()).filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_phone_may_use_the_music_apps_and_maps_but_not_sign_in_nor_youtubes_videos() {
        assert!(allowed(r#"{"type":"ready"}"#));
        assert!(allowed(r#"{"type":"spotify","command":{"action":"next"}}"#));
        assert!(allowed(r#"{"type":"youtubeMusic","command":{"action":"volume","percent":40}}"#));
        assert!(allowed(r#"{"type":"appleMusic","command":{"action":"ask","id":1000000001}}"#));
        assert!(allowed(r#"{"type":"route","id":1000000001,"action":"end"}"#));
        assert!(allowed(r#"{"type":"volumeShown","volume":0.4}"#));
        assert!(allowed(r#"{"type":"youtube","command":{"action":"pause"}}"#));
        assert!(!allowed(r#"{"type":"spotify","command":{"action":"signOut"}}"#));
        assert!(!allowed(r#"{"type":"youtubeMusic","command":{"action":"show","x":0}}"#));
        assert!(!allowed(r#"{"type":"youtube","command":{"action":"load","video":"x"}}"#));
        assert!(!allowed(r#"{"type":"youtube","command":{"action":"signIn"}}"#));
        assert!(!allowed(r#"{"type":"wallpaper","id":"x"}"#));
        assert!(!allowed(r#"{"type":"typing","selection":1}"#));
        assert!(!allowed("not json"));
    }

    #[test]
    fn reads_a_messages_type() {
        assert_eq!(message_type(r#"{"type":"keybind","action":"volumeUp"}"#), "keybind");
        assert_eq!(message_type(r#"{ "type" : "telemetry", "data": {}}"#), "telemetry");
        assert_eq!(message_type(r#"{"data":1}"#), "");
    }

    #[test]
    fn a_phone_may_fit_the_cabs_screen_and_only_the_cab_tells_its_fit() {
        let change = r#"{"type":"screenFit","change":{"top":0.05}}"#;
        assert!(allowed(change));
        assert!(Remote::relayed(change));
        let shown = r#"{"type":"screenFitShown","key":null,"fit":{}}"#;
        assert!(!allowed(shown));
        assert!(Remote::relayed(shown));
        assert!(Remote::mirrored(shown));
        assert!(!Remote::relayed(r#"{"type":"truckCommand","command":"horn","down":true}"#));
    }

    #[test]
    fn serves_only_files_inside_its_folder() {
        let root = std::env::temp_dir().join(format!("westplay-phone-{}", std::process::id()));
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("assets").join("a.js"), "x").unwrap();
        assert!(inside(&root, "assets/a.js").is_some());
        assert!(inside(&root, "assets/missing.js").is_none());
        assert!(inside(&root, "../secret.txt").is_none());
        assert!(inside(&root, "assets/../../secret.txt").is_none());
        assert!(inside(&root, "C:/Windows/win.ini").is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn undoes_a_paths_escapes() {
        assert_eq!(decode("/maps/usa%20map/map.json"), "/maps/usa map/map.json");
        assert_eq!(decode("/a%zz"), "/a%zz");
    }

    #[test]
    fn keeps_its_key() {
        let dir = std::env::temp_dir().join(format!("westplay-phone-key-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = key(&dir);
        assert_eq!(first.len(), 32);
        assert_eq!(key(&dir), first);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
