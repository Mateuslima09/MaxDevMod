//! Mod vehicles' screens, measured and assigned slots automatically:
//! Custom GPS scripts brought by installed mods (`gps_path` not in `game_mod::own_scripts`) get a `mod-N` slot,
//! using the vehicle's 3D `gps.tobj` screen mesh (`screen_mesh`) when readable, or the whole "screen on" group.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use westplay_mapdata::archive::Archive;
use westplay_mapdata::files::{GameFiles, Mount};
use westplay_mapdata::sii;
use westplay_shared::{MOD_PAGE_HEIGHTS, MOD_SCREENS_FILE, SCREEN_WIDTH, is_mod_slot, screen_layouts};

use crate::game_mod::{self, patch_gps_script, texture_name};
use crate::screen_mesh::{EDGE_BLEED, MeshCache, ScreenMesh};

/// The slots in use and their pages' heights, a slot a line, kept in the mod: what [`current`] compares.
pub(crate) const LIST: &str = "material/ui/westplay/mod_screens.txt";

/// The mod screens found, any extra files (none by default to keep mod interiors untouched), and scripts left as they are.
#[derive(Debug, Default, PartialEq)]
pub struct ModScreens {
    pub screens: Vec<ModScreen>,
    pub extra_files: Vec<(String, Vec<u8>)>,
    /// A script left as it is: its path and why.
    pub left: Vec<(String, String)>,
}

/// One allocated `mod-N` screen: its script path in `westplay.scs`, slot id, patched script bytes, and page height.
#[derive(Debug, PartialEq)]
pub struct ModScreen {
    pub script: String,
    pub layout: &'static str,
    pub text: Vec<u8>,
    pub page_height: u32,
    /// The vehicles whose interiors name it.
    pub vehicles: Vec<String>,
}

impl ModScreens {
    /// The slots in use, each its page's height (`westplay_shared::MOD_SCREENS_FILE`, and [`LIST`]).
    pub fn list(&self) -> String {
        self.screens.iter().map(|screen| format!("{} {}\n", screen.layout, screen.page_height)).collect()
    }
}

fn page_height_for_mesh(mesh: &ScreenMesh) -> u32 {
    ((f64::from(SCREEN_WIDTH) / mesh.aspect()).round() as u32).clamp(MOD_PAGE_HEIGHTS.0, MOD_PAGE_HEIGHTS.1)
}

