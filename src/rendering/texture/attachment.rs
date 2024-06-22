//! Textures used as render attachments.

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use num_traits::AsPrimitive;
use std::{fmt::Display, path::Path};

bitflags! {
    /// Bitflag encoding a set of quantities that can be rendered to dedicated
    /// render attachment textures.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct RenderAttachmentQuantitySet: u16 {
        const DEPTH              = 1 << 0;
        const SURFACE            = 1 << 1;
        const POSITION           = 1 << 2;
        const NORMAL_VECTOR      = 1 << 3;
        const TEXTURE_COORDS     = 1 << 4;
        const AMBIENT_COLOR      = 1 << 5;
        const EMISSIVE_COLOR     = 1 << 6;
        const EMISSIVE_COLOR_AUX = 1 << 7;
        const OCCLUSION          = 1 << 8;
    }
}

/// A quantity that can be rendered to a dedicated render attachment texture.
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderAttachmentQuantity {
    Depth = 0,
    Surface = 1,
    Position = 2,
    NormalVector = 3,
    TextureCoords = 4,
    AmbientColor = 5,
    EmissiveColor = 6,
    EmissiveColorAux = 7,
    Occlusion = 8,
}

/// Manager for textures used as render attachments.
#[derive(Debug)]
pub struct RenderAttachmentTextureManager {
    quantity_textures:
        [Option<MaybeWithMultisampling<RenderAttachmentTexture>>; N_RENDER_ATTACHMENT_QUANTITIES],
    quantity_texture_bind_group_layouts:
        [Option<wgpu::BindGroupLayout>; N_RENDER_ATTACHMENT_QUANTITIES],
    quantity_texture_bind_groups: [Option<wgpu::BindGroup>; N_RENDER_ATTACHMENT_QUANTITIES],
}

/// A texture that can be used as a color attachment for rendering a specific
/// quantity into.
#[derive(Debug)]
pub struct RenderAttachmentTexture {
    quantity: RenderAttachmentQuantity,
    texture: wgpu::Texture,
    attachment_view: wgpu::TextureView,
    binding_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

#[derive(Debug)]
pub struct MaybeWithMultisampling<T> {
    pub regular: T,
    pub multisampled: Option<T>,
}

/// The total number of separate render attachment quantities.
const N_RENDER_ATTACHMENT_QUANTITIES: usize = 9;

/// Each individual render attachment quantity.
///
/// # Note
/// This is the order expected by the shaders.
const RENDER_ATTACHMENT_QUANTITIES: [RenderAttachmentQuantity; N_RENDER_ATTACHMENT_QUANTITIES] = [
    RenderAttachmentQuantity::Depth,
    RenderAttachmentQuantity::Surface,
    RenderAttachmentQuantity::Position,
    RenderAttachmentQuantity::NormalVector,
    RenderAttachmentQuantity::TextureCoords,
    RenderAttachmentQuantity::AmbientColor,
    RenderAttachmentQuantity::EmissiveColor,
    RenderAttachmentQuantity::EmissiveColorAux,
    RenderAttachmentQuantity::Occlusion,
];

/// The bitflag of each individual render attachment quantity.
const RENDER_ATTACHMENT_FLAGS: [RenderAttachmentQuantitySet; N_RENDER_ATTACHMENT_QUANTITIES] = [
    RenderAttachmentQuantitySet::DEPTH,
    RenderAttachmentQuantitySet::SURFACE,
    RenderAttachmentQuantitySet::POSITION,
    RenderAttachmentQuantitySet::NORMAL_VECTOR,
    RenderAttachmentQuantitySet::TEXTURE_COORDS,
    RenderAttachmentQuantitySet::AMBIENT_COLOR,
    RenderAttachmentQuantitySet::EMISSIVE_COLOR,
    RenderAttachmentQuantitySet::EMISSIVE_COLOR_AUX,
    RenderAttachmentQuantitySet::OCCLUSION,
];

/// The name of each individual render attachment quantity.
const RENDER_ATTACHMENT_NAMES: [&str; N_RENDER_ATTACHMENT_QUANTITIES] = [
    "depth",
    "surface",
    "position",
    "normal_vector",
    "texture_coords",
    "ambient_color",
    "emissive_color",
    "emissive_color_aux",
    "occlusion",
];

/// The texture format used for each render attachment quantity.
const RENDER_ATTACHMENT_FORMATS: [wgpu::TextureFormat; N_RENDER_ATTACHMENT_QUANTITIES] = [
    wgpu::TextureFormat::Depth32FloatStencil8, // Depth
    wgpu::TextureFormat::Rgba8UnormSrgb, // Surface (this is ignored in favor of the actual surface format)
    wgpu::TextureFormat::Rgba32Float,    // Position
    wgpu::TextureFormat::Rgba8Unorm,     // Normal vector
    wgpu::TextureFormat::Rg32Float,      // Texture coordinates
    wgpu::TextureFormat::Rgba8UnormSrgb, // Ambient color
    wgpu::TextureFormat::Rgba16Float,    // Emissive color
    wgpu::TextureFormat::Rgba16Float,    // Emissive color (auxiliary)
    wgpu::TextureFormat::R8Unorm,        // Occlusion
];

/// Whether multisampling will be used when requested for each render attachment quantity.
const RENDER_ATTACHMENT_MULTISAMPLING_SUPPORT: [bool; N_RENDER_ATTACHMENT_QUANTITIES] = [
    true, // Depth
    true, // Surface
    true, // Position
    true, // Normal vector
    true, // Texture coordinates
    true, // Ambient color
    true, // Emissive color
    true, // Emissive color (auxiliary)
    true, // Occlusion
];

/// The clear color used for each color render attachment quantity (depth is not
/// included).
const RENDER_ATTACHMENT_CLEAR_COLORS: [wgpu::Color; N_RENDER_ATTACHMENT_QUANTITIES - 1] = [
    wgpu::Color::BLACK,
    wgpu::Color::BLACK,
    wgpu::Color::BLACK,
    wgpu::Color::BLACK,
    wgpu::Color::BLACK,
    wgpu::Color::BLACK,
    wgpu::Color::BLACK,
    wgpu::Color::WHITE,
];

/// The texture and sampler bind group bindings used for each render attachment
/// quantity.
const RENDER_ATTACHMENT_BINDINGS: [(u32, u32); N_RENDER_ATTACHMENT_QUANTITIES] = [
    (0, 1),
    (0, 1),
    (0, 1),
    (0, 1),
    (0, 1),
    (0, 1),
    (0, 1),
    (0, 1),
    (0, 1),
];

impl RenderAttachmentQuantity {
    /// The total number of separate render attachment quantities.
    pub const fn count() -> usize {
        N_RENDER_ATTACHMENT_QUANTITIES
    }

