//! Where the game keeps a person's animation time, and whether it takes a long update whole (owner 2026-11-06: the
//! jump's animation behind the jump itself, a step more stopping; `docs/personagem/TRANSICOES.md` 7.9). The plugin
//! counts each person's time from 0 as it makes them (`Person::clock`) and sends an animation to a frame by updating
//! it by the time between, at once (up to 2 s). If the game starts each mover's animation somewhere of its own, or
//! takes no more than so much of an update, every such send misses.
//!
//! In the log only: for a person's first [`SAMPLES`] updates, its instance (and what it points to, two steps on) read
//! as numbers (4 and 8 bytes), and the ones going on in step with the time sent written, with their rate and where
//! they started. Then, if asked (`big`: a person not drawn), one update of [`BIG`] s, and how far each went on with
//! it: all of it, or less (the game's limit).

use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Threading::GetCurrentProcess;

use crate::log;

/// Updates watched.
const SAMPLES: usize = 20;
/// The long update tried (seconds).
pub(super) const BIG: f32 = 1.0;
/// How much is read: of the instance, of each thing it points to, of each thing those point to (bytes); and where
/// in each the pointers followed are.
const INSTANCE_ROOM: usize = 0x1000;
const POINTED_ROOM: usize = 0x400;
const SECOND_ROOM: usize = 0x200;
const POINTERS_IN: usize = 0x400;
const SECOND_POINTERS_IN: usize = 0x100;
/// The most blocks read, and candidates written.
const MOST_BLOCKS: usize = 256;
const MOST_FOUND: usize = 16;

/// A block read each update: where, how long, and what it's called in the log.
pub(super) struct Block {
    at: usize,
    len: usize,
    name: String,
    /// How it was reached from the instance: the offsets of the pointers followed (none: the instance itself).
    path: Vec<usize>,
}

/// Where `path` leads from `instance` (each offset's pointer followed in turn).
fn follow(instance: usize, path: &[usize]) -> Option<usize> {
    let mut at = instance;
    for &off in path {
        at = usize::from_le_bytes(read_block(at + off, 8)?.try_into().ok()?);
        if at < 0x10000 {
            return None;
        }
    }
    Some(at)
}

pub(super) struct Probe {
    person: String,
    big: bool,
    pub(super) blocks: Vec<Block>,
    /// Each update's bytes, block after block, and the time it was sent.
    samples: Vec<Vec<u8>>,
    seconds: Vec<f32>,
    /// Found going on with time: where in the bytes, 8 bytes or 4, its rate (per second), its name.
    found: Vec<(usize, bool, f64, String)>,
}

/// What to do after a sample.
pub(super) enum Next {
    More,
    /// Update the person by [`BIG`] s now, then [`Probe::after_big`].
    Big,
    Done,
}

/// `len` bytes at `at`, read through the system's checked copy (`None`: not readable).
fn read_block(at: usize, len: usize) -> Option<Vec<u8>> {
    let mut bytes = vec![0u8; len];
    let mut got = 0usize;
    // SAFETY: into `bytes`, its own length; the process's own memory, through the system's checked copy.
    let ok = unsafe {
        ReadProcessMemory(GetCurrentProcess(), at as *const _, bytes.as_mut_ptr().cast(), len, Some(&mut got))
    };
    (ok.is_ok() && got == len).then_some(bytes)
}

/// The pointers in `bytes[..within]` that lead to `room` readable bytes.
fn pointers(bytes: &[u8], within: usize, room: usize) -> Vec<(usize, usize)> {
    (0..within.min(bytes.len()))
        .step_by(8)
        .filter_map(|off| {
            let pointer = usize::from_le_bytes(bytes[off..off + 8].try_into().ok()?);
            (pointer > 0x10000 && pointer % 8 == 0 && pointer < 0x7fff_ffff_ffff && read_block(pointer, room).is_some())
                .then_some((off, pointer))
        })
        .collect()
}

fn number(bytes: &[u8], at: usize, wide: bool) -> f64 {
    if wide {
        f64::from_le_bytes(bytes[at..at + 8].try_into().unwrap_or([0; 8]))
    } else {
        f64::from(f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4])))
    }
}

