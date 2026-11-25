//! The game's suspension going back to its normal height by itself (`g_suspension_auto_reset`, owner 2026-11-18: "the
//! game sets the suspension back on its own; that's to be turned off to keep the height we want"). The game has no
//! option for it in its menus: it's a line of its `config.cfg` (in Documents, beside its profiles), which the game
//! writes back as it closes. So it's changed there with the games closed, as the developer mode is (`developer`).

use std::path::{Path, PathBuf};

use crate::game::Kind;
use crate::low_suspension::Damping;

/// The option's name, as the game writes it.
const NAME: &str = "g_suspension_auto_reset";
/// The profile spring stiffness and stability cvars (owner 2026-11-22).
const CHASSIS_STIFFNESS: &str = "g_suspension_stiffness";
const CABIN_STIFFNESS: &str = "g_cabin_suspension_stiffness";
const TRUCK_STABILITY: &str = "g_truck_stability";

/// Whether the game sets the suspension back by itself, as the first game's options found say (`None`: no options
/// file, or no such line: the game's default, on).
pub fn auto_reset() -> Option<bool> {
    for documents in crate::places::documents_folders() {
        for game in Kind::ALL {
            let Ok(text) = std::fs::read_to_string(documents.join(game.name()).join("config.cfg")) else { continue };
            if let Some(on) = read(&text) {
                return Some(on);
            }
        }
    }
    None
}

/// Sets it in each game's options found, the games closed; whether it was set (false: a game runs, so nothing was).
pub fn set_auto_reset(on: bool) -> bool {
    if crate::game::running() {
        return false;
    }
    for documents in crate::places::documents_folders() {
        for game in Kind::ALL {
            let _ = set_in(&documents.join(game.name()).join("config.cfg"), on);
        }
    }
    true
}

fn set_in(file: &Path, on: bool) -> std::io::Result<()> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if let Some(changed) = with(&text, on) {
        std::fs::write(file, changed)?;
    }
    Ok(())
}

/// The option's value in an options file's text.
fn read(text: &str) -> Option<bool> {
    text.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next() == Some("uset") && words.next() == Some(NAME))
            .then(|| words.next().map(|value| value.trim_matches('"') != "0"))
            .flatten()
    })
}

/// Whether every profile's `config.cfg` already matches the spring stiffness & stability for `damping`
/// (`Normal` leaves the player's own profile sliders untouched, so it is always applied).
pub fn stiffness_applied(damping: Damping) -> bool {
    let Some((chassis_target, cabin_target, stability_target)) = damping.spring_stiffness() else { return true };
    profile_configs().into_iter().all(|file| {
        let Ok(text) = std::fs::read_to_string(&file) else { return true };
        let chassis = read_f32(&text, CHASSIS_STIFFNESS).is_some_and(|v| (v - chassis_target).abs() < 1e-3);
        let cabin = read_f32(&text, CABIN_STIFFNESS).is_some_and(|v| (v - cabin_target).abs() < 1e-3);
        let stability = read_f32(&text, TRUCK_STABILITY).is_some_and(|v| (v - stability_target).abs() < 1e-3);
        chassis && cabin && stability
    })
}

/// Sets `g_suspension_stiffness`, `g_cabin_suspension_stiffness`, and `g_truck_stability` in every ETS2/ATS profile's
/// `config.cfg` (local and Steam Cloud), the games closed; `Normal` leaves profiles untouched.
pub fn set_stiffness(damping: Damping) -> bool {
    let Some((chassis_target, cabin_target, stability_target)) = damping.spring_stiffness() else { return true };
    if crate::game::running() {
        return false;
    }
    let chassis = format!("{chassis_target}");
    let cabin = format!("{cabin_target}");
    let stability = format!("{stability_target}");
    for file in profile_configs() {
        let _ = set_stiffness_in(&file, &chassis, &cabin, &stability);
    }
    true
}

fn set_stiffness_in(file: &Path, chassis: &str, cabin: &str, stability: &str) -> std::io::Result<()> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let mut current = text;
    let mut changed = false;
    for (name, value) in [(CHASSIS_STIFFNESS, chassis), (CABIN_STIFFNESS, cabin), (TRUCK_STABILITY, stability)] {
        if let Some(updated) = with_cvar(&current, name, value) {
            current = updated;
            changed = true;
        }
    }
    if changed {
        std::fs::write(file, current)?;
    }
    Ok(())
}

