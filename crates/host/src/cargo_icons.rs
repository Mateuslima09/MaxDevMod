//! On-demand cargo icon loader & DDS/DXT decoder for the phone's Job Market ("Ver Detalhes").
//!
//! Lazily mounts the active game's `.scs` archives (base + DLCs) and the player's `mod/` folder on a
//! background worker thread so the WebView2 UI thread never stalls. Resolves `.mat` -> `.tobj` ->
//! HashFS v2 texture entries or HashFS v1 / ZIP `.dds` files (RGBA, BGRA, DXT1/BC1, DXT3/BC2,
//! DXT5/BC3), encodes to PNG, and returns cached `data:image/png;base64,...` data URLs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use serde_json::json;
use westplay_mapdata::archive::{Archive, RawTexture};
use westplay_mapdata::files::{GameFiles, Mount};
use westplay_mapdata::texture::Texture;

struct IconRequest {
    id: u32,
    icon: String,
    cargo_id: String,
    game_dir: Option<PathBuf>,
    mod_dir: Option<PathBuf>,
}

struct IconReply {
    id: u32,
    data_url: Option<String>,
}

pub struct CargoIcons {
    tx: Sender<IconRequest>,
    rx: Receiver<IconReply>,
}

impl CargoIcons {
    pub fn start() -> Self {
        let (tx, req_rx) = mpsc::channel::<IconRequest>();
        let (rep_tx, rx) = mpsc::channel::<IconReply>();
        let _ = std::thread::Builder::new().name("westplay-cargo-icons".into()).spawn(move || {
            let mut state = WorkerState::default();
            while let Ok(req) = req_rx.recv() {
                let data_url = state.resolve(&req.icon, &req.cargo_id, req.game_dir.as_deref(), req.mod_dir.as_deref());
                if rep_tx.send(IconReply { id: req.id, data_url }).is_err() {
                    break;
                }
            }
        });
        Self { tx, rx }
    }

    pub fn ask(&self, id: u32, icon: String, cargo_id: String, game_dir: Option<PathBuf>) {
        let mod_dir = westplay_setup::places::game_documents().map(|d| d.join("mod"));
        let _ = self.tx.send(IconRequest { id, icon, cargo_id, game_dir, mod_dir });
    }

    /// Drains any ready icon replies as JSON messages for the pages.
    pub fn poll_all(&self) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(reply) = self.rx.try_recv() {
            out.push(json!({ "type": "jobMarketIcon", "id": reply.id, "dataUrl": reply.data_url }).to_string());
        }
        out
    }
}

#[derive(Default)]
struct WorkerState {
    key: Option<(Option<PathBuf>, Option<PathBuf>)>,
    files: Option<GameFiles>,
    cache: HashMap<String, Option<String>>,
}

impl WorkerState {
    fn ensure_mounts(&mut self, game_dir: Option<&Path>, mod_dir: Option<&Path>) {
        let new_key = (game_dir.map(Path::to_path_buf), mod_dir.map(Path::to_path_buf));
        if self.key.as_ref() == Some(&new_key) && self.files.is_some() {
            return;
        }
        let mut mounts = Vec::new();
        if let Some(dir) = game_dir {
            // Base archives first
            for name in ["base.scs", "base_share.scs", "base_vehicle.scs"] {
                let p = dir.join(name);
                if p.is_file()
                    && let Ok(arch) = Archive::open(&p)
                {
                    mounts.push(Mount::Archive(arch));
                }
            }
            // DLC archives (heavy cargo, trailers, agriculture, construction, etc.)
            if let Ok(entries) = std::fs::read_dir(dir) {
                let mut dlcs: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_file()
                            && p.file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.starts_with("dlc_") && n.ends_with(".scs"))
                    })
                    .collect();
                dlcs.sort();
                for p in dlcs {
                    if let Ok(arch) = Archive::open(&p) {
                        mounts.push(Mount::Archive(arch));
                    }
                }
            }
        }
        // User mods last so custom map/cargo mods (e.g. Mapa Minas Gerais, EAA, Promods) override base assets.
        if let Some(mdir) = mod_dir
            && let Ok(entries) = std::fs::read_dir(mdir)
        {
            let mut mods: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            mods.sort();
            for p in mods {
                if p.is_dir() {
                    mounts.push(Mount::Folder(p));
                } else if p.is_file() {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                    if matches!(ext.as_str(), "scs" | "zip")
                        && let Ok(arch) = Archive::open(&p)
                    {
                        mounts.push(Mount::Archive(arch));
                    }
                }
            }
        }
        log!("[job_market] cargo icon mounts loaded: {} archive(s)/mod(s)", mounts.len());
        self.files = Some(GameFiles::new(mounts));
        self.key = Some(new_key);
        self.cache.clear();
    }

    fn resolve(&mut self, icon: &str, cargo_id: &str, game_dir: Option<&Path>, mod_dir: Option<&Path>) -> Option<String> {
        self.ensure_mounts(game_dir, mod_dir);
        let cache_key = format!("{}|{}", icon.trim(), cargo_id.trim());
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }
        let files = self.files.as_ref()?;
        let found = find_cargo_texture(files, icon, cargo_id).map(|(path, tex)| {
            log!("[job_market] cargo icon '{cargo_id}' (icon='{icon}') -> {path} ({}x{})", tex.width, tex.height);
            let png = tex.png();
            format!("data:image/png;base64,{}", base64_encode(&png))
        });
        if found.is_none() {
            log!("[job_market] cargo icon '{cargo_id}' (icon='{icon}') -> not found in mounts");
        }
        self.cache.insert(cache_key, found.clone());
        found
    }
}

