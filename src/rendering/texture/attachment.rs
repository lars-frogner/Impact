//! Textures used as render attachments.

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use std::{fmt::Display, path::Path};

bitflags! {
    /// Bitflag encoding a set of quantities that can be rendered to dedicated
    /// render attachment textures.
    pub struct RenderAttachmentQuantitySet: u8 {
        const DEPTH = 0b00000001;
        const POSITION = 0b00000010;
        const NORMAL_VECTOR = 0b00000100;
        const TEXTURE_COORDS = 0b00001000;
        const COLOR = 0b00010000;
        const OCCLUSION = 0b00100000;
    }
}

/// A quantity that can be rendered to a dedicated render attachment texture.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderAttachmentQuantity {
    Depth = 0,
    Position = 1,
    NormalVector = 2,
    TextureCoords = 3,
    Color = 4,
    Occlusion = 5,
}

/// Manager for textures used as render attachments.
#[derive(Debug)]
pub struct RenderAttachmentTextureManager {
    quantity_textures: [Option<RenderAttachmentTexture>; N_RENDER_ATTACHMENT_QUANTITIES],
    quantity_texture_bind_group_layouts:
        [Option<wgpu::BindGroupLayout>; N_RENDER_ATTACHMENT_QUANTITIES],
    quantity_texture_bind_groups: [Option<wgpu::BindGroup>; N_RENDER_ATTACHMENT_QUANTITIES],
    available_quantities: RenderAttachmentQuantitySet,
}

/// A texture that can be used as a color attachment for rendering a specific
/// quantity into.
#[derive(Debug)]
pub struct RenderAttachmentTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

/// The total number of separate render attachment quantities.
pub const N_RENDER_ATTACHMENT_QUANTITIES: usize = 6;

/// The bitflag of each individual render attachment quantity.
pub const RENDER_ATTACHMENT_FLAGS: [RenderAttachmentQuantitySet; N_RENDER_ATTACHMENT_QUANTITIES] = [
    RenderAttachmentQuantitySet::DEPTH,
    RenderAttachmentQuantitySet::POSITION,
    RenderAttachmentQuantitySet::NORMAL_VECTOR,
    RenderAttachmentQuantitySet::TEXTURE_COORDS,
    RenderAttachmentQuantitySet::COLOR,
    RenderAttachmentQuantitySet::OCCLUSION,
];

/// The name of each individual render attachment quantity.
pub const RENDER_ATTACHMENT_NAMES: [&str; N_RENDER_ATTACHMENT_QUANTITIES] = [
    "depth",
    "position",
    "normal_vector",
    "texture_coords",
    "color",
    "occlusion",
];

/// The texture format used for each render attachment quantity.
pub const RENDER_ATTACHMENT_FORMATS: [wgpu::TextureFormat; N_RENDER_ATTACHMENT_QUANTITIES] = [
    wgpu::TextureFormat::Depth32Float,
    wgpu::TextureFormat::Rgba32Float,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Rg32Float,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::R8Unorm,
];

/// The texture and sampler bind group bindings used for each render attachment
/// quantity.
pub const RENDER_ATTACHMENT_BINDINGS: [(u32, u32); N_RENDER_ATTACHMENT_QUANTITIES] =
    [(0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1)];

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
    /// render attachment textures for the given set of quantities.
    pub fn new(core_system: &CoreRenderingSystem, quantities: RenderAttachmentQuantitySet) -> Self {
        let mut manager = Self {
            quantity_textures: [None, None, None, None, None, None],
            quantity_texture_bind_group_layouts: [None, None, None, None, None, None],
            quantity_texture_bind_groups: [None, None, None, None, None, None],
            available_quantities: RenderAttachmentQuantitySet::empty(),
        };

        manager.recreate_render_attachment_textures(core_system, quantities);

        manager
    }

    /// Returns the set of available render attachment quantities.
    pub fn available_quantities(&self) -> RenderAttachmentQuantitySet {
        self.available_quantities
    }

    /// Returns the set of available render attachment quantities that can be
    /// used as color attachments (as opposed to depth).
    pub fn available_color_quantities(&self) -> RenderAttachmentQuantitySet {
        self.available_quantities - RenderAttachmentQuantitySet::DEPTH
    }

    /// Returns the render attachment texture for the requested quantity.
    ///
    /// # Panics
    /// If the requested quantity is missing.
    pub fn render_attachment_texture(
        &self,
        quantity: RenderAttachmentQuantity,
    ) -> &RenderAttachmentTexture {
        self.quantity_textures[quantity as usize]
            .as_ref()
            .expect("Requested missing render attachment quantity")
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

    /// Saves the texture for the given render attachment quantity as a color or
    /// grayscale image at the given output path. The image file format is
    /// automatically determined from the file extension.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The requested quantity is missing.
    /// - The format of the given texture is not supported.
    pub fn save_render_attachment_texture_as_image_file<P: AsRef<Path>>(
        &self,
        core_system: &CoreRenderingSystem,
        quantity: RenderAttachmentQuantity,
        output_path: P,
    ) -> Result<()> {
        let texture = self.quantity_textures[quantity as usize]
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "Tried to save image for missing render attachment quantity: {}",
                    RENDER_ATTACHMENT_NAMES[quantity as usize]
                )
            })?;

        super::save_texture_as_image_file(core_system, texture.texture(), 0, output_path)
    }

    /// Recreates all render attachment textures for the current state of the
    /// core system.
    pub fn recreate_textures(&mut self, core_system: &CoreRenderingSystem) {
        self.recreate_render_attachment_textures(core_system, self.available_quantities);
    }

    /// Creates a view into the given surface texture.
    pub fn create_surface_texture_view(
        surface_texture: &wgpu::SurfaceTexture,
    ) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
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
                    quantity_texture.texture().format(),
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
        texture_format: wgpu::TextureFormat,
        texture_binding: u32,
        sampler_binding: u32,
        label: &str,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                RenderAttachmentTexture::create_texture_bind_group_layout_entry(
                    texture_format,
                    texture_binding,
                ),
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
            texture,
            view,
            sampler,
        }
    }

    /// Returns the render attachment texture.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
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
    pub fn create_texture_bind_group_layout_entry(
        texture_format: wgpu::TextureFormat,
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: if texture_format.has_depth_aspect() {
                    wgpu::TextureSampleType::Depth
                } else {
                    wgpu::TextureSampleType::Float { filterable: false }
                },
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
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

impl RenderAttachmentQuantity {
    /// Returns the enum variant corresponding to the given integer, or [`None`]
    /// if the integer has no corresponding enum variant.
    pub fn from_u8(number: u8) -> Option<Self> {
        match number {
            0 => Some(Self::Depth),
            1 => Some(Self::Position),
            2 => Some(Self::NormalVector),
            3 => Some(Self::TextureCoords),
            4 => Some(Self::Color),
            5 => Some(Self::Occlusion),
            _ => None,
        }
    }
}

impl Display for RenderAttachmentQuantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", RENDER_ATTACHMENT_NAMES[*self as usize])
    }
}