impl Probe {
    /// Watches the person `person`'s instance at `instance`; `big`: try a long update at the end (the person not
    /// drawn meanwhile).
    pub(super) fn new(person: &str, instance: usize, big: bool) -> Option<Self> {
        // As much of the instance as can be read (a whole read fails past the end of what's there).
        let mut room = INSTANCE_ROOM;
        let first = loop {
            if let Some(bytes) = read_block(instance, room) {
                break bytes;
            }
            room /= 2;
            if room < 0x40 {
                log!("probe: {person}: its instance can't be read");
                return None;
            }
        };
        let mut blocks = vec![Block { at: instance, len: room, name: String::new(), path: Vec::new() }];
        let mut seen = vec![instance];
        for (off, pointer) in pointers(&first, POINTERS_IN, POINTED_ROOM) {
            if seen.contains(&pointer) || blocks.len() >= MOST_BLOCKS {
                continue;
            }
            seen.push(pointer);
            blocks.push(Block { at: pointer, len: POINTED_ROOM, name: format!("[+{off:#x}]"), path: vec![off] });
            let Some(pointed) = read_block(pointer, POINTED_ROOM) else { continue };
            for (off2, second) in pointers(&pointed, SECOND_POINTERS_IN, SECOND_ROOM) {
                if seen.contains(&second) || blocks.len() >= MOST_BLOCKS {
                    continue;
                }
                seen.push(second);
                blocks.push(Block {
                    at: second,
                    len: SECOND_ROOM,
                    name: format!("[[+{off:#x}]+{off2:#x}]"),
                    path: vec![off, off2],
                });
            }
        }
        log!(
            "probe: {person}: watching {} blocks ({} bytes of the instance){}",
            blocks.len(),
            room,
            if big { ", then one long update" } else { "" }
        );
        Some(Probe {
            person: person.to_owned(),
            big,
            blocks,
            samples: Vec::new(),
            seconds: Vec::new(),
            found: Vec::new(),
        })
    }

    fn read_all(&self) -> Vec<u8> {
        let mut all = Vec::new();
        for block in &self.blocks {
            all.extend(read_block(block.at, block.len).unwrap_or_else(|| vec![0; block.len]));
        }
        all
    }

    /// The name of the number at `at` in the bytes read.
    fn name_of(&self, mut at: usize, wide: bool) -> String {
        for block in &self.blocks {
            if at < block.len {
                return format!("{}+{at:#x} ({})", block.name, if wide { "f64" } else { "f32" });
            }
            at -= block.len;
        }
        "?".into()
    }

    /// After an update that sent `seconds`: the numbers now, and what to do next.
    pub(super) fn sample(&mut self, seconds: f32) -> Next {
        let all = self.read_all();
        self.samples.push(all);
        self.seconds.push(seconds);
        if self.samples.len() < SAMPLES {
            return Next::More;
        }
        self.find();
        if self.big && !self.found.is_empty() { Next::Big } else { Next::Done }
    }

    /// The numbers going on in step with the time sent, into the log.
    fn find(&mut self) {
        let len = self.samples[0].len();
        for wide in [false, true] {
            let size = if wide { 8 } else { 4 };
            for at in (0..len.saturating_sub(size - 1)).step_by(size) {
                let series: Vec<f64> = self.samples.iter().map(|s| number(s, at, wide)).collect();
                if series.iter().any(|v| !v.is_finite()) {
                    continue;
                }
                let rates: Vec<f64> = series
                    .windows(2)
                    .zip(&self.seconds[1..])
                    .filter(|(_, dt)| **dt > 1e-4)
                    .map(|(pair, &dt)| (pair[1] - pair[0]) / f64::from(dt))
                    .collect();
                if rates.len() < SAMPLES / 2 {
                    continue;
                }
                let mut sorted = rates.clone();
                sorted.sort_by(f64::total_cmp);
                let rate = sorted[sorted.len() / 2];
                if !(0.01..=1000.0).contains(&rate.abs()) {
                    continue;
                }
                // In step: every frame at the same rate (within 5%), but for once round its loop.
                let off = rates.iter().filter(|r| ((**r - rate) / rate).abs() > 0.05).count();
                if off <= 1 && self.found.len() < MOST_FOUND {
                    self.found.push((at, wide, rate, self.name_of(at, wide)));
                }
            }
        }
        let sent: f32 = self.seconds[1..].iter().sum();
        let found: Vec<String> = self
            .found
            .iter()
            .map(|(at, wide, rate, name)| {
                let start = number(&self.samples[0], *at, *wide) - rate * f64::from(self.seconds[0]);
                format!("{name} at {rate:.4} a second, from {start:.4}")
            })
            .collect();
        log!(
            "probe: {}: {} updates, {sent:.3} s sent; going on with it: {}",
            self.person,
            self.samples.len(),
            if found.is_empty() { "nothing found".into() } else { found.join("; ") }
        );
    }

