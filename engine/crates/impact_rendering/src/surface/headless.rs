//! Render surfaces backed by a texture.

use anyhow::{Result, bail};
use impact_gpu::{device::GraphicsDevice, wgpu};
use std::num::NonZeroU32;

/// A render surface attached to a window.
#[derive(Debug)]
pub struct HeadlessRenderingSurface {
    /// Where graphics will be drawn.
    surface: SurfaceTexture,
}

#[derive(Debug)]
enum SurfaceTexture {
    Initialized(wgpu::Texture),
    Uninitialized {
        width: NonZeroU32,
        height: NonZeroU32,
    },
}

impl HeadlessRenderingSurface {
    /// The format used for the surface texture.
    pub const fn texture_format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8Unorm
    }

    /// Creates a rendering surface with the given dimensions.
    pub fn new(width: NonZeroU32, height: NonZeroU32) -> Self {
        let surface = SurfaceTexture::Uninitialized { width, height };
        Self { surface }
    }

    /// Creates the actual surface texture for the device.
    ///
    /// # Errors
    /// Returns an error if this method has already been called.
    pub fn initialize_for_device(&mut self, graphics_device: &GraphicsDevice) -> Result<()> {
        let SurfaceTexture::Uninitialized { width, height } = self.surface else {
            bail!("Headless surface texture was already initialized");
        };

        self.surface = SurfaceTexture::Initialized(Self::create_surface_texture(
            graphics_device.device(),
            width,
            height,
        ));

        Ok(())
    }

    /// Returns a reference to the surface texture.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    pub fn surface_texture(&self) -> &wgpu::Texture {
        self.surface
            .initialized()
            .expect("`initialize` must be called before `surface_texture`")
    }

    /// Creates a view into the surface texture.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    pub fn create_surface_texture_view(&self) -> wgpu::TextureView {
        self.surface
            .initialized()
            .expect("`initialize` must be called before `create_surface_texture_view`")
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Returns the `(width, height)` dimensions of the rendering surface in
    /// physical pixels.
    pub fn surface_dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        self.surface.dimensions()
    }

    /// Resizes the rendering surface to the given width and height.
    pub fn resize(
        &mut self,
        graphics_device: &GraphicsDevice,
        new_width: NonZeroU32,
        new_height: NonZeroU32,
    ) {
        self.surface = SurfaceTexture::Initialized(Self::create_surface_texture(
            graphics_device.device(),
            new_width,
            new_height,
        ));
    }

    /// Creates a new 2D [`wgpu::Texture`] with the given size for use as a
    /// surface target.
    fn create_surface_texture(
        device: &wgpu::Device,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: width.into(),
                height: height.into(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::texture_format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            label: Some("Surface texture"),
            view_formats: &[],
        })
    }
}

impl SurfaceTexture {
    fn initialized(&self) -> Option<&wgpu::Texture> {
        match self {
            Self::Initialized(texture) => Some(texture),
            Self::Uninitialized { .. } => None,
        }
    }

    fn dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        match self {
            Self::Initialized(texture) => {
                let size = texture.size();
                (
                    NonZeroU32::new(size.width).unwrap(),
                    NonZeroU32::new(size.height).unwrap(),
                )
            }
            Self::Uninitialized { width, height } => (*width, *height),
        }
    }
}