fn candidate_stems(icon: &str, cargo_id: &str) -> Vec<String> {
    let mut stems: Vec<String> = Vec::new();
    let mut push_unique = |s: String| {
        if !stems.contains(&s) {
            stems.push(s);
        }
    };
    let mut push_name = |raw: &str| {
        let s = raw.trim().trim_start_matches('/');
        if s.is_empty() || s.eq_ignore_ascii_case("fallback") {
            return;
        }
        let s = s
            .strip_suffix(".mat")
            .or_else(|| s.strip_suffix(".tobj"))
            .or_else(|| s.strip_suffix(".dds"))
            .unwrap_or(s);
        if s.eq_ignore_ascii_case("fallback") {
            return;
        }
        if s.contains('/') {
            push_unique(format!("/{s}"));
            if !s.starts_with("material/") {
                push_unique(format!("/material/ui/{s}"));
            }
        } else {
            // ETS2/ATS truck cargos live primarily in `/material/ui/cargo_icons/<id>.mat` and `/material/ui/cargo_logo/`.
            push_unique(format!("/material/ui/cargo_icons/{s}"));
            push_unique(format!("/material/ui/cargo_logo/{s}"));
            push_unique(format!("/material/ui/cargo/{s}"));
        }
    };

    // Prefer explicit non-fallback `icon`, then exact `cargo_id`, then `cargo_id` without trailing digits
    // (many map mods reuse `caixavazia.mat` for `caixavazia12`, `plasticom.mat` for `plasticom2`, etc.).
    push_name(icon);
    push_name(cargo_id);
    let stripped = cargo_id.trim().trim_end_matches(|c: char| c.is_ascii_digit());
    if !stripped.is_empty() && stripped != cargo_id.trim() {
        push_name(stripped);
    }
    stems
}

fn find_cargo_texture(files: &GameFiles, icon: &str, cargo_id: &str) -> Option<(String, Texture)> {
    for stem in candidate_stems(icon, cargo_id) {
        // 1. Try `.mat` first (it points to the exact `.tobj` even when renamed)
        let mat_path = format!("{stem}.mat");
        if let Ok(Some(mat_bytes)) = files.read(&mat_path)
            && let Some(tex_rel) = parse_mat_texture(&mat_bytes)
        {
            let tobj = resolve_relative_path(&mat_path, &tex_rel);
            if let Some(tex) = load_tobj_or_dds(files, &tobj) {
                return Some((tobj, tex));
            }
        }
        // 2. Direct `.tobj`
        let tobj_path = format!("{stem}.tobj");
        if let Some(tex) = load_tobj_or_dds(files, &tobj_path) {
            return Some((tobj_path, tex));
        }
        // 3. Direct `.dds`
        let dds_path = format!("{stem}.dds");
        if let Ok(Some(dds)) = files.read(&dds_path)
            && let Ok(tex) = decode_dds(&dds)
        {
            return Some((dds_path, tex));
        }
    }
    None
}

