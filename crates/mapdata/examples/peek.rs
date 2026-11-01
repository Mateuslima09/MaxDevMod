//! Dev tool: lists the game's GPS screen scripts and which trucks' interiors name them.
//! `cargo run -p westplay-mapdata --example peek -- <game folder> [truck name part]`

use std::path::Path;

use westplay_mapdata::files::GameFiles;

fn walk(files: &GameFiles, dir: &str, found: &mut Vec<String>) {
    for name in files.list(dir).unwrap_or_default() {
        found.push(format!("{dir}/{name}"));
    }
    for name in files.dirs(dir).unwrap_or_default() {
        walk(files, &format!("{dir}/{name}"), found);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let files = GameFiles::install(Path::new(&args[1])).expect("the game");
    let truck = args.get(2).map(String::as_str).unwrap_or("freightliner");

    let mut ui = Vec::new();
    walk(&files, "/ui/dashboard", &mut ui);
    println!("== GPS scripts in /ui and /ui/dashboard");
    for name in files.list("/ui").unwrap_or_default().iter().filter(|name| name.contains("gps")) {
        println!("/ui/{name}");
    }
    for path in ui.iter().filter(|path| path.contains("gps")) {
        println!("{path}");
    }

    println!("== gps_path in {truck}'s definitions");
    for dir in files.dirs("/def/vehicle/truck").unwrap_or_default().iter().filter(|dir| dir.contains(truck)) {
        let mut defs = Vec::new();
        walk(&files, &format!("/def/vehicle/truck/{dir}"), &mut defs);
        for path in defs {
            let Ok(Some(bytes)) = files.read(&path) else { continue };
            let text = String::from_utf8_lossy(&bytes);
            if !text.is_char_boundary(0) || bytes.starts_with(b"ScsC") {
                println!("{path}: encrypted");
                continue;
            }
            for line in text.lines().filter(|line| line.contains("gps_path") || line.contains("@include")) {
                println!("{path}: {}", line.trim());
            }
        }
    }
}
