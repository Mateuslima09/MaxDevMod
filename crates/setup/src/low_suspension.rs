//! The truck's front lower than the game lets it go (owner 2026-11-18: "lower it till the front nearly touches the
//! ground"; and then: "the mod worked, but I'm left depending on installing a mod"). How far a truck's air suspension
//! lowers is its chassis' `residual_travel` (each axle's, the front's first: 0.1 m on most), found by a test mod on the
//! owner's Actros (0.3 at the front: it went down). So WestPlay makes, from the player's own game, each truck's chassis
//! with the front's travel longer, in an archive of its own beside its mod (`westplay_suspensao.scs`), which the
//! plugin mounts with it (no Mod Manager). Off, the archive is empty. The game reads the chassis as it starts: a
//! change counts from the next start (made while the game runs, it waits beside it, as the main mod does).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

use westplay_mapdata::files::GameFiles;

/// The archive's name, beside WestPlay's mod (the plugin mounts both).
pub const FILE_NAME: &str = "westplay_suspensao.scs";
/// What it was made for (the level and the game's version), beside it: made again when that changes.
const MADE_FOR: &str = "westplay_suspensao.txt";

/// How low the front may go: as the game has it, lower, or the lowest (the test's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    #[default]
    Normal,
    Low,
    Lowest,
}

impl Level {
    /// The front axle's travel (metres) for this level (`None`: the game's own).
    pub fn front_travel(self) -> Option<f32> {
        match self {
            Self::Normal => None,
            Self::Low => Some(0.2),
            Self::Lowest => Some(0.3),
        }
    }

    fn word(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Low => "low",
            Self::Lowest => "lowest",
        }
    }
}

/// How firm the dampers are (owner 2026-11-18: "the damper, firmer"): the game's physics' damping factors (its
/// `def/vehicle/physics.sii`: the front's, the rear's and the trailer's) times this.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Damping {
    Soft,
    #[default]
    Normal,
    Firm,
    Firmest,
}

impl Damping {
    /// The factor the game's damping is multiplied by (`None`: the game's own).
    pub fn factor(self) -> Option<f32> {
        match self {
            Self::Soft => Some(0.6),
            Self::Normal => None,
            Self::Firm => Some(1.5),
            Self::Firmest => Some(2.2),
        }
    }

    fn word(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Normal => "normal",
            Self::Firm => "firm",
            Self::Firmest => "firmest",
        }
    }
}

static DAMPING: AtomicU8 = AtomicU8::new(1);

/// The dampers the archive is made for, from the app's settings.
pub fn set_damping(damping: Damping) {
    DAMPING.store(damping as u8, Ordering::Relaxed);
}

pub fn damping() -> Damping {
    match DAMPING.load(Ordering::Relaxed) {
        0 => Damping::Soft,
        2 => Damping::Firm,
        3 => Damping::Firmest,
        _ => Damping::Normal,
    }
}

/// The game's physics with its damping factors times `factor` (`None`: no physics file, or no such factor).
fn damped_physics(files: &GameFiles, factor: f32) -> Option<(String, Vec<u8>)> {
    const PATH: &str = "def/vehicle/physics.sii";
    let text = String::from_utf8_lossy(&files.read(PATH).ok()??).into_owned();
    with_damping(&text, factor).map(|text| (PATH.to_owned(), text.into_bytes()))
}

/// The physics' values made firmer by `factor` (owner 2026-11-19: "even on the firmest the truck stayed very soft, the
/// front lifting as I accelerate"): the game has no spring stiffness to change (it works each spring out from the
/// axle's travel), and the wheels' dampers alone only slow the swaying; what is felt in the cab is mostly the cab's own
/// suspension. So, times `factor`: the wheels' dampers, the front's anti-roll bar, the cab's damping and its anti-roll
/// bar; divided by it: how far the cab may pitch and roll. Below 1 (soft) all go the other way.
const TIMES: [&str; 4] = ["damping_scaling_", "sway_bar_stiffness_factor", "cabin_suspension_damping_factor", "cabin_sway_bar_factor"];
const DIVIDED: [&str; 3] = ["cabin_max_pitch_angle_front", "cabin_max_pitch_angle_rear", "cabin_max_roll_angle"];

/// The physics' text with those values made firmer by `factor`, if it has any.
fn with_damping(text: &str, factor: f32) -> Option<String> {
    let mut changed = false;
    let mut out = String::with_capacity(text.len() + 16);
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        let ending = &line[content.len()..];
        let trimmed = content.trim_start();
        let scale = if TIMES.iter().any(|key| trimmed.starts_with(key)) {
            Some(factor)
        } else if DIVIDED.iter().any(|key| trimmed.starts_with(key)) {
            Some(1.0 / factor)
        } else {
            None
        };
        if let Some(scale) = scale
            && let Some((name, value)) = trimmed.split_once(':')
            && let Ok(now) = value.split('#').next().unwrap_or("").trim().parse::<f32>()
        {
            changed = true;
            let indent = &content[..content.len() - trimmed.len()];
            let _ = write!(out, "{indent}{}: {:.3}{ending}", name.trim_end(), now * scale);
            continue;
        }
        out.push_str(line);
    }
    changed.then_some(out)
}

static LEVEL: AtomicU8 = AtomicU8::new(0);

