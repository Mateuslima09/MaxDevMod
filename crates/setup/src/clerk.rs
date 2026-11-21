//! The clerk who signs the cargo's papers by the trailer (the plugin's `character::freight`), as one of the game's
//! own workers (owner 2026-11-11: "use the game's own people, its workers, two or three"): the new generation's
//! worker in a vest, its security guard and its service man, each standing (the game's own idle animations) and
//! walking (its walk), as `mover_model`s of WestPlay's mod pointing at the game's own files (the same paths in ETS2
//! and ATS: `def/world/mover_desc.sii`, `mover_anim.sii`). Nothing of theirs is in the mod but the definitions.
//!
//! And the game's own advice to couple from the cab ("Press the following button to couple the trailer: T"), told
//! the way it's done now (on foot, at the fifth wheel): its text, `@@attach_load@@`, given in each language by the
//! game's own file for replacing its words, `locale/<language>/local.override.sii` (its own is empty; any keys it
//! has are kept), the mod's over it. A localization module (`local_module.*.sii`) only adds words: the game's log
//! says "Failed to add token 'attach_load' … already defined" (2026-11-11). The game adds the key after it: our words
//! end where it goes.

use westplay_mapdata::files::GameFiles;

/// The clerks: the first part of their movers' names (`mover.<name>_idle`, `_walk`: each part 12 letters at most),
/// their model and their standing animation, in the order the plugin picks them in (`CLERKS` there).
pub const CLERKS: [(&str, &str, &str); 3] = [
    ("wp_clk1", "worker_vest/ngn_worker_vest", "crosshand_idle_anim"),
    ("wp_clk2", "security/ngn_security", "handback_idle_anim"),
    ("wp_clk3", "serviceman/ngn_serviceman", "crosshand_idle2_anim"),
];

const MODELS: &str = "/model/mover/characters/new_generation/new/models/man";
const ANIMATIONS: &str = "/model/mover/characters/new_generation/new/animations/universal";

/// What the mod's clerks are made as now: a mod without it is made again.
const MARK: &str = "# westplay clerks 3";

/// The files a clerk's mover needs: its model and its two animations.
fn clerk_files(model: &str, idle: &str) -> [String; 3] {
    [format!("{MODELS}/{model}_lod0.pmd"), format!("{ANIMATIONS}/{idle}.pma"), format!("{ANIMATIONS}/ngn_walk.pma")]
}

/// The mod's clerks' definitions: only those whose files the game has (`present`). ATS has no security guard: its
/// mover failed to load ("Unable to load model (mover.wp_clk2_idle)") and the game closed minutes later, twice
/// ("Index outside array boundaries" in its movers' models: owner 2026-11-11/12).
fn movers(present: impl Fn(&str) -> bool) -> String {
    // (The mark inside, as a comment: the file starts as every one of the game's does.)
    let mut text = format!("SiiNunit\n{{\n{MARK}\n");
    for (name, model, idle) in CLERKS {
        if !clerk_files(model, idle).iter().all(|path| present(path)) {
            continue;
        }
        for (what, animation) in [("idle", idle), ("walk", "ngn_walk")] {
            text.push_str(&format!(
                "mover_model : mover.{name}_{what}\n{{\n\tmover_name: \"westplay clerk 3 {name} {what}\"\n\tmodel_desc: \"{MODELS}/{model}_lod0.pmd\"\n\tmodel_anim: \"{ANIMATIONS}/{animation}.pma\"\n}}\n\n"
            ));
        }
    }
    text.push_str("}\n");
    text
}

/// The advice to couple, in a language (the game's folder name: `pt_br`), English if not one of these. The game
/// adds the key (": T") after it.
fn attach_words(language: &str) -> &'static str {
    match language.split('_').next().unwrap_or_default() {
        "pt" => "Engate a carreta a pé, na quinta roda (tecla E). Pela cabine não funciona",
        "es" => "Engancha el remolque a pie, en la quinta rueda (tecla E). Desde la cabina no funciona",
        _ => "Couple the trailer on foot, at the fifth wheel (key E). From the cab it doesn't work",
    }
}

