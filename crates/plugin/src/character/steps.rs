//! Modo Personagem: the sound of the player's steps (owner 2026-11-05: "the steps, and the sound"). The game has no
//! footsteps (docs/personagem/INVESTIGACAO.md §7.4).
//!
//! Fourth version (owner 2026-11-05: the asphalt recording's steps sounded "very strange"; the owner gave walking
//! and running recordings, from Pixabay, free to use): walking, ten steps cut from "walking on concrete ver 2"
//! (`assets/steps_walk.raw`, each 0.36 s); running, twelve from "running on the floor" (`assets/steps_run.raw`,
//! each 0.26 s). Each from 20 ms before the heel, mono, 16-bit little-endian, 24 kHz, faded in over 15 ms and out
//! over 80 ms, brought to the same loudness; taken at random (never the same twice in a row). Played through one sound stream of the plugin's own (`waveOut`), opened while walking and
//! closed after a quiet while, where steps are mixed: one never cuts another, and Windows doesn't open and close
//! its sound for each step (the first version's `PlaySound` did: a pop).

use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use windows::Win32::Media::Audio::{
    CALLBACK_NULL, HWAVEOUT, WAVE_FORMAT_PCM, WAVE_MAPPER, WAVEFORMATEX, WAVEHDR, WHDR_DONE, waveOutClose, waveOutOpen,
    waveOutPrepareHeader, waveOutReset, waveOutUnprepareHeader, waveOutWrite,
};

use crate::log;

/// The recorded steps, walking and running, one after another, each `*_STEP` samples at `RATE` (mono, 16 bits).
const WALK: &[u8] = include_bytes!("../../assets/steps_walk.raw");
const RUN: &[u8] = include_bytes!("../../assets/steps_run.raw");
const RATE: u32 = 24_000;
const WALK_STEP: usize = 8_640;
const RUN_STEP: usize = 6_240;
const WALKS: usize = WALK.len() / 2 / WALK_STEP;
const RUNS: usize = RUN.len() / 2 / RUN_STEP;
/// How loud the steps play, walking and running (of the recording, which peaks at about a third of full scale):
/// lower since 2026-11-05 (owner: "a bit loud, mostly running"; then "still a bit loud").
const WALK_VOLUME: f32 = 0.45;
const RUN_VOLUME: f32 = 0.3;
/// The door's recordings (`door`), each one sound, peaking at 0.9 of full scale; how loud they play (the steps
/// peak at about 0.15: the slam a bit over twice, the handle softer).
const DOOR_OPEN: &[u8] = include_bytes!("../../assets/door_open.raw");
const DOOR_SHUT: &[u8] = include_bytes!("../../assets/door_shut.raw");
const DOOR_OPEN_VOLUME: f32 = 0.3;
const DOOR_SHUT_VOLUME: f32 = 0.4;
/// The brake fade's sounds (owner 2026-11-21, THIRD_PARTY_NOTICES.md): iron grinding on iron (2 s) and the
/// dashboard's warning chime (2.8 s), like the steps' (mono, 16-bit, 24 kHz), peaking at 0.9 of full scale.
const BRAKE_SQUEAL: &[u8] = include_bytes!("../../assets/brake_squeal.raw");
const BRAKE_WARNING: &[u8] = include_bytes!("../../assets/brake_warning.raw");

/// The stream: this many buffers of this many samples (20 ms each: a step starts at most ~60 ms late).
const BUFFERS: usize = 3;
const BUFFER: usize = 480;
/// The stream is closed after this long without a step.
const QUIET: Duration = Duration::from_secs(3);

static SOUNDS: OnceLock<Vec<Vec<f32>>> = OnceLock::new();
static STREAM: OnceLock<Mutex<Sender<(usize, isize, f32)>>> = OnceLock::new();
static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
static SEED: AtomicU32 = AtomicU32::new(0x2545_f491);

/// A step's sound now, walking or running (one of its variants, not the last one). The sounds are the walking
/// ones, then the running ones.
pub(super) fn play(running: bool) {
    let (first, count) = if running { (WALKS, RUNS) } else { (0, WALKS) };
    let mut seed = SEED.load(Ordering::Relaxed);
    let last = LAST.load(Ordering::Relaxed) as usize;
    let mut pick = last;
    while pick == last {
        seed = xorshift(seed);
        pick = first + seed as usize % count;
    }
    SEED.store(seed, Ordering::Relaxed);
    LAST.store(pick as u32, Ordering::Relaxed);
    send(pick, 0, 1.0);
}

