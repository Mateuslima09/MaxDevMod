//! Reads a vehicle interior's 3D GPS screen mesh (`.pmd` + `.mat` + `.pmg`) to measure:
//! 1. Which sub-rectangle (`ShownPart`) of the GPS render target (`gps.tobj`) the display glass maps, and
//! 2. The physical width, height, and aspect ratio of that 3D display in the cabin.

use std::collections::HashMap;

use westplay_mapdata::buildings;
use westplay_mapdata::files::GameFiles;
use westplay_mapdata::token::string_to_token;

use crate::game_mod::ShownPart;

const PMG_MAGIC: u32 = 0x506d_6715;
const PIECE_COUNT_AT: usize = 4;
const PIECES_AT: usize = 84;
const PIECE_SIZE: usize = 100;

const PIECE_INDICES: usize = 0;
const PIECE_VERTICES: usize = 1;
const PIECE_MATERIAL: usize = 4;
const PIECE_STRIDE: usize = 15;
const PIECE_POSITIONS: usize = 16;
const PIECE_UVS: usize = 18;
const PIECE_INDEX_AT: usize = 24;

/// Small outward UV pad (`0.6%`) applied when placing MaxDevMod's texture over a sub-rectangle of a GPS script so
/// sub-pixel rounding at the 3D display's edge never leaks a line of the underlying stock GPS.
pub const EDGE_BLEED: f64 = 0.006;

/// Measured geometry and UV bounds of a GPS display piece inside an interior `.pmd`/`.pmg` model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenMesh {
    /// Portion of the GPS UI window shown by the mesh (fractions from left and top, `0.0..=1.0`).
    pub shown: ShownPart,
    /// Physical width and height of the display in metres.
    pub width_m: f32,
    pub height_m: f32,
}

impl ScreenMesh {
    /// Physical width-to-height aspect ratio of the display (`width_m / height_m`).
    pub fn aspect(&self) -> f64 {
        if self.height_m > 1e-4 { f64::from(self.width_m) / f64::from(self.height_m) } else { 16.0 / 9.0 }
    }

    /// `shown` expanded outward by `bleed` on each side (clamped to `0.0..=1.0`), preventing edge leaks.
    pub fn padded_shown(&self, bleed: f64) -> ShownPart {
        ShownPart {
            left: (self.shown.left - bleed).clamp(0.0, 1.0),
            right: (self.shown.right + bleed).clamp(0.0, 1.0),
            top: (self.shown.top - bleed).clamp(0.0, 1.0),
            bottom: (self.shown.bottom + bleed).clamp(0.0, 1.0),
        }
    }

    /// Whether `shown` covers virtually the entire `[0, 1] x [0, 1]` GPS script (`>= 96%` each way).
    pub fn covers_whole_canvas(&self) -> bool {
        self.shown.left <= 0.02 && self.shown.right >= 0.98 && self.shown.top <= 0.02 && self.shown.bottom >= 0.98
    }
}

/// Caches `.mat` and `(model, variant)` lookups across interiors so scanning an entire game + mods is fast.
#[derive(Default)]
pub struct MeshCache {
    mats: HashMap<String, bool>,
    models: HashMap<(String, Option<u64>), Option<ScreenMesh>>,
}

impl MeshCache {
    /// Reads the GPS screen mesh of `pmd_path` (in `variant`, e.g. `"standard"`, if specified) from `files`
    /// (falling back to `fallback` for shared `/automat/...` or base-game assets).
    pub fn lookup(
        &mut self,
        files: &GameFiles,
        fallback: Option<&GameFiles>,
        pmd_path: &str,
        variant: Option<&str>,
    ) -> Option<ScreenMesh> {
        let pmd_clean = pmd_path.trim().trim_matches('"').trim_start_matches('/');
        if pmd_clean.is_empty() {
            return None;
        }
        let variant_tok = variant.and_then(|v| {
            let clean = v.trim().trim_matches('"');
            (!clean.is_empty()).then(|| string_to_token(clean)).flatten()
        });
        let key = (pmd_clean.to_owned(), variant_tok);
        if let Some(&cached) = self.models.get(&key) {
            return cached;
        }
        let found = read_model_gps_mesh_cached(files, fallback, pmd_clean, variant_tok, &mut self.mats);
        self.models.insert(key, found);
        found
    }
}