    /// Each individual render attachment quantity.
    pub const fn all() -> &'static [Self; Self::count()] {
        &RENDER_ATTACHMENT_QUANTITIES
    }

    /// The bitflag of each individual render attachment quantity.
    pub const fn flags() -> &'static [RenderAttachmentQuantitySet; Self::count()] {
        &RENDER_ATTACHMENT_FLAGS
    }

    /// The name of each individual render attachment quantity.
    pub const fn names() -> &'static [&'static str; Self::count()] {
        &RENDER_ATTACHMENT_NAMES
    }

    /// The texture format used for the depth render attachment texture.
    pub const fn depth_texture_format() -> wgpu::TextureFormat {
        RENDER_ATTACHMENT_FORMATS[Self::Depth.index()]
    }

    /// The clear color used for each color render attachment quantity (the
    /// first quantity, depth, is not included).
    pub const fn clear_colors() -> &'static [wgpu::Color; Self::count() - 1] {
        &RENDER_ATTACHMENT_CLEAR_COLORS
    }

    /// The texture and sampler bind group bindings used for each render attachment
    /// quantity.
    pub const fn all_bindings() -> &'static [(u32, u32); Self::count()] {
        &RENDER_ATTACHMENT_BINDINGS
    }

    /// Returns the enum variant corresponding to the given integer, or [`None`]
    /// if the integer has no corresponding enum variant.
    pub fn from_index(number: impl AsPrimitive<usize>) -> Option<Self> {
        RENDER_ATTACHMENT_QUANTITIES.get(number.as_()).copied()
    }

    /// The index of this render attachment quantity.
    pub const fn index(&self) -> usize {
        *self as usize
    }

    /// The bitflag of this render attachment quantity.
    pub const fn flag(&self) -> RenderAttachmentQuantitySet {
        Self::flags()[self.index()]
    }

    /// The name of this render attachment quantity.
    pub const fn name(&self) -> &'static str {
        Self::names()[self.index()]
    }

    /// The texture format used for this render attachment quantity.
    pub fn texture_format(&self, core_system: &CoreRenderingSystem) -> wgpu::TextureFormat {
        if *self == Self::Surface {
            core_system.surface_config().format
        } else {
            RENDER_ATTACHMENT_FORMATS[self.index()]
        }
    }

    /// Whether multisampling is supported for this render attachment quantity.
    pub const fn supports_multisampling(&self) -> bool {
        RENDER_ATTACHMENT_MULTISAMPLING_SUPPORT[self.index()]
    }

    /// The clear color of this render attachment quantity.
    ///
    /// # Panics
    /// If this quantity is depth.
    pub fn clear_color(&self) -> wgpu::Color {
        assert_ne!(*self, Self::Depth);
        Self::clear_colors()[self.index() - 1]
    }

    /// The texture and sampler bind group bindings used for this render
    /// attachment quantity.
    pub const fn bindings(&self) -> (u32, u32) {
        Self::all_bindings()[self.index()]
    }
}

