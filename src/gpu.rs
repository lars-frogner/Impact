pub mod buffer;
pub mod compute;
mod device;
pub mod push_constant;
pub mod rendering;
pub mod resource_group;
pub mod shader;
pub mod storage;
pub mod texture;
pub mod uniform;

pub use device::GraphicsDevice;

use crate::window::Window;
use anyhow::Result;
use rendering::surface::RenderingSurface;
use std::sync::Arc;

/// Creates a rendering surface for the given window, connects to a graphics
/// device compatible with the surface and initializes the surface for
/// presentation through the connected graphics device.
///
/// # Errors
/// See [`RenderingSurface::new`] and [`GraphicsDevice::connect`].
pub fn initialize_for_rendering(
    window: &Window,
) -> Result<(Arc<GraphicsDevice>, RenderingSurface)> {
    let wgpu_instance = create_wgpu_instance();

    let mut rendering_surface = RenderingSurface::new(&wgpu_instance, window)?;

    let graphics_device = pollster::block_on(GraphicsDevice::connect(
        &wgpu_instance,
        wgpu::Features::PUSH_CONSTANTS
            | wgpu::Features::POLYGON_MODE_LINE
            | wgpu::Features::DEPTH32FLOAT_STENCIL8
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::FLOAT32_FILTERABLE,
        wgpu::Limits {
            max_bind_groups: 7,
            max_push_constant_size: 128,
            max_color_attachment_bytes_per_sample: 64,
            ..wgpu::Limits::default()
        },
        wgpu::MemoryHints::Performance,
        Some(rendering_surface.surface()),
    ))?;

    rendering_surface.initialize_for_device(&graphics_device);

    Ok((Arc::new(graphics_device), rendering_surface))
}

/// Creates a new instance of `wgpu`.
fn create_wgpu_instance() -> wgpu::Instance {
    // Allow all backends
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
    })
}
