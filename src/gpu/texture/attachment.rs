//! Textures used as render attachments.

use crate::gpu::{
    GraphicsDevice,
    rendering::surface::RenderingSurface,
    texture::{Sampler, SamplerConfig, Texture, TextureAddressingConfig, TextureFilteringConfig},
};
use bitflags::bitflags;
use num_traits::AsPrimitive;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

bitflags! {
    /// Bitflag encoding a set of quantities that can be rendered to dedicated
    /// render attachment textures.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct RenderAttachmentQuantitySet: u16 {
        const DEPTH_STENCIL              = 1 << 0;
        const LINEAR_DEPTH               = 1 << 1;
        const NORMAL_VECTOR              = 1 << 2;
        const MOTION_VECTOR              = 1 << 3;
        const MATERIAL_COLOR             = 1 << 4;
        const MATERIAL_PROPERTIES        = 1 << 5;
        const LUMINANCE                  = 1 << 6;
        const LUMINANCE_AUX              = 1 << 7;
        const LUMINANCE_HISTORY          = 1 << 8;
        const PREVIOUS_LUMINANCE_HISTORY = 1 << 9;
        const OCCLUSION                  = 1 << 10;
    }
}

/// A quantity that can be rendered to a dedicated render attachment texture.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RenderAttachmentQuantity {
    DepthStencil = 0,
    LinearDepth = 1,
    NormalVector = 2,
    MotionVector = 3,
    MaterialColor = 4,
    MaterialProperties = 5,
    Luminance = 6,
    LuminanceAux = 7,
    LuminanceHistory = 8,
    PreviousLuminanceHistory = 9,
    Occlusion = 10,
}

/// A sampler variant for render attachment textures.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RenderAttachmentSampler {
    NonFiltering = 0,
    Filtering = 1,
}

/// Specifies how a render attachment should be used when bound as an input.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderAttachmentInputDescription {
    quantity: RenderAttachmentQuantity,
    sampler: RenderAttachmentSampler,
    visibility: wgpu::ShaderStages,
    mip_level: u32,
}

/// Specifies how a render attachment should be used when bound as an output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderAttachmentOutputDescription {
    quantity: RenderAttachmentQuantity,
    blending: Blending,
    write_mask: wgpu::ColorWrites,
    mip_level: u32,
}

/// The blending mode to use when writing to a render attachment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blending {
    Replace,
    Additive,
}

/// A set of descriptions for render attachments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderAttachmentDescriptionSet<D> {
    quantities: RenderAttachmentQuantitySet,
    descriptions: Vec<D>,
}

/// A set of input descriptions for render attachments.
pub type RenderAttachmentInputDescriptionSet =
    RenderAttachmentDescriptionSet<RenderAttachmentInputDescription>;

/// A set of output descriptions for render attachments.
pub type RenderAttachmentOutputDescriptionSet =
    RenderAttachmentDescriptionSet<RenderAttachmentOutputDescription>;

/// Manager for textures used as render attachments.
#[derive(Debug)]
pub struct RenderAttachmentTextureManager {
    quantity_textures: [RenderAttachmentTexture; N_RENDER_ATTACHMENT_QUANTITIES],
    samplers: [Sampler; 2],
    bind_groups_and_layouts:
        HashMap<RenderAttachmentInputDescription, (wgpu::BindGroupLayout, wgpu::BindGroup)>,
}

/// A texture that can be used as a color attachment for rendering a specific
/// quantity into.
#[derive(Debug)]
pub struct RenderAttachmentTexture {
    quantity: RenderAttachmentQuantity,
    texture: Texture,
    mip_texture_views: Vec<wgpu::TextureView>,
}

pub trait RenderAttachmentDescription {
    /// Creates a new default render attachment description for the given
    /// quantity.
    fn default_for(quantity: RenderAttachmentQuantity) -> Self;

    /// Returns the render attachment quantity for this description.
    fn quantity(&self) -> RenderAttachmentQuantity;
}

/// The total number of separate render attachment quantities.
const N_RENDER_ATTACHMENT_QUANTITIES: usize = 11;

/// Each individual render attachment quantity.
///
/// # Note
/// This is the order expected by the shaders.
const RENDER_ATTACHMENT_QUANTITIES: [RenderAttachmentQuantity; N_RENDER_ATTACHMENT_QUANTITIES] = [
    RenderAttachmentQuantity::DepthStencil,
    RenderAttachmentQuantity::LinearDepth,
    RenderAttachmentQuantity::NormalVector,
    RenderAttachmentQuantity::MotionVector,
    RenderAttachmentQuantity::MaterialColor,
    RenderAttachmentQuantity::MaterialProperties,
    RenderAttachmentQuantity::Luminance,
    RenderAttachmentQuantity::LuminanceAux,
    RenderAttachmentQuantity::LuminanceHistory,
    RenderAttachmentQuantity::PreviousLuminanceHistory,
    RenderAttachmentQuantity::Occlusion,
];