/// The level the archive is made for, from the app's settings.
pub fn set_level(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn level() -> Level {
    match LEVEL.load(Ordering::Relaxed) {
        1 => Level::Low,
        2 => Level::Lowest,
        _ => Level::Normal,
    }
}

/// What the archive beside the mod in `dir` should say it was made for.
fn made_for(level: Level, game_version: &str) -> String {
    // `v2`: the firmness also reaches the cab (made again over an archive from before).
    format!("v2 {} {} {game_version}", level.word(), damping().word())
}

/// Whether the archive in `dir` is made for the level now and this game version.
pub fn current(dir: &Path, game_version: &str) -> bool {
    std::fs::read_to_string(dir.join(MADE_FOR)).is_ok_and(|text| text.trim() == made_for(level(), game_version))
}

/// Makes the archive for the level now from the game's files and writes it into `dir`; the number of chassis in it.
pub fn make(files: &GameFiles, dir: &Path, game_version: &str) -> std::io::Result<usize> {
    let mut chassis = match level().front_travel() {
        Some(travel) => lowered_chassis(files, travel),
        None => Vec::new(),
    };
    // The dampers, if not the game's own.
    if let Some(physics) = damping().factor().and_then(|factor| damped_physics(files, factor)) {
        chassis.push(physics);
    }
    let count = chassis.iter().filter(|(path, _)| path.ends_with(".sii")).count();
    install(&crate::game_mod::zip(&chassis), dir)?;
    std::fs::write(dir.join(MADE_FOR), made_for(level(), game_version))?;
    Ok(count)
}

/// Every truck's chassis in the game's files, its front's travel at least `travel`, with what they include beside them
/// (`sound.sui`: an archive with a chassis but not its include leaves the game without the chassis).
fn lowered_chassis(files: &GameFiles, travel: f32) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for truck in files.dirs("def/vehicle/truck").unwrap_or_default() {
        let dir = format!("def/vehicle/truck/{truck}/chassis");
        let names = files.list(&dir).unwrap_or_default();
        let mut lowered_any = false;
        let mut beside = Vec::new();
        for name in names {
            let path = format!("{dir}/{name}");
            let Ok(Some(bytes)) = files.read(&path) else { continue };
            if name.ends_with(".sii") {
                if let Some(changed) = with_front_travel(&String::from_utf8_lossy(&bytes), travel) {
                    out.push((path, changed.into_bytes()));
                    lowered_any = true;
                }
            } else if name.ends_with(".sui") {
                beside.push((path, bytes));
            }
        }
        if lowered_any {
            out.extend(beside);
        }
    }
    out
}

/// A chassis' text with its first `residual_travel` (the front axle's) at least `travel`, if it has one.
fn with_front_travel(text: &str, travel: f32) -> Option<String> {
    let mut done = false;
    let mut out = String::with_capacity(text.len() + 8);
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        let ending = &line[content.len()..];
        let trimmed = content.trim_start();
        if !done && trimmed.starts_with("residual_travel[]:") {
            done = true;
            let indent = &content[..content.len() - trimmed.len()];
            let now: f32 = trimmed["residual_travel[]:".len()..].trim().parse().unwrap_or(0.0);
            let _ = write!(out, "{indent}residual_travel[]: {}{ending}", now.max(travel));
            continue;
        }
        out.push_str(line);
    }
    done.then_some(out)
}

/// Writes the archive into `dir`, replacing the one there: beside it first, then over it; with the game holding it
/// open, left beside it (`.next`) for the plugin to put in place as the game starts.
fn install(bytes: &[u8], dir: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(FILE_NAME);
    let partial = dir.join(format!("{FILE_NAME}.partial"));
    let next = dir.join(format!("{FILE_NAME}.next"));
    std::fs::write(&partial, bytes)?;
    if std::fs::rename(&partial, &path).is_ok() {
        let _ = std::fs::remove_file(&next);
        return Ok(path);
    }
    std::fs::rename(&partial, &next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengthens_the_front_axles_travel_only() {
        let chassis = "accessory_chassis_data : 4x2.x.chassis\r\n{\r\n\tresidual_travel[]: 0.1\r\n\tresidual_travel[]: 0.088\r\n}\r\n";
        assert_eq!(
            with_front_travel(chassis, 0.3).unwrap(),
            "accessory_chassis_data : 4x2.x.chassis\r\n{\r\n\tresidual_travel[]: 0.3\r\n\tresidual_travel[]: 0.088\r\n}\r\n"
        );
        // Never shorter than the truck's own.
        assert!(with_front_travel("\tresidual_travel[]: 0.4\n", 0.3).unwrap().contains("0.4"));
        assert_eq!(with_front_travel("name: x\n", 0.3), None);
        assert_eq!(Level::Normal.front_travel(), None);
    }

    #[test]
    fn multiplies_the_damping_factors_only() {
        let physics = "physics_data: .phdata\n{\n\tdamping_scaling_front_factor: \t\t1.0\n\tdamping_scaling_rear_factor: 0.8\n\tsway_bar_stiffness_factor:\t1.0\n\tcabin_max_pitch_angle_front:\t\t2.25\n\tcabin_mass: 700.0\n\tair_resistance:\t3.0\n}\n";
        let firm = with_damping(physics, 1.5).unwrap();
        assert!(firm.contains("\tdamping_scaling_front_factor: 1.500\n"), "{firm}");
        assert!(firm.contains("\tdamping_scaling_rear_factor: 1.200\n"), "{firm}");
        assert!(firm.contains("\tsway_bar_stiffness_factor: 1.500\n"), "{firm}");
        assert!(firm.contains("\tcabin_max_pitch_angle_front: 1.500\n"), "{firm}");
        assert!(firm.contains("\tcabin_mass: 700.0\n"), "{firm}");
        assert!(firm.contains("\tair_resistance:\t3.0\n"), "{firm}");
        assert_eq!(with_damping("name: x\n", 1.5), None);
        assert_eq!(Damping::Normal.factor(), None);
    }
}
