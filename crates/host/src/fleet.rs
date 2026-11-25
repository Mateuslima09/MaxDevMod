//! Host bridge for the phone's live "Minha Empresa / Garagem" app.
//!
//! Sends `Refresh`, `DriveTruck`, `UseTrailer`, `StoreTrailer`, and `UpgradeGarage` RPCs to the game plugin
//! over [`westplay_shared::FleetBlock`], reads `Documents\WestPlay\fleet.json`, and enriches city IDs with
//! localized city names from the active map data.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use westplay_shared::{FLEET_FILE, FleetAction, FleetAnswer, FleetBlock};

/// How long the plugin has to answer before we report `noAnswer`.
const ANSWER_WITHIN: Duration = Duration::from_secs(6);

#[derive(Default)]
pub struct Fleet {
    pending: Option<Pending>,
    cities_key: Option<String>,
    cities: HashMap<String, String>,
}

struct Pending {
    request: u32,
    id: u32,
    action: FleetAction,
    sent: Instant,
}

impl Fleet {
    pub fn ask_refresh(&mut self, block: &FleetBlock, id: u32, now: Instant) -> Option<String> {
        self.send(block, id, FleetAction::Refresh, 0, now)
    }

    pub fn ask_drive_truck(&mut self, block: &FleetBlock, id: u32, truck_ptr: &str, now: Instant) -> Result<Option<String>, String> {
        let Some(ptr) = parse_hex_u64(truck_ptr) else {
            return Err(answer_message(id, "driveTruck", "notFound", None));
        };
        Ok(self.send(block, id, FleetAction::DriveTruck, ptr, now))
    }

    pub fn ask_use_trailer(&mut self, block: &FleetBlock, id: u32, trailer_ptr: &str, now: Instant) -> Result<Option<String>, String> {
        let Some(ptr) = parse_hex_u64(trailer_ptr) else {
            return Err(answer_message(id, "useTrailer", "notFound", None));
        };
        Ok(self.send(block, id, FleetAction::UseTrailer, ptr, now))
    }

    pub fn ask_store_trailer(&mut self, block: &FleetBlock, id: u32, now: Instant) -> Option<String> {
        self.send(block, id, FleetAction::StoreTrailer, 0, now)
    }

    pub fn ask_upgrade_garage(&mut self, block: &FleetBlock, id: u32, garage_ptr: &str, now: Instant) -> Result<Option<String>, String> {
        let Some(ptr) = parse_hex_u64(garage_ptr) else {
            return Err(answer_message(id, "upgradeGarage", "notFound", None));
        };
        Ok(self.send(block, id, FleetAction::UpgradeGarage, ptr, now))
    }

    fn send(&mut self, block: &FleetBlock, id: u32, action: FleetAction, target_ptr: u64, now: Instant) -> Option<String> {
        let overtaken = self.pending.take().map(|p| timeout_message(p.id, p.action));
        block.action.store(action as u32, Ordering::Relaxed);
        block.target_ptr.store(target_ptr, Ordering::Relaxed);
        let request = block.request.load(Ordering::Relaxed).wrapping_add(1);
        block.request.store(request, Ordering::Release);
        self.pending = Some(Pending { request, id, action, sent: now });
        overtaken
    }

    pub fn poll(
        &mut self,
        block: &FleetBlock,
        maps_dir: &Path,
        map_folder: Option<&str>,
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
        let answer = FleetAnswer::from_code(block.answer.load(Ordering::Relaxed));
        let code = answer.map_or("failed", FleetAnswer::name);

        self.ensure_cities(maps_dir, map_folder);
        match p.action {
            FleetAction::Refresh => {
                if answer != Some(FleetAnswer::Ok) {
                    return Some(json!({ "type": "fleetSnapshot", "id": p.id, "answer": code }).to_string());
                }
                match load_and_enrich_snapshot(&self.cities) {
                    Some(data) => Some(json!({ "type": "fleetSnapshot", "id": p.id, "answer": "ok", "data": data }).to_string()),
                    None => Some(json!({ "type": "fleetSnapshot", "id": p.id, "answer": "failed" }).to_string()),
                }
            }
            other => {
                let data = if answer == Some(FleetAnswer::Ok) { load_and_enrich_snapshot(&self.cities) } else { None };
                Some(answer_message(p.id, action_name(other), code, data))
            }
        }
    }