/// The bitflag of each individual render attachment quantity.
const RENDER_ATTACHMENT_FLAGS: [RenderAttachmentQuantitySet; N_RENDER_ATTACHMENT_QUANTITIES] = [
    RenderAttachmentQuantitySet::DEPTH_STENCIL,
    RenderAttachmentQuantitySet::LINEAR_DEPTH,
    RenderAttachmentQuantitySet::NORMAL_VECTOR,
    RenderAttachmentQuantitySet::MOTION_VECTOR,
    RenderAttachmentQuantitySet::MATERIAL_COLOR,
    RenderAttachmentQuantitySet::MATERIAL_PROPERTIES,
    RenderAttachmentQuantitySet::LUMINANCE,
    RenderAttachmentQuantitySet::LUMINANCE_AUX,
    RenderAttachmentQuantitySet::LUMINANCE_HISTORY,
    RenderAttachmentQuantitySet::PREVIOUS_LUMINANCE_HISTORY,
    RenderAttachmentQuantitySet::OCCLUSION,
];

/// The name of each individual render attachment quantity.
const RENDER_ATTACHMENT_NAMES: [&str; N_RENDER_ATTACHMENT_QUANTITIES] = [
    "depth_stencil",
    "linear_depth",
    "normal_vector",
    "motion_vector",
    "material_color",
    "material_properties",
    "luminance",
    "auxiliary_luminance",
    "luminance_history",
    // We use the same name for the previous luminance history attachment so
    // that their `BindGroupLayout`s can be used interchangeably
    "luminance_history",
    "occlusion",
];

/// The texture format used for each render attachment quantity.
const RENDER_ATTACHMENT_FORMATS: [wgpu::TextureFormat; N_RENDER_ATTACHMENT_QUANTITIES] = [
    wgpu::TextureFormat::Depth32FloatStencil8, // Depth-stencil
    wgpu::TextureFormat::R32Float,             // Linear depth
    wgpu::TextureFormat::Rgba8Unorm,           // Normal vector
    wgpu::TextureFormat::Rg32Float,            // Motion vector
    wgpu::TextureFormat::Rgba8Unorm,           // Material color
    wgpu::TextureFormat::Rgba16Float,          // Material properties
    wgpu::TextureFormat::Rgba16Float,          // Luminance
    wgpu::TextureFormat::Rgba16Float,          // Auxiliary luminance
    wgpu::TextureFormat::Rgba16Float,          // Luminance history
    wgpu::TextureFormat::Rgba16Float,          // Previous luminance history
    wgpu::TextureFormat::R16Float,             // Occlusion
];

/// The maximum mip level for each render attachment quantity.
const RENDER_ATTACHMENT_MAX_MIP_LEVEL: [u32; N_RENDER_ATTACHMENT_QUANTITIES] = [
    0, // Depth-stencil
    0, // Linear depth
    0, // Normal vector
    0, // Motion vector
    0, // Material color
    0, // Material properties
    0, // Luminance
    6, // Auxiliary luminance
    0, // Luminance history
    0, // Previous luminance history
    0, // Occlusion
];

/// The clear color used for each render attachment quantity, or [`None`] if the
/// render attachment should never be cleared with a color.
const RENDER_ATTACHMENT_CLEAR_COLORS: [Option<wgpu::Color>; N_RENDER_ATTACHMENT_QUANTITIES] = [
    None,                     // Depth-stencil
    Some(wgpu::Color::BLACK), // Linear depth
    Some(wgpu::Color::BLACK), // Normal vector
    Some(wgpu::Color::BLACK), // Motion vector
    Some(wgpu::Color::BLACK), // Material color
    Some(wgpu::Color::BLACK), // Material properties
    Some(wgpu::Color::BLACK), // Luminance
    Some(wgpu::Color::BLACK), // Auxiliary luminance
    Some(wgpu::Color::BLACK), // Luminance history
    None,                     // Previous luminance history
    Some(wgpu::Color::WHITE), // Occlusion
];