fn load_tobj_or_dds(files: &GameFiles, tobj_path: &str) -> Option<Texture> {
    // 1. HashFS v2 direct texture entry (most base game & DLC icons)
    if let Ok(Some(raw)) = files.read_texture_top(tobj_path)
        && let Ok(tex) = decode_raw_texture(&raw)
    {
        return Some(tex);
    }
    // 2. HashFS v1 / ZIP / Folder: `.tobj` contains the `.dds` path inside its binary payload
    if let Ok(Some(tobj_bytes)) = files.read(tobj_path) {
        if let Some(dds_rel) = parse_tobj_dds_path(&tobj_bytes) {
            let dds_path = resolve_relative_path(tobj_path, &dds_rel);
            if let Ok(Some(dds_bytes)) = files.read(&dds_path)
                && let Ok(tex) = decode_dds(&dds_bytes)
            {
                return Some(tex);
            }
        }
    }
    // 3. Fallback: same path with `.dds` extension
    let dds_fallback = tobj_path.strip_suffix(".tobj").map(|s| format!("{s}.dds")).unwrap_or_else(|| tobj_path.to_owned());
    if let Ok(Some(dds_bytes)) = files.read(&dds_fallback)
        && let Ok(tex) = decode_dds(&dds_bytes)
    {
        return Some(tex);
    }
    None
}

fn parse_mat_texture(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("texture")
            && (rest.starts_with(':') || rest.starts_with(' ') || rest.starts_with('\t'))
            && let Some(first_q) = rest.find('"')
        {
            let after = &rest[first_q + 1..];
            if let Some(end_q) = after.find('"') {
                let val = after[..end_q].trim();
                if !val.is_empty() {
                    return Some(val.to_owned());
                }
            }
        }
    }
    None
}

