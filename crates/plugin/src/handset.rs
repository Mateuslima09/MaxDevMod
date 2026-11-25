//! The phone in the game (owner 2026-11-01: "a phone that shows up in the game at a key"): while the app says
//! it's out (`HostBlock::phone_shown`), its frame (the app's phone page, `FrameKind::Phone`) is drawn over the
//! game's picture as it's presented, on the right, most of the screen's height.
//!
//! Put away with a notification on it (`westplay_shared::PHONE_PEEKS`, owner 2026-11-11: as GTA RP's phones), only
//! its top shows, at the bottom of the screen: it slides up into view, and down again once the notification is
//! gone ([`Slide`]).
//!
//! It's drawn on the game's own immediate context, in a device context state of its own
//! (`SwapDeviceContextState`): the game's state is swapped out whole and back after, untouched.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use westplay_shared::{PHONE_HEIGHT, PHONE_PAPER, PHONE_PEEKS, PHONE_WIDTH};
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D_SRV_DIMENSION_TEXTURE2D, ID3DBlob};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
    D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_CREATE_DEVICE_SINGLETHREADED, D3D11_CULL_NONE,
    D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
    D3D11_SAMPLER_DESC, D3D11_SDK_VERSION, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0,
    D3D11_TEX2D_SRV, D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT,
    ID3D11BlendState, ID3D11Buffer, ID3D11Device, ID3D11Device1, ID3D11DeviceContext, ID3D11DeviceContext1,
    ID3D11PixelShader, ID3D11RasterizerState, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D,
    ID3D11VertexShader, ID3DDeviceContextState,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
    DXGI_FORMAT_R16G16B16A16_FLOAT,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::{Interface, PCSTR, s};

use crate::frame::{FrameKind, FrameSource};
use crate::{bridge, log};

/// The phone's height on the game's screen, of the screen's; and its gap to the screen's right edge, of the
/// screen's height.
const HEIGHT_OF_SCREEN: f32 = 0.62;
const RIGHT_GAP_OF_SCREEN: f32 = 0.04;
/// Vertical iPad mode: slightly taller and tucked cleanly near the right edge for crisp readability.
const HEIGHT_OF_SCREEN_IPAD: f32 = 0.78;
const RIGHT_GAP_OF_SCREEN_IPAD: f32 = 0.03;
/// How much of the phone shows while it peeks, of its height (its status bar and a notification under it); and
/// how long it takes to slide there, or away.
/// (A fifth showed half the banner, the gift's words cut: owner 2026-11-12, "the phone is too low".)
const PEEK_OF_PHONE: f32 = 0.34;
/// A paper to sign in the phone's frame (`PHONE_PAPER`: the cargo's nota fiscal): its height on the game's screen,
/// of the screen's, in the middle of it.
const PAPER_OF_SCREEN: f32 = 0.9;
const SLIDE_TIME: Duration = Duration::from_millis(280);

/// Where the phone stands: whole on the screen, only its top showing, or under the screen's bottom edge.
const WHOLE: f32 = 0.0;
const PEEKING: f32 = 1.0;
const AWAY: f32 = 2.0;

/// The phone's way between where it stands: it slides to its peek and away from it (and from the peek up into
/// the hand); taken out or put away with its key, it's there or gone at once, as it always was.
#[derive(Clone, Copy, Debug)]
struct Slide {
    /// `HostBlock::phone_shown` as last seen.
    shown: u32,
    from: f32,
    to: f32,
    started: Option<Instant>,
}

impl Slide {
    const fn new() -> Self {
        Self { shown: 0, from: AWAY, to: AWAY, started: None }
    }