fn word(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

fn float(data: &[u8], at: usize) -> Option<f32> {
    word(data, at).map(f32::from_bits).filter(|v| v.is_finite())
}

/// Whether a `.mat` file draws the vehicle's live GPS render target (`gps.tobj`).
fn is_gps_material(mat_text: &str) -> bool {
    mat_text.lines().any(|line| {
        let trimmed = line.trim();
        (trimmed.starts_with("source") || trimmed.starts_with("texture")) && trimmed.contains("gps.tobj")
    })
}

/// Resolves a relative texture/material path inside `.mat` or `.pmd` against `base_file`.
fn resolve_path(base_file: &str, rel: &str) -> String {
    if let Some(stripped) = rel.strip_prefix('/') {
        return stripped.to_owned();
    }
    match base_file.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/{rel}"),
        None => rel.to_owned(),
    }
}

/// Standalone helper without an external cache.
pub fn read_model_gps_mesh(
    files: &GameFiles,
    fallback: Option<&GameFiles>,
    pmd_path: &str,
    variant: Option<&str>,
) -> Option<ScreenMesh> {
    MeshCache::default().lookup(files, fallback, pmd_path, variant)
}

fn read_model_gps_mesh_cached(
    files: &GameFiles,
    fallback: Option<&GameFiles>,
    pmd_clean: &str,
    variant: Option<u64>,
    mat_cache: &mut HashMap<String, bool>,
) -> Option<ScreenMesh> {
    let read_any = |path: &str| -> Option<Vec<u8>> {
        files.read(path).ok().flatten().or_else(|| fallback.and_then(|fb| fb.read(path).ok().flatten()))
    };
    let pmd = read_any(pmd_clean)?;
    let materials = buildings::pmd_materials(&pmd);
    if materials.is_empty() {
        return None;
    }
    let gps_mats: Vec<usize> = materials
        .iter()
        .enumerate()
        .filter_map(|(idx, mat_path)| {
            let resolved = resolve_path(pmd_clean, mat_path);
            let is_gps = if let Some(&cached) = mat_cache.get(&resolved) {
                cached
            } else {
                let Some(bytes) = read_any(&resolved) else {
                    mat_cache.insert(resolved, false);
                    return None;
                };
                let ok = is_gps_material(&String::from_utf8_lossy(&bytes));
                mat_cache.insert(resolved, ok);
                ok
            };
            is_gps.then_some(idx)
        })
        .collect();
    if gps_mats.is_empty() {
        return None;
    }
    let pmg_path = pmd_clean.strip_suffix(".pmd")?.to_owned() + ".pmg";
    let pmg = read_any(&pmg_path)?;
    if word(&pmg, 0) != Some(PMG_MAGIC) {
        return None;
    }
    let visible = variant.and_then(|v| buildings::visible_pieces(&pmd, &pmg, v));
    let piece_count = word(&pmg, PIECE_COUNT_AT)? as usize;
    let table = word(&pmg, PIECES_AT)? as usize;

    let mut best: Option<(f32, ScreenMesh)> = None;
    for piece in 0..piece_count {
        if visible.as_ref().is_some_and(|vis| !vis.get(piece).copied().unwrap_or(true)) {
            continue;
        }
        let rec = table.checked_add(piece.checked_mul(PIECE_SIZE)?)?;
        let field = |w: usize| word(&pmg, rec + w * 4).map(|v| v as usize);
        let mat_idx = field(PIECE_MATERIAL)?;
        if !gps_mats.contains(&mat_idx) {
            continue;
        }
        let (indices, vertices, stride, pos_at, uv_at, _idx_at) = (
            field(PIECE_INDICES)?,
            field(PIECE_VERTICES)?,
            field(PIECE_STRIDE)?,
            field(PIECE_POSITIONS)?,
            field(PIECE_UVS)?,
            field(PIECE_INDEX_AT)?,
        );
        if vertices < 3 || indices < 3 || uv_at == u32::MAX as usize {
            continue;
        }
        let pos_step = if stride == 0 { 12 } else { stride };
        let uv_step = if stride == 0 { 8 } else { stride };

        let mut u_min = f32::INFINITY;
        let mut u_max = f32::NEG_INFINITY;
        let mut v_min = f32::INFINITY;
        let mut v_max = f32::NEG_INFINITY;
        let mut pts = Vec::with_capacity(vertices);
        let mut uvs = Vec::with_capacity(vertices);

        let mut valid = true;
        for v_idx in 0..vertices {
            let (Some(p_off), Some(u_off)) =
                (pos_at.checked_add(v_idx * pos_step), uv_at.checked_add(v_idx * uv_step))
            else {
                valid = false;
                break;
            };
            let (Some(px), Some(py), Some(pz), Some(u), Some(v)) = (
                float(&pmg, p_off),
                float(&pmg, p_off + 4),
                float(&pmg, p_off + 8),
                float(&pmg, u_off),
                float(&pmg, u_off + 4),
            ) else {
                valid = false;
                break;
            };
            u_min = u_min.min(u);
            u_max = u_max.max(u);
            v_min = v_min.min(v);
            v_max = v_max.max(v);
            pts.push([px, py, pz]);
            uvs.push([u, v]);
        }
        if !valid || u_max - u_min < 0.05 || v_max - v_min < 0.05 {
            continue;
        }
        let (width_m, height_m) = physical_size_from_uvs(&pts, &uvs, [u_min, u_max], [v_min, v_max]);
        let aspect = if height_m > 1e-4 { width_m / height_m } else { 0.0 };
        // Real in-cabin displays are between 4 cm and 55 cm across, with aspect ratios between 0.5:1 and 4.0:1.
        if !(0.04..=0.55).contains(&width_m) || !(0.03..=0.45).contains(&height_m) || !(0.5..=4.0).contains(&aspect) {
            continue;
        }
        let area = width_m * height_m;

        // Normalize UVs into the [0, 1] unit square by removing any integer UV tile offset.
        let tile_u = ((u_min + u_max) * 0.5).floor();
        let tile_v = ((v_min + v_max) * 0.5).floor();
        let u0 = (u_min - tile_u).clamp(0.0, 1.0);
        let u1 = (u_max - tile_u).clamp(0.0, 1.0);
        let v0 = (v_min - tile_v).clamp(0.0, 1.0);
        let v1 = (v_max - tile_v).clamp(0.0, 1.0);
        let left = f64::from(u0);
        let right = f64::from(u1);
        let v_at_top = top_v(&pts, &uvs) - tile_v;
        let (top, bottom) = if (v_at_top - v0).abs() < (v_at_top - v1).abs() {
            (f64::from(v0), f64::from(v1))
        } else {
            (f64::from((1.0 - v1).clamp(0.0, 1.0)), f64::from((1.0 - v0).clamp(0.0, 1.0)))
        };
        if right - left < 0.05 || bottom - top < 0.05 {
            continue;
        }
        let mesh = ScreenMesh { shown: ShownPart { left, right, top, bottom }, width_m, height_m };
        if best.is_none_or(|(best_area, _)| area > best_area) {
            best = Some((area, mesh));
        }
    }
    best.map(|(_, mesh)| mesh)
}