fn parse_tobj_dds_path(bytes: &[u8]) -> Option<String> {
    // Binary TOBJ has a 40-byte header followed by length (u32) + padding (u32) + ASCII path ending in `.dds`.
    // Text TOBJ (some older mods) has lines containing `.dds`.
    if bytes.len() >= 48 && bytes[0..4] == [0x01, 0x0A, 0xB1, 0x70] {
        let len = u32::from_le_bytes(bytes[40..44].try_into().ok()?) as usize;
        if len > 0 && len < 260 && 48 + len <= bytes.len() {
            let s = String::from_utf8_lossy(&bytes[48..48 + len]).trim().to_owned();
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    // Fallback: scan printable ASCII substring ending in `.dds`
    let text = String::from_utf8_lossy(bytes);
    for token in text.split(|c: char| c.is_control() || c == '"' || c == '\'' || c.is_whitespace()) {
        if token.to_ascii_lowercase().ends_with(".dds") && token.len() >= 5 {
            return Some(token.to_owned());
        }
    }
    None
}

fn resolve_relative_path(base_file: &str, rel: &str) -> String {
    let rel = rel.replace('\\', "/");
    if rel.starts_with('/') {
        return rel;
    }
    let dir = base_file.rsplit_once('/').map_or("", |(d, _)| d);
    if dir.is_empty() {
        format!("/{rel}")
    } else {
        format!("{dir}/{rel}")
    }
}

// ---------------------------------------------------------------------------
// Texture & DDS/BC1/BC2/BC3 decoding
// ---------------------------------------------------------------------------

fn aligned_pitch(row_bytes: usize, row_alignment: usize) -> usize {
    let align = row_alignment.max(1);
    row_bytes.div_ceil(align) * align
}

fn decode_raw_texture(raw: &RawTexture) -> Result<Texture, String> {
    let w = raw.width as usize;
    let h = raw.height as usize;
    if w == 0 || h == 0 || w > 2048 || h > 2048 {
        return Err(format!("invalid texture size {w}x{h}"));
    }
    let bw = w.div_ceil(4);
    let align = raw.row_alignment as usize;
    match raw.format {
        28 | 29 | 87 | 91 => {
            Texture::from_pixels(raw.width, raw.height, raw.format, &raw.data, Some(aligned_pitch(w * 4, align)))
        }
        71 | 72 => decode_bc1(w, h, &raw.data, aligned_pitch(bw * 8, align)),
        74 | 75 => decode_bc2(w, h, &raw.data, aligned_pitch(bw * 16, align)),
        77 | 78 => decode_bc3(w, h, &raw.data, aligned_pitch(bw * 16, align)),
        other => Err(format!("unsupported DXGI format {other}")),
    }
}

fn decode_dds(data: &[u8]) -> Result<Texture, String> {
    if data.len() < 128 || &data[..4] != b"DDS " {
        return Err("not a DDS file".into());
    }
    let height = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let width = u32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;
    if width == 0 || height == 0 || width > 2048 || height > 2048 {
        return Err(format!("invalid DDS size {width}x{height}"));
    }
    let fourcc = &data[84..88];
    let bw = width.div_ceil(4);
    match fourcc {
        b"DXT1" => decode_bc1(width, height, &data[128..], bw * 8),
        b"DXT3" => decode_bc2(width, height, &data[128..], bw * 16),
        b"DXT5" => decode_bc3(width, height, &data[128..], bw * 16),
        b"DX10" => {
            if data.len() < 148 {
                return Err("truncated DX10 header".into());
            }
            let fmt = u32::from_le_bytes(data[128..132].try_into().unwrap());
            decode_raw_texture(&RawTexture {
                width: width as u32,
                height: height as u32,
                format: fmt,
                row_alignment: 1,
                data: data[148..].to_vec(),
            })
        }
        _ => {
            // Try standard 32-bit RGBA/BGRA first, then 24-bit BGR/RGB.
            if let Ok(tex) = Texture::from_dds(data) {
                return Ok(tex);
            }
            let bits = u32::from_le_bytes(data[88..92].try_into().unwrap());
            let rmask = u32::from_le_bytes(data[92..96].try_into().unwrap());
            if bits == 24 && data.len() >= 128 + width * height * 3 {
                let bgr = rmask == 0x00ff_0000;
                let mut rgba = Vec::with_capacity(width * height * 4);
                for px in data[128..128 + width * height * 3].chunks_exact(3) {
                    if bgr {
                        rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
                    } else {
                        rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                    }
                }
                return Ok(Texture { width: width as u32, height: height as u32, rgba });
            }
            Err("unsupported DDS format".into())
        }
    }
}

fn rgb565_to_rgb8(c: u16) -> [u8; 3] {
    let r = ((c >> 11) & 0x1f) as u8;
    let g = ((c >> 5) & 0x3f) as u8;
    let b = (c & 0x1f) as u8;
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

fn bc_colors(c0: u16, c1: u16, opaque_mode: bool) -> [[u8; 4]; 4] {
    let [r0, g0, b0] = rgb565_to_rgb8(c0);
    let [r1, g1, b1] = rgb565_to_rgb8(c1);
    if opaque_mode || c0 > c1 {
        [
            [r0, g0, b0, 255],
            [r1, g1, b1, 255],
            [
                ((2 * r0 as u16 + r1 as u16) / 3) as u8,
                ((2 * g0 as u16 + g1 as u16) / 3) as u8,
                ((2 * b0 as u16 + b1 as u16) / 3) as u8,
                255,
            ],
            [
                ((r0 as u16 + 2 * r1 as u16) / 3) as u8,
                ((g0 as u16 + 2 * g1 as u16) / 3) as u8,
                ((b0 as u16 + 2 * b1 as u16) / 3) as u8,
                255,
            ],
        ]
    } else {
        [
            [r0, g0, b0, 255],
            [r1, g1, b1, 255],
            [((r0 as u16 + r1 as u16) / 2) as u8, ((g0 as u16 + g1 as u16) / 2) as u8, ((b0 as u16 + b1 as u16) / 2) as u8, 255],
            [0, 0, 0, 0],
        ]
    }
}

fn decode_bc1(width: usize, height: usize, data: &[u8], pitch: usize) -> Result<Texture, String> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let row_stride = pitch.max(bw * 8);
    if data.len() < row_stride * (bh - 1) + bw * 8 {
        return Err("truncated BC1 data".into());
    }
    let mut rgba = vec![0u8; width * height * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let off = by * row_stride + bx * 8;
            let c0 = u16::from_le_bytes([data[off], data[off + 1]]);
            let c1 = u16::from_le_bytes([data[off + 2], data[off + 3]]);
            let bits = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap());
            let pal = bc_colors(c0, c1, false);
            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height {
                    continue;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width {
                        continue;
                    }
                    let idx = ((bits >> ((py * 4 + px) * 2)) & 3) as usize;
                    let dst = (y * width + x) * 4;
                    rgba[dst..dst + 4].copy_from_slice(&pal[idx]);
                }
            }
        }
    }
    Ok(Texture { width: width as u32, height: height as u32, rgba })
}

fn decode_bc2(width: usize, height: usize, data: &[u8], pitch: usize) -> Result<Texture, String> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let row_stride = pitch.max(bw * 16);
    if data.len() < row_stride * (bh - 1) + bw * 16 {
        return Err("truncated BC2 data".into());
    }
    let mut rgba = vec![0u8; width * height * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let off = by * row_stride + bx * 16;
            let alpha_bits = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
            let c0 = u16::from_le_bytes([data[off + 8], data[off + 9]]);
            let c1 = u16::from_le_bytes([data[off + 10], data[off + 11]]);
            let bits = u32::from_le_bytes(data[off + 12..off + 16].try_into().unwrap());
            let pal = bc_colors(c0, c1, true);
            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height {
                    continue;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width {
                        continue;
                    }
                    let i = py * 4 + px;
                    let idx = ((bits >> (i * 2)) & 3) as usize;
                    let a4 = ((alpha_bits >> (i * 4)) & 0xf) as u8;
                    let dst = (y * width + x) * 4;
                    rgba[dst..dst + 3].copy_from_slice(&pal[idx][..3]);
                    rgba[dst + 3] = (a4 << 4) | a4;
                }
            }
        }
    }
    Ok(Texture { width: width as u32, height: height as u32, rgba })
}