/// The texture and sampler bind group bindings used for each render attachment
/// quantity.
const RENDER_ATTACHMENT_BINDINGS: [(u32, u32); N_RENDER_ATTACHMENT_QUANTITIES] = [
    (0, 1), // Depth-stencil
    (0, 1), // Linear depth
    (0, 1), // Normal vector
    (0, 1), // Motion vector
    (0, 1), // Material color
    (0, 1), // Material properties
    (0, 1), // Luminance
    (0, 1), // Auxiliary luminance
    (0, 1), // Luminance history
    (0, 1), // Previous luminance history
    (0, 1), // Occlusion
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

    /// The texture format of each individual render attachment quantity.
    pub const fn texture_formats() -> &'static [wgpu::TextureFormat; Self::count()] {
        &RENDER_ATTACHMENT_FORMATS
    }

    /// The texture format used for the depth render attachment texture.
    pub const fn depth_texture_format() -> wgpu::TextureFormat {
        RENDER_ATTACHMENT_FORMATS[Self::DepthStencil.index()]
    }

    /// The clear color used for each render attachment quantity.
    pub const fn clear_colors() -> &'static [Option<wgpu::Color>; Self::count()] {
        &RENDER_ATTACHMENT_CLEAR_COLORS
    }

    /// The texture and sampler bind group bindings used for each render
    /// attachment quantity.
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
    pub const fn texture_format(&self) -> wgpu::TextureFormat {
        RENDER_ATTACHMENT_FORMATS[self.index()]
    }

    /// The maximum mip level for this render attachment quantity.
    pub const fn max_mip_level(&self) -> u32 {
        RENDER_ATTACHMENT_MAX_MIP_LEVEL[self.index()]
    }

    /// The clear color of this render attachment quantity, or [`None`] if this
    /// quantity should never be cleared with a color.
    pub fn clear_color(&self) -> Option<wgpu::Color> {
        Self::clear_colors()[self.index()]
    }

    /// The bind group binding used for this render attachment quantity's
    /// texture.
    pub const fn texture_binding(&self) -> u32 {
        Self::all_bindings()[self.index()].0
    }

    /// The bind group binding used for this render attachment quantity's
    /// sampler.
    pub const fn sampler_binding(&self) -> u32 {
        Self::all_bindings()[self.index()].1
    }
}

impl Display for RenderAttachmentQuantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl RenderAttachmentQuantitySet {
    /// Returns the set of render attachment quantities comprising the G-buffer
    /// for deferred rendering.
    pub const fn g_buffer() -> Self {
        RenderAttachmentQuantitySet::LINEAR_DEPTH
            .union(RenderAttachmentQuantitySet::NORMAL_VECTOR)
            .union(RenderAttachmentQuantitySet::MOTION_VECTOR)
            .union(RenderAttachmentQuantitySet::MATERIAL_COLOR)
            .union(RenderAttachmentQuantitySet::MATERIAL_PROPERTIES)
    }

    /// Returns this set without the depth quantity.
    pub fn color_only(&self) -> Self {
        *self - Self::DEPTH_STENCIL
    }

    /// Returns this set without any quantities that do not have a clear color.
    pub fn with_clear_color_only(&self) -> Self {
        let mut quantities = Self::empty();
        for quantity in RenderAttachmentQuantity::all() {
            if self.contains(quantity.flag()) && quantity.clear_color().is_some() {
                quantities |= quantity.flag();
            }
        }
        quantities
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

impl RenderAttachmentSampler {
    /// Returns the configuration for this sampler variant.
    pub fn config(&self) -> SamplerConfig {
        match self {
            Self::NonFiltering => SamplerConfig {
                addressing: TextureAddressingConfig::CLAMPED,
                filtering: TextureFilteringConfig::NONE,
            },
            Self::Filtering => SamplerConfig {
                addressing: TextureAddressingConfig::CLAMPED,
                filtering: TextureFilteringConfig::BASIC,
            },
        }
    }
}

impl RenderAttachmentInputDescription {
    /// Creates a new render attachment input description.
    ///
    /// # Panics
    /// If `mip_level` is larger than the maximum mip level for the render
    /// attachment quantity.
    pub fn new(
        quantity: RenderAttachmentQuantity,
        sampler: RenderAttachmentSampler,
        visibility: wgpu::ShaderStages,
        mip_level: u32,
    ) -> Self {
        assert!(mip_level <= quantity.max_mip_level());
        Self {
            quantity,
            sampler,
            visibility,
            mip_level,
        }
    }

    /// Sets the render attachment quantity.
    pub fn with_quantity(mut self, quantity: RenderAttachmentQuantity) -> Self {
        self.quantity = quantity;
        self
    }

    /// Sets how the render attachment should be sampled.
    pub fn with_sampler(mut self, sampler: RenderAttachmentSampler) -> Self {
        self.sampler = sampler;
        self
    }

    /// Sets the shader stages where the render attachment texture and its
    /// sampler should be visible.
    pub fn with_visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }

    /// Sets the mip level of the render attachment that should be used.
    ///
    /// # Panics
    /// If `mip_level` is larger than the maximum mip level for the render
    /// attachment quantity.
    pub fn with_mip_level(mut self, mip_level: u32) -> Self {
        assert!(mip_level <= self.quantity.max_mip_level());
        self.mip_level = mip_level;
        self
    }

    /// Returns how the render attachment should be sampled.
    pub fn sampler(&self) -> RenderAttachmentSampler {
        self.sampler
    }

    /// Returns the shader stages where the render attachment texture and its
    /// sampler should be visible.
    pub fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }

