//! Dev tool: prints one file of a `.scs` archive. `cargo run -p westplay-mapdata --example zipcat -- <archive> <path>`

use westplay_mapdata::archive::Archive;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let archive = Archive::open(std::path::Path::new(&args[1])).expect("the archive");
    let bytes = archive.read(&args[2]).expect("readable").expect("in the archive");
    print!("{}", String::from_utf8_lossy(&bytes));
}
