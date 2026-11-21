//! The pump's nozzle in the player's hand while filling the tank on foot (owner 2026-11-12: "only the character's
//! arm, holding the pump's nozzle", in first person; no hose to the pump needed, "a standard for the hose").
//!
//! A model of WestPlay's, written here as the flashlight's is (`flashlight`: a descriptor `.pmd` version 4 and a
//! geometry `.pmg` 0x15, none of the game's bytes in them): plain shapes (boxes and tubes) in a few plain colours,
//! the nozzle with its spout, lever and guard, a work glove round its grip, the forearm going back to the player's
//! shoulder, and a length of hose from its back down to the ground. Shown as a `mover_model` the plugin puts by the
//! tank as the filling begins.
//!
//! Its space: metres, `+x` the right, `+y` up, `-z` ahead (the game's); the origin in the grip, the spout ahead.

use westplay_mapdata::token::string_to_token;

const DIR: &str = "model/westplay";
const NAME: &str = "nozzle";
/// Its mover (each part of the name 12 characters at most: the game drops the file otherwise).
pub const MOVER: &str = "mover.wp_nozzle";

/// Its colours (one material each): name, colour (the texture's white times it).
const MATERIALS: [(&str, [f32; 3]); 5] = [
    ("nozzle_body", [0.035, 0.035, 0.04]),
    ("nozzle_metal", [0.62, 0.62, 0.6]),
    ("nozzle_hose", [0.02, 0.02, 0.02]),
    ("nozzle_glove", [0.5, 0.36, 0.2]),
    ("nozzle_sleeve", [0.1, 0.11, 0.14]),
];
const BODY: usize = 0;
const METAL: usize = 1;
const HOSE: usize = 2;
const GLOVE: usize = 3;
const SLEEVE: usize = 4;

/// Its mover: the model, no animation.
fn mover() -> String {
    format!(
        "SiiNunit\n{{\nmover_model : {MOVER}\n{{\n\tmover_name: \"westplay nozzle\"\n\tmodel_desc: \"/{DIR}/{NAME}.pmd\"\n}}\n}}\n"
    )
}

/// A material: plain, its colour on a white texture.
fn material(colour: [f32; 3]) -> String {
    let [r, g, b] = colour;
    format!(
        "effect : \"eut2.dif\" {{\n\tdiffuse : {{ {r:.6} , {g:.6} , {b:.6} }}\n\ttexture : \"texture_base\" {{\n\t\tsource : \"/{DIR}/{NAME}.tobj\"\n\t\tsampler : default\n\t}}\n}}\n"
    )
}

