//! The game's developer mode (`g_developer`), which the Modo Personagem needs: its walking view is drawn through the
//! game's camera 0 (its developer camera), there only with `uset g_developer "1"` in the game's `config.cfg` (in
//! Documents, beside its profiles); and its console (`g_console`, owner 2026-11-07: "the console on, all ready").
//! With the Modo Personagem on, WestPlay sets them there for each game, with the game closed (the game writes its
//! options back as it closes: one changed while it runs would be lost). Only those lines change; they're left on
//! when the mode is turned off (the player may have had them on already).

use std::path::{Path, PathBuf};

use crate::game::Kind;

/// The options set on, as the game writes them.
const ON: [(&str, &str); 2] = [("g_developer", "uset g_developer \"1\""), ("g_console", "uset g_console \"1\"")];

/// Sets the developer mode on in each game's options found (every folder that may be Documents), the games
/// closed; returns the files changed. Nothing while a game runs.
pub fn turn_on() -> Vec<PathBuf> {
    if crate::game::running() {
        return Vec::new();
    }
    let mut changed = Vec::new();
    for documents in crate::places::documents_folders() {
        for game in Kind::ALL {
            let file = documents.join(game.name()).join("config.cfg");
            if let Ok(true) = turn_on_in(&file) {
                changed.push(file);
            }
        }
    }
    changed
}

/// Sets it on in one `config.cfg` (if there is one); whether it changed.
fn turn_on_in(file: &Path) -> std::io::Result<bool> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    let Some(on) = with_developer(&text) else { return Ok(false) };
    std::fs::write(file, on)?;
    Ok(true)
}

/// The options with the developer mode and the console on, if they don't have them yet: each one's line changed,
/// or added at the end.
fn with_developer(text: &str) -> Option<String> {
    let mut found = [false; ON.len()];
    let mut changed = false;
    let lines: Vec<String> = text
        .split_inclusive('\n')
        .map(|line| {
            let content = line.trim_end_matches(['\r', '\n']);
            let ending = &line[content.len()..];
            let mut words = content.split_whitespace();
            if words.next() == Some("uset") {
                let name = words.next();
                if let Some(at) = ON.iter().position(|(option, _)| Some(*option) == name) {
                    found[at] = true;
                    let on = ON[at].1;
                    if content.trim() != on {
                        changed = true;
                        return format!("{on}{ending}");
                    }
                }
            }
            line.to_owned()
        })
        .collect();
    let mut out = lines.concat();
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    for ((_, on), found) in ON.iter().zip(found) {
        if !found {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push_str(newline);
            }
            out.push_str(on);
            out.push_str(newline);
            changed = true;
        }
    }
    changed.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_the_developer_and_console_lines_on_and_nothing_else() {
        let off = "uset g_lang \"pt_br\"\r\nuset g_developer \"0\"\r\nuset g_console \"0\"\r\n";
        assert_eq!(
            with_developer(off).unwrap(),
            "uset g_lang \"pt_br\"\r\nuset g_developer \"1\"\r\nuset g_console \"1\"\r\n"
        );
        // Already on: left as it is.
        assert_eq!(with_developer("uset g_developer \"1\"\nuset g_console \"1\"\n"), None);
        // Not there: added at the end, with the file's own line ending.
        assert_eq!(
            with_developer("uset g_lang \"en\"\n").unwrap(),
            "uset g_lang \"en\"\nuset g_developer \"1\"\nuset g_console \"1\"\n"
        );
        assert_eq!(
            with_developer("uset g_lang \"en\"").unwrap(),
            "uset g_lang \"en\"\nuset g_developer \"1\"\nuset g_console \"1\"\n"
        );
        // A setting whose name only starts the same is another one.
        assert_eq!(
            with_developer("uset g_developer_x \"0\"\nuset g_console \"1\"\n").unwrap(),
            "uset g_developer_x \"0\"\nuset g_console \"1\"\nuset g_developer \"1\"\n"
        );
    }
}