    /// Returns the mip level of the render attachment that should be used.
    pub fn mip_level(&self) -> u32 {
        self.mip_level
    }
}

impl RenderAttachmentDescription for RenderAttachmentInputDescription {
    fn default_for(quantity: RenderAttachmentQuantity) -> Self {
        Self {
            quantity,
            sampler: RenderAttachmentSampler::NonFiltering,
            visibility: wgpu::ShaderStages::FRAGMENT,
            mip_level: 0,
        }
    }

    fn quantity(&self) -> RenderAttachmentQuantity {
        self.quantity
    }
}

impl RenderAttachmentOutputDescription {
    /// Creates a new render attachment output description.
    ///
    /// # Panics
    /// If `mip_level` is larger than the maximum mip level for the render
    /// attachment quantity.
    pub fn new(
        quantity: RenderAttachmentQuantity,
        blending: Blending,
        write_mask: wgpu::ColorWrites,
        mip_level: u32,
    ) -> Self {
        assert!(mip_level <= quantity.max_mip_level());
        Self {
            quantity,
            blending,
            write_mask,
            mip_level,
        }
    }

    /// Sets the render attachment quantity.
    pub fn with_quantity(mut self, quantity: RenderAttachmentQuantity) -> Self {
        self.quantity = quantity;
        self
    }

    /// Sets the blending mode that should be used when rendering to the render
    /// attachment.
    pub fn with_blending(mut self, blending: Blending) -> Self {
        self.blending = blending;
        self
    }

    /// Sets the color write mask that should be used when rendering to the
    /// render attachment.
    pub fn with_write_mask(mut self, write_mask: wgpu::ColorWrites) -> Self {
        self.write_mask = write_mask;
        self
    }

    /// Sets the mip level of the render attachment that should be rendered to.
    pub fn with_mip_level(mut self, mip_level: u32) -> Self {
        self.mip_level = mip_level;
        self
    }

    /// Returns the blending mode that should be used when rendering to the
    /// render attachment.
    pub fn blending(&self) -> Blending {
        self.blending
    }

    /// Returns the color write mask that should be used when rendering to the
    /// render attachment.
    pub fn write_mask(&self) -> wgpu::ColorWrites {
        self.write_mask
    }

    /// Returns the mip level of the render attachment that should be rendered
    /// to.
    pub fn mip_level(&self) -> u32 {
        self.mip_level
    }
}

impl RenderAttachmentDescription for RenderAttachmentOutputDescription {
    fn default_for(quantity: RenderAttachmentQuantity) -> Self {
        Self {
            quantity,
            blending: Blending::Replace,
            write_mask: wgpu::ColorWrites::ALL,
            mip_level: 0,
        }
    }

    fn quantity(&self) -> RenderAttachmentQuantity {
        self.quantity
    }
}

