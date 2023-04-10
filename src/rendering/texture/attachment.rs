//! Textures used as render attachments.

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use std::{fmt::Display, path::Path};

bitflags! {
    /// Bitflag encoding a set of quantities that can be rendered to dedicated
    /// color attachment textures.
    pub struct RenderAttachmentQuantitySet: u8 {
        const POSITION = 0b00000001;
        const NORMAL_VECTOR = 0b00000010;
        const TEXTURE_COORDS = 0b00000100;
    }
}

/// A quantity that can be rendered to a dedicated color attachment texture.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderAttachmentQuantity {
    Position = 0,
    NormalVector = 1,
    TextureCoords = 2,
}

/// Manager for textures used as render attachments.
#[derive(Debug)]
pub struct RenderAttachmentTextureManager {
    depth_texture: DepthTexture,
    multisampled_surface_texture: Option<MultisampledSurfaceTexture>,
    quantity_textures: [Option<RenderAttachmentTexture>; N_RENDER_ATTACHMENT_QUANTITIES],
    quantity_texture_bind_group_layouts:
        [Option<wgpu::BindGroupLayout>; N_RENDER_ATTACHMENT_QUANTITIES],
    quantity_texture_bind_groups: [Option<wgpu::BindGroup>; N_RENDER_ATTACHMENT_QUANTITIES],
    available_quantities: RenderAttachmentQuantitySet,
}

/// Texture for storing fragment depths.
#[derive(Debug)]
pub struct DepthTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// A surface texture that can be used as a multisampled render target.
#[derive(Debug)]
pub struct MultisampledSurfaceTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// A texture that can be used as a color attachment for rendering a specific
/// quantity into.
#[derive(Debug)]
pub struct RenderAttachmentTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

/// The total number of separate render attachment quantities.
pub const N_RENDER_ATTACHMENT_QUANTITIES: usize = 3;

/// The bitflag of each individual render attachment quantity.
pub const RENDER_ATTACHMENT_FLAGS: [RenderAttachmentQuantitySet; N_RENDER_ATTACHMENT_QUANTITIES] = [
    RenderAttachmentQuantitySet::POSITION,
    RenderAttachmentQuantitySet::NORMAL_VECTOR,
    RenderAttachmentQuantitySet::TEXTURE_COORDS,
];

/// The name of each individual render attachment quantity.
pub const RENDER_ATTACHMENT_NAMES: [&str; N_RENDER_ATTACHMENT_QUANTITIES] =
    ["position", "normal vector", "texture coords"];

/// The texture format used for each render attachment quantity.
pub const RENDER_ATTACHMENT_FORMATS: [wgpu::TextureFormat; N_RENDER_ATTACHMENT_QUANTITIES] = [
    wgpu::TextureFormat::Rgba32Float,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Rg32Float,
];

/// The texture and sampler bind group bindings used for each render attachment
/// quantity.
pub const RENDER_ATTACHMENT_BINDINGS: [(u32, u32); N_RENDER_ATTACHMENT_QUANTITIES] =
    [(0, 1), (0, 1), (0, 1)];

impl Display for RenderAttachmentQuantitySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ ")?;
        for (&attribute, name) in RENDER_ATTACHMENT_FLAGS
            .iter()
            .zip(RENDER_ATTACHMENT_NAMES.iter())
        {
            if self.contains(attribute) {
                write!(f, "`{}` ", name)?;
            }
        }
        write!(f, "}}")
    }
}

impl RenderAttachmentTextureManager {
    /// Creates a new manager for render attachment textures, initializing
    /// render attachment textures for the given set of quantities in addition
    /// to the depth texture and (if `sample_count` > 1) the multisampled
    /// surface texture.
    pub fn new(
        core_system: &CoreRenderingSystem,
        sample_count: u32,
        quantities: RenderAttachmentQuantitySet,
    ) -> Self {
        let depth_texture = DepthTexture::new(core_system, sample_count);

        let mut manager = Self {
            depth_texture,
            multisampled_surface_texture: None,
            quantity_textures: [None, None, None],
            quantity_texture_bind_group_layouts: [None, None, None],
            quantity_texture_bind_groups: [None, None, None],
            available_quantities: RenderAttachmentQuantitySet::empty(),
        };

        manager.recreate_multisampled_surface_texture(core_system, sample_count);

        manager.recreate_render_attachment_textures(core_system, quantities);

        manager
    }

