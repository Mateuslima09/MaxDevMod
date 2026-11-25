//! Host bridge for the phone's live Job Market (Bolsa de Fretes).
//!
//! Sends `Refresh` and `Accept` RPCs to the game plugin over [`westplay_shared::JobMarketBlock`],
//! reads `Documents\WestPlay\job_market.json` once the plugin snapshots live offers, and enriches
//! the payload with localized city names, localized cargo names (from `locale.scs`), and distance
//! from the player's current truck position for "Perto / Nearest" sorting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use westplay_mapdata::archive::Archive;
use westplay_mapdata::area::METRES_PER_DEGREE;
use westplay_mapdata::files::{GameFiles, Mount};
use westplay_mapdata::locale::Strings;
use westplay_shared::{JOB_MARKET_FILE, JobMarketAction, JobMarketAnswer, JobMarketBlock, JobMarketMode};

/// How long the plugin has to answer before we tell the page it didn't respond.
const ANSWER_WITHIN: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
struct CityInfo {
    name: String,
    at: [f64; 2], // world [x, z] in metres
}

#[derive(Default)]
pub struct JobMarket {
    pending: Option<Pending>,
    cities_key: Option<String>,
    cities: HashMap<String, CityInfo>,
    locale_key: Option<(PathBuf, String)>,
    locale: Option<Strings>,
}

struct Pending {
    request: u32,
    id: u32,
    action: JobMarketAction,
    sent: Instant,
}

impl JobMarket {
    /// Asks the plugin to snapshot all live Freight / Cargo / Quick Job offers.
    pub fn ask_refresh(&mut self, block: &JobMarketBlock, id: u32, now: Instant) -> Option<String> {
        let overtaken = self.pending.take().map(|p| timeout_message(p.id, p.action));
        block.action.store(JobMarketAction::Refresh as u32, Ordering::Relaxed);
        block.mode.store(0, Ordering::Relaxed);
        block.company_ptr.store(0, Ordering::Relaxed);
        block.offer_ptr.store(0, Ordering::Relaxed);
        let request = block.request.load(Ordering::Relaxed).wrapping_add(1);
        block.request.store(request, Ordering::Release);
        self.pending = Some(Pending { request, id, action: JobMarketAction::Refresh, sent: now });
        overtaken
    }

    /// Asks the plugin to accept a specific offer directly in memory on the game's main thread.
    pub fn ask_accept(
        &mut self,
        block: &JobMarketBlock,
        id: u32,
        mode: &str,
        company_ptr: &str,
        offer_ptr: &str,
        now: Instant,
    ) -> Result<Option<String>, String> {
        let Some(mode) = JobMarketMode::from_name(mode) else {
            return Err(answer_message(id, "failed"));
        };
        let (Some(cptr), Some(optr)) = (parse_hex_u64(company_ptr), parse_hex_u64(offer_ptr)) else {
            return Err(answer_message(id, "offerExpired"));
        };
        let overtaken = self.pending.take().map(|p| timeout_message(p.id, p.action));
        block.action.store(JobMarketAction::Accept as u32, Ordering::Relaxed);
        block.mode.store(mode as u32, Ordering::Relaxed);
        block.company_ptr.store(cptr, Ordering::Relaxed);
        block.offer_ptr.store(optr, Ordering::Relaxed);
        let request = block.request.load(Ordering::Relaxed).wrapping_add(1);
        block.request.store(request, Ordering::Release);
        self.pending = Some(Pending { request, id, action: JobMarketAction::Accept, sent: now });
        Ok(overtaken)
    }

    /// Asks the plugin to cancel the player's currently active job (the game's own `cancel_player_job`).
    pub fn ask_cancel(&mut self, block: &JobMarketBlock, id: u32, now: Instant) -> Option<String> {
        let overtaken = self.pending.take().map(|p| timeout_message(p.id, p.action));
        block.action.store(JobMarketAction::Cancel as u32, Ordering::Relaxed);
        block.mode.store(0, Ordering::Relaxed);
        block.company_ptr.store(0, Ordering::Relaxed);
        block.offer_ptr.store(0, Ordering::Relaxed);
        let request = block.request.load(Ordering::Relaxed).wrapping_add(1);
        block.request.store(request, Ordering::Release);
        self.pending = Some(Pending { request, id, action: JobMarketAction::Cancel, sent: now });
        overtaken
    }