fn decode_bc3(width: usize, height: usize, data: &[u8], pitch: usize) -> Result<Texture, String> {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    let row_stride = pitch.max(bw * 16);
    if data.len() < row_stride * (bh - 1) + bw * 16 {
        return Err("truncated BC3 data".into());
    }
    let mut rgba = vec![0u8; width * height * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let off = by * row_stride + bx * 16;
            let a0 = data[off];
            let a1 = data[off + 1];
            let mut apal = [0u8; 8];
            apal[0] = a0;
            apal[1] = a1;
            if a0 > a1 {
                for k in 1..=6 {
                    apal[k + 1] = (((7 - k) as u16 * a0 as u16 + k as u16 * a1 as u16) / 7) as u8;
                }
            } else {
                for k in 1..=4 {
                    apal[k + 1] = (((5 - k) as u16 * a0 as u16 + k as u16 * a1 as u16) / 5) as u8;
                }
                apal[6] = 0;
                apal[7] = 255;
            }
            let mut a_indices = 0u64;
            for b in 0..6 {
                a_indices |= (data[off + 2 + b] as u64) << (b * 8);
            }
            let c0 = u16::from_le_bytes([data[off + 8], data[off + 9]]);
            let c1 = u16::from_le_bytes([data[off + 10], data[off + 11]]);
            let bits = u32::from_le_bytes(data[off + 12..off + 16].try_into().unwrap());
            let pal = bc_colors(c0, c1, true);
            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height {
                    continue;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width {
                        continue;
                    }
                    let i = py * 4 + px;
                    let idx = ((bits >> (i * 2)) & 3) as usize;
                    let a_idx = ((a_indices >> (i * 3)) & 7) as usize;
                    let dst = (y * width + x) * 4;
                    rgba[dst..dst + 3].copy_from_slice(&pal[idx][..3]);
                    rgba[dst + 3] = apal[a_idx];
                }
            }
        }
    }
    Ok(Texture { width: width as u32, height: height as u32, rgba })
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((triple >> 6) & 0x3f) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(triple & 0x3f) as usize] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_fallback_and_prefers_cargo_icons_and_stripped_digits() {
        let stems = candidate_stems("fallback", "caixavazia12");
        assert!(!stems.iter().any(|s| s.contains("fallback")));
        assert_eq!(stems[0], "/material/ui/cargo_icons/caixavazia12");
        assert!(stems.contains(&"/material/ui/cargo_icons/caixavazia".to_string()));
    }

    #[test]
    fn parses_mat_and_relative_paths() {
        let mat = br#"material : "ui" {
    texture : "apples.tobj"
    texture_name : "texture"
}"#;
        assert_eq!(parse_mat_texture(mat).as_deref(), Some("apples.tobj"));
        assert_eq!(resolve_relative_path("/material/ui/cargo/apples.mat", "apples.tobj"), "/material/ui/cargo/apples.tobj");
        assert_eq!(resolve_relative_path("/material/ui/cargo/a.mat", "/material/ui/other.tobj"), "/material/ui/other.tobj");
    }

    #[test]
    fn decodes_bc1_and_bc3_blocks_and_encodes_base64() {
        // Pure red (0xF800) BC1 4x4 block (all pixels index 0)
        let bc1 = [0x00, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let tex = decode_bc1(4, 4, &bc1, 8).unwrap();
        assert_eq!(&tex.rgba[0..4], &[255, 0, 0, 255]);

        // Pure blue (0x001F) BC3 4x4 block with alpha 0x80 (index 0)
        let bc3 = [0x80, 0x00, 0, 0, 0, 0, 0, 0, 0x1F, 0x00, 0x00, 0x00, 0, 0, 0, 0];
        let tex3 = decode_bc3(4, 4, &bc3, 16).unwrap();
        assert_eq!(&tex3.rgba[0..4], &[0, 0, 255, 0x80]);

        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"M"), "TQ==");
    }
}