/// The driver's door: opening (the inside handle pulled, its latch) or shut (a metallic slam), after `after`
/// seconds. Two recordings the owner gave (2026-11-06, ElevenLabs), cut and brought to the same loudness:
/// `assets/door_open.raw` (0.7 s) and `door_shut.raw` (1 s), like the steps' (mono, 16-bit, 24 kHz).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Door {
    Open,
    Shut,
}

pub(super) fn door(which: Door, after: f32) {
    let sound = WALKS + RUNS + usize::from(which == Door::Shut);
    send(sound, -((after.max(0.0) * RATE as f32) as isize), 1.0);
}

/// The brake fade's sounds (crate::brake_sounds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BrakeSound {
    /// The dashboard's warning (the grinding loops: [`brake_loop`]).
    Warning,
}

/// `at` meaning "loop this sound (or keep looping it, at this gain)" and "stop looping it": the grinding goes on
/// as long as the brake is held, seamless (owner 2026-11-21: "holding the brake it should follow on, not end the
/// loop and start it all over").
const LOOP: isize = isize::MIN;
const STOP: isize = isize::MIN + 1;
/// A loop stopped fades out over this many samples (80 ms).
const STOP_FADE: isize = 1_920;

/// The grinding looping at `volume` (0..1) while `on`; let go (fading out) once not.
pub(crate) fn brake_loop(on: bool, volume: f32) {
    let sound = WALKS + RUNS + 2;
    if on { send(sound, LOOP, volume.clamp(0.0, 1.0)) } else { send(sound, STOP, 0.0) }
}

/// One of the brake fade's sounds now, at `volume` (0..1); how long it lasts.
pub(crate) fn brake(which: BrakeSound, volume: f32) -> Duration {
    let (sound, bytes) = match which {
        BrakeSound::Warning => (WALKS + RUNS + 3, BRAKE_WARNING.len()),
    };
    send(sound, 0, volume.clamp(0.0, 1.0));
    Duration::from_secs_f32(bytes as f32 / 2.0 / RATE as f32)
}

/// A sound to the stream, starting `at` samples in (negative: that many later), at `gain`.
fn send(sound: usize, at: isize, gain: f32) {
    let stream = STREAM.get_or_init(|| {
        let (send, receive) = channel();
        let _ = std::thread::Builder::new().name("westplay-steps".into()).spawn(move || run(receive));
        Mutex::new(send)
    });
    if let Ok(send) = stream.lock() {
        let _ = send.send((sound, at, gain));
    }
}

fn xorshift(mut seed: u32) -> u32 {
    seed ^= seed << 13;
    seed ^= seed >> 17;
    seed ^= seed << 5;
    seed
}

/// A step being played: which one, and how far into it.
struct Voice {
    sound: usize,
    at: isize,
    gain: f32,
    /// Played over and over until stopped.
    looping: bool,
    /// Where (in `at`'s samples) it started fading out, stopped.
    stopping: Option<isize>,
}

impl Voice {
    fn new(sound: usize, at: isize, gain: f32) -> Self {
        Self { sound, at, gain, looping: false, stopping: None }
    }
}

/// A command to the voices: a sound to play, a loop to start, keep at a gain, or stop.
fn command(voices: &mut Vec<Voice>, sound: usize, at: isize, gain: f32) {
    let looping = voices.iter_mut().find(|v| v.sound == sound && v.looping && v.stopping.is_none());
    match (at, looping) {
        (LOOP, Some(voice)) => voice.gain = gain,
        (LOOP, None) => voices.push(Voice { looping: true, ..Voice::new(sound, 0, gain) }),
        (STOP, Some(voice)) => voice.stopping = Some(voice.at),
        (STOP, None) => {}
        _ => voices.push(Voice::new(sound, at, gain)),
    }
}