    /// Where the phone stands `now`, with the app saying `shown`; `None`: nowhere (away, nothing to draw).
    fn at(&mut self, shown: u32, now: Instant) -> Option<f32> {
        if shown != self.shown {
            let here = self.position(now);
            let target = match shown {
                0 => AWAY,
                PHONE_PEEKS => PEEKING,
                _ => WHOLE,
            };
            // Only to or from the peek does it slide.
            let slides = shown == PHONE_PEEKS || self.shown == PHONE_PEEKS;
            (self.from, self.to) = (if slides { here } else { target }, target);
            self.started = slides.then_some(now);
            self.shown = shown;
        }
        let position = self.position(now);
        (position < AWAY).then_some(position)
    }

    fn position(&self, now: Instant) -> f32 {
        let Some(started) = self.started else { return self.to };
        let done = (now.saturating_duration_since(started).as_secs_f32() / SLIDE_TIME.as_secs_f32()).clamp(0.0, 1.0);
        // Quick at first, settling in.
        let eased = 1.0 - (1.0 - done).powi(3);
        self.from + (self.to - self.from) * eased
    }
}

static SLIDE: Mutex<Slide> = Mutex::new(Slide::new());

const SHADERS: &str = r"
cbuffer Place : register(b0) { float4 rect; };
struct Vertex { float4 position : SV_Position; float2 uv : TEXCOORD0; };
Vertex vs(uint id : SV_VertexID) {
    float2 corner = float2(id & 1, id >> 1);
    Vertex v;
    v.position = float4(lerp(rect.x, rect.z, corner.x), lerp(rect.y, rect.w, corner.y), 0, 1);
    v.uv = corner;
    return v;
}
Texture2D phone : register(t0);
SamplerState smooth : register(s0);
float4 ps(Vertex v) : SV_Target { return phone.Sample(smooth, v.uv); }
";

/// What a picture is drawn over the game's with: a context state of its own, the shaders, a place, a blend of
/// premultiplied pixels (also the key hints bar's, `hints`).
pub(crate) struct Quad {
    state: ID3DDeviceContextState,
    vertex: ID3D11VertexShader,
    pixel: ID3D11PixelShader,
    place: ID3D11Buffer,
    blend: ID3D11BlendState,
    rasterizer: ID3D11RasterizerState,
    sampler: ID3D11SamplerState,
}

struct Drawing {
    device: ID3D11Device,
    frames: FrameSource,
    quad: Quad,
    /// The view of the phone's frame, for the frame texture and format it was made for.
    view: Option<(ID3D11Texture2D, DXGI_FORMAT, ID3D11ShaderResourceView)>,
}

// SAFETY: used only on the game's render thread (Present), the lock only keeping it there.
unsafe impl Send for Drawing {}

/// What it's drawn with, made on the first frame it's out; `Err` once that failed (said once, not tried again).
static DRAWING: Mutex<Option<Result<Drawing, ()>>> = Mutex::new(None);

/// At each of the game's presents: the phone over its picture, while it's out.
///
/// # Safety
/// The game's live swap chain, in its `Present`, on its render thread.
pub unsafe fn on_present(swapchain: *mut c_void) {
    let Some(bridge) = bridge::get() else { return };
    let shown = if bridge::host_alive() { bridge.host.phone_shown.load(Ordering::Acquire) } else { 0 };
    let position = SLIDE.lock().unwrap_or_else(|e| e.into_inner()).at(shown, Instant::now());
    let Some(position) = position else { return };
    // SAFETY: the game's live swap chain, on its render thread (the caller's contract).
    unsafe {
        let Some(swapchain) = IDXGISwapChain::from_raw_borrowed(&swapchain) else { return };
        let Ok(device) = swapchain.GetDevice::<ID3D11Device>() else { return };
        let mut drawing = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
        // Made anew for another device (the game's renderer restarted).
        if matches!(&*drawing, Some(Ok(made)) if made.device != device) {
            *drawing = None;
        }
        if drawing.is_none() {
            *drawing = Some(Drawing::new(&device).map_err(|e| log!("phone: can't be drawn: {e}")));
        }
        let Some(Ok(made)) = drawing.as_mut() else { return };
        if let Err(e) = made.draw(swapchain, position, shown == PHONE_PAPER) {
            log!("phone: not drawn: {e}");
            *drawing = Some(Err(()));
        }
    }
}

