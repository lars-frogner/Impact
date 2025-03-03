pub mod buffer;
pub mod compute;
mod device;
pub mod indirect;
pub mod push_constant;
pub mod query;
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
            | wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::POLYGON_MODE_LINE
            | wgpu::Features::DEPTH32FLOAT_STENCIL8
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::FLOAT32_FILTERABLE
            | wgpu::Features::MULTI_DRAW_INDIRECT
            | wgpu::Features::INDIRECT_FIRST_INSTANCE,
        wgpu::Limits {
            max_bind_groups: 7,
            max_push_constant_size: 256,
            max_color_attachment_bytes_per_sample: 64,
            // This is a workaround for a (presumably) bug introduced in
            // wgpu 0.24 where the highest `VertexAttribute::shader_location`
            // rather than the actual sum of the lengths of
            // `VertexBufferLayout::attributes` is compared against
            // `max_vertex_attributes`
            max_vertex_attributes: 32,
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
    wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        backend_options: wgpu::BackendOptions::default(),
    })
}