    /// Polls for the plugin's answer (or timeout) and formats the JSON message to broadcast to the pages.
    #[allow(clippy::too_many_arguments)]
    pub fn poll(
        &mut self,
        block: &JobMarketBlock,
        maps_dir: &Path,
        map_folder: Option<&str>,
        game_dir: Option<&Path>,
        game_language: Option<&str>,
        truck_pos: Option<[f64; 2]>,
        now: Instant,
    ) -> Option<String> {
        let pending = self.pending.as_ref()?;
        let done = block.answered.load(Ordering::Acquire) == pending.request;
        if !done {
            if now.duration_since(pending.sent) >= ANSWER_WITHIN {
                let p = self.pending.take()?;
                return Some(timeout_message(p.id, p.action));
            }
            return None;
        }
        let p = self.pending.take()?;
        let answer = JobMarketAnswer::from_code(block.answer.load(Ordering::Relaxed));
        let code = answer.map_or("failed", JobMarketAnswer::name);

        match p.action {
            JobMarketAction::Accept | JobMarketAction::Cancel => Some(answer_message(p.id, code)),
            JobMarketAction::Refresh => {
                if answer != Some(JobMarketAnswer::Ok) {
                    return Some(json!({ "type": "jobMarketSnapshot", "id": p.id, "answer": code }).to_string());
                }
                self.ensure_cities(maps_dir, map_folder);
                self.ensure_locale(game_dir, game_language);
                match load_and_enrich_snapshot(&self.cities, self.locale.as_ref(), truck_pos) {
                    Some(data) => Some(
                        json!({ "type": "jobMarketSnapshot", "id": p.id, "answer": "ok", "data": data }).to_string(),
                    ),
                    None => Some(json!({ "type": "jobMarketSnapshot", "id": p.id, "answer": "failed" }).to_string()),
                }
            }
        }
    }

    fn ensure_cities(&mut self, maps_dir: &Path, map_folder: Option<&str>) {
        let Some(folder) = map_folder else { return };
        if self.cities_key.as_deref() == Some(folder) {
            return;
        }
        self.cities = load_cities(maps_dir, folder).unwrap_or_default();
        self.cities_key = Some(folder.to_owned());
    }

    fn ensure_locale(&mut self, game_dir: Option<&Path>, game_language: Option<&str>) {
        let Some(dir) = game_dir else { return };
        let detected;
        let lang = match game_language.filter(|l| !l.is_empty()) {
            Some(l) => l,
            None => {
                detected = westplay_setup::places::game_documents()
                    .and_then(|d| std::fs::read_to_string(d.join("config.cfg")).ok())
                    .and_then(|c| westplay_mapdata::locale::game_language(&c));
                detected.as_deref().unwrap_or("en_gb")
            }
        };
        let key = (dir.to_path_buf(), lang.to_owned());
        if self.locale_key.as_ref() == Some(&key) {
            return;
        }
        self.locale = load_locale_strings(dir, lang);
        self.locale_key = Some(key);
    }
}

fn timeout_message(id: u32, action: JobMarketAction) -> String {
    match action {
        JobMarketAction::Refresh => json!({ "type": "jobMarketSnapshot", "id": id, "answer": "noAnswer" }).to_string(),
        JobMarketAction::Accept | JobMarketAction::Cancel => answer_message(id, "noAnswer"),
    }
}

fn answer_message(id: u32, answer: &str) -> String {
    json!({ "type": "jobMarketAnswer", "id": id, "answer": answer }).to_string()
}

fn parse_hex_u64(s: &str) -> Option<u64> {
    let trimmed = s.trim();
    let hex = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")).unwrap_or(trimmed);
    u64::from_str_radix(hex, 16).ok().filter(|&v| v != 0)
}

