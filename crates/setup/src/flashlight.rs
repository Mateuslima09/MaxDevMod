//! The Modo Personagem's flashlight (owner 2026-11-05: "add a flashlight to use"): a light of the game's own (a
//! `flare_blink` carrying a light, as its blinking signs' are; a `light_source`, as its street lamps', lit nothing
//! on a mover: see [`light`]), on a model of WestPlay's that is nothing but a speck and a locator hooking that light,
//! shown as a `mover_model` the plugin puts in the player's hand, pointed where they look.
//!
//! Laid out as the game's own lit mover is (`mover.light_flare`, `/asset/animation/nescript/light_flare.pmd`: two
//! specks, each with a locator hooking a flare): a descriptor (`.pmd`, version 4) and a geometry (`.pmg`, version
//! 0x15) written here, none of the game's bytes in them.

use westplay_mapdata::token::string_to_token;

/// Where the flashlight's files are, in the game's file system.
const DIR: &str = "model/westplay";
const NAME: &str = "flashlight";
/// Its light (a hookup unit: each part of the name a token) and its mover.
pub const LIGHT: &str = "wp.flashlight";
pub const MOVER: &str = "mover.wp_torch";

/// The light, for now a test. A `light_source` on a mover lit nothing, even a bright point (owner's test,
/// 2026-11-05, ETS2 1.61). The game's one mover with hookups (`mover.light_flare`) hooks `flare_blink`s, and a
/// `flare_blink` can carry a light of its own (`flare.model.red.arrow2.blink0`: `type: spot`, lumen, a range). So:
/// a `flare_blink` always on (`blink_pattern: "X"`), a small white flare (the glow in the hand: its hookup is
/// there) with a bright point all around the hand, 8000 lumen to 12 m (the ground lit: its light too). The torch's
/// cone (2500 lumen, 30 m, 12 and 40 degrees) once that's known.
fn light() -> String {
    format!(
        "SiiNunit\n{{\nflare_blink : {LIGHT}\n{{\n\tsetup: lumen_hue_saturation\n\tambient_color: (0, 0, 0)\n\tdiffuse_color: (8000, 40, 12)\n\tspecular_color: (8000, 40, 12)\n\ttype: point\n\trange: 12.0\n\tforward_distance: false\n\n\tdir_type: all\n\n\tblink_pattern: \"X\"\n\tblink_step_length: 1.0\n\tstate_change_duration: 0.001\n\n\tmodel: \"/model/flare/lamp_flare_small.pmd\"\n}}\n}}\n"
    )
}

/// Its mover: the model, no animation.
fn mover() -> String {
    format!(
        "SiiNunit\n{{\nmover_model : {MOVER}\n{{\n\tmover_name: \"westplay flashlight\"\n\tmodel_desc: \"/{DIR}/{NAME}.pmd\"\n}}\n}}\n"
    )
}

/// Its speck's material: a plain one on a transparent texture.
fn material() -> String {
    format!(
        "effect : \"eut2.dif\" {{\n\tdiffuse : {{ 1.000000 , 1.000000 , 1.000000 }}\n\ttexture : \"texture_base\" {{\n\t\tsource : \"/{DIR}/{NAME}.tobj\"\n\t\tsampler : default\n\t}}\n}}\n"
    )
}

/// The flashlight's files: model, material, texture, light and mover (paths in the game's file system).
pub fn files() -> Vec<(String, Vec<u8>)> {
    vec![
        (format!("{DIR}/{NAME}.pmd"), descriptor()),
        (format!("{DIR}/{NAME}.pmg"), geometry()),
        (format!("{DIR}/{NAME}.mat"), material().into_bytes()),
        (format!("{DIR}/{NAME}.tobj"), crate::game_mod::tobj(&format!("/{DIR}/{NAME}.dds"))),
        (format!("{DIR}/{NAME}.dds"), crate::game_mod::screen_dds(4, 4, &[0; 64])),
        // The game reads every hookup in `/unit/hookup/`, and the `mover.*.sii` of `/def/world/` (its own DLCs'
        // are so named).
        ("unit/hookup/westplay_flashlight.sii".into(), light().into_bytes()),
        ("def/world/mover.westplay.sii".into(), mover().into_bytes()),
    ]
}

fn token(name: &str) -> u64 {
    string_to_token(name).expect("a token")
}

fn words(out: &mut Vec<u8>, words: &[u32]) {
    for word in words {
        out.extend(word.to_le_bytes());
    }
}

fn floats(out: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        out.extend(value.to_le_bytes());
    }
}

/// The descriptor: one material, look `default`, variant `default`, one part, visible.
fn descriptor() -> Vec<u8> {
    let material = format!("/{DIR}/{NAME}.mat\0");
    // Where each table is: looks, variants, parts' ranges, attributes, variants' values, materials, strings.
    let (looks, variants, ranges, attributes, values, materials, strings) =
        (0x40u32, 0x48, 0x50, 0x58, 0x68, 0x6c, 0x70);
    let mut out = Vec::new();
    // Version, materials, looks, pieces, variants, parts, attributes, a variant's values' bytes; the strings'
    // bytes; then where the tables are.
    words(&mut out, &[4, 1, 1, 0, 1, 1, 1, 4, material.len() as u32, looks, variants, ranges, values, attributes]);
    words(&mut out, &[materials, strings]);
    out.extend(token("default").to_le_bytes());
    out.extend(token("default").to_le_bytes());
    // The part's attributes: the first one, one of them.
    words(&mut out, &[0, 1]);
    // `visible`: a number, at the values' start.
    out.extend(token("visible").to_le_bytes());
    words(&mut out, &[0, 0]);
    // The variant: visible.
    words(&mut out, &[1]);
    words(&mut out, &[strings]);
    out.extend(material.as_bytes());
    out
}