/// A language's `local.override.sii` with the advice to couple in it: the game's own (`original`), its keys kept,
/// or a new one.
fn override_with_attach(original: Option<&str>, language: &str) -> String {
    let entry = format!("\tkey[]: \"attach_load\"\n\tval[]: \"{}\"\n", attach_words(language));
    let made = original.and_then(|text| {
        let at = text.find(".localization")?;
        let open = at + text[at..].find('{')? + 1;
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        Some(format!("{}{newline}{}{}", &text[..open], entry.replace('\n', newline), &text[open..]))
    });
    made.unwrap_or_else(|| format!("SiiNunit\n{{\nlocalization_db : .localization\n{{\n{entry}}}\n}}\n"))
}

/// Where the clerks' definitions are in the mod.
pub const MOVERS_FILE: &str = "def/world/mover.westplay_clerk.sii";

/// Whether the clerks' definitions in a mod (its bytes) are as this version makes them: a mod without it is made
/// again.
pub fn movers_current(bytes: Option<&[u8]>) -> bool {
    bytes.is_some_and(|bytes| bytes.windows(MARK.len()).any(|part| part == MARK.as_bytes()))
}

/// The mod's files: the clerks, and the advice to couple in every language the game has.
pub fn files(game: &GameFiles) -> Vec<(String, Vec<u8>)> {
    let present = |path: &str| game.source(path).is_some() || game.read(path).ok().flatten().is_some();
    let mut files = vec![(MOVERS_FILE.to_owned(), movers(present).into_bytes())];
    for language in game.dirs("locale").unwrap_or_default() {
        let language = language.trim_end_matches('/').rsplit('/').next().unwrap_or_default().to_owned();
        if language.is_empty() {
            continue;
        }
        let path = format!("locale/{language}/local.override.sii");
        let original = game.read(&path).ok().flatten().and_then(|bytes| String::from_utf8(bytes).ok());
        files.push((path, override_with_attach(original.as_deref(), &language).into_bytes()));
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each unit name's parts are of 1 to 12 letters (a longer one and the game drops the whole file).
    #[test]
    fn the_clerks_names_are_ones_the_game_reads() {
        let text = movers(|_| true);
        for line in text.lines().filter(|line| line.starts_with("mover_model")) {
            let name = line.rsplit(' ').next().unwrap();
            for part in name.split('.') {
                assert!((1..=12).contains(&part.len()), "{name}");
                assert!(part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'), "{name}");
            }
        }
        assert_eq!(text.matches("mover_model").count(), CLERKS.len() * 2);
        assert!(text.contains("/worker_vest/ngn_worker_vest_lod0.pmd"));
        assert!(text.contains("/universal/ngn_walk.pma"));
    }

    /// A clerk whose model the game hasn't isn't defined (ATS has no security guard); the mark is there either way.
    #[test]
    fn only_the_clerks_the_game_has() {
        let text = movers(|path| !path.contains("security"));
        assert!(!text.contains("wp_clk2") && text.contains("wp_clk1_idle") && text.contains("wp_clk3_walk"));
        assert!(movers_current(Some(text.as_bytes())));
        assert!(movers_current(Some(movers(|_| false).as_bytes())));
        assert!(!movers_current(Some(b"SiiNunit\n{\n}\n")) && !movers_current(None));
    }

    #[test]
    fn the_advice_to_couple_goes_into_the_games_own_overrides() {
        let empty = "SiiNunit\n{\nlocalization_db : .localization\n{\n}\n}\n";
        let made = override_with_attach(Some(empty), "pt_br");
        assert_eq!(
            made,
            "SiiNunit\n{\nlocalization_db : .localization\n{\n\tkey[]: \"attach_load\"\n\tval[]: \"Engate a carreta a pé, na quinta roda (tecla E). Pela cabine não funciona\"\n\n}\n}\n"
        );
        // Its own keys kept.
        let own = "SiiNunit\r\n{\r\nlocalization_db : .localization\r\n{\r\n\tkey[]: \"x\"\r\n\tval[]: \"y\"\r\n}\r\n}\r\n";
        let made = override_with_attach(Some(own), "de_de");
        assert!(made.contains("key[]: \"x\"") && made.contains("Couple the trailer on foot"));
        // None: a new one.
        assert!(override_with_attach(None, "es_la").contains("Engancha el remolque a pie"));
    }
}