/// Discovers custom mod GPS screens and measures them using their 3D screen mesh.
pub fn find(game: &GameFiles, mods: &[PathBuf]) -> ModScreens {
    let active_mods: Vec<(PathBuf, GameFiles)> = mods
        .iter()
        .filter_map(|path| {
            let files = mounted(path)?;
            (!crate::tablet::set_apart(path, &files)).then_some((path.clone(), files))
        })
        .collect();
    let own: BTreeSet<&str> = game_mod::own_scripts().collect();
    let own_vehicles: BTreeSet<&str> = game_mod::mod_screen_trucks().collect();
    let mut mesh_cache = MeshCache::default();

    // 1. Custom GPS scripts named by mod vehicle interiors (`gps_path` not in `own_scripts`)
    //    and by cabin GPS accessories (`accessory/*/*.sii`, `addon_hookups/*.sii`).
    let mut named: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut script_meshes: BTreeMap<String, Vec<ScreenMesh>> = BTreeMap::new();
    for (_, files) in &active_mods {
        for kind in ["truck", "car"] {
            for vehicle in files.dirs(&format!("def/vehicle/{kind}")).unwrap_or_default() {
                if own_vehicles.contains(vehicle.as_str()) {
                    continue;
                }
                let int_dir = format!("def/vehicle/{kind}/{vehicle}/interior");
                for info in read_interiors(files, Some(game), &int_dir, &mut mesh_cache) {
                    let Some(script) = info.gps_script else { continue };
                    if !own.contains(script.as_str()) {
                        named.entry(script.clone()).or_default().insert(vehicle.clone());
                        if let Some(mesh) = info.mesh {
                            script_meshes.entry(script).or_default().push(mesh);
                        }
                    }
                }
                // Cabin accessories (e.g. aftermarket/optional GPS & multimedia units on mod trucks).
                let acc_root = format!("def/vehicle/{kind}/{vehicle}/accessory");
                for slot_dir in files.dirs(&acc_root).unwrap_or_default() {
                    let dir = format!("{acc_root}/{slot_dir}");
                    for script in read_accessory_gps_scripts(files, Some(game), &dir) {
                        if !own.contains(script.as_str()) {
                            named.entry(script).or_default().insert(vehicle.clone());
                        }
                    }
                }
            }
        }
        // Global cabin addon hookups that declare a custom `gps_path`.
        for script in read_accessory_gps_scripts(files, Some(game), "def/vehicle/addon_hookups") {
            if !own.contains(script.as_str()) {
                named.entry(script).or_default().insert("addon_hookups".into());
            }
        }
        // 2. Direct GPS-only UI script discovery inside the mod (`ui/` and `ui/dashboard/`):
        //    only scripts that actually render a GPS map (`ui::map`) or are named `*gps*.sii` with `id: 100`
        //    so speedometers, radios, and non-GPS displays are never touched.
        for ui_dir in ["ui", "ui/dashboard"] {
            for name in files.list(ui_dir).unwrap_or_default().iter().filter(|n| n.ends_with(".sii")) {
                let script = format!("{ui_dir}/{name}");
                if own.contains(script.as_str()) || named.contains_key(&script) {
                    continue;
                }
                let Some(bytes) = files.read(&script).ok().flatten() else { continue };
                let Ok(text) = String::from_utf8(bytes) else { continue };
                if is_gps_only_script(&script, &text) {
                    named.entry(script).or_default().insert("mod-gps".into());
                }
            }
        }
    }

    let mut slots = screen_layouts().iter().filter(|layout| is_mod_slot(layout.id));
    let mut found = ModScreens::default();

    for (script, vehicles) in named {
        let mod_copies: Vec<Vec<u8>> = active_mods
            .iter()
            .filter_map(|(_, files)| files.read(&script).ok().flatten())
            .collect();
        let game_copy = game.read(&script).ok().flatten();
        // Prefer mod copies when a mod intentionally overrides a base-game GPS script;
        // still guard against conflicting copies across two different installed mods.
        let copies: Vec<Vec<u8>> = if !mod_copies.is_empty() {
            mod_copies
        } else if let Some(gc) = game_copy {
            vec![gc]
        } else {
            Vec::new()
        };
        let Some(first) = copies.first() else {
            found.left.push((script, "not in the game or an installed mod".into()));
            continue;
        };
        if copies.iter().any(|copy| copy != first) {
            found.left.push((script, format!("{} copies installed, not all the same", copies.len())));
            continue;
        }
        let Ok(text) = String::from_utf8(first.clone()) else {
            found.left.push((script, "not UTF-8 text".into()));
            continue;
        };
        let Some(slot) = slots.clone().next() else {
            found.left.push((script, "no slot left".into()));
            continue;
        };
        // If the mod's 3D interior mesh shows a specific band of a square/custom script, crop to that band!
        let measured = script_meshes.get(&script).and_then(|meshes| consistent_mesh(meshes));
        let shown_opt = measured.filter(|m| !m.covers_whole_canvas()).map(|m| m.padded_shown(EDGE_BLEED));
        match patch_gps_script(&text, &texture_name(slot), shown_opt.as_ref()) {
            Ok(patched) => {
                slots.next();
                let height = measured.map_or_else(|| page_height(&patched), |m| page_height_for_mesh(&m));
                found.screens.push(ModScreen {
                    page_height: height,
                    script: script.clone(),
                    layout: slot.id,
                    text: patched.into_bytes(),
                    vehicles: vehicles.iter().cloned().collect(),
                });
            }
            Err(why) => found.left.push((script, why)),
        }
    }

    found
}

struct InteriorInfo {
    gps_script: Option<String>,
    mesh: Option<ScreenMesh>,
}