impl Display for RenderAttachmentQuantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl RenderAttachmentQuantitySet {
    /// Returns the set of render attachment quantities that support
    /// multisampling.
    pub fn multisampling_quantities() -> Self {
        let mut quantities = Self::empty();
        for quantity in RenderAttachmentQuantity::all() {
            if quantity.supports_multisampling() {
                quantities |= quantity.flag();
            }
        }
        quantities
    }

    /// Returns the set of render attachment quantities that do not support
    /// multisampling.
    pub fn non_multisampling_quantities() -> Self {
        Self::all() - Self::multisampling_quantities()
    }

    /// Returns this set without the depth quantity.
    pub fn color_only(&self) -> Self {
        *self - Self::DEPTH
    }
}

impl Display for RenderAttachmentQuantitySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ ")?;
        for quantity in RenderAttachmentQuantity::all() {
            if self.contains(quantity.flag()) {
                write!(f, "`{}` ", quantity)?;
            }
        }
        write!(f, "}}")
    }
}

impl RenderAttachmentTextureManager {
    /// Creates a new manager for render attachment textures, initializing
    /// render attachment textures for the given set of quantities with the
    /// given sample count.
    pub fn new(core_system: &CoreRenderingSystem, sample_count: u32) -> Self {
        let mut manager = Self {
            quantity_textures: [None, None, None, None, None, None, None, None, None],
            quantity_texture_bind_group_layouts: [
                None, None, None, None, None, None, None, None, None,
            ],
            quantity_texture_bind_groups: [None, None, None, None, None, None, None, None, None],
        };

        manager.recreate_textures(core_system, sample_count);

        manager
    }

    /// Returns the render attachment texture for the requested quantity.
    ///
    /// # Panics
    /// If the requested quantity is missing.
    pub fn render_attachment_texture(
        &self,
        quantity: RenderAttachmentQuantity,
    ) -> &MaybeWithMultisampling<RenderAttachmentTexture> {
        self.quantity_textures[quantity.index()]
            .as_ref()
            .expect("Requested missing render attachment quantity")
    }

    /// Returns an iterator over the render attachment textures for the
    /// requested set of quantities, in the order in which the quantities are
    /// returned from the [`RenderAttachmentQuantity`] methods.
    pub fn request_render_attachment_textures(
        &self,
        requested_quantities: RenderAttachmentQuantitySet,
    ) -> impl Iterator<Item = &MaybeWithMultisampling<RenderAttachmentTexture>> {
        RenderAttachmentQuantity::flags()
            .iter()
            .zip(self.quantity_textures.iter())
            .filter_map(move |(&quantity_flag, quantity_texture)| {
                if requested_quantities.contains(quantity_flag) {
                    Some(quantity_texture.as_ref().unwrap())
                } else {
                    None
                }
            })
    }

    /// Returns an iterator over the render attachment texture bind group
    /// layouts for the requested set of quantities, in the order in which the
    /// quantities are returned from the [`RenderAttachmentQuantity`] methods.
    pub fn request_render_attachment_texture_bind_group_layouts(
        &self,
        requested_quantities: RenderAttachmentQuantitySet,
    ) -> impl Iterator<Item = &wgpu::BindGroupLayout> {
        RenderAttachmentQuantity::flags()
            .iter()
            .zip(self.quantity_texture_bind_group_layouts.iter())
            .filter_map(move |(&quantity_flag, bind_group_layout)| {
                if requested_quantities.contains(quantity_flag) {
                    Some(bind_group_layout.as_ref().unwrap())
                } else {
                    None
                }
            })
    }

