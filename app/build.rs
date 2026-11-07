use std::path::{Path, PathBuf};

fn main() {
    // The app bundles the built UI (`ui/dist`, tauri.conf.json's resources), which `tauri build` makes first
    // (`npm run build:release`) and which isn't in the repository. Without it, `cargo check` or `cargo test`
    // on a fresh clone would fail here: an empty folder stands in until the UI is built.
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/dist");
    if !dist.exists() {
        std::fs::create_dir_all(&dist).expect("creating ui/dist");
    }
    pack_people();
    tauri_build::build()
}

/// The Modo Personagem's people (models, animations, materials), packed into the app so a fresh PC has them on the
/// first start (owner 2026-11-07: "format the PC, run the program, and it's all there"). Read at build time from
/// `WESTPLAY_PERSONAGEM`, or else the folder the app reads them from (`%LOCALAPPDATA%\WestPlay\personagem`). They're
/// never in the repository (their animations are Mixamo's: `docs/personagem/PERSONAGEM_PROPRIO.md`), only in the
/// owner's private builds. None there: an empty pack, and the app works without people, as before.
///
/// The pack: `WPPK`, then each file as its path's length (u16), its path (`/`-separated, UTF-8), its length (u32)
/// and its bytes, all little-endian (`src/people.rs` reads it).
fn pack_people() {
    println!("cargo:rerun-if-env-changed=WESTPLAY_PERSONAGEM");
    let source = std::env::var_os("WESTPLAY_PERSONAGEM").map(PathBuf::from).or_else(|| {
        std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("WestPlay").join("personagem"))
    });
    let mut files = Vec::new();
    if let Some(source) = source.as_deref().filter(|dir| dir.is_dir()) {
        println!("cargo:rerun-if-changed={}", source.display());
        gather(source, source, &mut files);
    }
    files.sort();
    let mut pack = b"WPPK".to_vec();
    for (path, bytes) in &files {
        pack.extend_from_slice(&u16::try_from(path.len()).expect("a short path").to_le_bytes());
        pack.extend_from_slice(path.as_bytes());
        pack.extend_from_slice(&u32::try_from(bytes.len()).expect("a file under 4 GB").to_le_bytes());
        pack.extend_from_slice(bytes);
    }
    if files.is_empty() {
        println!("cargo:warning=no Modo Personagem people packed (none in {source:?})");
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("people.pack");
    std::fs::write(out, pack).expect("writing the people's pack");
}

/// Every file under `at`, by its path from `root`.
fn gather(root: &Path, at: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    for entry in std::fs::read_dir(at).expect("reading the people's folder").flatten() {
        let path = entry.path();
        if path.is_dir() {
            gather(root, &path, out);
        } else {
            let name = path.strip_prefix(root).expect("under the root").to_string_lossy().replace('\\', "/");
            out.push((name, std::fs::read(&path).expect("reading a person's file")));
        }
    }
}