fn read_interiors(
    files: &GameFiles,
    fallback: Option<&GameFiles>,
    int_dir: &str,
    cache: &mut MeshCache,
) -> Vec<InteriorInfo> {
    let mut out = Vec::new();
    for name in files.list(int_dir).unwrap_or_default().iter().filter(|n| n.ends_with(".sii")) {
        if let Some(info) = inspect_single_interior(files, fallback, int_dir, name, cache) {
            out.push(info);
        }
    }
    out
}

fn inspect_single_interior(
    files: &GameFiles,
    fallback: Option<&GameFiles>,
    int_dir: &str,
    name: &str,
    cache: &mut MeshCache,
) -> Option<InteriorInfo> {
    let path = format!("{int_dir}/{name}");
    let units = sii::read_units(files, &path).ok().filter(|u| !u.is_empty()).or_else(|| {
        read_units_with_fallback(files, fallback?, &path)
    })?;
    let unit = units.iter().find(|u| u.class == "accessory_interior_data")?;
    let gps_script = unit
        .get("gps_path")
        .map(|s| s.trim().trim_matches('"').trim_start_matches('/').to_owned())
        .filter(|s| !s.is_empty());
    let variant = unit.get("variant");
    let model = unit.get("model");
    let anim = unit.get("animated_model");
    let mesh = model
        .and_then(|m| cache.lookup(files, fallback, m, variant))
        .or_else(|| anim.and_then(|a| cache.lookup(files, fallback, a, variant)));
    Some(InteriorInfo { gps_script, mesh })
}

/// True only for UI scripts that actually draw a GPS screen (never speedometers, trip computers, or radios).
fn is_gps_only_script(script_path: &str, text: &str) -> bool {
    if text.contains("ui::map") {
        return true;
    }
    let file = script_path.rsplit('/').next().unwrap_or(script_path).to_ascii_lowercase();
    file.contains("gps") && text.contains("ui::group") && text.contains("id: 100")
}

/// Reads `gps_path` entries from cabin accessory `.sii` definitions in `dir` (strictly GPS accessories).
fn read_accessory_gps_scripts(files: &GameFiles, fallback: Option<&GameFiles>, dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    for name in files.list(dir).unwrap_or_default().iter().filter(|n| n.ends_with(".sii")) {
        let path = format!("{dir}/{name}");
        let units = sii::read_units(files, &path)
            .ok()
            .filter(|u| !u.is_empty())
            .or_else(|| read_units_with_fallback(files, fallback?, &path));
        let Some(units) = units else { continue };
        for unit in units {
            if let Some(script) = unit
                .get("gps_path")
                .map(|s| s.trim().trim_matches('"').trim_start_matches('/').to_owned())
                .filter(|s| !s.is_empty())
            {
                out.push(script);
            }
        }
    }
    out
}

fn read_units_with_fallback(primary: &GameFiles, fallback: &GameFiles, path: &str) -> Option<Vec<sii::Unit>> {
    let mut visited = BTreeSet::new();
    let expanded = expand_includes_recursive(primary, fallback, path, 0, &mut visited)?;
    Some(sii::parse(&expanded))
}

fn expand_includes_recursive(
    primary: &GameFiles,
    fallback: &GameFiles,
    path: &str,
    depth: u8,
    visited: &mut BTreeSet<String>,
) -> Option<String> {
    if depth > 6 || !visited.insert(path.to_owned()) {
        return None;
    }
    let bytes = primary.read(path).ok().flatten().or_else(|| fallback.read(path).ok().flatten())?;
    let text = String::from_utf8(bytes).ok()?;
    let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
    let mut expanded = String::new();
    for line in text.lines() {
        if let Some(inc) = line.trim().strip_prefix("@include")
            && let Some(inc_file) = inc.split('"').nth(1)
        {
            let inc_path = if let Some(abs) = inc_file.strip_prefix('/') {
                abs.to_owned()
            } else if dir.is_empty() {
                inc_file.to_owned()
            } else {
                format!("{dir}/{inc_file}")
            };
            if let Some(inc_text) = expand_includes_recursive(primary, fallback, &inc_path, depth + 1, visited) {
                expanded.push_str(&inc_text);
                expanded.push('\n');
                continue;
            }
        }
        expanded.push_str(line);
        expanded.push('\n');
    }
    Some(expanded)
}