    /// Returns a reference to the [`DepthTexture`].
    pub fn depth_texture(&self) -> &DepthTexture {
        &self.depth_texture
    }

    /// Returns a tuple where the first element is a view to the surface texture
    /// to use as an attachment and the second element, if not [`None`], is a
    /// view to the surface texture that should receive the resolved output.
    pub fn attachment_surface_view_and_resolve_target<'a, 'b, 'c>(
        &'a self,
        surface_texture_view: &'b wgpu::TextureView,
    ) -> (&'c wgpu::TextureView, Option<&'c wgpu::TextureView>)
    where
        'a: 'c,
        'b: 'c,
    {
        if let Some(multisampled_surface_texture) = self.multisampled_surface_texture.as_ref() {
            // If multisampling, use multisampled texture as render target
            // and surface texture as resolve target
            (
                multisampled_surface_texture.view(),
                Some(surface_texture_view),
            )
        } else {
            (surface_texture_view, None)
        }
    }

    /// Returns the set of available render attachment quantities.
    pub fn available_quantities(&self) -> RenderAttachmentQuantitySet {
        self.available_quantities
    }

    /// Returns an iterator over the render attachment texture views for the
    /// requested set of quantities, in the order in which the quantities are
    /// listed in the [`RENDER_ATTACHMENT_FLAGS`] constant.
    ///
    /// # Errors
    /// Returns an error if any of the requested quantities are missing.
    pub fn request_render_attachment_texture_views(
        &self,
        requested_quantities: RenderAttachmentQuantitySet,
    ) -> Result<impl Iterator<Item = &wgpu::TextureView>> {
        if self.available_quantities.contains(requested_quantities) {
            Ok(RENDER_ATTACHMENT_FLAGS
                .iter()
                .zip(self.quantity_textures.iter())
                .filter_map(move |(&quantity_flag, quantity_texture)| {
                    if requested_quantities.contains(quantity_flag) {
                        Some(quantity_texture.as_ref().unwrap().view())
                    } else {
                        None
                    }
                }))
        } else {
            Err(anyhow!(
                "Render attachment texture manager missing requested quantities: {}",
                requested_quantities.difference(self.available_quantities)
            ))
        }
    }

    /// Returns an iterator over the render attachment texture bind group
    /// layouts for the requested set of quantities, in the order in which the
    /// quantities are listed in the [`RENDER_ATTACHMENT_FLAGS`] constant.
    ///
    /// # Errors
    /// Returns an error if any of the requested quantities are missing.
    pub fn request_render_attachment_texture_bind_group_layouts(
        &self,
        requested_quantities: RenderAttachmentQuantitySet,
    ) -> Result<impl Iterator<Item = &wgpu::BindGroupLayout>> {
        if self.available_quantities.contains(requested_quantities) {
            Ok(RENDER_ATTACHMENT_FLAGS
                .iter()
                .zip(self.quantity_texture_bind_group_layouts.iter())
                .filter_map(move |(&quantity_flag, bind_group_layout)| {
                    if requested_quantities.contains(quantity_flag) {
                        Some(bind_group_layout.as_ref().unwrap())
                    } else {
                        None
                    }
                }))
        } else {
            Err(anyhow!(
                "Render attachment texture manager missing requested quantities: {}",
                requested_quantities.difference(self.available_quantities)
            ))
        }
    }

    /// Returns an iterator over the render attachment texture bind groups for
    /// the requested set of quantities, in the order in which the quantities
    /// are listed in the [`RENDER_ATTACHMENT_FLAGS`] constant.
    ///
    /// # Errors
    /// Returns an error if any of the requested quantities are missing.
    pub fn request_render_attachment_texture_bind_groups(
        &self,
        requested_quantities: RenderAttachmentQuantitySet,
    ) -> Result<impl Iterator<Item = &wgpu::BindGroup>> {
        if self.available_quantities.contains(requested_quantities) {
            Ok(RENDER_ATTACHMENT_FLAGS
                .iter()
                .zip(self.quantity_texture_bind_groups.iter())
                .filter_map(move |(&quantity_flag, bind_group)| {
                    if requested_quantities.contains(quantity_flag) {
                        Some(bind_group.as_ref().unwrap())
                    } else {
                        None
                    }
                }))
        } else {
            Err(anyhow!(
                "Render attachment texture manager missing requested quantities: {}",
                requested_quantities.difference(self.available_quantities)
            ))
        }
    }

    /// Recreates all render attachment textures (including the depth texture
    /// and multisampled surface texture) for the current state of the core
    /// system, using the given sample count.
    pub fn recreate_textures(&mut self, core_system: &CoreRenderingSystem, sample_count: u32) {
        self.recreate_multisampled_textures(core_system, sample_count);
        self.recreate_available_render_attachment_textures(core_system);
    }

    /// Recreates the depth texture and multisampled surface texture for the
    /// current state of the core system, using the given sample count.
    pub fn recreate_multisampled_textures(
        &mut self,
        core_system: &CoreRenderingSystem,
        sample_count: u32,
    ) {
        self.recreate_depth_texture(core_system, sample_count);
        self.recreate_multisampled_surface_texture(core_system, sample_count);
    }

    /// Creates a view into the given surface texture.
    pub fn create_surface_texture_view(
        surface_texture: &wgpu::SurfaceTexture,
    ) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn recreate_depth_texture(&mut self, core_system: &CoreRenderingSystem, sample_count: u32) {
        self.depth_texture = DepthTexture::new(core_system, sample_count);
    }

    fn recreate_multisampled_surface_texture(
        &mut self,
        core_system: &CoreRenderingSystem,
        sample_count: u32,
    ) {
        self.multisampled_surface_texture.take();

        if sample_count > 1 {
            self.multisampled_surface_texture =
                Some(MultisampledSurfaceTexture::new(core_system, sample_count));
        }
    }

    fn recreate_available_render_attachment_textures(&mut self, core_system: &CoreRenderingSystem) {
        self.recreate_render_attachment_textures(core_system, self.available_quantities);
    }

    fn recreate_render_attachment_textures(
        &mut self,
        core_system: &CoreRenderingSystem,
        quantities: RenderAttachmentQuantitySet,
    ) {
        for (idx, &quantity_flag) in RENDER_ATTACHMENT_FLAGS.iter().enumerate() {
            if quantities.contains(quantity_flag) {
                self.recreate_render_attachment_texture(core_system, quantity_flag, idx);
            }
        }
        self.available_quantities |= quantities;
    }

    fn recreate_render_attachment_texture(
        &mut self,
        core_system: &CoreRenderingSystem,
        quantity_flag: RenderAttachmentQuantitySet,
        idx: usize,
    ) {
        let quantity_texture =
            RenderAttachmentTexture::new(core_system, RENDER_ATTACHMENT_FORMATS[idx]);

        let label = format!("{} render attachment", quantity_flag);

        let (texture_binding, sampler_binding) = RENDER_ATTACHMENT_BINDINGS[idx];

        let bind_group_layout =
            self.quantity_texture_bind_group_layouts[idx].get_or_insert_with(|| {
                Self::create_render_attachment_texture_bind_group_layout(
                    core_system.device(),
                    texture_binding,
                    sampler_binding,
                    &label,
                )
            });

        self.quantity_texture_bind_groups[idx] =
            Some(Self::create_render_attachment_texture_bind_group(
                core_system.device(),
                texture_binding,
                sampler_binding,
                bind_group_layout,
                &quantity_texture,
                &label,
            ));

        self.quantity_textures[idx] = Some(quantity_texture);
    }

    fn create_render_attachment_texture_bind_group_layout(
        device: &wgpu::Device,
        texture_binding: u32,
        sampler_binding: u32,
        label: &str,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                RenderAttachmentTexture::create_texture_bind_group_layout_entry(texture_binding),
                RenderAttachmentTexture::create_sampler_bind_group_layout_entry(sampler_binding),
            ],
            label: Some(&format!("{} bind group layout", label)),
        })
    }

    fn create_render_attachment_texture_bind_group(
        device: &wgpu::Device,
        texture_binding: u32,
        sampler_binding: u32,
        layout: &wgpu::BindGroupLayout,
        render_attachment_texture: &RenderAttachmentTexture,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                render_attachment_texture.create_texture_bind_group_entry(texture_binding),
                render_attachment_texture.create_sampler_bind_group_entry(sampler_binding),
            ],
            label: Some(&format!("{} bind group", label)),
        })
    }
}

