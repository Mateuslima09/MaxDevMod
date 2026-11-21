//! Finding the game's rear-view mirror pictures (the reversing camera's next step, owner 2026-11-13: "use the
//! mirror, it's already a camera looking back").
//!
//! The game draws each mirror's view into a render target of its own and then shows it on the mirror. Which
//! target that is (its size and format) isn't known yet, so this is a one-off probe: a second after the player
//! puts the truck in reverse, every render pass of one whole game frame is copied as it ends (when the game
//! sets its next targets) and saved as a picture, in order, with a list of what each was. The owner looks at
//! the pictures and says which pass is the mirror; the next step then shows that one on the screen.
//!
//! Each pass is read back on the spot (the GPU waits for it), so the frame it's taken in stutters: at most
//! [`MAX_CAPTURES`] a game, [`COOLDOWN_FRAMES`] apart. Written to `Documents\WestPlay\logs\retrovisor`.

use std::ffi::c_void;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_DEVICE_CONTEXT_IMMEDIATE, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11DeviceContext, ID3D11RenderTargetView,
    ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};
use windows::core::Interface;

use crate::{bridge, log};

/// Captures a game at most (each stutters a frame).
/// 0 since the owner's captures of 2026-11-13 found the mirrors (`mirror`); 1 again from 2026-11-14 13:40 for the
/// trucks whose mirrors aren't found (glass mirrors: an Actros, a MAN), only while they aren't (`mirror::seen`).
/// 0 again from 2026-11-18: the mirrors are found on every truck now, and each capture stutters (owner: hitches).
const MAX_CAPTURES: u32 = 0;
/// Game frames between two captures (~20 s at 60 fps).
const COOLDOWN_FRAMES: u64 = 1200;
/// Game frames in reverse before the capture (~1 s: the view settled, the reversing screen shown).
const SETTLE_FRAMES: u64 = 60;
/// Passes saved in one frame at most.
const MAX_PASSES: usize = 160;
/// Pictures are saved at most this wide (the size said in the list is the real one).
const MAX_PICTURE_WIDTH: u32 = 640;
/// The camera counts as in the cab if the dashboard's mark was drawn this few frames ago.
const CAB_LATELY: u64 = 30;

/// Whether this frame's passes are being taken.
static ARMED: AtomicBool = AtomicBool::new(false);
/// Captures made so far.
static CAPTURES: AtomicU32 = AtomicU32::new(0);
/// The frame the last capture was taken in (0: none).
static LAST_CAPTURE: AtomicU64 = AtomicU64::new(0);
/// The frame the truck was first seen in reverse, this time (0: not in reverse).
static REVERSE_SINCE: AtomicU64 = AtomicU64::new(0);

/// One pass as read back: its target's description and its picture (RGB, top row first).
struct Pass {
    desc: D3D11_TEXTURE2D_DESC,
    /// Which of the pass's targets (0: the first).
    slot: usize,
    picture: Option<Picture>,
    /// Why there's no picture.
    skipped: &'static str,
}

