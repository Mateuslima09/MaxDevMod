//! What WestPlay's setup did, kept between runs (`setup.json`, in WestPlay's config folder): the game it set
//! WestPlay up in, the version of WestPlay that put its plugin and mod there, and whether the player went
//! through the companion's onboarding to its end. Nothing about the player: no profile, no mod, no account.

use std::io;
use std::path::{Path, PathBuf};

/// The file, in WestPlay's config folder.
pub const FILE_NAME: &str = "setup.json";

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Record {
    /// The game's folder WestPlay is set up in (its plugin, its mod, its map), once it is.
    pub game: Option<PathBuf>,
    /// The version of WestPlay that put its plugin and mod into that game: another one's (an update since)
    /// are put in again.
    pub version: Option<String>,
    /// The onboarding was gone through to its end: the companion opens on its own pages from then on.
    pub finished: bool,
    /// The other game WestPlay is set up in too (owner 2026-11-02: WestPlay in ETS2 as in ATS), each with the version
    /// that put its plugin and mod there: made [`Record::game`] when it's played (`Record::switch`).
    pub other_games: Vec<OtherGame>,
}

/// A game WestPlay is set up in that isn't the one it works with now.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OtherGame {
    pub game: PathBuf,
    pub version: Option<String>,
}

impl Record {
    /// Makes the game in `dir` the one WestPlay works with, set up by `version` (`None`: kept as it was, of `dir` among
    /// the others); the one it was, unless it's `dir` or the same game as it (`same_game`: another install), kept
    /// among the others, as is any other one of another game.
    pub fn switch(&mut self, dir: PathBuf, version: Option<String>, same_game: impl Fn(&Path, &Path) -> bool) {
        let kept = self.other_games.iter().position(|other| other.game == dir).map(|at| self.other_games.remove(at));
        let version = version.or_else(|| kept.and_then(|other| other.version));
        if let Some(previous) = self.game.take()
            && previous != dir
            && !same_game(&previous, &dir)
        {
            self.other_games.push(OtherGame { game: previous, version: self.version.take() });
        }
        self.other_games.retain(|other| !same_game(&other.game, &dir));
        self.game = Some(dir);
        self.version = version;
    }

    /// The record in `dir`: none yet (a first run), or one that can't be read, is a fresh one.
    pub fn load(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(FILE_NAME))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Keeps the record in `dir`, written beside the one there first, then moved over it.
    pub fn save(&self, dir: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let partial = dir.join(format!("{FILE_NAME}.partial"));
        std::fs::write(&partial, serde_json::to_string_pretty(self).map_err(io::Error::other)?)?;
        std::fs::rename(&partial, dir.join(FILE_NAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_between_the_games_keeping_the_other() {
        // The same game: the names' first letters alike.
        let same = |a: &Path, b: &Path| a.to_string_lossy()[..1] == b.to_string_lossy()[..1];
        let mut record = Record { game: Some("ats".into()), version: Some("2.1".into()), ..Record::default() };
        record.switch("ets2".into(), Some("2.2".into()), same);
        assert_eq!(record.game.as_deref(), Some(Path::new("ets2")));
        assert_eq!(record.version.as_deref(), Some("2.2"));
        assert_eq!(record.other_games, [OtherGame { game: "ats".into(), version: Some("2.1".into()) }]);
        // Back to ATS as it's played: its version as it was.
        record.switch("ats".into(), None, same);
        assert_eq!((record.game.as_deref(), record.version.as_deref()), (Some(Path::new("ats")), Some("2.1")));
        assert_eq!(record.other_games, [OtherGame { game: "ets2".into(), version: Some("2.2".into()) }]);
        // Another install of ATS: the old one forgotten.
        record.switch("another-ats".into(), Some("2.3".into()), same);
        assert_eq!(record.other_games, [OtherGame { game: "ets2".into(), version: Some("2.2".into()) }]);
    }

    #[test]
    fn keeps_what_setup_did() {
        let dir = std::env::temp_dir().join(format!("westplay-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Record::load(&dir), Record::default());

        let record = Record {
            game: Some(PathBuf::from(r"D:\SteamLibrary\steamapps\common\ETS2")),
            version: Some("1.0.0".into()),
            finished: true,
            other_games: vec![OtherGame { game: PathBuf::from(r"D:ATS"), version: Some("1.0.0".into()) }],
        };
        record.save(&dir).unwrap();
        assert_eq!(Record::load(&dir), record);

        // One it can't read starts again.
        std::fs::write(dir.join(FILE_NAME), "{ not json").unwrap();
        assert_eq!(Record::load(&dir), Record::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