fn consistent_mesh(meshes: &[ScreenMesh]) -> Option<ScreenMesh> {
    let first = *meshes.first()?;
    let agree = meshes.iter().all(|m| {
        (m.shown.top - first.shown.top).abs() < 0.04
            && (m.shown.bottom - first.shown.bottom).abs() < 0.04
            && (m.shown.left - first.shown.left).abs() < 0.04
            && (m.shown.right - first.shown.right).abs() < 0.04
    });
    agree.then_some(first)
}

/// A mod's files, mounted alone; `None` if it can't be opened.
fn mounted(path: &Path) -> Option<GameFiles> {
    let mount = if path.is_dir() { Mount::Folder(path.to_owned()) } else { Mount::Archive(Archive::open(path).ok()?) };
    Some(GameFiles::new(vec![mount]))
}

/// The height the page is laid out at on a screen of `patched`'s shape (its WestPlay image's, the whole of its
/// "screen on" group), [`SCREEN_WIDTH`] wide, within [`MOD_PAGE_HEIGHTS`]; the frame's own where it can't be read.
fn page_height(patched: &str) -> u32 {
    let start = patched.find("_nameless.westplay.screen {").unwrap_or(patched.len());
    let block = patched[start..].split('}').next().unwrap_or_default();
    let at = |name: &str| -> Option<f64> {
        block.lines().find_map(|line| line.trim().strip_prefix(name)?.trim().strip_prefix(':')?.trim().parse().ok())
    };
    let (Some(left), Some(right), Some(top), Some(bottom)) =
        (at("coords_l"), at("coords_r"), at("coords_t"), at("coords_b"))
    else {
        return westplay_shared::SCREEN_HEIGHT;
    };
    let (width, height) = ((right - left).abs(), (top - bottom).abs());
    if width < 1.0 || height < 1.0 {
        return westplay_shared::SCREEN_HEIGHT;
    }
    ((SCREEN_WIDTH as f64 * height / width).round() as u32).clamp(MOD_PAGE_HEIGHTS.0, MOD_PAGE_HEIGHTS.1)
}

/// Whether the mod built at `built` (WestPlay's `.scs`) has `screens` as they are: the same slots, scripts, and
/// rewritten interior definitions.
pub fn current(screens: &ModScreens, built: &Path) -> bool {
    let Ok(archive) = Archive::open(built) else { return false };
    let ours = GameFiles::new(vec![Mount::Archive(archive)]);
    let read = |path: &str| ours.read(path).ok().flatten();
    read(LIST).as_deref() == Some(screens.list().as_bytes())
        && screens.screens.iter().all(|screen| read(&screen.script).as_ref() == Some(&screen.text))
        && screens.extra_files.iter().all(|(path, bytes)| read(path).as_ref() == Some(bytes))
}