fn snapshot_file_path() -> Option<PathBuf> {
    Some(westplay_setup::places::documents()?.join("WestPlay").join(JOB_MARKET_FILE))
}

fn load_cities(maps_dir: &Path, folder: &str) -> Option<HashMap<String, CityInfo>> {
    let map = maps_dir.join(folder);
    let manifest: Value = serde_json::from_slice(&std::fs::read(map.join("map.json")).ok()?).ok()?;
    let places_rel = manifest["places"].as_str()?;
    let places: Vec<Value> = serde_json::from_slice(&std::fs::read(map.join(places_rel)).ok()?).ok()?;
    let mut out = HashMap::new();
    for entry in places {
        let Some(city_id) = entry["cityId"].as_str() else { continue };
        let name = entry["city"]
            .as_str()
            .or_else(|| entry["localized"].as_str())
            .or_else(|| entry["name"].as_str())
            .map(str::to_owned);
        let at = entry["at"].as_array().and_then(|a| {
            let lng = a.first()?.as_f64()?;
            let lat = a.get(1)?.as_f64()?;
            Some([lng * METRES_PER_DEGREE, -lat * METRES_PER_DEGREE])
        });
        if let (Some(name), Some(at)) = (name, at) {
            let is_city = entry["kind"].as_str() == Some("city");
            if is_city || !out.contains_key(city_id) {
                out.insert(city_id.to_owned(), CityInfo { name, at });
            }
        }
    }
    Some(out)
}

fn load_locale_strings(game_dir: &Path, lang: &str) -> Option<Strings> {
    let locale_scs = game_dir.join("locale.scs");
    if !locale_scs.is_file() {
        return None;
    }
    let mount = Mount::Archive(Archive::open(&locale_scs).ok()?);
    let files = GameFiles::new(vec![mount]);
    if let Ok(s) = Strings::load(&files, lang)
        && !s.is_empty()
    {
        return Some(s);
    }
    if lang != "en_gb"
        && let Ok(s) = Strings::load(&files, "en_gb")
        && !s.is_empty()
    {
        return Some(s);
    }
    None
}

fn resolve_cargo_name(raw: &str, cargo_id: &str, locale: Option<&Strings>) -> String {
    if let Some(strings) = locale
        && let Some(resolved) = strings.resolve(raw)
        && !resolved.is_empty()
        && !resolved.starts_with("@@")
    {
        return resolved.to_owned();
    }
    if !raw.is_empty() && !raw.starts_with("@@") {
        return raw.to_owned();
    }
    // Strip `@@cn_...@@` or `@@...@@` cleanly as human-readable fallback.
    let key = raw
        .strip_prefix("@@")
        .and_then(|s| s.strip_suffix("@@"))
        .map(|k| k.strip_prefix("cn_").unwrap_or(k))
        .unwrap_or(cargo_id);
    pretty_id(if key.is_empty() { cargo_id } else { key })
}