struct Picture {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

#[derive(Default)]
struct Capture {
    /// The targets set now, held (each a texture and its description), to be read when the pass ends.
    bound: Vec<(ID3D11Texture2D, D3D11_TEXTURE2D_DESC)>,
    /// The bound targets' addresses, to tell a pass that sets the same targets again from a new one.
    bound_addresses: Vec<usize>,
    passes: Vec<Pass>,
    /// Target sets the game set, this frame (passes with the same targets again included).
    sets: u32,
}

struct Shared(Capture);
// SAFETY: only used on the game's render thread (its immediate context's calls), through the mutex.
unsafe impl Send for Shared {}

static STATE: LazyLock<Mutex<Shared>> = LazyLock::new(|| Mutex::new(Shared(Capture::default())));

fn state() -> std::sync::MutexGuard<'static, Shared> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Called at the start of `OMSetRenderTargets*`, with the game's context: while armed, the pass that ends here
/// (the targets set before) is read back, and the new targets are held.
///
/// # Safety
/// From the `OMSetRenderTargets*` hooks, with the game's arguments, on its render thread.
pub unsafe fn on_set_render_targets(context: *mut c_void, count: u32, views: *const *mut c_void) {
    if !ARMED.load(Ordering::Relaxed) {
        return;
    }
    // SAFETY: the game's live context and its arguments (the caller's contract): `count` views (checked for a
    // sane number), each a live view or null.
    unsafe {
        let Some(context) = ID3D11DeviceContext::from_raw_borrowed(&context) else { return };
        // Deferred contexts only record; their passes run later, on the immediate one.
        if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
            return;
        }
        let views = if views.is_null() || count == 0 || count > 8 {
            &[][..]
        } else {
            std::slice::from_raw_parts(views, count as usize)
        };
        let addresses: Vec<usize> = views.iter().map(|view| *view as usize).collect();
        let mut state = state();
        let capture = &mut state.0;
        capture.sets += 1;
        if addresses == capture.bound_addresses {
            return;
        }
        read_bound(context, capture);
        capture.bound_addresses = addresses;
        for view in views {
            let Some(view) = ID3D11RenderTargetView::from_raw_borrowed(view) else { continue };
            let Ok(texture) = view.GetResource().and_then(|resource| resource.cast::<ID3D11Texture2D>()) else {
                continue;
            };
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc);
            capture.bound.push((texture, desc));
        }
    }
}

/// Reads back the pass that ends now (the targets held), then lets them go.
///
/// # Safety
/// The game's live immediate context, on its render thread.
unsafe fn read_bound(context: &ID3D11DeviceContext, capture: &mut Capture) {
    let bound = std::mem::take(&mut capture.bound);
    for (slot, (texture, desc)) in bound.into_iter().enumerate() {
        if capture.passes.len() >= MAX_PASSES {
            return;
        }
        // SAFETY: the game's live context and a texture of its, held (the caller's contract).
        let (picture, skipped) = match unsafe { read_back(context, &texture, &desc) } {
            Ok(picture) => (Some(picture), ""),
            Err(why) => (None, why),
        };
        capture.passes.push(Pass { desc, slot, picture, skipped });
    }
}

/// The texture's first mip and slice as a picture, read back through a staging copy (MSAA resolved first).
///
/// # Safety
/// The game's live immediate context and one of its textures, on its render thread.
unsafe fn read_back(
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    desc: &D3D11_TEXTURE2D_DESC,
) -> Result<Picture, &'static str> {
    let Some(pixel) = PixelFormat::of(desc.Format) else { return Err("formato não lido") };
    if desc.Width == 0 || desc.Height == 0 || desc.Width > 8192 || desc.Height > 8192 {
        return Err("tamanho fora do esperado");
    }
    // SAFETY: the game's live context and texture (the caller's contract); the textures made here are ours, the
    // mapped memory read within its rows only while mapped.
    unsafe {
        let device = context.GetDevice().map_err(|_| "sem device")?;
        let plain = D3D11_TEXTURE2D_DESC {
            Width: desc.Width,
            Height: desc.Height,
            MipLevels: 1,
            ArraySize: 1,
            Format: desc.Format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };
        let mut staging = None;
        device.CreateTexture2D(&plain, None, Some(&mut staging)).map_err(|_| "cópia não criada")?;
        let staging = staging.ok_or("cópia não criada")?;
        if desc.SampleDesc.Count > 1 {
            // Resolved into a plain texture first (a staging one can't be resolved into).
            let resolved_desc = D3D11_TEXTURE2D_DESC { Usage: D3D11_USAGE_DEFAULT, CPUAccessFlags: 0, ..plain };
            let mut resolved = None;
            device.CreateTexture2D(&resolved_desc, None, Some(&mut resolved)).map_err(|_| "MSAA não resolvido")?;
            let resolved = resolved.ok_or("MSAA não resolvido")?;
            context.ResolveSubresource(&resolved, 0, texture, 0, desc.Format);
            context.CopySubresourceRegion(&staging, 0, 0, 0, 0, &resolved, 0, None);
        } else {
            context.CopySubresourceRegion(&staging, 0, 0, 0, 0, texture, 0, None);
        }
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).map_err(|_| "cópia não lida")?;
        let step = desc.Width.div_ceil(MAX_PICTURE_WIDTH).max(1);
        let width = desc.Width / step;
        let height = desc.Height / step;
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        let base = mapped.pData as *const u8;
        let pitch = mapped.RowPitch as usize;
        for y in 0..height {
            let row = std::slice::from_raw_parts(base.add((y * step) as usize * pitch), pitch);
            for x in 0..width {
                let at = (x * step) as usize * pixel.bytes;
                rgb.extend_from_slice(&pixel.rgb(&row[at..at + pixel.bytes]));
            }
        }
        context.Unmap(&staging, 0);
        Ok(Picture { width, height, rgb })
    }
}