/// Writes `screens`' slots and heights ([`ModScreens::list`]) into WestPlay's own folder `dir`, beside the mod, for
/// the app: written beside it first, then moved over it, so the app never reads half a file.
pub fn write_list(screens: &ModScreens, dir: &Path) -> std::io::Result<()> {
    let path = dir.join(MOD_SCREENS_FILE);
    let partial = dir.join(format!("{MOD_SCREENS_FILE}.partial"));
    std::fs::write(&partial, screens.list())?;
    std::fs::rename(&partial, &path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of files, written for a test.
    fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("westplay-mod-screens-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        root
    }

    /// A GPS script `width` x `height`, with its "screen on" group.
    fn script(width: u32, height: u32) -> String {
        format!(
            "SiiNunit\n{{\nui::window : _nameless.wnd {{\n my_children: 1\n my_children[0]: _nameless.on\n coords_l: 0\n \
             coords_r: {width}\n coords_t: {height}\n coords_b: 0\n}}\n\nui::group : _nameless.on {{\n my_children: 0\n \
             coords_l: 0\n coords_r: {width}\n coords_t: {height}\n coords_b: 0\n id: 100\n my_parent: _nameless.wnd\n}}\n}}\n"
        )
    }

    fn interior(gps: &str) -> String {
        format!("SiiNunit\n{{\naccessory_interior_data : a.int.data\n{{\n\tgps_path: \"{gps}\"\n}}\n}}\n")
    }

    #[test]
    fn gives_each_mod_vehicles_screen_a_slot_at_its_shape() {
        let wide = script(1280, 480);
        let mods = [
            folder(
                "a",
                &[
                    ("def/vehicle/truck/acme.big/interior/cab.sii", &interior("/ui/dashboard/acme_gps.sii")),
                    ("ui/dashboard/acme_gps.sii", &wide),
                    // The game's own screens, and the stock one: WestPlay's own way with them.
                    ("def/vehicle/truck/acme.small/interior/cab.sii", &interior("/ui/gps.sii")),
                ],
            ),
            folder(
                "b",
                &[
                    ("def/vehicle/truck/zed.one/interior/cab.sii", &interior("/ui/dashboard/zed_gps.sii")),
                    ("ui/dashboard/zed_gps.sii", &script(1000, 1000)),
                ],
            ),
        ];
        let game = GameFiles::new(Vec::new());
        let found = find(&game, &mods);
        assert_eq!(found.left, []);
        let slots: Vec<(&str, &str, u32)> =
            found.screens.iter().map(|s| (s.script.as_str(), s.layout, s.page_height)).collect();
        assert_eq!(slots, [("ui/dashboard/acme_gps.sii", "mod-1", 480), ("ui/dashboard/zed_gps.sii", "mod-2", 1280)]);
        assert_eq!(found.screens[0].vehicles, ["acme.big"]);
        assert!(String::from_utf8_lossy(&found.screens[0].text).contains("screen_mod_1.mat"));
        assert_eq!(found.list(), "mod-1 480\nmod-2 1280\n");
        for root in mods {
            let _ = std::fs::remove_dir_all(root);
        }
    }

    #[test]
    #[ignore = "reads the PC's game and its installed mods"]
    fn finds_the_installed_mods_screens() {
        let game = crate::game::find().into_iter().next().expect("the game installed");
        let files = GameFiles::install(&game.dir).unwrap();
        let documents = crate::places::game_documents().expect("the game's documents");
        let mods: Vec<PathBuf> =
            crate::profiles::installed_files(&documents, Some(&game)).into_iter().map(|(path, _)| path).collect();
        let found = find(&files, &mods);
        println!("{} mods", mods.len());
        for screen in &found.screens {
            println!("{} {} 1280x{} {:?}", screen.layout, screen.script, screen.page_height, screen.vehicles);
        }
        for (script, why) in &found.left {
            println!("left {script}: {why}");
        }
    }

    #[test]
    fn leaves_a_script_whose_copies_differ_or_that_has_no_screen_on_group() {
        let truck = interior("/ui/dashboard/shared_gps.sii");
        let mods = [
            folder(
                "c",
                &[
                    ("def/vehicle/truck/one.t/interior/cab.sii", &truck),
                    ("ui/dashboard/shared_gps.sii", &script(800, 480)),
                    ("def/vehicle/truck/one.t/interior/bare.sii", &interior("/ui/dashboard/bare_gps.sii")),
                    ("ui/dashboard/bare_gps.sii", "SiiNunit\n{\n}\n"),
                ],
            ),
            folder(
                "d",
                &[
                    ("def/vehicle/truck/two.t/interior/cab.sii", &truck),
                    ("ui/dashboard/shared_gps.sii", &script(1024, 600)),
                ],
            ),
        ];
        let found = find(&GameFiles::new(Vec::new()), &mods);
        assert_eq!(found.screens, []);
        let left: Vec<&str> = found.left.iter().map(|(script, _)| script.as_str()).collect();
        assert_eq!(left, ["ui/dashboard/bare_gps.sii", "ui/dashboard/shared_gps.sii"]);
        for root in mods {
            let _ = std::fs::remove_dir_all(root);
        }
    }
}
