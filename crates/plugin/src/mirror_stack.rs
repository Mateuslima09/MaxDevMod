//! Which of the game's code draws a mirror (the reversing camera, owner 2026-11-14: a glass mirror out of view isn't
//! drawn, so the camera's gone when zoomed in on the dashboard's screen; the game decides that in its own code, not
//! with a GPU query). As the main mirror's target is set (`mirror`), the call stack is taken: for each of the
//! game's own frames, its function's start and code up to the return address, written to
//! `Documents\WestPlay\logs\retrovisor\pilha-N.txt` to be read (disassembled) by hand. The check that leaves the
//! mirror out is in one of these functions, before it calls on to draw it.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::log;
use crate::navigation::Image;

/// Stacks taken a game at most (a few frames apart: the same path each time, or not). 0 from 2026-11-18 (owner:
/// the game stuttered now and then; each stack written is a hitch, and the camera works).
const MOST: u32 = 0;
/// Code written per frame, at most, before its return address (bytes).
const BEFORE: usize = 6144;
static TAKEN: AtomicU32 = AtomicU32::new(0);

/// Called as the main mirror's target is set (the game's render thread): every 200th time, up to `MOST`.
pub fn on_mirror(count: u64) {
    if !count.is_multiple_of(200) || TAKEN.load(Ordering::Relaxed) >= MOST {
        return;
    }
    let number = TAKEN.fetch_add(1, Ordering::Relaxed) + 1;
    let mut frames = [std::ptr::null_mut(); 48];
    // SAFETY: our own buffer of frames, filled with this thread's return addresses.
    let taken = unsafe { RtlCaptureStackBackTrace(0, &mut frames, None) } as usize;
    // SAFETY: no name: the game's own executable, mapped for the process's life.
    let Some(base) = (unsafe { GetModuleHandleW(None) }).ok().map(|module| module.0 as usize) else { return };
    // SAFETY: `base` is the game's executable, mapped.
    let Some(image) = (unsafe { Image::of(base) }) else { return };
    let mut text = format!("# A pilha ao desenhar o espelho principal (base do jogo {base:#x}).\n");
    for (n, frame) in frames[..taken].iter().enumerate() {
        let at = *frame as usize;
        let Some(start) = image.function_start(at) else {
            let _ = writeln!(text, "\n== {n}: {at:#x} (fora do jogo)");
            continue;
        };
        let _ = writeln!(text, "\n== {n}: retorno +{:#x}, função +{:#x} (+{:#x} dentro dela)", at - base, start - base, at - start);
        let from = start.max(at.saturating_sub(BEFORE));
        if let Some(code) = image.code_at(from, at + 16 - from) {
            let _ = writeln!(text, "codigo desde +{:#x}:", from - base);
            for line in code.chunks(32) {
                let hex: Vec<String> = line.iter().map(|b| format!("{b:02x}")).collect();
                let _ = writeln!(text, "{}", hex.join(""));
            }
        }
    }
    let dir = crate::log::log_dir().join("retrovisor");
    let file = dir.join(format!("pilha-{number}.txt"));
    let _ = std::thread::Builder::new().name("westplay-mirror-stack".into()).spawn(move || {
        match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, text)) {
            Ok(()) => log!("retrovisor: the mirror's call stack saved in {}", file.display()),
            Err(e) => log!("retrovisor: the mirror's call stack not saved: {e}"),
        }
    });
}