impl<D> RenderAttachmentDescriptionSet<D>
where
    D: RenderAttachmentDescription,
{
    /// Gathers the given list of descriptions into a description set.
    pub fn new(descriptions: Vec<D>) -> Self {
        let mut quantities = RenderAttachmentQuantitySet::empty();
        for description in &descriptions {
            quantities |= description.quantity().flag();
        }
        Self {
            quantities,
            descriptions,
        }
    }

    /// Creates a new empty set of descriptions for render attachments.
    pub fn empty() -> Self {
        Self {
            quantities: RenderAttachmentQuantitySet::empty(),
            descriptions: Vec::new(),
        }
    }

    /// Creates a new empty set of descriptions for render attachments, with
    /// the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            quantities: RenderAttachmentQuantitySet::empty(),
            descriptions: Vec::with_capacity(capacity),
        }
    }

    /// Creates a new set of descriptions for the render attachments for the
    /// given quantities, using the default description for all of them.
    pub fn with_defaults(quantities: RenderAttachmentQuantitySet) -> Self {
        let mut descriptions = Vec::with_capacity(quantities.bits().count_ones() as usize);
        for quantity in RenderAttachmentQuantity::all() {
            if quantities.contains(quantity.flag()) {
                descriptions.push(D::default_for(*quantity));
            }
        }
        Self {
            quantities,
            descriptions,
        }
    }

    /// Creates a set comprised only of the given description.
    pub fn single(description: D) -> Self {
        Self::new(vec![description])
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.quantities.is_empty()
    }

    /// Returns the render attachment quantities in the set.
    pub fn quantities(&self) -> RenderAttachmentQuantitySet {
        self.quantities
    }

    /// Returns the descriptions for the render attachments in the set in the
    /// order in which they were added.
    pub fn descriptions(&self) -> &[D] {
        &self.descriptions
    }

    /// Returns an iterator over the descriptions for the given quantity.
    pub fn descriptions_for_quantity(
        &self,
        quantity: RenderAttachmentQuantity,
    ) -> impl Iterator<Item = &D> {
        self.descriptions
            .iter()
            .filter(move |description| description.quantity() == quantity)
    }

    /// Returns the description for the given quantity, or [`None`] if the
    /// quantity has none or multiple descriptions.
    pub fn only_description_for_quantity(&self, quantity: RenderAttachmentQuantity) -> Option<&D> {
        let mut descriptions = self.descriptions_for_quantity(quantity);
        let description = descriptions.next();
        if descriptions.next().is_none() {
            description
        } else {
            None
        }
    }

    /// Inserts the given description in the set. This will not overwrite any
    /// existing descriptions even if they are equal.
    pub fn insert_description(&mut self, description: D) {
        self.quantities |= description.quantity().flag();
        self.descriptions.push(description);
    }
}

impl RenderAttachmentTextureManager {
    /// Creates a new manager for render attachment textures, initializing
    /// all render attachment textures.
    pub fn new(graphics_device: &GraphicsDevice, rendering_surface: &RenderingSurface) -> Self {
        let samplers = [
            Sampler::create(
                graphics_device,
                RenderAttachmentSampler::NonFiltering.config(),
            ),
            Sampler::create(graphics_device, RenderAttachmentSampler::Filtering.config()),
        ];

        let quantity_textures = RenderAttachmentQuantity::all().map(|quantity| {
            RenderAttachmentTexture::new(graphics_device, rendering_surface, quantity)
        });

        let mut manager = Self {
            quantity_textures,
            samplers,
            bind_groups_and_layouts: HashMap::new(),
        };

        manager.recreate_bind_groups(graphics_device);

        manager
    }

    /// Returns the render attachment texture for the requested quantity.
    ///
    /// # Panics
    /// If the requested quantity is missing.
    pub fn render_attachment_texture(
        &self,
        quantity: RenderAttachmentQuantity,
    ) -> &RenderAttachmentTexture {
        &self.quantity_textures[quantity.index()]
    }

    /// Returns an iterator over the render attachment textures for the
    /// requested set of quantities, in the order in which the quantities are
    /// returned from the [`RenderAttachmentQuantity`] methods.
    pub fn request_render_attachment_textures(
        &self,
        requested_quantities: RenderAttachmentQuantitySet,
    ) -> impl Iterator<Item = &RenderAttachmentTexture> {
        RenderAttachmentQuantity::flags()
            .iter()
            .zip(self.quantity_textures.iter())
            .filter_map(move |(&quantity_flag, quantity_texture)| {
                if requested_quantities.contains(quantity_flag) {
                    Some(quantity_texture)
                } else {
                    None
                }
            })
    }

    /// Returns an iterator over the render attachment texture and sampler bind
    /// group layouts for the requested set of input descriptions, in the order
    /// in which the quantities are returned from the
    /// [`RenderAttachmentQuantity`] methods. Any layout that does not already
    /// exist will be created (regardless of whether the returned iterator is
    /// consumed).
    pub fn create_and_get_render_attachment_texture_bind_group_layouts<'a, 'b>(
        &'a mut self,
        graphics_device: &GraphicsDevice,
        input_descriptions: &'b RenderAttachmentInputDescriptionSet,
    ) -> impl Iterator<Item = &'a wgpu::BindGroupLayout> + use<'a, 'b>
    where
        'a: 'b,
    {
        self.create_missing_bind_groups_and_layouts(graphics_device, input_descriptions);
        self.get_render_attachment_texture_bind_group_layouts(input_descriptions)
    }

