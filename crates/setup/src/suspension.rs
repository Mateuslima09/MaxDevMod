//! The game's suspension going back to its normal height by itself (`g_suspension_auto_reset`, owner 2026-11-18: "the
//! game sets the suspension back on its own; that's to be turned off to keep the height we want"). The game has no
//! option for it in its menus: it's a line of its `config.cfg` (in Documents, beside its profiles), which the game
//! writes back as it closes. So it's changed there with the games closed, as the developer mode is (`developer`).

use std::path::Path;

use crate::game::Kind;

/// The option's name, as the game writes it.
const NAME: &str = "g_suspension_auto_reset";

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

/// The options with it set, if that changes them: its line changed, or added at the end.
fn with(text: &str, on: bool) -> Option<String> {
    let wanted = format!("uset {NAME} \"{}\"", u8::from(on));
    let mut found = false;
    let mut changed = false;
    let mut out: String = text
        .split_inclusive('\n')
        .map(|line| {
            let content = line.trim_end_matches(['\r', '\n']);
            let ending = &line[content.len()..];
            let mut words = content.split_whitespace();
            if words.next() == Some("uset") && words.next() == Some(NAME) {
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
}