/// The steps' thread: waits for a step, opens the stream, mixes what's playing into it, closes it when quiet.
fn run(steps: Receiver<(usize, isize, f32)>) {
    let sounds = SOUNDS.get_or_init(recorded);
    // Waiting for a step with the stream closed.
    while let Ok((first, at, gain)) = steps.recv() {
        let mut voices = Vec::new();
        command(&mut voices, first, at, gain);
        if voices.is_empty() {
            continue;
        }
        // SAFETY: the stream's own calls, on buffers that outlive it (reset and unprepared before they go).
        unsafe {
            let Some(mut stream) = Stream::open() else {
                log!("character: no sound device for the steps");
                continue;
            };
            let mut heard = Instant::now();
            loop {
                while let Ok((sound, at, gain)) = steps.try_recv() {
                    command(&mut voices, sound, at, gain);
                    heard = Instant::now();
                }
                stream.feed(|out| mix(sounds, &mut voices, out));
                if voices.is_empty() && heard.elapsed() > QUIET {
                    break;
                }
                // Until the next step or the next buffer (5 ms).
                match steps.recv_timeout(Duration::from_millis(5)) {
                    Ok((sound, at, gain)) => {
                        command(&mut voices, sound, at, gain);
                        heard = Instant::now();
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            stream.close();
        }
    }
}

/// The voices playing mixed into `out` (16-bit samples); the finished ones dropped.
fn mix(sounds: &[Vec<f32>], voices: &mut Vec<Voice>, out: &mut [i16]) {
    for (k, sample) in out.iter_mut().enumerate() {
        let sum: f32 = voices
            .iter()
            .map(|v| {
                let at = v.at + k as isize;
                let len = sounds[v.sound].len() as isize;
                let at = if v.looping && len > 0 { at.rem_euclid(len) } else { at };
                let fade = v.stopping.map_or(1.0, |from| (1.0 - (v.at + k as isize - from) as f32 / STOP_FADE as f32).max(0.0));
                usize::try_from(at).ok().and_then(|i| sounds[v.sound].get(i)).map_or(0.0, |s| s * v.gain * fade)
            })
            .sum();
        *sample = (sum.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
    }
    for voice in voices.iter_mut() {
        voice.at += out.len() as isize;
    }
    voices.retain(|v| match v.stopping {
        Some(from) => v.at - from < STOP_FADE,
        None => v.looping || v.at < sounds[v.sound].len() as isize,
    });
}

/// The plugin's sound stream: a `waveOut` device and its buffers, refilled as each is played.
struct Stream {
    device: HWAVEOUT,
    headers: Box<[WAVEHDR; BUFFERS]>,
    data: Box<[[i16; BUFFER]; BUFFERS]>,
}

impl Stream {
    /// # Safety
    /// Closed with `close` before it's dropped.
    unsafe fn open() -> Option<Self> {
        let format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nChannels: 1,
            nSamplesPerSec: RATE,
            nAvgBytesPerSec: RATE * 2,
            nBlockAlign: 2,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let mut device = HWAVEOUT::default();
        // SAFETY: a format on the stack, read during the call; no callback.
        if unsafe { waveOutOpen(Some(&mut device), WAVE_MAPPER, &format, None, None, CALLBACK_NULL) } != 0 {
            return None;
        }
        let mut stream =
            Stream { device, headers: Box::new([WAVEHDR::default(); BUFFERS]), data: Box::new([[0; BUFFER]; BUFFERS]) };
        for i in 0..BUFFERS {
            let header = &mut stream.headers[i];
            header.lpData = windows::core::PSTR(stream.data[i].as_mut_ptr().cast());
            header.dwBufferLength = (BUFFER * 2) as u32;
            // SAFETY: the header and its data are boxed: they stay put while the device has them.
            unsafe { waveOutPrepareHeader(device, header, size_of::<WAVEHDR>() as u32) };
            // Marked played, so the first `feed` fills and sends it.
            header.dwFlags |= WHDR_DONE;
        }
        Some(stream)
    }

    /// Each buffer the device is done with filled by `fill` and sent again.
    ///
    /// # Safety
    /// The stream open.
    unsafe fn feed(&mut self, mut fill: impl FnMut(&mut [i16])) {
        for i in 0..BUFFERS {
            let header: *mut WAVEHDR = &mut self.headers[i];
            // SAFETY: the device sets the flag from its own thread: read it as it is now.
            let flags = unsafe { std::ptr::addr_of!((*header).dwFlags).read_volatile() };
            if flags & WHDR_DONE == 0 {
                continue;
            }
            fill(&mut self.data[i]);
            // SAFETY: a prepared header of this device, its data filled.
            unsafe { waveOutWrite(self.device, header, size_of::<WAVEHDR>() as u32) };
        }
    }

    /// # Safety
    /// The stream open.
    unsafe fn close(mut self) {
        // SAFETY: reset gives every buffer back; then each is unprepared and the device closed.
        unsafe {
            waveOutReset(self.device);
            for header in self.headers.iter_mut() {
                waveOutUnprepareHeader(self.device, header, size_of::<WAVEHDR>() as u32);
            }
            waveOutClose(self.device);
        }
    }
}

/// The recorded steps, as samples (-1..1): the walking ones, then the running ones.
fn recorded() -> Vec<Vec<f32>> {
    let mut sounds = cut(WALK, WALK_STEP, WALK_VOLUME);
    sounds.extend(cut(RUN, RUN_STEP, RUN_VOLUME));
    sounds.extend(cut(DOOR_OPEN, DOOR_OPEN.len() / 2, DOOR_OPEN_VOLUME));
    sounds.extend(cut(DOOR_SHUT, DOOR_SHUT.len() / 2, DOOR_SHUT_VOLUME));
    sounds.extend(cut(BRAKE_SQUEAL, BRAKE_SQUEAL.len() / 2, 1.0));
    sounds.extend(cut(BRAKE_WARNING, BRAKE_WARNING.len() / 2, 1.0));
    sounds
}

fn cut(recorded: &[u8], step: usize, volume: f32) -> Vec<Vec<f32>> {
    let samples: Vec<f32> = recorded
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&b| f32::from(i16::from_le_bytes(b)) / f32::from(i16::MAX) * volume)
        .collect();
    samples.chunks_exact(step).map(<[f32]>::to_vec).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loop_goes_on_until_stopped_then_fades() {
        let sounds = vec![vec![0.5; 100]];
        let mut voices = Vec::new();
        command(&mut voices, 0, LOOP, 1.0);
        command(&mut voices, 0, LOOP, 0.5);
        assert_eq!(voices.len(), 1);
        let mut out = [0i16; 480];
        mix(&sounds, &mut voices, &mut out);
        mix(&sounds, &mut voices, &mut out);
        assert_eq!(voices.len(), 1);
        assert!(out.iter().all(|&s| s > 8000), "the loop has no gap");
        command(&mut voices, 0, STOP, 0.0);
        for _ in 0..5 {
            mix(&sounds, &mut voices, &mut out);
        }
        assert!(voices.is_empty());
    }

    #[test]
    fn the_recorded_steps_are_whole_and_smooth() {
        assert_eq!(WALK.len() % (2 * WALK_STEP), 0);
        assert_eq!(RUN.len() % (2 * RUN_STEP), 0);
        assert_eq!((WALKS, RUNS), (10, 12));
        let sounds = recorded();
        assert_eq!(sounds.len(), WALKS + RUNS + 4);
        // The brake fade's two, last: loud; the warning faded out at its end, the grinding a loop (its end runs on
        // into its start, no jump).
        for s in &sounds[WALKS + RUNS + 2..] {
            let loudest = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(loudest > 0.8 && loudest <= 1.0, "{loudest}");
        }
        let warning = &sounds[WALKS + RUNS + 3];
        assert!(warning[warning.len() - 1].abs() < 1e-2);
        let grinding = &sounds[WALKS + RUNS + 2];
        assert!((grinding[grinding.len() - 1] - grinding[0]).abs() < 0.2);
        // The door's two, after the steps: whole, faded, louder than a step.
        for s in &sounds[WALKS + RUNS..WALKS + RUNS + 2] {
            let loudest = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(loudest > 0.2 && loudest < 0.5, "{loudest}");
            assert!(s[0].abs() < 1e-3 && s[s.len() - 1].abs() < 1e-3);
        }
        assert_eq!(DOOR_OPEN.len() % 2 + DOOR_SHUT.len() % 2, 0);
        for (i, s) in sounds[..WALKS + RUNS].iter().enumerate() {
            let step = if i < WALKS { WALK_STEP } else { RUN_STEP };
            assert_eq!(s.len(), step);
            let loudest = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(loudest > 0.05 && loudest < 0.5, "step {i}: {loudest}");
            // Faded in and out: no edge to click on.
            assert!(s[0].abs() < 1e-3 && s[step - 1].abs() < 1e-3, "step {i}");
        }
        assert_ne!(sounds[0], sounds[1]);
    }

    #[test]
    fn steps_are_mixed_not_cut() {
        let sounds = vec![vec![0.1; 1000], vec![0.2; 300]];
        let mut voices = vec![Voice::new(0, 0, 1.0), Voice::new(1, 0, 1.0)];
        let mut out = [0i16; 441];
        mix(&sounds, &mut voices, &mut out);
        assert!((f32::from(out[0]) / f32::from(i16::MAX) - 0.3).abs() < 1e-3);
        assert!((f32::from(out[400]) / f32::from(i16::MAX) - 0.1).abs() < 1e-3);
        // The short one is done; the long one goes on.
        assert_eq!(voices.len(), 1);
        mix(&sounds, &mut voices, &mut out);
        mix(&sounds, &mut voices, &mut out);
        assert!(voices.is_empty());
    }

    #[test]
    fn a_sound_can_start_later() {
        let sounds = vec![vec![0.5; 100]];
        let mut voices = vec![Voice::new(0, -50, 1.0)];
        let mut out = [0i16; 60];
        mix(&sounds, &mut voices, &mut out);
        assert_eq!(out[49], 0);
        assert!(out[50] > 0);
        assert_eq!(voices[0].at, 10);
    }
}
