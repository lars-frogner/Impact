use anyhow::Result;
use impact_gpu::{device::GraphicsDevice, vertex_attribute_ranges, wgpu};
use impact_rendering::surface::RenderingSurface;
use std::num::NonZeroU32;

/// Interface to a graphics device and a surface that can be rendered to.
#[derive(Debug)]
pub struct GraphicsContext {
    pub device: GraphicsDevice,
    pub surface: RenderingSurface,
}

/// Connects to a graphics device and creates a headless rendering surface
/// (backed by an ordinary texture instead of a window) with the given
/// dimensions in physical pixels.
///
/// # Errors
/// See [`GraphicsDevice::connect`].
pub fn initialize_for_headless_rendering(
    width: NonZeroU32,
    height: NonZeroU32,
) -> Result<GraphicsContext> {
    let wgpu_instance = create_wgpu_instance();

    let mut rendering_surface = RenderingSurface::new_headless(width, height);

    let graphics_device =
        connect_to_graphics_device_for_rendering(&wgpu_instance, &mut rendering_surface)?;

    Ok(GraphicsContext {
        device: graphics_device,
        surface: rendering_surface,
    })
}

/// Creates a rendering surface for the given window, connects to a graphics
/// device compatible with the surface and initializes the surface for
/// presentation through the connected graphics device.
///
/// # Errors
/// See [`RenderingSurface::new_for_window`] and [`GraphicsDevice::connect`].
#[cfg(feature = "window")]
pub fn initialize_for_window_rendering(window: &crate::window::Window) -> Result<GraphicsContext> {
    let wgpu_instance = create_wgpu_instance();

    let mut rendering_surface = RenderingSurface::new_for_window(&wgpu_instance, window)?;

    let graphics_device =
        connect_to_graphics_device_for_rendering(&wgpu_instance, &mut rendering_surface)?;

    Ok(GraphicsContext {
        device: graphics_device,
        surface: rendering_surface,
    })
}

/// Creates a new instance of `wgpu`.
pub fn create_wgpu_instance() -> wgpu::Instance {
    wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        backend_options: wgpu::BackendOptions::default(),
    })
}

/// Connects to a graphics device compatible with the given surface and
/// initializes the surface for presentation through the connected graphics
/// device.
///
/// # Errors
/// See [`GraphicsDevice::connect`].
pub fn connect_to_graphics_device_for_rendering(
    wgpu_instance: &wgpu::Instance,
    rendering_surface: &mut RenderingSurface,
) -> Result<GraphicsDevice> {
    let graphics_device = pollster::block_on(GraphicsDevice::connect(
        wgpu_instance,
        wgpu::Features::PUSH_CONSTANTS
            | wgpu::Features::DEPTH32FLOAT_STENCIL8
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::FLOAT32_FILTERABLE
            | wgpu::Features::INDIRECT_FIRST_INSTANCE
            | wgpu::Features::DEPTH_CLIP_CONTROL,
        wgpu::Features::POLYGON_MODE_LINE
            | wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
        wgpu::Limits {
            max_bind_groups: 7,
            max_push_constant_size: 256,
            max_color_attachment_bytes_per_sample: 64,
            max_vertex_attributes: vertex_attribute_ranges::TOTAL_LOCATIONS,
            ..wgpu::Limits::default()
        },
        wgpu::MemoryHints::Performance,
        rendering_surface.presentable_surface(),
    ))?;

    rendering_surface.initialize_for_device(&graphics_device)?;

    Ok(graphics_device)
}