    /// After the long update ([`BIG`] s): how far each number found went on with it, against all of it.
    pub(super) fn after_big(&self) {
        let before = self.samples.last().cloned().unwrap_or_default();
        let after = self.read_all();
        let went: Vec<String> = self
            .found
            .iter()
            .map(|(at, wide, rate, name)| {
                let moved = number(&after, *at, *wide) - number(&before, *at, *wide);
                format!("{name}: {:.3} s of {BIG} s", moved / rate)
            })
            .collect();
        log!("probe: {}: one update of {BIG} s: {}", self.person, went.join("; "));
    }
}

/// Two people made alike, never drawn, to tell what moving one's animation by a long update does
/// (`docs/personagem/TRANSICOES.md` 7.9), by their poses (the numbers of theirs that move as they're updated):
/// first the same updates to both (the same poses: their animations start at the same point, the game's own;
/// unlike, each starts somewhere of its own), then [`BIG`] s to each, to one in 1/60 s steps, to the other at once
/// (unlike: the game takes no more than so much of an update).
pub(super) struct Twins {
    a: Probe,
    b: Probe,
    frames: usize,
    /// The numbers of the first that have moved as it was updated, and its bytes last frame.
    moving: Vec<bool>,
    /// The numbers the second hasn't (its block not reached): left out of the comparison.
    unreached: Vec<bool>,
    last: Vec<u8>,
}

/// How many frames of the same updates before comparing.
const TWIN_FRAMES: usize = 20;

/// What the twins want this frame.
pub(super) enum TwinStep {
    /// The same update to both (the frame's time).
    Same,
    /// [`BIG`] s to the first in 1/60 s steps, to the second at once; then done.
    Big,
    Done,
}

impl Twins {
    pub(super) fn new(a: usize, b: usize) -> Option<Self> {
        let a = Probe::new("twin a", a, false)?;
        // The second read the same way as the first: each block by the same path from its own instance (not found
        // anew: its pointers needn't come in the same order). A block it hasn't is read as nothing, 0s alike.
        let mut reached = 0;
        let blocks = a
            .blocks
            .iter()
            .map(|block| {
                let at = follow(b, &block.path).filter(|&at| read_block(at, block.len).is_some());
                reached += usize::from(at.is_some());
                Block { at: at.unwrap_or(0), len: block.len, name: block.name.clone(), path: block.path.clone() }
            })
            .collect::<Vec<_>>();
        log!("probe: twin b: {reached} of {} blocks reached by the first's paths", blocks.len());
        let b = Probe {
            person: "twin b".into(),
            big: false,
            blocks,
            samples: Vec::new(),
            seconds: Vec::new(),
            found: Vec::new(),
        };
        let unreached = b.blocks.iter().flat_map(|block| std::iter::repeat_n(block.at == 0, block.len / 4)).collect();
        Some(Twins { a, b, frames: 0, moving: Vec::new(), unreached, last: Vec::new() })
    }

    pub(super) fn next(&self) -> TwinStep {
        match self.frames {
            f if f < TWIN_FRAMES => TwinStep::Same,
            f if f == TWIN_FRAMES => TwinStep::Big,
            _ => TwinStep::Done,
        }
    }

    /// After this frame's step ([`Twins::next`]): their poses compared, into the log when it's time.
    pub(super) fn after(&mut self) {
        let (a, b) = (self.a.read_all(), self.b.read_all());
        let step = self.next();
        if self.last.len() == a.len() {
            self.moving.resize(a.len() / 4, false);
            for (i, moving) in self.moving.iter_mut().enumerate() {
                if a[i * 4..i * 4 + 4] != self.last[i * 4..i * 4 + 4] {
                    *moving = true;
                }
            }
        }
        self.last = a.clone();
        self.frames += 1;
        let compare = |what: &str| {
            let mut differ = 0;
            let mut most = 0.0f32;
            let mut moving = 0;
            for (i, _) in
                self.moving.iter().enumerate().filter(|(i, m)| **m && !self.unreached.get(*i).copied().unwrap_or(true))
            {
                let x = f32::from_le_bytes(a[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]));
                let y = f32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]));
                if !x.is_finite() || !y.is_finite() || x.abs() > 1e4 {
                    continue;
                }
                moving += 1;
                let d = (x - y).abs();
                if d > 1e-4 {
                    differ += 1;
                }
                most = most.max(d);
            }
            log!("probe: twins {what}: {differ} of {moving} moving numbers differ, the most by {most:.4}");
        };
        match step {
            TwinStep::Same if self.frames == TWIN_FRAMES => compare("after the same updates"),
            TwinStep::Big => compare(&format!("after {BIG} s, the first in 1/60 s steps, the second at once")),
            _ => {}
        }
    }
}
