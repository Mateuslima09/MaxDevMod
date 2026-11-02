//! Dev tool: prints a file from whichever installed mod has it. `cargo run -p westplay-setup --example modfile -- <path>`
use westplay_mapdata::archive::Archive;
use westplay_mapdata::files::{GameFiles, Mount};

fn main() {
    let path = std::env::args().nth(1).expect("a path");
    let game = westplay_setup::game::find().into_iter().next().expect("the game");
    let documents = westplay_setup::places::game_documents().expect("documents");
    for (file, _) in westplay_setup::profiles::installed_files(&documents, Some(&game)) {
        let mount = if file.is_dir() { Mount::Folder(file.clone()) } else { Mount::Archive(Archive::open(&file).unwrap()) };
        if let Some(bytes) = GameFiles::new(vec![mount]).read(&path).ok().flatten() {
            println!("== {}", file.display());
            println!("{}", String::from_utf8_lossy(&bytes));
        }
    }
}
