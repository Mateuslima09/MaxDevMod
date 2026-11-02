//! Mod vehicles' own screens, found (owner 2026-11-02: "identify the screen in mods automatically, rather than
//! one by one"): each installed mod's vehicles' interiors name the GPS script their screen draws (`gps_path`), and
//! a script WestPlay has no way of its own with (`game_mod::own_scripts`) gets a slot of
//! `westplay_shared::screen_layouts`' `mod-1`... and WestPlay's image over its whole "screen on" group, as the game's
//! screens do. The page is laid out at the group's shape, told the app by `westplay_shared::MOD_SCREENS_FILE`; what
//! the display cuts off of it the player fits from Adjust Screen (the UI's `state/screenFit.ts`).
//!
//! A script is the mod's or the game's, and one path may be in several installed mods (two trucks' mods bringing one
//! script, a mod and its older copy): which the game uses depends on the profile played. So, as with the cab's mark
//! (`cab_mark`), a script is patched only where every copy installed is the same, and left as it is otherwise; nor
//! are the mods whose vehicles WestPlay keeps out of (`tablet::set_apart`) looked into.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use westplay_mapdata::archive::Archive;
use westplay_mapdata::files::{GameFiles, Mount};
use westplay_mapdata::sii;
use westplay_shared::{MOD_PAGE_HEIGHTS, MOD_SCREENS_FILE, SCREEN_WIDTH, is_mod_slot, screen_layouts};

use crate::game_mod::{self, patch_gps_script, texture_name};

/// The slots in use and their pages' heights, a slot a line, kept in the mod: what [`current`] compares.
pub(crate) const LIST: &str = "material/ui/westplay/mod_screens.txt";

/// The mod screens found, and the scripts left as they are.
#[derive(Debug, Default, PartialEq)]
pub struct ModScreens {
    pub screens: Vec<ModScreen>,
    /// A script left as it is: its path and why.
    pub left: Vec<(String, String)>,
}

/// One mod screen: its script, the slot it's in, the script with WestPlay's image, and the page's height on it.
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

/// The mod screens of `mods` (the installed mods' files, archives or folders, active in a profile or not), their
/// scripts read from them and from `game` (the installed game mounted).
pub fn find(game: &GameFiles, mods: &[PathBuf]) -> ModScreens {
    let mods: Vec<(PathBuf, GameFiles)> =
        mods.iter().filter_map(|path| mounted(path).map(|files| (path.clone(), files))).collect();
    let own: BTreeSet<&str> = game_mod::own_scripts().collect();
    let own_vehicles: BTreeSet<&str> = game_mod::mod_screen_trucks().collect();
    // Each script, and the vehicles naming it.
    let mut named: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, files) in &mods {
        if crate::tablet::set_apart(path, files) {
            continue;
        }
        for kind in ["truck", "car"] {
            for vehicle in files.dirs(&format!("def/vehicle/{kind}")).unwrap_or_default() {
                if own_vehicles.contains(vehicle.as_str()) {
                    continue;
                }
                for script in gps_paths(files, &format!("def/vehicle/{kind}/{vehicle}/interior")) {
                    if !own.contains(script.as_str()) {
                        named.entry(script).or_default().insert(vehicle.clone());
                    }
                }
            }
        }
    }
    let mut slots = screen_layouts().iter().filter(|layout| is_mod_slot(layout.id));
    let mut found = ModScreens::default();
    for (script, vehicles) in named {
        let copies: Vec<Vec<u8>> = std::iter::once(game)
            .chain(mods.iter().map(|(_, files)| files))
            .filter_map(|files| files.read(&script).ok().flatten())
            .collect();
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
        // A slot is taken only by a script made: the next one's left free for the next script.
        let Some(slot) = slots.clone().next() else {
            found.left.push((script, "no slot left".into()));
            continue;
        };
        match patch_gps_script(&text, &texture_name(slot), None) {
            Ok(patched) => {
                slots.next();
                found.screens.push(ModScreen {
                    page_height: page_height(&patched),
                    script,
                    layout: slot.id,
                    text: patched.into_bytes(),
                    vehicles: vehicles.into_iter().collect(),
                });
            }
            Err(why) => found.left.push((script, why)),
        }
    }
    found
}

/// A mod's files, mounted alone; `None` if it can't be opened.
fn mounted(path: &Path) -> Option<GameFiles> {
    let mount = if path.is_dir() { Mount::Folder(path.to_owned()) } else { Mount::Archive(Archive::open(path).ok()?) };
    Some(GameFiles::new(vec![mount]))
}

/// The GPS scripts (game paths, no leading slash) the interiors in `dir` of `files` name, their includes followed.
/// Definitions that aren't text (encrypted) name none.
fn gps_paths(files: &GameFiles, dir: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for name in files.list(dir).unwrap_or_default().iter().filter(|name| name.ends_with(".sii")) {
        for unit in sii::read_units(files, &format!("{dir}/{name}")).unwrap_or_default() {
            if let Some(script) = unit.get("gps_path") {
                let script = script.trim().trim_matches('"').trim_start_matches('/');
                if !script.is_empty() {
                    paths.insert(script.to_owned());
                }
            }
        }
    }
    paths
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

/// Whether the mod built at `built` (WestPlay's `.scs`) has `screens` as they are: the same slots, each script the
/// same.
pub fn current(screens: &ModScreens, built: &Path) -> bool {
    let Ok(archive) = Archive::open(built) else { return false };
    let ours = GameFiles::new(vec![Mount::Archive(archive)]);
    let read = |path: &str| ours.read(path).ok().flatten();
    read(LIST).as_deref() == Some(screens.list().as_bytes())
        && screens.screens.iter().all(|screen| read(&screen.script).as_ref() == Some(&screen.text))
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