/// A speck's corners (metres): a triangle a millimetre across, at the origin.
const SPECK: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [0.001, 0.0, 0.0], [0.0, 0.0, -0.001]];
const PMG_MAGIC: u32 = 0x506d_6715;

fn align(out: &mut Vec<u8>, to: usize) {
    while !out.len().is_multiple_of(to) {
        out.push(0);
    }
}

/// The geometry: one part (`light`) with one piece (the speck) and one locator (`light`, at the origin, not
/// turned) hooking [`LIGHT`].
fn geometry() -> Vec<u8> {
    const HEADER: usize = 0x70;
    const PARTS: usize = HEADER;
    const LOCATORS: usize = PARTS + 24;
    const PIECES: usize = LOCATORS + 44;
    let hookup = format!("{LIGHT}\0");
    let strings = (PIECES + 100).next_multiple_of(16);
    let strings_size = hookup.len().next_multiple_of(4);
    let vertices = (strings + strings_size).next_multiple_of(16);
    const STRIDE: usize = 28;
    let vertices_size = 3 * STRIDE;
    let indices = (vertices + vertices_size).next_multiple_of(16);
    let indices_size = 8;
    // Its bounds: a sphere (centre, radius), then a box (least, greatest corner).
    let bounds = [0.0005, 0.0, -0.0005, 0.0008, 0.0, 0.0, -0.001, 0.001, 0.0, 0.0];

    let mut out = Vec::new();
    // Version, pieces, parts, bones, weight width, locators, two more counts.
    words(&mut out, &[PMG_MAGIC, 1, 1, 0, 0, 1, 0, 0]);
    floats(&mut out, &bounds);
    // Where the bones, parts, locators, pieces, strings (and their size), vertices (size), indices (size) are.
    let at = |at: usize| at as u32;
    words(&mut out, &[at(PARTS), at(PARTS), at(LOCATORS), at(PIECES), at(strings), at(strings_size)]);
    words(&mut out, &[at(vertices), at(vertices_size), at(indices), at(indices_size)]);
    debug_assert_eq!(out.len(), HEADER);
    // The part: its name, one piece (the first), one locator (the first).
    out.extend(token("light").to_le_bytes());
    words(&mut out, &[1, 0, 1, 0]);
    // The locator: its name, place, scale, turn (w first), its hookup (at the strings' start).
    out.extend(token("light").to_le_bytes());
    floats(&mut out, &[0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
    words(&mut out, &[0]);
    // The piece: indices, vertices, no texture coordinate mask, no bones, material 0, its bounds, the vertices'
    // stride and where their position, normal, (no texture coordinate) and colour are, nothing else, its indices.
    let none = u32::MAX;
    words(&mut out, &[3, 3, none, 0, 0]);
    floats(&mut out, &bounds);
    words(&mut out, &[at(STRIDE), at(vertices), at(vertices + 12), none, at(vertices + 24), none, none, none, none]);
    words(&mut out, &[at(indices)]);
    align(&mut out, 16);
    debug_assert_eq!(out.len(), strings);
    out.extend(hookup.as_bytes());
    out.resize(strings + strings_size, 0);
    align(&mut out, 16);
    for corner in SPECK {
        floats(&mut out, &corner);
        floats(&mut out, &[0.0, 1.0, 0.0]);
        out.extend([0, 0, 0, 0xff]);
    }
    align(&mut out, 16);
    for index in [0u16, 1, 2] {
        out.extend(index.to_le_bytes());
    }
    out.resize(indices + indices_size, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use westplay_mapdata::buildings;

    #[test]
    fn the_flashlight_is_a_speck_hooking_its_light() {
        let pmg = geometry();
        let locators = buildings::model_locators(&pmg).unwrap();
        assert_eq!(locators.len(), 1);
        assert_eq!(locators[0].name, "light");
        assert_eq!(locators[0].rotation, [1.0, 0.0, 0.0, 0.0]);
        let triangles = buildings::model_triangles(&pmg).unwrap();
        assert_eq!(triangles, vec![SPECK]);
        // The hookup's name, where the locator says (the strings' start).
        let strings = u32::from_le_bytes(pmg[22 * 4..23 * 4].try_into().unwrap()) as usize;
        assert!(pmg[strings..].starts_with(b"wp.flashlight\0"));
    }

    #[test]
    fn the_descriptor_names_its_material_and_shows_its_part() {
        let pmd = descriptor();
        let word = |i: usize| u32::from_le_bytes(pmd[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!([word(0), word(1), word(2), word(4), word(5), word(6)], [4, 1, 1, 1, 1, 1]);
        let strings = word(15) as usize;
        assert_eq!(&pmd[strings..], b"/model/westplay/flashlight.mat\0");
        assert_eq!(word(8) as usize, pmd.len() - strings);
        // The game's lit mover's `visible` attribute, the same token.
        assert_eq!(token("visible"), 0x0a_ed71_b9ce);
        assert_eq!(word(word(12) as usize / 4), 1);
    }

    #[test]
    fn its_light_and_mover_are_units_the_game_reads() {
        let files = files();
        let light = &files.iter().find(|(path, _)| path.starts_with("unit/hookup/")).unwrap().1;
        let light = String::from_utf8_lossy(light);
        assert!(light.contains("flare_blink : wp.flashlight") && light.contains("range:"));
        assert!(light.contains("blink_pattern: \"X\"") && light.contains("model: \"/model/flare/"));
        let mover = &files.iter().find(|(path, _)| path.starts_with("def/world/mover.")).unwrap().1;
        assert!(String::from_utf8_lossy(mover).contains("model_desc: \"/model/westplay/flashlight.pmd\""));
        assert!(LIGHT.split('.').all(|part| string_to_token(part).is_some()));
    }
}
