//! The phone in the game (owner 2026-11-01: "a phone that shows up in the game at a key"): while the app says
//! it's out (`HostBlock::phone_shown`), its frame (the app's phone page, `FrameKind::Phone`) is drawn over the
//! game's picture as it's presented, on the right, most of the screen's height.
//!
//! It's drawn on the game's own immediate context, in a device context state of its own
//! (`SwapDeviceContextState`): the game's state is swapped out whole and back after, untouched.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

use westplay_shared::{PHONE_HEIGHT, PHONE_WIDTH};
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
const HEIGHT_OF_SCREEN: f32 = 0.82;
const RIGHT_GAP_OF_SCREEN: f32 = 0.04;

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

struct Drawing {
    device: ID3D11Device,
    frames: FrameSource,
    state: ID3DDeviceContextState,
    vertex: ID3D11VertexShader,
    pixel: ID3D11PixelShader,
    place: ID3D11Buffer,
    blend: ID3D11BlendState,
    rasterizer: ID3D11RasterizerState,
    sampler: ID3D11SamplerState,
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
    if bridge.host.phone_shown.load(Ordering::Acquire) == 0 || !bridge::host_alive() {
        return;
    }
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
        if let Err(e) = made.draw(swapchain) {
            log!("phone: not drawn: {e}");
            *drawing = Some(Err(()));
        }
    }
}

impl Drawing {
    /// # Safety
    /// The game's live device, on its render thread.
    unsafe fn new(device: &ID3D11Device) -> windows::core::Result<Self> {
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
            log!("phone: ready to be drawn");
            Ok(Self {
                device: device.clone(),
                frames: FrameSource::of(FrameKind::Phone),
                state: state.ok_or_else(none)?,
                vertex: vertex.ok_or_else(none)?,
                pixel: pixel.ok_or_else(none)?,
                place: place.ok_or_else(none)?,
                blend: blend.ok_or_else(none)?,
                rasterizer: rasterizer.ok_or_else(none)?,
                sampler: sampler.ok_or_else(none)?,
                view: None,
            })
        }
    }

    /// # Safety
    /// As [`on_present`]: the game's swap chain, of this drawing's device.
    unsafe fn draw(&mut self, swapchain: &IDXGISwapChain) -> windows::core::Result<()> {
        // SAFETY: the game's live device, context and back buffer, on its render thread; the game's context state
        // is swapped back as it was before this returns.
        unsafe {
            let context = self.device.GetImmediateContext()?;
            // No frame of the phone's yet (the app opening it): nothing to draw.
            let Some((frame, _)) = self.frames.update(&self.device, &context) else { return Ok(()) };
            let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
            let mut back_desc = D3D11_TEXTURE2D_DESC::default();
            back.GetDesc(&mut back_desc);
            let mut target = None;
            self.device.CreateRenderTargetView(&back, None, Some(&mut target))?;
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

            let (width, height) = (back_desc.Width as f32, back_desc.Height as f32);
            let rect = place_on(width, height);
            context.UpdateSubresource(&self.place, 0, None, rect.as_ptr().cast(), 0, 0);

            let context1: ID3D11DeviceContext1 = context.cast()?;
            let mut game = None;
            context1.SwapDeviceContextState(&self.state, Some(&mut game));
            draw_quad(&context, self, target.as_ref(), view, width, height);
            if let Some(game) = &game {
                context1.SwapDeviceContextState(game, None);
            }
            Ok(())
        }
    }
}

/// # Safety
/// The drawing's live context, in its own context state.
unsafe fn draw_quad(
    context: &ID3D11DeviceContext,
    drawing: &Drawing,
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

/// Where the phone goes on a `width` x `height` screen, in clip space: left, top, right, bottom.
fn place_on(width: f32, height: f32) -> [f32; 4] {
    let tall = (height * HEIGHT_OF_SCREEN).round();
    let wide = (tall * PHONE_WIDTH as f32 / PHONE_HEIGHT as f32).round();
    let right = width - (height * RIGHT_GAP_OF_SCREEN).round();
    let left = right - wide;
    let top = ((height - tall) / 2.0).round();
    let bottom = top + tall;
    [left / width * 2.0 - 1.0, 1.0 - top / height * 2.0, right / width * 2.0 - 1.0, 1.0 - bottom / height * 2.0]
}

/// How the phone's frame is read for a back buffer of `format`: its pixels are sRGB-encoded, so decoded for a
/// buffer that encodes again as it's written (an sRGB one) or holds light as it is (a floating-point one).
fn view_format(format: DXGI_FORMAT) -> DXGI_FORMAT {
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
        let [left, top, right, bottom] = place_on(1920.0, 1080.0);
        assert!(right < 1.0 && right > 0.8);
        assert!(top > 0.0 && top < 1.0 && bottom < 0.0 && bottom > -1.0);
        let wide = (right - left) / 2.0 * 1920.0;
        let tall = (top - bottom) / 2.0 * 1080.0;
        assert!((wide / tall - PHONE_WIDTH as f32 / PHONE_HEIGHT as f32).abs() < 0.01);
    }
}