    fn ensure_cities(&mut self, maps_dir: &Path, map_folder: Option<&str>) {
        let Some(folder) = map_folder else { return };
        if self.cities_key.as_deref() == Some(folder) {
            return;
        }
        self.cities = load_city_names(maps_dir, folder).unwrap_or_default();
        self.cities_key = Some(folder.to_owned());
    }
}

fn action_name(action: FleetAction) -> &'static str {
    match action {
        FleetAction::Refresh => "refresh",
        FleetAction::DriveTruck => "driveTruck",
        FleetAction::UseTrailer => "useTrailer",
        FleetAction::StoreTrailer => "storeTrailer",
        FleetAction::UpgradeGarage => "upgradeGarage",
    }
}

fn timeout_message(id: u32, action: FleetAction) -> String {
    match action {
        FleetAction::Refresh => json!({ "type": "fleetSnapshot", "id": id, "answer": "noAnswer" }).to_string(),
        other => answer_message(id, action_name(other), "noAnswer", None),
    }
}

fn answer_message(id: u32, action: &str, answer: &str, data: Option<Value>) -> String {
    match data {
        Some(d) => json!({ "type": "fleetAnswer", "id": id, "action": action, "answer": answer, "data": d }).to_string(),
        None => json!({ "type": "fleetAnswer", "id": id, "action": action, "answer": answer }).to_string(),
    }
}

fn parse_hex_u64(s: &str) -> Option<u64> {
    let trimmed = s.trim();
    let hex = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")).unwrap_or(trimmed);
    u64::from_str_radix(hex, 16).ok().filter(|&v| v != 0)
}

fn snapshot_file_path() -> Option<PathBuf> {
    Some(westplay_setup::places::documents()?.join("WestPlay").join(FLEET_FILE))
}

fn load_city_names(maps_dir: &Path, folder: &str) -> Option<HashMap<String, String>> {
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
        if let Some(name) = name {
            let is_city = entry["kind"].as_str() == Some("city");
            if is_city || !out.contains_key(city_id) {
                out.insert(city_id.to_owned(), name);
            }
        }
    }
    Some(out)
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

fn load_and_enrich_snapshot(cities: &HashMap<String, String>) -> Option<Value> {
    let path = snapshot_file_path()?;
    let bytes = std::fs::read(path).ok()?;
    let mut root: Value = serde_json::from_slice(&bytes).ok()?;
    let city_name = |id: &str| cities.get(id).cloned().unwrap_or_else(|| pretty_id(id));

    if let Some(garages) = root.get_mut("garages").and_then(Value::as_array_mut) {
        for g in garages.iter_mut() {
            let Some(obj) = g.as_object_mut() else { continue };
            let id = obj.get("cityId").and_then(Value::as_str).unwrap_or("").to_owned();
            obj.insert("city".into(), Value::String(city_name(&id)));
        }
    }
    if let Some(trucks) = root.get_mut("trucks").and_then(Value::as_array_mut) {
        for t in trucks.iter_mut() {
            let Some(obj) = t.as_object_mut() else { continue };
            let id = obj.get("garageCityId").and_then(Value::as_str).unwrap_or("").to_owned();
            obj.insert("garageCity".into(), Value::String(city_name(&id)));
        }
    }
    if let Some(trailers) = root.get_mut("trailers").and_then(Value::as_array_mut) {
        for tr in trailers.iter_mut() {
            let Some(obj) = tr.as_object_mut() else { continue };
            let id = obj.get("garageCityId").and_then(Value::as_str).unwrap_or("").to_owned();
            obj.insert("garageCity".into(), Value::String(city_name(&id)));
        }
    }
    Some(root)
}
