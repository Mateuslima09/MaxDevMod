//! Dev tool: lists a game directory's folders and files. `cargo run -p westplay-mapdata --example ls -- <game folder> <dir>`

use std::path::Path;

use westplay_mapdata::files::GameFiles;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let files = GameFiles::install(Path::new(&args[1])).expect("the game");
    for dir in files.dirs(&args[2]).unwrap_or_default() {
        println!("{dir}/");
    }
    for file in files.list(&args[2]).unwrap_or_default() {
        println!("{file}");
    }
}