    /// Returns an iterator over the render attachment texture and sampler bind
    /// group layouts for the requested set of input descriptions, in the order
    /// in which the quantities are returned from the
    /// [`RenderAttachmentQuantity`] methods.
    ///
    /// This method should only be called after
    /// [`Self::create_and_get_render_attachment_texture_bind_group_layouts`] or
    /// [`Self::create_missing_bind_groups_and_layouts`] has been called
    /// with the same input descriptions, otherwise the requested bind
    /// group layouts may not exist yet.
    ///
    /// # Panics
    /// If the bind group layout for any of the input descriptions has not been
    /// created.
    pub fn get_render_attachment_texture_bind_group_layouts<'a, 'b>(
        &'a self,
        input_descriptions: &'b RenderAttachmentInputDescriptionSet,
    ) -> impl Iterator<Item = &'a wgpu::BindGroupLayout> + use<'a, 'b>
    where
        'a: 'b,
    {
        RenderAttachmentQuantity::all().iter().flat_map(|quantity| {
            input_descriptions
                .descriptions_for_quantity(*quantity)
                .map(|description| {
                    let (layout, _) = self.bind_groups_and_layouts.get(description).unwrap();
                    layout
                })
        })
    }

    /// Returns the render attachment texture and sampler bind group layout for
    /// the requested input description, or [`None`] if the bind group
    /// layout for the input description has not been created.
    pub fn get_render_attachment_texture_bind_group_layout(
        &self,
        input_description: &RenderAttachmentInputDescription,
    ) -> Option<&wgpu::BindGroupLayout> {
        self.bind_groups_and_layouts
            .get(input_description)
            .map(|(bind_group_layout, _)| bind_group_layout)
    }

    /// Returns an iterator over the render attachment texture and sampler bind
    /// groups for the requested set of input descriptions, in the order in
    /// which the quantities are returned from the [`RenderAttachmentQuantity`]
    /// methods.
    ///
    /// This method should only be called after
    /// [`Self::create_and_get_render_attachment_texture_bind_group_layouts`] or
    /// [`Self::create_missing_bind_groups_and_layouts`] has been called
    /// with the same input descriptions, otherwise the requested bind
    /// groups may not exist yet.
    ///
    /// # Panics
    /// If the bind group for any of the input descriptions has not been
    /// created.
    pub fn get_render_attachment_texture_bind_groups<'a, 'b>(
        &'a self,
        input_descriptions: &'b RenderAttachmentInputDescriptionSet,
    ) -> impl Iterator<Item = &'a wgpu::BindGroup> + 'b
    where
        'a: 'b,
    {
        RenderAttachmentQuantity::all().iter().flat_map(|quantity| {
            input_descriptions
                .descriptions_for_quantity(*quantity)
                .map(|description| {
                    let (_, bind_group) = self
                        .bind_groups_and_layouts
                        .get(description)
                        .expect("Missing bind group for render attachment input description");
                    bind_group
                })
        })
    }

    /// Returns the render attachment texture and sampler bind group for the
    /// requested input description, or [`None`] if the bind group for the
    /// input description has not been created.
    pub fn get_render_attachment_texture_bind_group(
        &self,
        input_description: &RenderAttachmentInputDescription,
    ) -> Option<&wgpu::BindGroup> {
        self.bind_groups_and_layouts
            .get(input_description)
            .map(|(_, bind_group)| bind_group)
    }

    /// Recreates all render attachment textures for the current state of the
    /// core system.
    pub fn recreate_textures(
        &mut self,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
    ) {
        for quantity_texture in self.quantity_textures.iter_mut() {
            *quantity_texture = RenderAttachmentTexture::new(
                graphics_device,
                rendering_surface,
                quantity_texture.quantity(),
            );
        }
        self.recreate_bind_groups(graphics_device);
    }

    /// Swaps the current and previous render attachment for each render
    /// attachment quantity that has a `Previous<Quantity>` variant. May create
    /// new bind groups to accommodate the swapped attachments.
    pub fn swap_previous_and_current_attachment_variants(
        &mut self,
        graphics_device: &GraphicsDevice,
    ) {
        self.swap_two_attachments(
            graphics_device,
            RenderAttachmentQuantity::LuminanceHistory,
            RenderAttachmentQuantity::PreviousLuminanceHistory,
        );
    }

