//! The Modo Personagem's people packed into the app (`build.rs`), put in the folder they're read from
//! (`westplay_setup::own_character::dir`) when they aren't there: on a fresh PC the first start has them all, before
//! WestPlay's mod is made (owner 2026-11-07: "format the PC, run the program, and it's all ready").
//!
//! Only files missing, or not the size packed, are written: a person re-exported on this PC since (the same size is
//! taken as the same file) stays as it is, and a file the owner added that the pack doesn't have is left alone.

use std::path::Path;

/// The pack (`build.rs`): empty of files in a build without the people.
const PACK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/people.pack"));

/// Puts the packed people's files that are missing in their folder; how many were written.
pub fn install() -> usize {
    let Some(dir) = westplay_setup::own_character::dir() else { return 0 };
    let mut written = 0;
    for (path, bytes) in files(PACK) {
        let to = dir.join(path);
        if std::fs::metadata(&to).is_ok_and(|m| m.len() == bytes.len() as u64) {
            continue;
        }
        match write(&to, bytes) {
            Ok(()) => written += 1,
            Err(e) => westplay_host::log!("character: couldn't write {}: {e}", to.display()),
        }
    }
    written
}

/// Writes `bytes` beside `to` first, then moves them over it: never half a file there.
fn write(to: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let partial = to.with_extension("partial");
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, to)
}

/// The pack's files, by path (`/`-separated, relative): none if it isn't one.
fn files(pack: &[u8]) -> Vec<(&str, &[u8])> {
    let mut out = Vec::new();
    let Some(mut rest) = pack.strip_prefix(b"WPPK") else { return out };
    while !rest.is_empty() {
        let Some((path, after)) = take(rest, 2) else { break };
        let Some((path, after)) = take(after, u16::from_le_bytes([path[0], path[1]]) as usize) else { break };
        let Ok(path) = std::str::from_utf8(path) else { break };
        let Some((len, after)) = take(after, 4) else { break };
        let Some((bytes, after)) = take(after, u32::from_le_bytes([len[0], len[1], len[2], len[3]]) as usize) else {
            break;
        };
        // Never a path leaving the folder.
        if !path.split('/').any(|part| part.is_empty() || part == "." || part == "..") && !path.contains(':') {
            out.push((path, bytes));
        }
        rest = after;
    }
    out
}

/// `n` bytes off the front of `bytes`, and the rest.
fn take(bytes: &[u8], n: usize) -> Option<(&[u8], &[u8])> {
    (bytes.len() >= n).then(|| bytes.split_at(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_packs_files_and_never_a_path_out_of_the_folder() {
        let mut pack = b"WPPK".to_vec();
        for (path, bytes) in [("model/a.pmd", &b"abc"[..]), ("../x", b"no"), ("automat/b.mat", b"")] {
            pack.extend_from_slice(&(path.len() as u16).to_le_bytes());
            pack.extend_from_slice(path.as_bytes());
            pack.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            pack.extend_from_slice(bytes);
        }
        assert_eq!(files(&pack), vec![("model/a.pmd", &b"abc"[..]), ("automat/b.mat", &b""[..])]);
        assert!(files(b"WPPK").is_empty());
        assert!(files(b"").is_empty());
        // Cut short: what came whole before it.
        assert_eq!(files(&pack[..pack.len() - 20]).len(), 1);
    }
}