/// How a target's pixels are read as RGB.
#[derive(Clone, Copy)]
struct PixelFormat {
    bytes: usize,
    kind: Kind,
}

#[derive(Clone, Copy)]
enum Kind {
    Rgba8,
    Bgra8,
    Rgb10A2,
    Rgba16Float,
    Rg11B10Float,
    Rgba32Float,
    R8,
    Rg8,
    R16Float,
    Rg16Float,
    R32Float,
}

impl PixelFormat {
    fn of(format: DXGI_FORMAT) -> Option<Self> {
        let (bytes, kind) = match format.0 {
            27..=29 => (4, Kind::Rgba8),
            87 | 88 | 90..=93 => (4, Kind::Bgra8),
            23 | 24 => (4, Kind::Rgb10A2),
            9 | 10 => (8, Kind::Rgba16Float),
            26 => (4, Kind::Rg11B10Float),
            1 | 2 => (16, Kind::Rgba32Float),
            60 | 61 => (1, Kind::R8),
            48 | 49 => (2, Kind::Rg8),
            53 | 54 => (2, Kind::R16Float),
            33 | 34 => (4, Kind::Rg16Float),
            39 | 41 => (4, Kind::R32Float),
            _ => return None,
        };
        Some(Self { bytes, kind })
    }

    fn rgb(self, p: &[u8]) -> [u8; 3] {
        let f16 = |at: usize| half(u16::from_le_bytes([p[at], p[at + 1]]));
        let f32_ = |at: usize| f32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]]);
        match self.kind {
            Kind::Rgba8 => [p[0], p[1], p[2]],
            Kind::Bgra8 => [p[2], p[1], p[0]],
            Kind::Rgb10A2 => {
                let v = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
                [(v & 0x3ff) >> 2, (v >> 10 & 0x3ff) >> 2, (v >> 20 & 0x3ff) >> 2].map(|c| c as u8)
            }
            Kind::Rgba16Float => [f16(0), f16(2), f16(4)].map(hdr),
            Kind::Rg11B10Float => {
                let v = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
                [small_float(v & 0x7ff, 6), small_float(v >> 11 & 0x7ff, 6), small_float(v >> 22 & 0x3ff, 5)].map(hdr)
            }
            Kind::Rgba32Float => [f32_(0), f32_(4), f32_(8)].map(hdr),
            Kind::R8 => [p[0]; 3],
            Kind::Rg8 => [p[0], p[1], 0],
            Kind::R16Float => [hdr(f16(0)); 3],
            Kind::Rg16Float => [hdr(f16(0)), hdr(f16(2)), 0],
            Kind::R32Float => [hdr(f32_(0)); 3],
        }
    }
}

