//! The screen faded to black and back, over the whole game (owner 2026-11-19: run over by a car on foot, "a blink on
//! the screen, a loading, and the person back beside the truck"): [`start`] it, the moment it's all black
//! ([`covered`]) is when what's behind it changes, then it clears. Drawn last on each presented frame with the HUD's
//! own quad (`hud::HudQuad`), a black texture over the whole back buffer.

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Graphics::Direct3D11::{D3D11_TEXTURE2D_DESC, ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::Interface;

use crate::hud::HudQuad;
use crate::log;

/// Darkening, black, clearing (seconds from the start): long blackout (run over) and quick blink (entering cab).
const DARK_IN: Duration = Duration::from_millis(250);
const BLACK_FOR: Duration = Duration::from_millis(550);
const CLEAR_IN: Duration = Duration::from_millis(400);

const BLINK_IN: Duration = Duration::from_millis(140);
const BLINK_BLACK: Duration = Duration::from_millis(90);
const BLINK_OUT: Duration = Duration::from_millis(220);

#[derive(Clone, Copy)]
struct Profile {
    dark_in: Duration,
    black_for: Duration,
    clear_in: Duration,
}

const BLACKOUT: Profile = Profile { dark_in: DARK_IN, black_for: BLACK_FOR, clear_in: CLEAR_IN };
const BLINK: Profile = Profile { dark_in: BLINK_IN, black_for: BLINK_BLACK, clear_in: BLINK_OUT };

static STARTED: Mutex<Option<(Instant, Profile)>> = Mutex::new(None);

/// Starts a full blackout fade (none while one runs).
pub fn start() {
    let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
    if started.is_none() {
        *started = Some((Instant::now(), BLACKOUT));
    }
}

/// Starts a brief dark eye-blink across a camera transition (none while another fade runs).
pub fn blink() {
    let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
    if started.is_none() {
        *started = Some((Instant::now(), BLINK));
    }
}

/// How dark at `since` the start (0 clear, 1 black); `None` once done.
fn darkness(since: Duration, profile: Profile) -> Option<f32> {
    let black_at = profile.dark_in;
    let clear_at = profile.dark_in + profile.black_for;
    let done_at = clear_at + profile.clear_in;
    if since < black_at {
        Some(since.as_secs_f32() / profile.dark_in.as_secs_f32())
    } else if since < clear_at {
        Some(1.0)
    } else if since < done_at {
        Some(1.0 - (since - clear_at).as_secs_f32() / profile.clear_in.as_secs_f32())
    } else {
        None
    }
}

/// Whether the screen is all black now (what's behind may change unseen).
pub fn covered() -> bool {
    STARTED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some_and(|(at, profile)| darkness(at.elapsed(), profile).is_some_and(|d| d >= 1.0))
}

struct Drawing {
    device: ID3D11Device,
    quad: HudQuad,
    black: ID3D11ShaderResourceView,
}

// SAFETY: used only on the game's render thread, through the mutex.
unsafe impl Send for Drawing {}

static DRAWING: Mutex<Option<Drawing>> = Mutex::new(None);

/// Draws the fade over the frame, if one runs.
///
/// # Safety
/// `swapchain` is the game's live swap chain, on its render thread, in `Present`.
pub unsafe fn on_present(swapchain: *mut c_void) {
    let opacity = {
        let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
        let Some((at, profile)) = *started else { return };
        match darkness(at.elapsed(), profile) {
            Some(d) => d,
            None => {
                *started = None;
                return;
            }
        }
    };
    // SAFETY: the caller's contract: the live swap chain on the render thread.
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let mut drawing = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        if drawing.as_ref().is_some_and(|d| d.device != device) {
            *drawing = None;
        }
        if drawing.is_none() {
            let Ok(back) = swapchain.GetBuffer::<ID3D11Texture2D>(0) else { return };
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut desc);
            let format = crate::handset::view_format(desc.Format);
            let made = HudQuad::new(&device).and_then(|quad| {
                let black = crate::hints::upload(&device, 1, 1, &[0, 0, 0, 255], format)?;
                Ok(Drawing { device: device.clone(), quad, black })
            });
            match made {
                Ok(made) => *drawing = Some(made),
                Err(e) => {
                    log!("fade: can't be drawn: {e}");
                    return;
                }
            }
        }
        let Some(made) = drawing.as_ref() else { return };
        let Ok(context) = made.device.GetImmediateContext() else { return };
        // The whole back buffer (the quad's corners in clip space, top left first).
        if let Err(e) = made.quad.draw(&made.device, &context, swapchain, &made.black, [-1.0, 1.0, 1.0, -1.0], opacity, None) {
            log!("fade: not drawn: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darkens_holds_black_and_clears() {
        assert_eq!(darkness(Duration::ZERO, BLACKOUT), Some(0.0));
        assert_eq!(darkness(DARK_IN, BLACKOUT), Some(1.0));
        assert_eq!(darkness(DARK_IN + BLACK_FOR / 2, BLACKOUT), Some(1.0));
        assert!(darkness(DARK_IN + BLACK_FOR + CLEAR_IN / 2, BLACKOUT).is_some_and(|d| (d - 0.5).abs() < 0.01));
        assert_eq!(darkness(DARK_IN + BLACK_FOR + CLEAR_IN, BLACKOUT), None);
        assert_eq!(darkness(BLINK_IN, BLINK), Some(1.0));
        assert_eq!(darkness(BLINK_IN + BLINK_BLACK + BLINK_OUT, BLINK), None);
    }
}