impl DepthTexture {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    /// Creates a new depth texture of the same size as the rendering surface in
    /// `core_system` and with the given sample count for multisampling.
    pub fn new(core_system: &CoreRenderingSystem, sample_count: u32) -> Self {
        let device = core_system.device();
        let surface_config = core_system.surface_config();

        let texture_size = wgpu::Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };

        let texture =
            Self::create_empty_depth32_texture(device, texture_size, sample_count, "Depth texture");

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }

    /// Returns a view into the depth texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Saves the texture as a grayscale image at the given output path. The
    /// image file format is automatically determined from the file extension.
    pub fn save_as_image_file<P: AsRef<Path>>(
        &self,
        core_system: &CoreRenderingSystem,
        output_path: P,
    ) -> Result<()> {
        super::save_depth_texture_as_image_file(core_system, &self.texture, 0, output_path)
    }

    /// Creates a new [`wgpu::Texture`] configured to hold 2D depth data
    /// in 32-bit float format.
    fn create_empty_depth32_texture(
        device: &wgpu::Device,
        texture_size: wgpu::Extent3d,
        sample_count: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: Some(label),
            view_formats: &[],
        })
    }
}

impl RenderAttachmentTexture {
    /// Creates a new render attachment texture of the same size as the
    /// rendering surface in `core_system` and with the given texture format.
    pub fn new(core_system: &CoreRenderingSystem, format: wgpu::TextureFormat) -> Self {
        let device = core_system.device();
        let surface_config = core_system.surface_config();

        let texture_size = wgpu::Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };

