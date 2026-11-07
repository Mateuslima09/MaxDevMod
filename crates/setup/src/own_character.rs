//! WestPlay's own people for the Modo Personagem (owner 2026-11-05: "a character of our own", then a woman too):
//! each a model, its skeleton and its animations made in Blender and converted with SCS's tools
//! (`docs/personagem/PERSONAGEM_PROPRIO.md`), put in WestPlay's mod with a `mover_model` per animation, as the game's
//! walking passers-by are laid out (`mover.m_walk_06`: a model and an animation each).
//!
//! Not part of WestPlay: the converted files are read from a folder on this PC ([`dir`]), and the mod has a person
//! only when their files are there. Their animations are Mixamo's for now, fine to test with but not to hand out in
//! an open mod (the licence note in that document), so none of them is in WestPlay's sources or its releases.

use std::path::{Path, PathBuf};

/// One of WestPlay's own people: where their files are in the game's file system, their model and the same body
/// without the head (seen in first person, the camera where the head is: owner 2026-11-05, "see the arms, the
/// legs"), and the first part of their movers' names (`mover.<prefix>_<animation>`, and `<prefix>f_` the
/// headless body's: a unit name's parts are 12 characters at most, or the game drops the file); and both with a phone
/// in the right hand, for the phone's animations (`PHONE_ANIMATIONS`; owner 2026-11-05: "not pull the phone out of
/// nowhere").
pub struct Person {
    pub dir: &'static str,
    pub model: &'static str,
    pub body: &'static str,
    pub phone: &'static str,
    pub phone_body: &'static str,
    pub prefix: &'static str,
}

/// WestPlay's own people, in the order the player chooses among them (`westplay_shared::CHARACTER_PEOPLE`: each
/// one's walking mover, `<prefix>_walk`).
pub const PEOPLE: [Person; 3] = [
    // The tattooed man (owner 2026-11-06, in place of the first man, `wp`).
    Person {
        dir: "model/westplay/tatuado",
        model: "wt_player.pmd",
        body: "wt_body.pmd",
        phone: "wt_phone.pmd",
        phone_body: "wt_phbody.pmd",
        prefix: "wt",
    },
    Person {
        dir: "model/westplay/woman",
        model: "wm_player.pmd",
        body: "wm_body.pmd",
        phone: "wm_phone.pmd",
        phone_body: "wm_phbody.pmd",
        prefix: "wm",
    },
    // The cowboy (owner 2026-11-06, the third: his own Mixamo rig, the tattooed man's animations).
    Person {
        dir: "model/westplay/cowboy",
        model: "wc_player.pmd",
        body: "wc_body.pmd",
        phone: "wc_phone.pmd",
        phone_body: "wc_phbody.pmd",
        prefix: "wc",
    },
];

/// The materials' folder (the converter's, for every model).
const MATERIALS: &str = "automat";
/// The animations a person may have, by name (`anim/<name>.pma`): walking, standing, running; stopping, the phone
/// standing and walking with it, jumping on the move (none standing still: the standard, owner 2026-11-06), crouching
/// (the woman's: owner 2026-11-05); and
/// walking into standing, one piece for each eighth of the walk's cycle (`tools/personagem/transicao_parar.py`:
/// owner 2026-11-06, "no sudden change to standing").
const ANIMATIONS: [&str; 12] = [
    "walk", "idle", "run", "stop", "text", "txwalk", "jumprun", "crouch", "settle", "cwalk", "cstart", "csettle",
];
/// The phone's animations: standing with it and walking with it.
const PHONE_ANIMATIONS: [&str; 2] = ["text", "txwalk"];

/// The folder the converted files are read from: `%LOCALAPPDATA%\WestPlay\personagem`, laid out as the game's file
/// system (`model/westplay/<person>/…`, `automat/…`), as SCS's converter leaves them (its `@cache`).
pub fn dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("WestPlay").join("personagem"))
}

/// The people's files for the mod (paths in the game's file system): the converted ones in [`dir`], and their
/// movers. None if no model is there.
pub fn files() -> Vec<(String, Vec<u8>)> {
    dir().map_or_else(Vec::new, |dir| files_in(&dir))
}