/// Every profile's `config.cfg` found across ETS2 and ATS (local `profiles`, `steam_profiles`, and Steam Cloud).
fn profile_configs() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for documents in crate::places::documents_folders() {
        for game in Kind::ALL {
            let game_dir = documents.join(game.name());
            roots.push(game_dir.join("profiles"));
            roots.push(game_dir.join("steam_profiles"));
        }
    }
    if let Some(userdata) = crate::places::steam().map(|steam| steam.join("userdata")) {
        for user in std::fs::read_dir(userdata).into_iter().flatten().flatten() {
            for game in Kind::ALL {
                roots.push(user.path().join(game.steam_app()).join("remote").join("profiles"));
            }
        }
    }
    let mut configs = Vec::new();
    for root in roots {
        for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
            let cfg = entry.path().join("config.cfg");
            if cfg.is_file() {
                configs.push(cfg);
            }
        }
    }
    configs
}

fn read_f32(text: &str, name: &str) -> Option<f32> {
    text.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next() == Some("uset") && words.next() == Some(name))
            .then(|| words.next().and_then(|v| v.trim_matches('"').parse::<f32>().ok()))
            .flatten()
    })
}

/// The options with `uset <name> "<value>"` set, if that changes them: its line changed, or added at the end.
fn with_cvar(text: &str, name: &str, value: &str) -> Option<String> {
    let wanted = format!("uset {name} \"{value}\"");
    let mut found = false;
    let mut changed = false;
    let mut out: String = text
        .split_inclusive('\n')
        .map(|line| {
            let content = line.trim_end_matches(['\r', '\n']);
            let ending = &line[content.len()..];
            let mut words = content.split_whitespace();
            if words.next() == Some("uset") && words.next() == Some(name) {
                found = true;
                if content.trim() != wanted {
                    changed = true;
                    return format!("{wanted}{ending}");
                }
            }
            line.to_owned()
        })
        .collect();
    if !found {
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline);
        }
        out.push_str(&wanted);
        out.push_str(newline);
        changed = true;
    }
    changed.then_some(out)
}

/// The options with it set, if that changes them: its line changed, or added at the end.
fn with(text: &str, on: bool) -> Option<String> {
    with_cvar(text, NAME, if on { "1" } else { "0" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_sets_the_line_and_nothing_else() {
        let on = "uset g_lang \"pt_br\"\r\nuset g_suspension_auto_reset \"1\"\r\n";
        assert_eq!(read(on), Some(true));
        let off = with(on, false).unwrap();
        assert_eq!(off, "uset g_lang \"pt_br\"\r\nuset g_suspension_auto_reset \"0\"\r\n");
        assert_eq!(read(&off), Some(false));
        assert_eq!(with(&off, false), None);
        assert_eq!(read("uset g_lang \"en\"\n"), None);
        assert_eq!(with("uset g_lang \"en\"\n", false).unwrap(), "uset g_lang \"en\"\nuset g_suspension_auto_reset \"0\"\n");
        // A setting whose name only starts the same is another one.
        assert_eq!(read("uset g_suspension_auto_reset_x \"0\"\n"), None);
    }

    #[test]
    fn reads_and_sets_spring_stiffness_cvars() {
        let cfg = "uset g_suspension_stiffness \"0.23\"\r\nuset g_cabin_suspension_stiffness \"0.21\"\r\n";
        assert!((read_f32(cfg, CHASSIS_STIFFNESS).unwrap() - 0.23).abs() < 1e-4);
        let stiff = with_cvar(cfg, CHASSIS_STIFFNESS, "2").unwrap();
        let stiff = with_cvar(&stiff, CABIN_STIFFNESS, "2").unwrap();
        assert_eq!(stiff, "uset g_suspension_stiffness \"2\"\r\nuset g_cabin_suspension_stiffness \"2\"\r\n");
        assert!((read_f32(&stiff, CHASSIS_STIFFNESS).unwrap() - 2.0).abs() < 1e-4);
        assert_eq!(with_cvar(&stiff, CHASSIS_STIFFNESS, "2"), None);
    }
}
