//! Dev tool: prints one game file. `cargo run -p westplay-mapdata --example cat -- <game folder> <path>`

use std::path::Path;

use westplay_mapdata::files::GameFiles;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let files = GameFiles::install(Path::new(&args[1])).expect("the game");
    let bytes = files.read(&args[2]).expect("readable").expect("in the game");
    print!("{}", String::from_utf8_lossy(&bytes));
}