/// Every file in [`dir`] by its size and when it was changed: a person re-exported (new animations, a model fixed)
/// is seen by it without reading the files (owner 2026-11-06: the phone fixed in her hand, and the mod not made
/// again until the app was restarted).
pub fn stamp() -> Vec<(PathBuf, u64, Option<std::time::SystemTime>)> {
    fn walk(at: &Path, out: &mut Vec<(PathBuf, u64, Option<std::time::SystemTime>)>) {
        let Ok(entries) = std::fs::read_dir(at) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if let Ok(metadata) = entry.metadata() {
                out.push((path, metadata.len(), metadata.modified().ok()));
            }
        }
    }
    let mut out = Vec::new();
    if let Some(dir) = dir() {
        walk(&dir, &mut out);
    }
    out.sort();
    out
}

/// [`files`], from `dir`.
fn files_in(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let present: Vec<&Person> =
        PEOPLE.iter().filter(|person| dir.join(person.dir).join(person.model).is_file()).collect();
    if present.is_empty() {
        return Vec::new();
    }
    let mut files = Vec::new();
    for person in &present {
        gather(dir, &dir.join(person.dir), &mut files);
    }
    gather(dir, &dir.join(MATERIALS), &mut files);
    files.sort();
    let mut text = String::from("SiiNunit\n{\n");
    for person in present {
        for name in ANIMATIONS
            .into_iter()
            .filter(|name| dir.join(person.dir).join("anim").join(format!("{name}.pma")).is_file())
        {
            // The phone's animations with the phone in the hand, if that model is there.
            let there = |model: &str| dir.join(person.dir).join(model).is_file();
            let phone = PHONE_ANIMATIONS.contains(&name);
            let (model, body) = match phone {
                true => (
                    if there(person.phone) { person.phone } else { person.model },
                    if there(person.phone_body) { person.phone_body } else { person.body },
                ),
                false => (person.model, person.body),
            };
            text += &mover(person.dir, model, person.prefix, name);
            // And in first person, the headless body, if it's there.
            if there(body) {
                text += &mover(person.dir, body, &format!("{}f", person.prefix), name);
            }
        }
    }
    // The game reads the `mover.*.sii` of `/def/world/` (the flashlight's is `mover.westplay.sii`).
    files.push(("def/world/mover.westplay_player.sii".into(), (text + "}\n").into_bytes()));
    files
}

/// Every file under `at`, by its path from `root` (with `/`).
fn gather(root: &Path, at: &Path, files: &mut Vec<(String, Vec<u8>)>) {
    let Ok(entries) = std::fs::read_dir(at) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            gather(root, &path, files);
        } else if let (Ok(relative), Ok(bytes)) = (path.strip_prefix(root), std::fs::read(&path)) {
            let name = relative.components().map(|part| part.as_os_str().to_string_lossy()).collect::<Vec<_>>();
            files.push((name.join("/"), bytes));
        }
    }
}

