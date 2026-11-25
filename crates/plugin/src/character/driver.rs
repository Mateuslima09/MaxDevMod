//! Hides the player truck's exterior driver model (`/asset/character/human/driver/$driver_type$/driver.pmd`)
//! while the player is walking outside the cab in Modo Personagem (`super::walking()`).
//!
//! In ETS2/ATS the game draws the seated driver model through a single function (`driver::draw(driver, ctx)`)
//! called from only two sites: the player's own truck (guarded by interior/mirror render-pass flags
//! `0x180000000001FC00`) and other trucks (traffic/Convoy). Hooking `driver::draw` and skipping only when
//! `super::walking()` is true and the return address matches the player truck's call site hides the seated driver
//! (and its shadow) across every stock, DLC, and mod truck without touching AI or Convoy drivers.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use windows::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;

use super::{pattern, rel};
use crate::log;
use crate::navigation::Image;

/// Player truck exterior draw call to `driver::draw`:
/// ```text
///   48 8B 47 18                    mov rax, [rdi+18h]
///   48 B9 00 FC 01 00 00 00 00 18  mov rcx, 0x180000000001FC00
///   48 85 C1                       test rcx, rax
///   75 13                          jne +13h
///   48 8D 8E ?? ?? ?? ??           lea rcx, [rsi+driver_offset]
///   48 8B D7                       mov rdx, rdi
///   E8 ?? ?? ?? ??                 call driver::draw
///   48 8B 47 18                    mov rax, [rdi+18h]
///   48 B9 01 FC 01 00 00 00 00 18  mov rcx, 0x180000000001FC01
/// ```
const PLAYER_DRIVER_CALL: &str = "48 8B 47 18 48 B9 00 FC 01 00 00 00 00 18 48 85 C1 75 13 \
                                  48 8D 8E ?? ?? ?? ?? 48 8B D7 E8 ?? ?? ?? ?? \
                                  48 8B 47 18 48 B9 01 FC 01 00 00 00 00 18";
/// Byte offset of the 32-bit relative displacement in `E8 ?? ?? ?? ??` within [`PLAYER_DRIVER_CALL`].
const CALL_DISP_OFFSET: usize = 30;
/// Byte offset of the return address immediately after `call driver::draw` within [`PLAYER_DRIVER_CALL`].
const CALL_RET_OFFSET: usize = 34;

type DrawDriver = unsafe extern "system" fn(*mut c_void, *mut c_void);

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static PLAYER_CALL_RET: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static HIDDEN_LOGGED: AtomicBool = AtomicBool::new(false);

/// Locates the player truck's `driver::draw` call site in `image` and hooks `driver::draw` once.
///
/// # Safety
/// `image` is the live game executable's mapped image loaded at `base`.
pub(super) unsafe fn install(image: &Image, base: usize) {
    if INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let Some(site) = image.unique_masked(&pattern(PLAYER_DRIVER_CALL)) else {
        log!("character: cab driver draw site not found (seated driver will stay visible on foot)");
        return;
    };
    let Some(target) = rel(image, site + CALL_DISP_OFFSET) else {
        log!("character: cab driver draw target unreadable");
        return;
    };
    if !image.in_code(target) {
        log!("character: cab driver draw target outside code ({target:#x})");
        return;
    }
    let ret_addr = site + CALL_RET_OFFSET;
    PLAYER_CALL_RET.store(ret_addr, Ordering::Release);
    // SAFETY: `target` is the entry of `driver::draw(driver, ctx)` resolved from its unique call site in `.text`,
    // and `detour_driver_draw` has the exact same `extern "system"` signature.
    match unsafe { crate::hooks::hook_now(target, detour_driver_draw as *mut c_void, &ORIGINAL, "driver::draw") } {
        Ok(()) => {
            INSTALLED.store(true, Ordering::Release);
            log!(
                "character: cab driver hide hook ready (+{:#x} driver::draw, player caller +{:#x})",
                target - base,
                ret_addr - base
            );
        }
        Err(e) => {
            PLAYER_CALL_RET.store(0, Ordering::Release);
            log!("character: cab driver hide hook failed: {e}");
        }
    }
}

/// Whether the current `driver::draw` invocation came from the player truck's draw site (not AI/Convoy).
#[inline(never)]
fn called_from_player_truck() -> bool {
    let expected = PLAYER_CALL_RET.load(Ordering::Acquire);
    if expected == 0 {
        return false;
    }
    let mut frames = [std::ptr::null_mut::<c_void>(); 4];
    // SAFETY: `frames` is a valid stack buffer of 4 pointers; `RtlCaptureStackBackTrace` writes at most `frames.len()`.
    let count = unsafe { RtlCaptureStackBackTrace(0, &mut frames, None) } as usize;
    frames[..count.min(frames.len())].contains(&(expected as *mut c_void))
}

/// Detour for `driver::draw(driver, ctx)`: skips rendering the player's seated cab driver while walking on foot.
///
/// # Safety
/// Called by the game via MinHook with live `driver` and render `ctx` pointers.
#[inline(never)]
unsafe extern "system" fn detour_driver_draw(driver: *mut c_void, ctx: *mut c_void) {
    if super::walking() && called_from_player_truck() {
        if !HIDDEN_LOGGED.swap(true, Ordering::Relaxed) {
            log!("character: hiding seated cab driver while outside the truck");
        }
        return;
    }
    let orig = ORIGINAL.load(Ordering::Acquire);
    if orig != 0 {
        // SAFETY: `orig` is the MinHook trampoline for `driver::draw`, matching `DrawDriver`.
        let draw: DrawDriver = unsafe { std::mem::transmute(orig) };
        // SAFETY: forwarding the game's live arguments unchanged.
        unsafe { draw(driver, ctx) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_offsets_match_call_instruction() {
        let pat = pattern(PLAYER_DRIVER_CALL);
        assert_eq!(pat.len(), 48);
        assert_eq!(pat[CALL_DISP_OFFSET - 1], Some(0xE8));
        assert_eq!(pat[CALL_DISP_OFFSET], None);
        assert_eq!(pat[CALL_RET_OFFSET], Some(0x48));
    }

    #[test]
    fn stack_backtrace_captures_caller_frame() {
        #[inline(never)]
        fn inner(out: &mut [*mut c_void; 4]) -> usize {
            // SAFETY: valid 4-element buffer.
            unsafe { RtlCaptureStackBackTrace(0, out, None) as usize }
        }
        let mut frames = [std::ptr::null_mut::<c_void>(); 4];
        let count = inner(&mut frames);
        assert!(count >= 2, "expected at least 2 stack frames, got {count}");
        assert!(!frames[1].is_null());
    }
}