/// The nozzle's files: model, materials, texture and mover (paths in the game's file system).
pub fn files() -> Vec<(String, Vec<u8>)> {
    let mut files = vec![
        (format!("{DIR}/{NAME}.pmd"), descriptor()),
        (format!("{DIR}/{NAME}.pmg"), geometry()),
        (format!("{DIR}/{NAME}.tobj"), crate::game_mod::tobj(&format!("/{DIR}/{NAME}.dds"))),
        (format!("{DIR}/{NAME}.dds"), crate::game_mod::screen_dds(4, 4, &[0xff; 64])),
        ("def/world/mover.westplay_nozzle.sii".into(), mover().into_bytes()),
    ];
    for (name, colour) in MATERIALS {
        files.push((format!("{DIR}/{name}.mat"), material(colour).into_bytes()));
    }
    files
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

fn align(out: &mut Vec<u8>, to: usize) {
    while !out.len().is_multiple_of(to) {
        out.push(0);
    }
}

/// The descriptor: its materials, look `default`, variant `default`, one part, visible (as the flashlight's, with
/// more materials).
fn descriptor() -> Vec<u8> {
    let count = MATERIALS.len() as u32;
    let names: Vec<String> = MATERIALS.iter().map(|(name, _)| format!("/{DIR}/{name}.mat\0")).collect();
    let strings_size: usize = names.iter().map(String::len).sum();
    let (looks, variants, ranges, attributes, values, materials) = (0x40u32, 0x48, 0x50, 0x58, 0x68, 0x6c);
    let strings = materials + 4 * count;
    let mut out = Vec::new();
    words(&mut out, &[4, count, 1, 0, 1, 1, 1, 4, strings_size as u32, looks, variants, ranges, values, attributes]);
    words(&mut out, &[materials, strings]);
    out.extend(token("default").to_le_bytes());
    out.extend(token("default").to_le_bytes());
    words(&mut out, &[0, 1]);
    out.extend(token("visible").to_le_bytes());
    words(&mut out, &[0, 0]);
    words(&mut out, &[1]);
    let mut at = strings;
    for name in &names {
        words(&mut out, &[at]);
        at += name.len() as u32;
    }
    for name in &names {
        out.extend(name.as_bytes());
    }
    out
}

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn normalized(a: V3) -> V3 {
    let length = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if length > 0.0 { scale(a, 1.0 / length) } else { [0.0, 1.0, 0.0] }
}

/// The shapes, as triangles by material (each drawn from both sides, so it's seen whichever way the game takes a
/// triangle's front).
struct Mesh {
    triangles: [Vec<[V3; 3]>; MATERIALS.len()],
}

impl Mesh {
    fn quad(&mut self, material: usize, corners: [V3; 4]) {
        let [a, b, c, d] = corners;
        self.triangles[material].push([a, b, c]);
        self.triangles[material].push([a, c, d]);
    }

    /// A box between two corners, turned about `x` by `pitch` (radians) round `about`.
    fn cuboid(&mut self, material: usize, low: V3, high: V3, pitch: f32, about: V3) {
        let turn = |p: V3| -> V3 {
            let (s, c) = pitch.sin_cos();
            let [x, y, z] = sub(p, about);
            add([x, y * c - z * s, y * s + z * c], about)
        };
        let corner = |i: usize| -> V3 {
            turn([
                if i & 1 == 0 { low[0] } else { high[0] },
                if i & 2 == 0 { low[1] } else { high[1] },
                if i & 4 == 0 { low[2] } else { high[2] },
            ])
        };
        for face in [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]] {
            self.quad(material, face.map(corner));
        }
    }

    /// A tube along `points`, `radius` round, closed at both ends.
    fn tube(&mut self, material: usize, points: &[V3], radius: f32) {
        const SIDES: usize = 10;
        let rings: Vec<Vec<V3>> = points
            .iter()
            .enumerate()
            .map(|(i, &at)| {
                let along = normalized(match i {
                    0 => sub(points[1], points[0]),
                    _ if i == points.len() - 1 => sub(points[i], points[i - 1]),
                    _ => sub(points[i + 1], points[i - 1]),
                });
                let other = if along[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
                let u = normalized(cross(along, other));
                let v = cross(along, u);
                (0..SIDES)
                    .map(|k| {
                        let angle = k as f32 / SIDES as f32 * std::f32::consts::TAU;
                        add(at, add(scale(u, radius * angle.cos()), scale(v, radius * angle.sin())))
                    })
                    .collect()
            })
            .collect();
        for pair in rings.windows(2) {
            for k in 0..SIDES {
                let next = (k + 1) % SIDES;
                self.quad(material, [pair[0][k], pair[0][next], pair[1][next], pair[1][k]]);
            }
        }
        for (ring, centre) in [(&rings[0], points[0]), (&rings[rings.len() - 1], points[points.len() - 1])] {
            for k in 0..SIDES {
                self.triangles[material].push([centre, ring[k], ring[(k + 1) % SIDES]]);
            }
        }
    }
}

/// The nozzle, the glove, the arm and the hose.
fn mesh() -> Mesh {
    let mut mesh = Mesh { triangles: Default::default() };
    // The nozzle's body over the grip, ahead of it; the grip under the body's back (the hand round it).
    mesh.cuboid(BODY, [-0.022, 0.0, -0.13], [0.022, 0.055, 0.06], 0.0, [0.0; 3]);
    mesh.cuboid(BODY, [-0.019, -0.10, -0.015], [0.019, 0.005, 0.035], -0.25, [0.0, 0.0, 0.01]);
    // The guard round the lever, under the body's front, and the lever in it.
    mesh.cuboid(BODY, [-0.012, -0.065, -0.115], [0.012, 0.0, -0.105], 0.0, [0.0; 3]);
    mesh.cuboid(BODY, [-0.012, -0.072, -0.115], [0.012, -0.062, -0.015], 0.0, [0.0; 3]);
    mesh.cuboid(METAL, [-0.007, -0.05, -0.095], [0.007, -0.005, -0.025], 0.15, [0.0, -0.005, -0.06]);
    // The spout: metal, out of the body's front and bending down into the tank.
    mesh.tube(METAL, &[[0.0, 0.03, -0.125], [0.0, 0.026, -0.2], [0.0, 0.0, -0.27], [0.0, -0.05, -0.32]], 0.011);
    // Its collar.
    mesh.tube(METAL, &[[0.0, 0.03, -0.12], [0.0, 0.03, -0.15]], 0.017);
    // The hose out of the body's back, down past the arm to the ground and away behind (the grip some 1.25 m up).
    mesh.tube(
        HOSE,
        &[
            [0.0, 0.03, 0.055],
            [-0.015, 0.045, 0.11],
            [-0.07, 0.0, 0.15],
            [-0.13, -0.25, 0.19],
            [-0.16, -0.7, 0.24],
            [-0.17, -1.1, 0.36],
            [-0.18, -1.22, 0.65],
            [-0.2, -1.23, 1.6],
        ],
        0.019,
    );
    // The glove round the grip: the back of the hand, the fingers wrapped round the front, the thumb on top.
    mesh.cuboid(GLOVE, [-0.042, -0.085, -0.01], [0.04, 0.0, 0.06], -0.25, [0.0, 0.0, 0.01]);
    mesh.cuboid(GLOVE, [-0.036, -0.08, -0.04], [0.036, -0.005, -0.01], -0.25, [0.0, 0.0, 0.01]);
    mesh.cuboid(GLOVE, [-0.03, 0.0, 0.0], [0.01, 0.022, 0.06], 0.0, [0.0; 3]);
    // The cuff and the forearm back to the elbow, the upper arm up to the shoulder (by the player's eyes).
    mesh.tube(GLOVE, &[[0.0, -0.03, 0.055], [0.01, -0.045, 0.1]], 0.042);
    mesh.tube(SLEEVE, &[[0.01, -0.045, 0.1], [0.06, -0.09, 0.24], [0.11, -0.13, 0.36]], 0.045);
    mesh.tube(SLEEVE, &[[0.11, -0.13, 0.36], [0.15, -0.02, 0.44], [0.18, 0.2, 0.5]], 0.052);
    mesh
}

const PMG_MAGIC: u32 = 0x506d_6715;
/// A vertex: place, normal, colour (its middle: as it is), a texture coordinate.
const STRIDE: usize = 36;
const PIECE: usize = 100;

/// The geometry: one part, one piece a material, each triangle both ways, no bones, no locators.
fn geometry() -> Vec<u8> {
    let mesh = mesh();
    // Each piece's vertices: each triangle's corners, then the same the other way round.
    let pieces: Vec<Vec<(V3, V3)>> = mesh
        .triangles
        .iter()
        .map(|triangles| {
            let mut vertices = Vec::new();
            for &[a, b, c] in triangles {
                let normal = normalized(cross(sub(b, a), sub(c, a)));
                vertices.extend([(a, normal), (b, normal), (c, normal)]);
                let back = scale(normal, -1.0);
                vertices.extend([(a, back), (c, back), (b, back)]);
            }
            vertices
        })
        .collect();
    let all = pieces.iter().flatten().map(|(at, _)| *at);
    let (mut low, mut high) = ([f32::MAX; 3], [f32::MIN; 3]);
    for at in all {
        for i in 0..3 {
            low[i] = low[i].min(at[i]);
            high[i] = high[i].max(at[i]);
        }
    }
    let centre = scale(add(low, high), 0.5);
    let radius = {
        let half = scale(sub(high, low), 0.5);
        (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt()
    };
    let bounds = [centre[0], centre[1], centre[2], radius, low[0], low[1], low[2], high[0], high[1], high[2]];

    const HEADER: usize = 0x70;
    const PARTS: usize = HEADER;
    let locators = PARTS + 24;
    let pieces_at = locators;
    let strings = (pieces_at + PIECE * pieces.len()).next_multiple_of(16);
    let vertices_at = strings;
    let vertex_count: usize = pieces.iter().map(Vec::len).sum();
    let vertices_size = vertex_count * STRIDE;
    let indices_at = (vertices_at + vertices_size).next_multiple_of(16);
    let indices_size = (vertex_count * 2).next_multiple_of(4);

    let at = |at: usize| at as u32;
    let mut out = Vec::new();
    words(&mut out, &[PMG_MAGIC, pieces.len() as u32, 1, 0, 0, 0, 0, 0]);
    floats(&mut out, &bounds);
    words(&mut out, &[at(PARTS), at(PARTS), at(locators), at(pieces_at), at(strings), 0]);
    words(&mut out, &[at(vertices_at), at(vertices_size), at(indices_at), at(indices_size)]);
    debug_assert_eq!(out.len(), HEADER);
    // The part: its name, its pieces (all, from the first), no locators.
    out.extend(token("nozzle").to_le_bytes());
    words(&mut out, &[pieces.len() as u32, 0, 0, 0]);
    // The pieces: indices, vertices, texture coordinate 0 for the texture, one set, the material; bounds; stride and
    // where position, normal, texture coordinate and colour are (no second colour, tangent, bones); indices.
    let none = u32::MAX;
    let (mut vertex, mut index) = (vertices_at, indices_at);
    for (material, piece) in pieces.iter().enumerate() {
        words(&mut out, &[piece.len() as u32, piece.len() as u32, 0xffff_fff0, 1, material as u32]);
        floats(&mut out, &bounds);
        words(&mut out, &[at(STRIDE), at(vertex), at(vertex + 12), at(vertex + 28), at(vertex + 24)]);
        words(&mut out, &[none, none, none, none, at(index)]);
        vertex += piece.len() * STRIDE;
        index += piece.len() * 2;
    }
    align(&mut out, 16);
    debug_assert_eq!(out.len(), vertices_at);
    for (at, normal) in pieces.iter().flatten() {
        floats(&mut out, at);
        floats(&mut out, normal);
        out.extend([0x80, 0x80, 0x80, 0xff]);
        floats(&mut out, &[0.5, 0.5]);
    }
    out.resize(indices_at, 0);
    for piece in &pieces {
        for i in 0..piece.len() {
            out.extend((i as u16).to_le_bytes());
        }
    }
    out.resize(indices_at + indices_size, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use westplay_mapdata::buildings;

    #[test]
    fn the_nozzle_is_read_as_its_shapes() {
        let pmg = geometry();
        let triangles = buildings::model_triangles(&pmg).unwrap();
        let made: usize = mesh().triangles.iter().map(Vec::len).sum();
        assert_eq!(triangles.len(), made * 2, "each triangle both ways");
        // Each piece's indices within its own vertices (16 bits).
        for piece in mesh().triangles {
            assert!(piece.len() * 6 < usize::from(u16::MAX));
        }
        // The spout ahead, the hose down to the ground.
        let lowest = triangles.iter().flatten().map(|c| c[1]).fold(f32::MAX, f32::min);
        let foremost = triangles.iter().flatten().map(|c| c[2]).fold(f32::MAX, f32::min);
        assert!(lowest < -1.2 && foremost < -0.3);
    }

    #[test]
    fn the_descriptor_names_each_material() {
        let pmd = descriptor();
        let word = |i: usize| u32::from_le_bytes(pmd[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!([word(0), word(1), word(2), word(4), word(5), word(6)], [4, MATERIALS.len() as u32, 1, 1, 1, 1]);
        let (table, strings) = (word(14) as usize, word(15) as usize);
        assert_eq!(word(8) as usize, pmd.len() - strings);
        for (i, (name, _)) in MATERIALS.iter().enumerate() {
            let at = word(table / 4 + i) as usize;
            assert!(pmd[at..].starts_with(format!("/model/westplay/{name}.mat\0").as_bytes()));
        }
    }

    /// Its files written out to look at (`WESTPLAY_NOZZLE_OUT`): `cargo test -p westplay-setup nozzle -- --ignored`.
    #[test]
    #[ignore]
    fn writes_its_files_out() {
        let Some(dir) = std::env::var_os("WESTPLAY_NOZZLE_OUT") else { return };
        for (path, bytes) in files() {
            let path = std::path::Path::new(&dir).join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
    }

    #[test]
    fn its_mover_names_its_model() {
        let files = files();
        let mover = &files.iter().find(|(path, _)| path.starts_with("def/world/mover.")).unwrap().1;
        assert!(String::from_utf8_lossy(mover).contains("mover_model : mover.wp_nozzle"));
        assert!(String::from_utf8_lossy(mover).contains("model_desc: \"/model/westplay/nozzle.pmd\""));
        assert!(files.iter().filter(|(path, _)| path.ends_with(".mat")).count() == MATERIALS.len());
    }
}