impl Drawing {
    /// # Safety
    /// The game's live device, on its render thread.
    unsafe fn new(device: &ID3D11Device) -> windows::core::Result<Self> {
        // SAFETY: as the caller says.
        let quad = unsafe { Quad::new(device) }?;
        log!("phone: ready to be drawn");
        Ok(Self { device: device.clone(), frames: FrameSource::of(FrameKind::Phone), quad, view: None })
    }
}

impl Quad {
    /// # Safety
    /// The game's live device, on its render thread.
    pub(crate) unsafe fn new(device: &ID3D11Device) -> windows::core::Result<Self> {
        // SAFETY: the game's live device (the caller's contract); every description is a local, the shaders' code
        // our own.
        unsafe {
            let device1: ID3D11Device1 = device.cast()?;
            let flags = if device.GetCreationFlags() & D3D11_CREATE_DEVICE_SINGLETHREADED.0 != 0 { 1 } else { 0 };
            let mut state = None;
            device1.CreateDeviceContextState(
                flags,
                &[device.GetFeatureLevel()],
                D3D11_SDK_VERSION,
                &ID3D11Device::IID,
                None,
                Some(&mut state),
            )?;
            let vertex_code = compile(s!("vs"), s!("vs_4_0"))?;
            let pixel_code = compile(s!("ps"), s!("ps_4_0"))?;
            let mut vertex = None;
            device.CreateVertexShader(blob_bytes(&vertex_code), None, Some(&mut vertex))?;
            let mut pixel = None;
            device.CreatePixelShader(blob_bytes(&pixel_code), None, Some(&mut pixel))?;
            let mut place = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: 16,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut place),
            )?;
            // The page's pixels come premultiplied (as Windows composes them).
            let mut blend_desc = D3D11_BLEND_DESC::default();
            blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: true.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            let mut blend = None;
            device.CreateBlendState(&blend_desc, Some(&mut blend))?;
            let mut rasterizer = None;
            device.CreateRasterizerState(
                &D3D11_RASTERIZER_DESC {
                    FillMode: D3D11_FILL_SOLID,
                    CullMode: D3D11_CULL_NONE,
                    DepthClipEnable: true.into(),
                    ..Default::default()
                },
                Some(&mut rasterizer),
            )?;
            let mut sampler = None;
            device.CreateSamplerState(
                &D3D11_SAMPLER_DESC {
                    Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                    AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                    MaxLOD: f32::MAX,
                    ..Default::default()
                },
                Some(&mut sampler),
            )?;
            let none = || windows::core::Error::empty();
            Ok(Self {
                state: state.ok_or_else(none)?,
                vertex: vertex.ok_or_else(none)?,
                pixel: pixel.ok_or_else(none)?,
                place: place.ok_or_else(none)?,
                blend: blend.ok_or_else(none)?,
                rasterizer: rasterizer.ok_or_else(none)?,
                sampler: sampler.ok_or_else(none)?,
            })
        }
    }

    /// Draws `view` at `rect` (clip space: left, top, right, bottom) over the swap chain's back buffer, in the
    /// quad's own context state (the game's swapped back after).
    ///
    /// # Safety
    /// The game's live device, context and swap chain, on its render thread.
    pub(crate) unsafe fn draw(
        &self,
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        swapchain: &IDXGISwapChain,
        view: &ID3D11ShaderResourceView,
        rect: [f32; 4],
    ) -> windows::core::Result<()> {
        // SAFETY: as the caller says; the game's context state is swapped back as it was before this returns.
        unsafe {
            let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
            let mut back_desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut back_desc);
            let mut target = None;
            device.CreateRenderTargetView(&back, None, Some(&mut target))?;
            context.UpdateSubresource(&self.place, 0, None, rect.as_ptr().cast(), 0, 0);
            let context1: ID3D11DeviceContext1 = context.cast()?;
            let mut game = None;
            context1.SwapDeviceContextState(&self.state, Some(&mut game));
            draw_quad(context, self, target.as_ref(), view, back_desc.Width as f32, back_desc.Height as f32);
            if let Some(game) = &game {
                context1.SwapDeviceContextState(game, None);
            }
            Ok(())
        }
    }
}

