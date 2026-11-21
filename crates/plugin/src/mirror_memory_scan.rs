//! Where the game keeps its mirrors' cameras in its memory (the reversing camera's step 3, owner 2026-11-14:
//! "like the walking mode moves the person, move the camera"; docs/camera-de-re/RETROVISOR.md).
//!
//! The camera probe (`mirror_camera_probe`) sees each mirror's view in its draws: an object at the camera (the
//! sky's, its translation nil) gives the mirror's turn alone, as a 3 x 3 matrix. Here, after the frame, the game's
//! memory is searched for that turn: its rows and its columns (three floats in a row, or three doubles) and its
//! quaternion (both signs, x y z w and w x y z). Around each place found, the floats are written down (where the
//! camera's position, near the turn, would be), the ones near the truck's position (the SDK's) marked. The places
//! are kept, and the next capture says what they hold then: the live one follows the truck as it turns.
//!
//! The search reads the memory with `ReadProcessMemory` (a page freed meanwhile fails, it doesn't crash), on a
//! thread of its own, so the game doesn't wait for it.

use std::fmt::Write as _;
use std::sync::Mutex;

use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE, PAGE_GUARD, PAGE_NOACCESS, PAGE_READWRITE,
    PAGE_WRITECOPY, VirtualQuery,
};
use windows::Win32::System::Threading::GetCurrentProcess;

/// How near a float must be to count as the same.
const NEAR: f32 = 2e-4;
/// Places written per pattern, at most.
const MAX_HITS: usize = 24;
/// The memory is read this much at a time.
const CHUNK: usize = 1 << 20;
/// Floats written before and after a place.
const BEFORE: usize = 24;
const AFTER: usize = 40;

/// A mirror's (or the main view's) turn, as seen in a draw.
#[derive(Clone)]
pub struct View {
    pub what: String,
    /// The view matrix's first three rows' first three numbers.
    pub rows: [[f32; 3]; 3],
}

/// The view in a draw's first constants, if it's an object at the camera (translation nil, a true turn).
pub fn view_in(what: &str, bytes: &[u8]) -> Option<View> {
    if bytes.len() < 48 {
        return None;
    }
    let f = |i: usize| f32::from_le_bytes([bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2], bytes[i * 4 + 3]]);
    let rows = [[f(0), f(1), f(2)], [f(4), f(5), f(6)], [f(8), f(9), f(10)]];
    let translation = [f(3), f(7), f(11)];
    if translation.iter().any(|t| t.abs() > 1e-3) {
        return None;
    }
    // A turn: unit rows, at right angles, not the identity (a picture drawn flat on the target).
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let unit = rows.iter().all(|r| (dot(*r, *r) - 1.0).abs() < 1e-3);
    let square = dot(rows[0], rows[1]).abs() < 1e-3 && dot(rows[1], rows[2]).abs() < 1e-3;
    let identity =
        (rows[0][0] - 1.0).abs() < 1e-4 && (rows[1][1] - 1.0).abs() < 1e-4 && (rows[2][2] - 1.0).abs() < 1e-4;
    (unit && square && !identity).then(|| View { what: what.to_string(), rows })
}

/// The places found by the last capture: the address, as floats or doubles, and what it was.
static KEPT: Mutex<Vec<(usize, bool, String)>> = Mutex::new(Vec::new());

/// The quaternion of a turn (x, y, z, w).
fn quaternion(m: [[f32; 3]; 3]) -> [f32; 4] {
    let trace = m[0][0] + m[1][1] + m[2][2];
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [(m[2][1] - m[1][2]) / s, (m[0][2] - m[2][0]) / s, (m[1][0] - m[0][1]) / s, 0.25 * s]
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        [0.25 * s, (m[0][1] + m[1][0]) / s, (m[0][2] + m[2][0]) / s, (m[2][1] - m[1][2]) / s]
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        [(m[0][1] + m[1][0]) / s, 0.25 * s, (m[1][2] + m[2][1]) / s, (m[0][2] - m[2][0]) / s]
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        [(m[0][2] + m[2][0]) / s, (m[1][2] + m[2][1]) / s, 0.25 * s, (m[1][0] - m[0][1]) / s]
    }
}