/// A linear (HDR) value as an 8-bit one to look at: Reinhard, then gamma.
fn hdr(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    let mapped = value / (1.0 + value);
    (mapped.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
}

/// An IEEE half float.
fn half(bits: u16) -> f32 {
    let exponent = (bits >> 10 & 0x1f) as i32;
    let mantissa = (bits & 0x3ff) as f32;
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    sign * match exponent {
        0 => mantissa * 2f32.powi(-24),
        31 => f32::INFINITY,
        _ => (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

/// An unsigned float of R11G11B10 (5 exponent bits, `mantissa_bits` mantissa bits).
pub(crate) fn small_float(bits: u32, mantissa_bits: u32) -> f32 {
    let exponent = (bits >> mantissa_bits) as i32;
    let scale = (1u32 << mantissa_bits) as f32;
    let mantissa = (bits & ((1 << mantissa_bits) - 1)) as f32;
    match exponent {
        0 => mantissa / scale * 2f32.powi(-14),
        31 => f32::INFINITY,
        _ => (1.0 + mantissa / scale) * 2f32.powi(exponent - 15),
    }
}

/// Called at each `Present`, before it: arms the next frame once the truck has been in reverse a second (within
/// the limits), and saves a frame's passes once it's been taken.
///
/// # Safety
/// `swapchain` is the game's, alive through its `Present`, on its render thread.
pub unsafe fn on_present(swapchain: *mut c_void, frame: u64) {
    if ARMED.swap(false, Ordering::Relaxed) {
        // SAFETY: as this function's.
        unsafe { finish(swapchain, frame) };
        return;
    }
    let Some(bridge) = bridge::get() else { return };
    let plugin = &bridge.plugin;
    let reversing = plugin.telemetry.read().is_some_and(|t| t.electric_enabled && (t.displayed_gear < 0 || t.gear < 0));
    if !reversing || crate::telemetry::paused() || crate::mirror::seen() {
        REVERSE_SINCE.store(0, Ordering::Relaxed);
        return;
    }
    let since = REVERSE_SINCE.load(Ordering::Relaxed);
    if since == 0 {
        REVERSE_SINCE.store(frame, Ordering::Relaxed);
        return;
    }
    let last = LAST_CAPTURE.load(Ordering::Relaxed);
    if frame - since < SETTLE_FRAMES
        || CAPTURES.load(Ordering::Relaxed) >= MAX_CAPTURES
        || (last != 0 && frame - last < COOLDOWN_FRAMES)
        // Once a reverse: the next waits for another.
        || last > since
    {
        return;
    }
    LAST_CAPTURE.store(frame, Ordering::Relaxed);
    *state() = Shared(Capture::default());
    ARMED.store(true, Ordering::Relaxed);
}

/// The frame's last pass read back, then everything saved (on a thread of its own).
///
/// # Safety
/// As [`on_present`].
unsafe fn finish(swapchain: *mut c_void, frame: u64) {
    let number = CAPTURES.fetch_add(1, Ordering::Relaxed) + 1;
    let capture = {
        let mut state = state();
        // SAFETY: the game's live swap chain (the caller's contract), only asked for its device's context.
        unsafe {
            if let Some(swapchain) = windows::Win32::Graphics::Dxgi::IDXGISwapChain::from_raw_borrowed(&swapchain)
                && let Ok(device) = swapchain.GetDevice::<windows::Win32::Graphics::Direct3D11::ID3D11Device>()
                && let Ok(context) = device.GetImmediateContext()
            {
                read_bound(&context, &mut state.0);
            }
        }
        std::mem::take(&mut state.0)
    };
    let in_cab = bridge::get().is_some_and(|bridge| {
        let plugin = &bridge.plugin;
        frame.saturating_sub(plugin.cab_frame.load(Ordering::Relaxed)) <= CAB_LATELY
    });
    log!(
        "retrovisor: capture {number} taken in frame {frame}: {} passes ({} target sets), camera in the cab: {in_cab}",
        capture.passes.len(),
        capture.sets
    );
    let dir = crate::log::log_dir().join("retrovisor").join(format!("captura-{number}"));
    let _ = std::thread::Builder::new().name("westplay-mirror-probe".into()).spawn(move || {
        save(&dir, &capture, frame, in_cab);
    });
}

/// The pictures and their list written to `dir` (emptied first).
fn save(dir: &PathBuf, capture: &Capture, frame: u64, in_cab: bool) {
    let _ = std::fs::remove_dir_all(dir);
    if let Err(e) = std::fs::create_dir_all(dir) {
        log!("retrovisor: cannot make {}: {e}", dir.display());
        return;
    }
    let mut list = format!(
        "# Passes de um quadro do jogo, na ordem (quadro {frame}). Câmera na cabine: {}.\n\
         # n  arquivo  largura x altura  formato  amostras  bind  mips  array  alvo\n",
        if in_cab { "sim" } else { "não (ou não visto)" }
    );
    for (n, pass) in capture.passes.iter().enumerate() {
        let d = &pass.desc;
        let name = format!("{:03}_{}x{}_f{}.bmp", n + 1, d.Width, d.Height, d.Format.0);
        let file = match &pass.picture {
            Some(picture) => match std::fs::write(dir.join(&name), bmp(picture)) {
                Ok(()) => name,
                Err(_) => "(não gravado)".to_owned(),
            },
            None => format!("({})", pass.skipped),
        };
        let _ = writeln!(
            list,
            "{:>3}  {file}  {}x{}  {}  {}  {:#x}  {}  {}  {}",
            n + 1,
            d.Width,
            d.Height,
            d.Format.0,
            d.SampleDesc.Count,
            d.BindFlags,
            d.MipLevels,
            d.ArraySize,
            pass.slot
        );
    }
    let _ = std::fs::write(dir.join("lista.txt"), list);
    log!("retrovisor: capture saved in {}", dir.display());
}

/// A picture as a 24-bit BMP.
fn bmp(picture: &Picture) -> Vec<u8> {
    let row = (picture.width * 3).next_multiple_of(4);
    let size = 54 + row * picture.height;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(picture.width as i32).to_le_bytes());
    // Negative: top row first.
    out.extend_from_slice(&(-(picture.height as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in 0..picture.height as usize {
        let start = y * picture.width as usize * 3;
        for pixel in picture.rgb[start..start + picture.width as usize * 3].chunks_exact(3) {
            out.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
        out.resize(out.len() + (row - picture.width * 3) as usize, 0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_float_formats_the_game_may_draw_in() {
        assert_eq!(half(0x3c00), 1.0);
        assert_eq!(half(0xc000), -2.0);
        assert_eq!(half(0), 0.0);
        // R11G11B10: 1.0 is exponent 15, mantissa 0.
        assert_eq!(small_float(15 << 6, 6), 1.0);
        assert_eq!(small_float(15 << 5, 5), 1.0);
        assert_eq!(hdr(0.0), 0);
        assert_eq!(hdr(f32::NAN), 0);
        // Brighter is brighter, never past white.
        assert!(hdr(0.5) < hdr(1.0) && hdr(1.0) < hdr(100.0) && hdr(1e9) == 255);
    }

    #[test]
    fn reads_eight_bit_pixels_in_either_order() {
        let rgba = PixelFormat::of(DXGI_FORMAT(28)).unwrap();
        let bgra = PixelFormat::of(DXGI_FORMAT(87)).unwrap();
        assert_eq!(rgba.rgb(&[1, 2, 3, 4]), [1, 2, 3]);
        assert_eq!(bgra.rgb(&[1, 2, 3, 4]), [3, 2, 1]);
        // Depth formats aren't pictures.
        assert!(PixelFormat::of(DXGI_FORMAT(40)).is_none());
    }

    #[test]
    fn writes_a_bmp_with_padded_rows() {
        let picture = Picture { width: 3, height: 2, rgb: vec![255; 18] };
        let file = bmp(&picture);
        // 3 pixels of 3 bytes padded to 12 per row.
        assert_eq!(file.len(), 54 + 12 * 2);
        assert_eq!(&file[0..2], b"BM");
        assert_eq!(u32::from_le_bytes(file[2..6].try_into().unwrap()) as usize, file.len());
    }
}