    /// Creates any bind group layouts and bind groups represented in the given
    /// input description set that have not already been created.
    pub fn create_missing_bind_groups_and_layouts(
        &mut self,
        graphics_device: &GraphicsDevice,
        input_descriptions: &RenderAttachmentInputDescriptionSet,
    ) {
        for description in input_descriptions.descriptions() {
            self.bind_groups_and_layouts
                .entry(*description)
                .or_insert_with(|| {
                    Self::create_bind_group_and_layout_from_description(
                        graphics_device,
                        &self.quantity_textures,
                        &self.samplers,
                        description,
                    )
                });
        }
    }

    fn recreate_bind_groups(&mut self, graphics_device: &GraphicsDevice) {
        for (description, (bind_group_layout, bind_group)) in &mut self.bind_groups_and_layouts {
            let quantity = description.quantity;

            let quantity_texture = &self.quantity_textures[quantity.index()];
            let texture_view = quantity_texture
                .texture_view(description.mip_level())
                .unwrap();

            let sampler = &self.samplers[description.sampler as usize];

            let label = format!(
                "{} render attachment (mip level {}) with {:?} sampler in stages {:?}",
                quantity,
                description.mip_level(),
                description.sampler(),
                description.visibility()
            );

            *bind_group = Self::create_bind_group(
                graphics_device.device(),
                quantity.texture_binding(),
                quantity.sampler_binding(),
                bind_group_layout,
                texture_view,
                sampler.sampler(),
                &label,
            );
        }
    }

    fn swap_two_attachments(
        &mut self,
        graphics_device: &GraphicsDevice,
        first: RenderAttachmentQuantity,
        second: RenderAttachmentQuantity,
    ) {
        let input_descriptions_for_first: HashSet<_> = self
            .bind_groups_and_layouts
            .keys()
            .filter_map(|description| {
                if description.quantity == first {
                    Some(*description)
                } else {
                    None
                }
            })
            .collect();

        let input_descriptions_for_second: HashSet<_> = self
            .bind_groups_and_layouts
            .keys()
            .filter_map(|description| {
                if description.quantity == second {
                    // Since we don't care abount the quantity field when comparing descriptions
                    // between the two quantities, we make sure it is set to the first quantity also
                    // for the second quantity's descriptions. This way, when we take the difference
                    // between the two sets of descriptions, only differences in the non-quantity
                    // fields will be detected.
                    Some(description.with_quantity(first))
                } else {
                    None
                }
            })
            .collect();

        // Create any bind groups and layouts that one of the quantities has but
        // not the other
        for (quantity, missing_descriptions) in [
            (
                second,
                input_descriptions_for_first.difference(&input_descriptions_for_second),
            ),
            (
                first,
                input_descriptions_for_second.difference(&input_descriptions_for_first),
            ),
        ] {
            for description in missing_descriptions {
                let missing_description = description.with_quantity(quantity);
                let missing_bind_group_and_layout =
                    Self::create_bind_group_and_layout_from_description(
                        graphics_device,
                        &self.quantity_textures,
                        &self.samplers,
                        &missing_description,
                    );
                self.bind_groups_and_layouts
                    .insert(missing_description, missing_bind_group_and_layout);
            }
        }

        // Now that both quantities have all the same bind groups and layouts,
        // we can swap them all
        input_descriptions_for_first
            .union(&input_descriptions_for_second)
            .for_each(|description| {
                let description_for_first = description.with_quantity(first);
                let description_for_second = description.with_quantity(second);

                let bind_group_and_layout_for_first = self
                    .bind_groups_and_layouts
                    .remove(&description_for_first)
                    .unwrap();
                let bind_group_and_layout_for_second = self
                    .bind_groups_and_layouts
                    .remove(&description_for_second)
                    .unwrap();

                self.bind_groups_and_layouts
                    .insert(description_for_second, bind_group_and_layout_for_first);
                self.bind_groups_and_layouts
                    .insert(description_for_first, bind_group_and_layout_for_second);
            });

        // First we swap the locations of the textures, then we update the quantities
        // assigned to the textures to make the quantities and indices match
        self.quantity_textures.swap(first.index(), second.index());
        self.quantity_textures[first.index()].set_quantity(first);
        self.quantity_textures[second.index()].set_quantity(second);
    }

