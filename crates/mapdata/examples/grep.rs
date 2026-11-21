//! Dev tool: lists the game's text files under a directory whose content has a word, with the lines that have it.
//! `cargo run -p westplay-mapdata --example grep -- <game folder> <dir> <word> [extension]`

use std::path::Path;

use westplay_mapdata::files::GameFiles;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let files = GameFiles::install(Path::new(&args[1])).expect("the game");
    let word = args[3].to_lowercase();
    let extension = args.get(4).cloned().unwrap_or_else(|| ".sii".to_owned());
    let mut stack = vec![args[2].trim_end_matches('/').to_owned()];
    let mut seen = 0usize;
    while let Some(dir) = stack.pop() {
        for sub in files.dirs(&dir).unwrap_or_default() {
            stack.push(if dir.is_empty() { sub } else { format!("{dir}/{sub}") });
        }
        for file in files.list(&dir).unwrap_or_default() {
            if !file.ends_with(&extension) {
                continue;
            }
            let path = if dir.is_empty() { file } else { format!("{dir}/{file}") };
            let Ok(Some(bytes)) = files.read(&path) else { continue };
            seen += 1;
            let text = String::from_utf8_lossy(&bytes);
            let hits: Vec<&str> = text.lines().filter(|line| line.to_lowercase().contains(&word)).take(6).collect();
            if !hits.is_empty() {
                println!("== {path}");
                for hit in hits {
                    println!("   {}", hit.trim());
                }
            }
        }
    }
    eprintln!("{seen} files read");
}