        let texture = Self::create_empty_render_attachment_texture(
            device,
            texture_size,
            format,
            &format!("Render attachment texture (format = {:?})", format),
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = Self::create_sampler(device);

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }

    /// Returns a view into the render attachment texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns a sampler for the render attachment texture.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Creates the bind group layout entry for this texture type, assigned to
    /// the given binding.
    pub const fn create_texture_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }
    }

    /// Creates the bind group layout entry for this texture's sampler type,
    /// assigned to the given binding.
    pub const fn create_sampler_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            // The sampler binding type must be consistent with the `filterable`
            // field in the texture sample type.
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        }
    }

    /// Creates the bind group entry for this texture, assigned to the given
    /// binding.
    pub fn create_texture_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(self.view()),
        }
    }

    /// Creates the bind group entry for this texture's sampler, assigned to the
    /// given binding.
    pub fn create_sampler_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(self.sampler()),
        }
    }

    /// Creates a new 2D [`wgpu::Texture`] with the given size and format for
    /// use as a render attachment.
    fn create_empty_render_attachment_texture(
        device: &wgpu::Device,
        texture_size: wgpu::Extent3d,
        format: wgpu::TextureFormat,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some(label),
            view_formats: &[],
        })
    }

    fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        })
    }
}

impl MultisampledSurfaceTexture {
    /// Creates a texture corresponding to the core system surface texture, but
    /// with the given sample count for multisampling.
    pub fn new(core_system: &CoreRenderingSystem, sample_count: u32) -> Self {
        let texture = Self::create_empty_surface_texture(
            core_system.device(),
            core_system.surface_config(),
            sample_count,
            "Multisampled surface texture",
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }

    /// Returns the multisampling sample count.
    pub fn sample_count(&self) -> u32 {
        self.texture.sample_count()
    }

    /// Returns a view into the texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Creates a new [`wgpu::Texture`] with the same dimensions and format as
    /// the specified surface, but with the given sample count for
    /// multisampling.
    fn create_empty_surface_texture(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        sample_count: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some(label),
            view_formats: &[],
        })
    }
}