fn top_v(pts: &[[f32; 3]], uvs: &[[f32; 2]]) -> f32 {
    pts.iter()
        .zip(uvs.iter())
        .max_by(|(a, _), (b, _)| a[1].partial_cmp(&b[1]).unwrap_or(std::cmp::Ordering::Equal))
        .map_or(0.0, |(_, uv)| uv[1])
}

fn physical_size_from_uvs(pts: &[[f32; 3]], uvs: &[[f32; 2]], [u0, u1]: [f32; 2], [v0, v1]: [f32; 2]) -> (f32, f32) {
    let n = pts.len() as f64;
    let mean_u = uvs.iter().map(|uv| f64::from(uv[0])).sum::<f64>() / n;
    let mean_v = uvs.iter().map(|uv| f64::from(uv[1])).sum::<f64>() / n;
    let mut mean_p = [0.0f64; 3];
    for p in pts {
        for k in 0..3 {
            mean_p[k] += f64::from(p[k]) / n;
        }
    }
    let (mut suu, mut svv, mut suv) = (0.0f64, 0.0f64, 0.0f64);
    let (mut spu, mut spv) = ([0.0f64; 3], [0.0f64; 3]);
    for (p, uv) in pts.iter().zip(uvs.iter()) {
        let du = f64::from(uv[0]) - mean_u;
        let dv = f64::from(uv[1]) - mean_v;
        suu += du * du;
        svv += dv * dv;
        suv += du * dv;
        for k in 0..3 {
            let dp = f64::from(p[k]) - mean_p[k];
            spu[k] += dp * du;
            spv[k] += dp * dv;
        }
    }
    let det = suu * svv - suv * suv;
    if det.abs() < 1e-10 {
        return (0.0, 0.0);
    }
    let mut d_du = [0.0f64; 3];
    let mut d_dv = [0.0f64; 3];
    for k in 0..3 {
        d_du[k] = (spu[k] * svv - spv[k] * suv) / det;
        d_dv[k] = (spv[k] * suu - spu[k] * suv) / det;
    }
    let len = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() as f32;
    (len(d_du) * (u1 - u0), len(d_dv) * (v1 - v0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_shown_bounds_outward_within_unit_square() {
        let mesh = ScreenMesh {
            shown: ShownPart { left: 0.004, right: 0.989, top: 0.142, bottom: 0.790 },
            width_m: 0.163,
            height_m: 0.089,
        };
        let padded = mesh.padded_shown(EDGE_BLEED);
        assert_eq!(padded.left, 0.0);
        assert!((padded.right - 0.995).abs() < 1e-6);
        assert!((padded.top - 0.136).abs() < 1e-6);
        assert!((padded.bottom - 0.796).abs() < 1e-6);
        assert!((mesh.aspect() - (0.163 / 0.089)).abs() < 1e-4);
    }

    #[test]
    #[ignore = "reads installed ETS2 and ATS archives"]
    fn scan_installed_games_screens() {
        for game in crate::game::find() {
            println!("\n=== {:?} ({}) ===", game.kind, game.dir.display());
            let files = GameFiles::install(&game.dir).expect("game files");
            let mut cache = MeshCache::default();
            for root in ["def/vehicle/truck", "def/vehicle/car"] {
                let Ok(vehicles) = files.dirs(root) else { continue };
                for vehicle in vehicles {
                    let int_dir = format!("{root}/{vehicle}/interior");
                    let Ok(names) = files.list(&int_dir) else { continue };
                    let mut seen = std::collections::BTreeSet::new();
                    for name in names.iter().filter(|n| n.ends_with(".sii")) {
                        let path = format!("{int_dir}/{name}");
                        let Some(bytes) = files.read(&path).ok().flatten() else { continue };
                        let mut full = String::from_utf8_lossy(&bytes).into_owned();
                        for line in full.clone().lines() {
                            if let Some(inc) = line.trim().strip_prefix("@include")
                                && let Some(file) = inc.split('"').nth(1)
                            {
                                let inc_path = if let Some(abs) = file.strip_prefix('/') {
                                    abs.to_owned()
                                } else {
                                    format!("{int_dir}/{file}")
                                };
                                if let Some(ib) = files.read(&inc_path).ok().flatten() {
                                    full.push('\n');
                                    full.push_str(&String::from_utf8_lossy(&ib));
                                }
                            }
                        }
                        let field = |key: &str| -> Option<String> {
                            full.lines().find_map(|l| {
                                let rest = l.trim().strip_prefix(key)?.trim().strip_prefix(':')?;
                                Some(rest.trim().trim_matches('"').to_owned())
                            })
                        };
                        let gps = field("gps_path").unwrap_or_else(|| "(none)".into());
                        let size = field("gps_drawable_size").unwrap_or_else(|| "(default 256x256)".into());
                        let variant = field("variant");
                        let model = field("model").unwrap_or_default();
                        let anim = field("animated_model").unwrap_or_default();
                        let mesh = cache
                            .lookup(&files, None, &model, variant.as_deref())
                            .or_else(|| cache.lookup(&files, None, &anim, variant.as_deref()));
                        let sig = format!("{gps} | {size} | {mesh:?}");
                        if seen.insert(sig)
                            && let Some(m) = mesh
                        {
                            println!(
                                "  {vehicle:<26} ({name:<22}) gps={gps:<42} size={size:<18} shown=[L:{:.3} R:{:.3} T:{:.3} B:{:.3}] {:.1}x{:.1}cm ({:.3}:1)",
                                m.shown.left,
                                m.shown.right,
                                m.shown.top,
                                m.shown.bottom,
                                m.width_m * 100.0,
                                m.height_m * 100.0,
                                m.aspect()
                            );
                        }
                    }
                }
            }
        }
    }
}