    /// Returns an iterator over the render attachment texture bind groups for
    /// the requested set of quantities, in the order in which the quantities
    /// are returned from the [`RenderAttachmentQuantity`] methods.
    pub fn request_render_attachment_texture_bind_groups(
        &self,
        requested_quantities: RenderAttachmentQuantitySet,
    ) -> impl Iterator<Item = &wgpu::BindGroup> {
        RenderAttachmentQuantity::flags()
            .iter()
            .zip(self.quantity_texture_bind_groups.iter())
            .filter_map(move |(&quantity_flag, bind_group)| {
                if requested_quantities.contains(quantity_flag) {
                    Some(bind_group.as_ref().unwrap())
                } else {
                    None
                }
            })
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
        let texture = self.quantity_textures[quantity.index()]
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "Tried to save image for missing render attachment quantity: {}",
                    quantity
                )
            })?;

        super::save_texture_as_image_file(core_system, texture.regular.texture(), 0, output_path)
    }
    /// Recreates all render attachment textures for the current state of the
    /// core system, using the given sample count.
    pub fn recreate_textures(&mut self, core_system: &CoreRenderingSystem, sample_count: u32) {
        for &quantity in RenderAttachmentQuantity::all() {
            self.recreate_render_attachment_texture(core_system, quantity, sample_count);
        }
    }

    /// Creates a view into the given surface texture.
    pub fn create_surface_texture_view(
        surface_texture: &wgpu::SurfaceTexture,
    ) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn recreate_render_attachment_texture(
        &mut self,
        core_system: &CoreRenderingSystem,
        quantity: RenderAttachmentQuantity,
        sample_count: u32,
    ) {
        let format = quantity.texture_format(core_system);

        let quantity_texture =
            MaybeWithMultisampling::new(core_system, quantity, format, sample_count);

        let label = format!("{} render attachment", quantity);

        let (texture_binding, sampler_binding) = quantity.bindings();

        let bind_group_layout = self.quantity_texture_bind_group_layouts[quantity.index()]
            .get_or_insert_with(|| {
                Self::create_render_attachment_texture_bind_group_layout(
                    core_system.device(),
                    format,
                    texture_binding,
                    sampler_binding,
                    &label,
                )
            });

        self.quantity_texture_bind_groups[quantity.index()] =
            Some(Self::create_render_attachment_texture_bind_group(
                core_system.device(),
                texture_binding,
                sampler_binding,
                bind_group_layout,
                &quantity_texture.regular,
                &label,
            ));

        self.quantity_textures[quantity.index()] = Some(quantity_texture);
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
    /// rendering surface in `core_system` and with the given texture format and
    /// sample count.
    pub fn new(
        core_system: &CoreRenderingSystem,
        quantity: RenderAttachmentQuantity,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
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
            sample_count,
            &format!("Render attachment texture (format = {:?})", format),
        );

        let attachment_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // When using a depth texture as a binding, we must exclude the stencil
        // aspect
        let binding_view = texture.create_view(&wgpu::TextureViewDescriptor {
            aspect: if format.has_depth_aspect() {
                wgpu::TextureAspect::DepthOnly
            } else {
                wgpu::TextureAspect::All
            },
            ..Default::default()
        });

        let sampler = Self::create_sampler(device);

        Self {
            quantity,
            texture,
            attachment_view,
            binding_view,
            sampler,
        }
    }

    /// Returns the render attachment quantity.
    pub fn quantity(&self) -> RenderAttachmentQuantity {
        self.quantity
    }

    /// Returns the render attachment texture.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Returns a view into the texture for use as a render attachment.
    pub fn attachment_view(&self) -> &wgpu::TextureView {
        &self.attachment_view
    }

    /// Returns a view into the texture for use as a binding.
    pub fn binding_view(&self) -> &wgpu::TextureView {
        &self.binding_view
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
            resource: wgpu::BindingResource::TextureView(self.binding_view()),
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
        sample_count: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count,
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

impl<T> MaybeWithMultisampling<T> {
    pub fn multisampled_if_available_and(&self, use_multisampling: bool) -> &T {
        if use_multisampling {
            self.multisampled.as_ref().unwrap_or(&self.regular)
        } else {
            &self.regular
        }
    }
}

impl MaybeWithMultisampling<RenderAttachmentTexture> {
    fn new(
        core_system: &CoreRenderingSystem,
        quantity: RenderAttachmentQuantity,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let regular = RenderAttachmentTexture::new(core_system, quantity, format, 1);

        let multisampled = if sample_count > 1 && quantity.supports_multisampling() {
            Some(RenderAttachmentTexture::new(
                core_system,
                quantity,
                format,
                sample_count,
            ))
        } else {
            None
        };

        Self {
            regular,
            multisampled,
        }
    }

    /// Returns the render attachment quantity.
    pub fn quantity(&self) -> RenderAttachmentQuantity {
        self.regular.quantity()
    }

    /// Returns the render attachment texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.regular.texture().format()
    }

    /// Returns the sample count for the multisampled texture, or 1 if there is
    /// no multisampled texture.
    pub fn multisampling_sample_count(&self) -> u32 {
        self.multisampled
            .as_ref()
            .map_or(1, |texture| texture.texture().sample_count())
    }

    /// Returns the appropriate `view` and `resolve_target` for
    /// [`wgpu::RenderPassColorAttachment`] based on whether the multisampled
    /// texture should be used if available and whether it should be resolved
    /// into the regular texture.
    pub fn view_and_resolve_target(
        &self,
        should_be_multisampled_if_available: bool,
        should_resolve: bool,
    ) -> (&wgpu::TextureView, Option<&wgpu::TextureView>) {
        match &self.multisampled {
            Some(multisampled) if should_be_multisampled_if_available => (
                multisampled.attachment_view(),
                if should_resolve {
                    Some(self.regular.attachment_view())
                } else {
                    None
                },
            ),
            _ => (self.regular.attachment_view(), None),
        }
    }
}