fn pretty_id(id: &str) -> String {
    id.split('_')
        .filter(|s| !s.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn load_and_enrich_snapshot(
    cities: &HashMap<String, CityInfo>,
    locale: Option<&Strings>,
    truck_pos: Option<[f64; 2]>,
) -> Option<Value> {
    let path = snapshot_file_path()?;
    let bytes = std::fs::read(path).ok()?;
    let mut root: Value = serde_json::from_slice(&bytes).ok()?;
    let offers = root.get_mut("offers")?.as_array_mut()?;
    for offer in offers.iter_mut() {
        let Some(obj) = offer.as_object_mut() else { continue };
        let origin_city_id = obj.get("originCityId").and_then(Value::as_str).unwrap_or("").to_owned();
        let dest_city_id = obj.get("destCityId").and_then(Value::as_str).unwrap_or("").to_owned();
        let cargo_id = obj.get("cargoId").and_then(Value::as_str).unwrap_or("").to_owned();
        let raw_cargo = obj.get("cargoName").and_then(Value::as_str).unwrap_or("").to_owned();

        let origin_city = cities
            .get(&origin_city_id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| pretty_id(&origin_city_id));
        let dest_city = cities
            .get(&dest_city_id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| pretty_id(&dest_city_id));
        let cargo_name = resolve_cargo_name(&raw_cargo, &cargo_id, locale);

        obj.insert("originCity".into(), Value::String(origin_city));
        obj.insert("destCity".into(), Value::String(dest_city));
        obj.insert("cargoName".into(), Value::String(cargo_name));

        if let (Some(truck), Some(city)) = (truck_pos, cities.get(&origin_city_id)) {
            // Map metres -> approximate game-scale km (1:19 scale for ETS2/ATS highways).
            let map_m = (city.at[0] - truck[0]).hypot(city.at[1] - truck[1]);
            let km = (map_m * 19.0 / 1000.0).round() as u64;
            obj.insert("distFromPlayerKm".into(), json!(km));
        }
    }
    Some(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64};

    fn block() -> JobMarketBlock {
        JobMarketBlock {
            action: AtomicU32::new(0),
            mode: AtomicU32::new(0),
            company_ptr: AtomicU64::new(0),
            offer_ptr: AtomicU64::new(0),
            request: AtomicU32::new(0),
            answered: AtomicU32::new(0),
            answer: AtomicI32::new(0),
            snapshot_seq: AtomicU32::new(0),
        }
    }

    #[test]
    fn parses_hex_pointers_and_resolves_cargo_keys() {
        assert_eq!(parse_hex_u64("0x1a2b3c4d"), Some(0x1a2b_3c4d));
        assert_eq!(parse_hex_u64("0x0"), None);
        assert_eq!(parse_hex_u64("invalid"), None);
        assert_eq!(resolve_cargo_name("@@cn_fresh_apples@@", "apples", None), "Fresh Apples");
        assert_eq!(resolve_cargo_name("Apples", "apples", None), "Apples");
        let strings: Strings = [("cn_apples".to_owned(), "Maçãs".to_owned())].into_iter().collect();
        assert_eq!(resolve_cargo_name("@@cn_apples@@", "apples", Some(&strings)), "Maçãs");
    }

    #[test]
    fn accept_rpc_sends_and_returns_plugin_answer() {
        let b = block();
        let mut jm = JobMarket::default();
        let now = Instant::now();
        assert_eq!(jm.ask_accept(&b, 5, "freight", "0x1000", "0x2000", now), Ok(None));
        assert_eq!(b.action.load(Ordering::Relaxed), JobMarketAction::Accept as u32);
        assert_eq!(b.mode.load(Ordering::Relaxed), JobMarketMode::Freight as u32);
        assert_eq!(b.company_ptr.load(Ordering::Relaxed), 0x1000);
        assert_eq!(b.offer_ptr.load(Ordering::Relaxed), 0x2000);
        assert_eq!(b.request.load(Ordering::Relaxed), 1);

        // Plugin responds `OnJob`
        b.answer.store(JobMarketAnswer::OnJob as i32, Ordering::Relaxed);
        b.answered.store(1, Ordering::Release);
        let msg = jm.poll(&b, Path::new("."), None, None, None, None, now).unwrap();
        assert_eq!(msg, r#"{"answer":"onJob","id":5,"type":"jobMarketAnswer"}"#);
        assert!(jm.poll(&b, Path::new("."), None, None, None, None, now).is_none());
    }

    #[test]
    fn times_out_when_plugin_does_not_answer() {
        let b = block();
        let mut jm = JobMarket::default();
        let now = Instant::now();
        jm.ask_refresh(&b, 9, now);
        assert!(jm.poll(&b, Path::new("."), None, None, None, None, now + Duration::from_secs(1)).is_none());
        let msg = jm.poll(&b, Path::new("."), None, None, None, None, now + ANSWER_WITHIN).unwrap();
        assert_eq!(msg, r#"{"answer":"noAnswer","id":9,"type":"jobMarketSnapshot"}"#);
    }
}