/// What's looked for: a name, its numbers, and whether as doubles.
fn patterns(views: &[View]) -> Vec<(String, Vec<f32>, bool)> {
    let mut out = Vec::new();
    for view in views {
        let m = view.rows;
        let t = [[m[0][0], m[1][0], m[2][0]], [m[0][1], m[1][1], m[2][1]], [m[0][2], m[1][2], m[2][2]]];
        for (i, row) in m.iter().enumerate() {
            out.push((format!("{} linha {i}", view.what), row.to_vec(), false));
        }
        for (i, column) in t.iter().enumerate() {
            out.push((format!("{} coluna {i}", view.what), column.to_vec(), false));
        }
        out.push((format!("{} linha 0 (double)", view.what), m[0].to_vec(), true));
        out.push((format!("{} coluna 0 (double)", view.what), t[0].to_vec(), true));
        for (name, turn) in [("quaternion", m), ("quaternion da transposta", t)] {
            let q = quaternion(turn);
            let minus = q.map(|x| -x);
            for (sign, q) in [("", q), (" negativo", minus)] {
                out.push((format!("{} {name}{sign} xyzw", view.what), q.to_vec(), false));
                out.push((format!("{} {name}{sign} wxyz", view.what), vec![q[3], q[0], q[1], q[2]], false));
            }
        }
    }
    out
}

/// The game's writable memory, region by region: its address and size.
fn regions() -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    loop {
        let mut info = MEMORY_BASIC_INFORMATION::default();
        // SAFETY: a query of this process's own address space, into a local.
        let size =
            unsafe { VirtualQuery(Some(at as *const _), &mut info, std::mem::size_of::<MEMORY_BASIC_INFORMATION>()) };
        if size == 0 {
            break;
        }
        let base = info.BaseAddress as usize;
        let length = info.RegionSize;
        let writable = info.Protect.0 & (PAGE_READWRITE.0 | PAGE_WRITECOPY.0 | PAGE_EXECUTE_READWRITE.0) != 0;
        if info.State == MEM_COMMIT && writable && info.Protect.0 & (PAGE_GUARD.0 | PAGE_NOACCESS.0) == 0 {
            out.push((base, length));
        }
        let Some(next) = base.checked_add(length) else { break };
        if next <= at {
            break;
        }
        at = next;
    }
    out
}

/// Reads `len` bytes at `address` of this process (`None` if any of it can't be read).
fn read(address: usize, len: usize) -> Option<Vec<u8>> {
    let mut buffer = vec![0u8; len];
    let mut got = 0usize;
    // SAFETY: a read of this process's memory into our buffer, which the call checks (it fails, not faults).
    let ok = unsafe {
        ReadProcessMemory(GetCurrentProcess(), address as *const _, buffer.as_mut_ptr().cast(), len, Some(&mut got))
    };
    (ok.is_ok() && got == len).then_some(buffer)
}

/// Reads into `into` at `address` of this process: how much was read (`None` if it can't be).
fn read_into(address: usize, into: &mut [u8]) -> Option<usize> {
    let mut got = 0usize;
    // SAFETY: a read of this process's memory into our buffer, which the call checks (it fails, not faults).
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const _,
            into.as_mut_ptr().cast(),
            into.len(),
            Some(&mut got),
        )
    };
    (ok.is_ok() && got > 0).then_some(got)
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
}

/// The floats around a place, the ones near the truck's position marked.
fn around(address: usize, truck: [f64; 3]) -> String {
    let start = address.saturating_sub(BEFORE * 4);
    let Some(bytes) = read(start, (BEFORE + AFTER) * 4) else { return "      (não deu para ler em volta)\n".into() };
    let mut out = String::new();
    for (row, chunk) in floats(&bytes).chunks(4).enumerate() {
        let at = start + row * 16;
        let cells: Vec<String> = chunk
            .iter()
            .map(|v| {
                let near_truck = truck.iter().any(|t| (*v as f64 - t).abs() < 60.0 && t.abs() > 100.0);
                format!("{:14.5}{}", v, if near_truck { "*" } else { " " })
            })
            .collect();
        let mark = if at <= address && address < at + 16 { "  <==" } else { "" };
        let _ = writeln!(out, "      {at:#014x} {}{mark}", cells.join(""));
    }
    out
}

