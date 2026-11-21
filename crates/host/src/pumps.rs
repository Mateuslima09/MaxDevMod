//! Where the game's fuel pumps are (owner 2026-11-11: "the green icon, I can only refuel standing on it"): each
//! pump's own point from the played map (`pumps.json` beside its tiles, written as the map is built:
//! `westplay_mapdata::export`), and whether the truck stands at one, told to the plugin
//! (`HostBlock::at_pump`) for refuelling by hand.

use std::path::Path;

/// How near a pump's point the truck (its own point, by the cab) is taken to stand at it, metres along the
/// ground: about as far as the game's green icon lets the tank be filled.
const REACH: f32 = 7.0;

/// The pumps of the map in `folder` (in WestPlay's `maps`), each as the game's x and z (metres); `None`: a map
/// built before pumps were kept, or none to read.
pub fn load(maps: &Path, folder: &str) -> Option<Vec<[f32; 2]>> {
    let map = maps.join(folder);
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(map.join("map.json")).ok()?).ok()?;
    let file = map.join(manifest["pumps"].as_str()?);
    serde_json::from_slice(&std::fs::read(file).ok()?).ok()
}

/// Whether `truck` (the game's x and z) stands at one of `pumps`.
pub fn at(pumps: &[[f32; 2]], truck: [f64; 2]) -> bool {
    pumps.iter().any(|pump| (f64::from(pump[0]) - truck[0]).hypot(f64::from(pump[1]) - truck[1]) <= f64::from(REACH))
}

/// What the plugin is told (`HostBlock::at_pump`): 0 the pumps aren't known, 1 at one, 2 at none.
pub fn told(pumps: Option<&[[f32; 2]]>, truck: [f64; 2]) -> u32 {
    match pumps {
        None => 0,
        Some(pumps) if at(pumps, truck) => 1,
        Some(_) => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_truck_is_at_a_pump_within_its_reach() {
        let pumps = [[100.0, 200.0], [-5000.0, 30.0]];
        assert!(at(&pumps, [103.0, 204.0]));
        assert!(at(&pumps, [-5000.0, 36.5]));
        assert!(!at(&pumps, [110.0, 200.0]));
        assert_eq!(told(Some(&pumps), [100.0, 200.0]), 1);
        assert_eq!(told(Some(&pumps), [0.0, 0.0]), 2);
        assert_eq!(told(None, [100.0, 200.0]), 0);
    }

    #[test]
    fn reads_a_maps_pumps_from_beside_its_tiles() {
        let maps = std::env::temp_dir().join(format!("westplay-pumps-{}", std::process::id()));
        let tiles = maps.join("europe-1").join("tiles-9");
        std::fs::create_dir_all(&tiles).unwrap();
        std::fs::write(maps.join("europe-1").join("map.json"), r#"{"pumps":"tiles-9/pumps.json"}"#).unwrap();
        std::fs::write(tiles.join("pumps.json"), "[[1.5,-2.0],[3.0,4.0]]").unwrap();
        assert_eq!(load(&maps, "europe-1"), Some(vec![[1.5, -2.0], [3.0, 4.0]]));
        // A map built before pumps were kept.
        std::fs::write(maps.join("europe-1").join("map.json"), "{}").unwrap();
        assert_eq!(load(&maps, "europe-1"), None);
        assert_eq!(load(&maps, "none"), None);
        let _ = std::fs::remove_dir_all(&maps);
    }
}