/// A mover for one animation: a person's model (in `dir`), played with it, named `<prefix>_<animation>`.
fn mover(dir: &str, model: &str, prefix: &str, animation: &str) -> String {
    format!(
        "mover_model : mover.{prefix}_{animation}\n{{\n\tmover_name: \"westplay {prefix} {animation}\"\n\tmodel_desc: \"/{dir}/{model}\"\n\tmodel_anim: \"/{dir}/anim/{animation}.pma\"\n}}\n\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_people_the_player_walks_as_are_ours_walking() {
        let people = westplay_shared::CHARACTER_PEOPLE;
        assert_eq!(people.len(), PEOPLE.len());
        for (name, person) in people.iter().zip(&PEOPLE) {
            assert_eq!(*name, format!("{}_walk", person.prefix));
        }
    }

    /// The game drops a whole `.sii` with a unit name it can't read ("is in wrong format"): each part 1 to 12 of
    /// `a-z`, `0-9`, `_` (ETS2 1.61 dropped `mover.wp_player_walk` and the flashlight's `mover.wp_flashlight`).
    #[test]
    fn every_unit_name_of_ours_is_one_the_game_reads() {
        let readable = |unit: &str| {
            unit.split('.').all(|part| {
                (1..=12).contains(&part.len())
                    && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            })
        };
        let mut units: Vec<String> = PEOPLE
            .iter()
            .flat_map(|person| {
                ANIMATIONS.iter().flat_map(move |name| {
                    [format!("mover.{}_{name}", person.prefix), format!("mover.{}f_{name}", person.prefix)]
                })
            })
            .collect();
        units.extend([crate::flashlight::MOVER.to_owned(), crate::flashlight::LIGHT.to_owned()]);
        units.extend(westplay_shared::CHARACTER_PEOPLE.iter().map(|name| format!("mover.{name}")));
        for unit in &units {
            assert!(readable(unit), "{unit}");
        }
        assert!(!readable("mover.wp_player_walk"));
    }

    #[test]
    fn no_model_no_person() {
        let dir = std::env::temp_dir().join("westplay_own_character_none");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(files_in(&dir).is_empty());
    }

    #[test]
    fn each_persons_converted_files_and_a_mover_per_animation() {
        let dir = std::env::temp_dir().join("westplay_own_character_some");
        let _ = std::fs::remove_dir_all(&dir);
        let [man, woman, _cowboy] = &PEOPLE;
        for person in [man, woman] {
            std::fs::create_dir_all(dir.join(person.dir).join("anim")).unwrap();
            std::fs::write(dir.join(person.dir).join(person.model), b"pmd").unwrap();
            std::fs::write(dir.join(person.dir).join("anim/walk.pma"), b"pma").unwrap();
        }
        std::fs::write(dir.join(woman.dir).join("anim/text.pma"), b"pma").unwrap();
        std::fs::write(dir.join(man.dir).join(man.body), b"pmd").unwrap();
        std::fs::create_dir_all(dir.join("automat/e0")).unwrap();
        std::fs::write(dir.join("automat/e0/a.mat"), b"mat").unwrap();
        let files = files_in(&dir);
        let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "automat/e0/a.mat",
                "model/westplay/tatuado/anim/walk.pma",
                "model/westplay/tatuado/wt_body.pmd",
                "model/westplay/tatuado/wt_player.pmd",
                "model/westplay/woman/anim/text.pma",
                "model/westplay/woman/anim/walk.pma",
                "model/westplay/woman/wm_player.pmd",
                "def/world/mover.westplay_player.sii"
            ]
        );
        let movers = String::from_utf8(files.last().unwrap().1.clone()).unwrap();
        assert!(movers.contains("mover_model : mover.wt_walk\n"));
        assert!(movers.contains("mover_model : mover.wm_walk\n") && movers.contains("mover_model : mover.wm_text\n"));
        assert!(movers.contains("model_anim: \"/model/westplay/woman/anim/text.pma\""));
        assert!(!movers.contains("wt_idle") && !movers.contains("wt_text"));
        // The man's headless body walks in first person; the woman has none here.
        assert!(
            movers.contains(
                "mover_model : mover.wtf_walk
"
            ) && movers.contains("model_desc: \"/model/westplay/tatuado/wt_body.pmd\"")
        );
        assert!(!movers.contains("mover.wmf_"));
    }

    #[test]
    fn the_phone_animations_have_the_phone_in_the_hand() {
        let dir = std::env::temp_dir().join("westplay_own_character_phone");
        let _ = std::fs::remove_dir_all(&dir);
        let woman = &PEOPLE[1];
        std::fs::create_dir_all(dir.join(woman.dir).join("anim")).unwrap();
        for model in [woman.model, woman.body, woman.phone, woman.phone_body] {
            std::fs::write(dir.join(woman.dir).join(model), b"pmd").unwrap();
        }
        for anim in ["walk", "text"] {
            std::fs::write(dir.join(woman.dir).join(format!("anim/{anim}.pma")), b"pma").unwrap();
        }
        let movers = String::from_utf8(files_in(&dir).last().unwrap().1.clone()).unwrap();
        let model_of = |unit: &str| {
            let at = movers
                .find(&format!(
                    "mover.{unit}
"
                ))
                .unwrap();
            movers[at..].lines().find_map(|line| line.trim().strip_prefix("model_desc: ")).unwrap().to_owned()
        };
        assert!(model_of("wm_text").ends_with("wm_phone.pmd\""));
        assert!(model_of("wmf_text").ends_with("wm_phbody.pmd\""));
        assert!(model_of("wm_walk").ends_with("wm_player.pmd\""));
    }

    #[test]
    fn only_the_people_whose_model_is_there() {
        let dir = std::env::temp_dir().join("westplay_own_character_woman");
        let _ = std::fs::remove_dir_all(&dir);
        let woman = &PEOPLE[1];
        std::fs::create_dir_all(dir.join(woman.dir).join("anim")).unwrap();
        std::fs::write(dir.join(woman.dir).join(woman.model), b"pmd").unwrap();
        std::fs::write(dir.join(woman.dir).join("anim/walk.pma"), b"pma").unwrap();
        let movers = String::from_utf8(files_in(&dir).last().unwrap().1.clone()).unwrap();
        assert!(movers.contains("mover.wm_walk") && !movers.contains("mover.wt_"));
    }
}