impl Drawing {
    /// # Safety
    /// As [`on_present`]: the game's swap chain, of this drawing's device.
    unsafe fn draw(&mut self, swapchain: &IDXGISwapChain, position: f32, paper: bool) -> windows::core::Result<()> {
        // SAFETY: the game's live device, context and back buffer, on its render thread; the game's context state
        // is swapped back as it was before this returns.
        unsafe {
            let context = self.device.GetImmediateContext()?;
            // No frame of the phone's yet (the app opening it): nothing to draw.
            let Some((frame, _)) = self.frames.update(&self.device, &context) else { return Ok(()) };
            let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
            let mut back_desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut back_desc);
            let format = view_format(back_desc.Format);
            if !matches!(&self.view, Some((texture, made, _)) if *texture == frame && *made == format) {
                let mut view = None;
                self.device.CreateShaderResourceView(
                    &frame,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: format,
                        ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
                        },
                    }),
                    Some(&mut view),
                )?;
                self.view = Some((frame.clone(), format, view.ok_or_else(windows::core::Error::empty)?));
            }
            let Some((_, _, view)) = &self.view else { return Ok(()) };
            let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
            frame.GetDesc(&mut tex_desc);
            let (tex_w, tex_h) = (tex_desc.Width.max(1), tex_desc.Height.max(1));
            let (width, height) = (back_desc.Width as f32, back_desc.Height as f32);
            let rect = if paper {
                paper_on(width, height, tex_w, tex_h)
            } else {
                place_on(width, height, position, tex_w, tex_h)
            };
            self.quad.draw(&self.device, &context, swapchain, view, rect)
        }
    }
}

/// # Safety
/// The drawing's live context, in its own context state.
unsafe fn draw_quad(
    context: &ID3D11DeviceContext,
    drawing: &Quad,
    target: Option<&windows::Win32::Graphics::Direct3D11::ID3D11RenderTargetView>,
    view: &ID3D11ShaderResourceView,
    width: f32,
    height: f32,
) {
    // SAFETY: the context and everything set on it are alive (the caller's contract and `drawing`'s).
    unsafe {
        context.OMSetRenderTargets(Some(&[target.cloned()]), None);
        context.OMSetBlendState(&drawing.blend, None, 0xffff_ffff);
        context.RSSetState(&drawing.rasterizer);
        context.RSSetViewports(Some(&[D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width,
            Height: height,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        }]));
        context.IASetInputLayout(None);
        context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
        context.VSSetShader(&drawing.vertex, None);
        context.VSSetConstantBuffers(0, Some(&[Some(drawing.place.clone())]));
        context.PSSetShader(&drawing.pixel, None);
        context.PSSetShaderResources(0, Some(&[Some(view.clone())]));
        context.PSSetSamplers(0, Some(&[Some(drawing.sampler.clone())]));
        context.Draw(4, 0);
        // Let go of the game's back buffer: it's resized only once nothing holds it.
        context.OMSetRenderTargets(Some(&[None]), None);
        context.PSSetShaderResources(0, Some(&[None]));
    }
}