/// The search, written to `file` (on a thread of its own).
pub fn scan(views: Vec<View>, truck: [f64; 3], file: std::path::PathBuf) {
    let started = std::time::Instant::now();
    let mut text = String::new();
    let _ = writeln!(
        text,
        "# Busca na memória do jogo pela direção de cada espelho (a posição do caminhão: {truck:?}; * = perto dela)."
    );
    for view in &views {
        let _ = writeln!(text, "# {}: {:?}", view.what, view.rows);
    }
    // What the last capture's places hold now.
    let kept = std::mem::take(&mut *KEPT.lock().unwrap_or_else(|e| e.into_inner()));
    if !kept.is_empty() {
        let _ = writeln!(text, "\n== Os lugares da captura anterior, agora");
        for (address, double, what) in &kept {
            let now = if *double {
                read(*address, 24).map(|b| {
                    b.chunks_exact(8).map(|c| f64::from_le_bytes(c.try_into().unwrap_or([0; 8])) as f32).collect()
                })
            } else {
                read(*address, 16).map(|b| floats(&b))
            };
            let _ = writeln!(text, "  {address:#014x} {what}: {now:?}");
        }
    }
    let patterns = patterns(&views);
    let mut hits: Vec<Vec<usize>> = vec![Vec::new(); patterns.len()];
    // Each pattern by its largest number (its anchor: a value read is looked up among them, sorted), and where in
    // the pattern that number is.
    let anchored = |double: bool| {
        let mut index: Vec<(f32, usize, usize)> = patterns
            .iter()
            .enumerate()
            .filter(|(_, (_, _, d))| *d == double)
            .filter_map(|(p, (_, numbers, _))| {
                let (k, x) = numbers.iter().enumerate().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))?;
                Some((*x, p, k))
            })
            .collect();
        index.sort_by(|a, b| a.0.total_cmp(&b.0));
        index
    };
    // Our own copies of these numbers (the patterns, the views, the buffer read into) are not the game's.
    let mut buffer = vec![0u8; CHUNK + 32];
    let mut mine: Vec<(usize, usize)> = patterns
        .iter()
        .map(|(_, numbers, _)| (numbers.as_ptr() as usize, numbers.as_ptr() as usize + numbers.len() * 4))
        .collect();
    mine.push((views.as_ptr() as usize, views.as_ptr() as usize + views.len() * std::mem::size_of::<View>()));
    mine.push((buffer.as_ptr() as usize, buffer.as_ptr() as usize + buffer.len()));
    let singles = anchored(false);
    let doubles = anchored(true);
    let regions = regions();
    let total: usize = regions.iter().map(|r| r.1).sum();
    for (base, size) in &regions {
        let mut offset = 0;
        while offset < *size {
            let len = CHUNK.min(size - offset) + 32;
            let len = len.min(size - offset);
            let at = base + offset;
            offset += CHUNK;
            let Some(got) = read_into(at, &mut buffer[..len]) else { continue };
            let bytes = &buffer[..got];
            let value = |position: usize, double: bool| -> Option<f32> {
                if double {
                    let b = bytes.get(position..position + 8)?;
                    Some(f64::from_le_bytes(b.try_into().ok()?) as f32)
                } else {
                    let b = bytes.get(position..position + 4)?;
                    Some(f32::from_le_bytes(b.try_into().ok()?))
                }
            };
            for double in [false, true] {
                let index = if double { &doubles } else { &singles };
                let step = if double { 8 } else { 4 };
                let mut i = 0;
                while i + step <= len.min(CHUNK + step) {
                    let Some(v) = value(i, double) else { break };
                    i += step;
                    let size = v.abs();
                    if !(0.01..=1.0002).contains(&size) {
                        continue;
                    }
                    let from = index.partition_point(|(x, _, _)| *x < v - NEAR);
                    for (x, p, k) in &index[from..] {
                        if *x > v + NEAR {
                            break;
                        }
                        let (_, numbers, _) = &patterns[*p];
                        let Some(start) = (i - step).checked_sub(k * step) else { continue };
                        let same = numbers
                            .iter()
                            .enumerate()
                            .all(|(j, n)| value(start + j * step, double).is_some_and(|w| (w - n).abs() < NEAR));
                        let place = at + start;
                        let ours = mine.iter().any(|(from, to)| (*from..*to).contains(&place));
                        if same && !ours && hits[*p].len() < MAX_HITS && !hits[*p].contains(&place) {
                            hits[*p].push(place);
                        }
                    }
                }
            }
        }
    }
    let _ = writeln!(
        text,
        "\n# {} regiões, {} MB lidos em {:.1} s",
        regions.len(),
        total / (1 << 20),
        started.elapsed().as_secs_f32()
    );
    let mut keep = Vec::new();
    for ((what, numbers, double), places) in patterns.iter().zip(&hits) {
        let _ = writeln!(text, "\n== {what} {numbers:?}: {} lugar(es)", places.len());
        for address in places {
            let _ = writeln!(text, "   {address:#014x}");
            text.push_str(&around(*address, truck));
            keep.push((*address, *double, what.clone()));
        }
    }
    *KEPT.lock().unwrap_or_else(|e| e.into_inner()) = keep;
    match std::fs::write(&file, text) {
        Ok(()) => crate::log!(
            "retrovisor: memory search saved in {} ({:.1} s)",
            file.display(),
            started.elapsed().as_secs_f32()
        ),
        Err(e) => crate::log!("retrovisor: memory search not saved ({}): {e}", file.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The right mirror's turn from the owner's capture of 2026-11-14 (its sky's draw).
    const RIGHT: [[f32; 3]; 3] =
        [[-0.32335, -0.91117, 0.25538], [0.65763, -0.41043, -0.63172], [0.68042, -0.03632, 0.73192]];

    #[test]
    fn a_view_is_read_from_a_draw_at_the_camera_only() {
        let mut bytes = Vec::new();
        for row in RIGHT {
            for x in row.iter().chain([0.0f32].iter()) {
                bytes.extend_from_slice(&x.to_le_bytes());
            }
        }
        assert!(view_in("m", &bytes).is_some());
        bytes[12..16].copy_from_slice(&0.5f32.to_le_bytes());
        assert!(view_in("m", &bytes).is_none(), "an object away from the camera");
    }

    #[test]
    fn the_quaternion_turns_as_the_matrix() {
        let [x, y, z, w] = quaternion(RIGHT);
        assert!(((x * x + y * y + z * z + w * w) - 1.0).abs() < 1e-3);
        // The matrix back from the quaternion.
        let m = [
            [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w)],
            [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w)],
            [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y)],
        ];
        for i in 0..3 {
            for j in 0..3 {
                assert!((m[i][j] - RIGHT[i][j]).abs() < 1e-3, "{i},{j}: {} vs {}", m[i][j], RIGHT[i][j]);
            }
        }
    }

    #[test]
    #[ignore = "searches this whole process: the other tests running at once can fill its places first (flaky); the
                search isn't used by the camera since 2026-11-14"]
    fn the_search_finds_a_turn_in_this_process() {
        let planted: Box<[f32; 8]> = Box::new([7.0, RIGHT[1][0], RIGHT[1][1], RIGHT[1][2], 9.0, 0.0, 0.0, 0.0]);
        let file = std::env::temp_dir().join("westplay-memory-scan-test.txt");
        scan(vec![View { what: "teste".into(), rows: RIGHT }], [0.0; 3], file.clone());
        let text = std::fs::read_to_string(&file).unwrap();
        let place = format!("{:#014x}", planted.as_ptr() as usize + 4);
        assert!(text.contains(&place), "{place} not found");
        let _ = std::fs::remove_file(file);
    }
}