    fn create_bind_group_and_layout_from_description(
        graphics_device: &GraphicsDevice,
        quantity_textures: &[RenderAttachmentTexture; RenderAttachmentQuantity::count()],
        samplers: &[Sampler; 2],
        description: &RenderAttachmentInputDescription,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let quantity_texture = &quantity_textures[description.quantity.index()];
        let texture_view = quantity_texture
            .texture_view(description.mip_level())
            .unwrap();

        let sampler = &samplers[description.sampler() as usize];

        let texture_binding = description.quantity.texture_binding();
        let sampler_binding = description.quantity.sampler_binding();

        let label = format!(
            "{} render attachment with {:?} sampler in stages {:?}",
            description.quantity,
            description.sampler(),
            description.visibility()
        );

        let bind_group_layout = Self::create_bind_group_layout(
            graphics_device.device(),
            texture_binding,
            sampler_binding,
            quantity_texture.texture(),
            sampler,
            description.visibility(),
            &label,
        );

        let bind_group = Self::create_bind_group(
            graphics_device.device(),
            texture_binding,
            sampler_binding,
            &bind_group_layout,
            texture_view,
            sampler.sampler(),
            &label,
        );

        (bind_group_layout, bind_group)
    }

    fn create_bind_group_layout(
        device: &wgpu::Device,
        texture_binding: u32,
        sampler_binding: u32,
        texture: &Texture,
        sampler: &Sampler,
        visibility: wgpu::ShaderStages,
        label: &str,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                texture.create_bind_group_layout_entry(texture_binding, visibility),
                sampler.create_bind_group_layout_entry(sampler_binding, visibility),
            ],
            label: Some(&format!("{} bind group layout", label)),
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        texture_binding: u32,
        sampler_binding: u32,
        layout: &wgpu::BindGroupLayout,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: texture_binding,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: sampler_binding,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some(&format!("{} bind group", label)),
        })
    }
}

impl RenderAttachmentTexture {
    /// Creates a new render attachment texture of the same size as the given
    /// rendering surface for the given render attachment quantity.
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        quantity: RenderAttachmentQuantity,
    ) -> Self {
        let device = graphics_device.device();
        let (width, height) = rendering_surface.surface_dimensions();

        let format = quantity.texture_format();

        let texture_size = wgpu::Extent3d {
            width: width.into(),
            height: height.into(),
            depth_or_array_layers: 1,
        };

        let mip_level_count = u32::min(
            1 + quantity.max_mip_level(),
            texture_size.max_mips(wgpu::TextureDimension::D2),
        );

        let texture = Self::create_empty_render_attachment_texture(
            device,
            texture_size,
            format,
            mip_level_count,
            1,
            &format!(
                "Render attachment texture (quantity = {:?}, format = {:?})",
                quantity, format
            ),
        );

        let base_texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            base_mip_level: 0,
            mip_level_count: Some(1),
            ..Default::default()
        });

        let mip_texture_views: Vec<_> = (1..mip_level_count)
            .map(|mip_level| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: mip_level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();

        let texture = Texture::new(
            texture,
            base_texture_view,
            wgpu::TextureViewDimension::D2,
            None,
        );

        Self {
            quantity,
            texture,
            mip_texture_views,
        }
    }

    /// Returns the render attachment quantity.
    pub fn quantity(&self) -> RenderAttachmentQuantity {
        self.quantity
    }

    /// Returns the render attachment [`Texture`].
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Returns the render attachment texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture.texture().format()
    }

    /// Returns a view into the given mip level of the render attachment
    /// texture, or [`None`] if the texture does not have the given mip
    /// level.
    pub fn texture_view(&self, mip_level: u32) -> Option<&wgpu::TextureView> {
        if mip_level == 0 {
            Some(self.base_texture_view())
        } else {
            self.mip_texture_views.get(mip_level as usize - 1)
        }
    }

    /// Returns a view into the base (mip level 0) render attachment texture.
    pub fn base_texture_view(&self) -> &wgpu::TextureView {
        self.texture.view()
    }

    fn set_quantity(&mut self, quantity: RenderAttachmentQuantity) {
        // As long as the associated texture format and mip level count match, we can
        // safely assign another quantity to the attachment (useful for swapping)
        assert_eq!(self.format(), quantity.texture_format());
        assert_eq!(
            self.texture().texture().mip_level_count(),
            1 + quantity.max_mip_level()
        );
        self.quantity = quantity;
    }

    /// Creates a new 2D [`wgpu::Texture`] with the given size and format for
    /// use as a render attachment.
    fn create_empty_render_attachment_texture(
        device: &wgpu::Device,
        texture_size: wgpu::Extent3d,
        format: wgpu::TextureFormat,
        mip_level_count: u32,
        sample_count: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[],
        })
    }
}