/// Where the phone/iPad goes on a `width` x `height` screen, in clip space: left, top, right, bottom. `position`:
/// whole ([`WHOLE`]), only its top showing at the screen's bottom ([`PEEKING`]), under it ([`AWAY`]), or on its
/// way between.
fn place_on(width: f32, height: f32, position: f32, tex_w: u32, tex_h: u32) -> [f32; 4] {
    let is_ipad = tex_w > PHONE_WIDTH;
    let tall = (height * if is_ipad { HEIGHT_OF_SCREEN_IPAD } else { HEIGHT_OF_SCREEN }).round();
    let wide = (tall * tex_w as f32 / tex_h as f32).round();
    let right = width - (height * if is_ipad { RIGHT_GAP_OF_SCREEN_IPAD } else { RIGHT_GAP_OF_SCREEN }).round();
    let left = right - wide;
    let whole = (height - tall) / 2.0;
    let peeking = height - tall * PEEK_OF_PHONE;
    let top = if position <= PEEKING {
        whole + (peeking - whole) * position.max(WHOLE)
    } else {
        peeking + (height - peeking) * (position.min(AWAY) - PEEKING)
    }
    .round();
    let bottom = top + tall;
    [left / width * 2.0 - 1.0, 1.0 - top / height * 2.0, right / width * 2.0 - 1.0, 1.0 - bottom / height * 2.0]
}

/// Where a paper goes on a `width` x `height` screen, in clip space (left, top, right, bottom): the frame's own
/// shape, in the middle.
fn paper_on(width: f32, height: f32, tex_w: u32, tex_h: u32) -> [f32; 4] {
    let tall = (height * PAPER_OF_SCREEN).round();
    let wide = (tall * tex_w as f32 / tex_h as f32).round();
    let left = ((width - wide) / 2.0).round();
    let top = ((height - tall) / 2.0).round();
    let (right, bottom) = (left + wide, top + tall);
    [left / width * 2.0 - 1.0, 1.0 - top / height * 2.0, right / width * 2.0 - 1.0, 1.0 - bottom / height * 2.0]
}

/// How the phone's frame is read for a back buffer of `format`: its pixels are sRGB-encoded, so decoded for a
/// buffer that encodes again as it's written (an sRGB one) or holds light as it is (a floating-point one).
pub(crate) fn view_format(format: DXGI_FORMAT) -> DXGI_FORMAT {
    match format {
        DXGI_FORMAT_R8G8B8A8_UNORM_SRGB | DXGI_FORMAT_B8G8R8A8_UNORM_SRGB | DXGI_FORMAT_R16G16B16A16_FLOAT => {
            DXGI_FORMAT_B8G8R8A8_UNORM_SRGB
        }
        _ => DXGI_FORMAT_B8G8R8A8_UNORM,
    }
}

fn compile(entry: PCSTR, target: PCSTR) -> windows::core::Result<ID3DBlob> {
    let mut code = None;
    let mut errors = None;
    // SAFETY: the source is a static string of the length given; the blobs out are locals.
    let compiled = unsafe {
        D3DCompile(
            SHADERS.as_ptr().cast(),
            SHADERS.len(),
            s!("westplay-phone"),
            None,
            None,
            entry,
            target,
            0,
            0,
            &mut code,
            Some(&mut errors),
        )
    };
    if let Err(e) = compiled {
        let said = errors.map(|blob| String::from_utf8_lossy(blob_bytes(&blob)).into_owned()).unwrap_or_default();
        return Err(windows::core::Error::new(e.code(), said));
    }
    code.ok_or_else(windows::core::Error::empty)
}

fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
    // SAFETY: the blob's buffer is its own, alive and of its size while the blob is.
    unsafe { std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_shaders_compile() {
        compile(s!("vs"), s!("vs_4_0")).unwrap();
        compile(s!("ps"), s!("ps_4_0")).unwrap();
    }

    #[test]
    fn the_phone_stands_on_the_right_in_its_own_shape() {
        let [left, top, right, bottom] = place_on(1920.0, 1080.0, WHOLE, PHONE_WIDTH, PHONE_HEIGHT);
        assert!(right < 1.0 && right > 0.8);
        assert!(top > 0.0 && top < 1.0 && bottom < 0.0 && bottom > -1.0);
        let wide = (right - left) / 2.0 * 1920.0;
        let tall = (top - bottom) / 2.0 * 1080.0;
        assert!((wide / tall - PHONE_WIDTH as f32 / PHONE_HEIGHT as f32).abs() < 0.01);

        let [il, it, ir, ib] = place_on(1920.0, 1080.0, WHOLE, westplay_shared::IPAD_WIDTH, westplay_shared::IPAD_HEIGHT);
        let iw = (ir - il) / 2.0 * 1920.0;
        let ih = (it - ib) / 2.0 * 1080.0;
        assert!((iw / ih - westplay_shared::IPAD_WIDTH as f32 / westplay_shared::IPAD_HEIGHT as f32).abs() < 0.01);
        assert!(ih > tall);
    }

    #[test]
    fn a_paper_is_in_the_middle_of_the_screen_in_the_frames_shape() {
        let [left, top, right, bottom] = paper_on(1920.0, 1080.0, PHONE_WIDTH, PHONE_HEIGHT);
        assert!((left + right).abs() < 0.01 && (top + bottom).abs() < 0.01);
        let wide = (right - left) / 2.0 * 1920.0;
        let tall = (top - bottom) / 2.0 * 1080.0;
        assert!((wide / tall - PHONE_WIDTH as f32 / PHONE_HEIGHT as f32).abs() < 0.01);
        assert!(tall > 900.0);
    }

    /// Peeking, only the phone's top is on the screen, at its bottom, where it stands when it's out; away, none.
    #[test]
    fn only_its_top_shows_while_it_peeks() {
        let whole = place_on(1920.0, 1080.0, WHOLE, PHONE_WIDTH, PHONE_HEIGHT);
        let [left, top, right, bottom] = place_on(1920.0, 1080.0, PEEKING, PHONE_WIDTH, PHONE_HEIGHT);
        assert_eq!((left, right), (whole[0], whole[2]));
        let seen = (top + 1.0) / 2.0 * 1080.0;
        let tall = (top - bottom) / 2.0 * 1080.0;
        assert!((seen / tall - PEEK_OF_PHONE).abs() < 0.01, "{seen} of {tall}");
        assert!(bottom < -1.0);
        let [_, top, _, _] = place_on(1920.0, 1080.0, AWAY, PHONE_WIDTH, PHONE_HEIGHT);
        assert!(top <= -1.0);
    }

    /// A notification slides the phone's top into view and away again; its key takes it out and puts it away at
    /// once; out of a peek it comes up into the hand.
    #[test]
    fn slides_to_its_peek_and_away() {
        let start = Instant::now();
        let later = |ms: u64| start + Duration::from_millis(ms);
        let mut slide = Slide::new();
        assert_eq!(slide.at(0, start), None);
        // A notification: up from under the screen.
        slide.at(PHONE_PEEKS, later(0));
        let middle = slide.at(PHONE_PEEKS, later(100)).unwrap();
        assert!(middle > PEEKING && middle < AWAY, "{middle}");
        assert_eq!(slide.at(PHONE_PEEKS, later(1000)), Some(PEEKING));
        // Gone: down again, then nothing.
        slide.at(0, later(1100));
        let leaving = slide.at(0, later(1200)).unwrap();
        assert!(leaving > PEEKING && leaving < AWAY, "{leaving}");
        assert_eq!(slide.at(0, later(2000)), None);
        // Its key: there at once, gone at once.
        assert_eq!(slide.at(1, later(2100)), Some(WHOLE));
        assert_eq!(slide.at(0, later(2200)), None);
        // Taken out while it peeks: up into the hand.
        slide.at(PHONE_PEEKS, later(3000));
        assert_eq!(slide.at(PHONE_PEEKS, later(4000)), Some(PEEKING));
        slide.at(1, later(4100));
        let rising = slide.at(1, later(4200)).unwrap();
        assert!(rising > WHOLE && rising < PEEKING, "{rising}");
        assert_eq!(slide.at(1, later(5000)), Some(WHOLE));
    }
}
