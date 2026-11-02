//! Prints the units of a class in a save (dev tool): `savepeek <game.sii> <class>`.
fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(class)) = (args.next(), args.next()) else { return };
    let data = std::fs::read(path).unwrap();
    for unit in westplay_mapdata::save::read(&data).unwrap().iter().filter(|unit| unit.class == class) {
        println!("{} : {}", unit.class, unit.name);
        for (key, value) in unit.attributes.iter().take(40) {
            println!("  {key}: {value}");
        }
    }
}
