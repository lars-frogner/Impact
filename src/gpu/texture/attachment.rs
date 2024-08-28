//! Textures used as render attachments.

use crate::gpu::{
    rendering::surface::RenderingSurface,
    texture::{
        mipmap::{Mipmapper, MipmapperGenerator},
        Sampler, SamplerConfig, Texture, TextureAddressingConfig, TextureFilteringConfig,
    },
    GraphicsDevice,
};
use bitflags::bitflags;
use num_traits::AsPrimitive;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
};

bitflags! {
    /// Bitflag encoding a set of quantities that can be rendered to dedicated
    /// render attachment textures.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct RenderAttachmentQuantitySet: u16 {
        const DEPTH_STENCIL               = 1 << 0;
        const LINEAR_DEPTH                = 1 << 1;
        const NORMAL_VECTOR               = 1 << 2;
        const MOTION_VECTOR               = 1 << 3;
        const MATERIAL_COLOR              = 1 << 4;
        const MATERIAL_PROPERTIES         = 1 << 5;
        const LUMINANCE                   = 1 << 6;
        const LUMINANCE_AUX               = 1 << 7;
        const PREVIOUS_LUMINANCE_AUX      = 1 << 8;
        const AMBIENT_REFLECTED_LUMINANCE = 1 << 9;
        const OCCLUSION                   = 1 << 10;
        const EMISSIVE_LUMINANCE          = 1 << 11;
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
    PreviousLuminanceAux = 8,
    AmbientReflectedLuminance = 9,
    Occlusion = 10,
    EmissiveLuminance = 11,
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
    sampler: RenderAttachmentSampler,
    visibility: wgpu::ShaderStages,
}

/// Specifies how a render attachment should be used when bound as an output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderAttachmentOutputDescription {
    blending: Blending,
    write_mask: wgpu::ColorWrites,
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
    descriptions: HashMap<RenderAttachmentQuantity, D>,
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
    quantity_textures: [Option<RenderAttachmentTexture>; N_RENDER_ATTACHMENT_QUANTITIES],
    samplers: [Sampler; 2],
    bind_groups_and_layouts:
        HashMap<FullRenderAttachmentInputDescription, (wgpu::BindGroupLayout, wgpu::BindGroup)>,
}

/// A texture that can be used as a color attachment for rendering a specific
/// quantity into.
#[derive(Debug)]
pub struct RenderAttachmentTexture {
    quantity: RenderAttachmentQuantity,
    texture: Texture,
    attachment_view: wgpu::TextureView,
    mipmapper: Option<Mipmapper>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FullRenderAttachmentInputDescription {
    quantity: RenderAttachmentQuantity,
    description: RenderAttachmentInputDescription,
}

/// The total number of separate render attachment quantities.
const N_RENDER_ATTACHMENT_QUANTITIES: usize = 12;

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
    RenderAttachmentQuantity::PreviousLuminanceAux,
    RenderAttachmentQuantity::AmbientReflectedLuminance,
    RenderAttachmentQuantity::Occlusion,
    RenderAttachmentQuantity::EmissiveLuminance,
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
    RenderAttachmentQuantitySet::PREVIOUS_LUMINANCE_AUX,
    RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
    RenderAttachmentQuantitySet::OCCLUSION,
    RenderAttachmentQuantitySet::EMISSIVE_LUMINANCE,
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
    "luminance_aux",
    // We use the same name for the previous auxiliary luminance attachment so
    // that their `BindGroupLayout`s can be used interchangeably
    "luminance_aux",
    "ambient_reflected_luminance",
    "occlusion",
    "emissive_luminance",
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
    wgpu::TextureFormat::Rgba16Float,          // Previous auxiliary luminance
    wgpu::TextureFormat::Rgba16Float,          // Ambient reflected luminance
    wgpu::TextureFormat::R16Float,             // Occlusion
    wgpu::TextureFormat::Rgba16Float,          // Emissive luminance
];

/// Whether the texture for each render attachment quantity will have mipmaps.
const RENDER_ATTACHMENT_MIPMAPPED: [bool; N_RENDER_ATTACHMENT_QUANTITIES] = [
    false, // Depth-stencil
    false, // Linear depth
    false, // Normal vector
    false, // Motion vector
    false, // Material color
    false, // Material properties
    false, // Luminance
    false, // Auxiliary luminance
    false, // Previous auxiliary luminance
    false, // Ambient reflected luminance
    false, // Occlusion
    false, // Emissive luminance
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
    None,                     // Previous auxiliary luminance
    Some(wgpu::Color::BLACK), // Ambient reflected luminance
    Some(wgpu::Color::WHITE), // Occlusion
    Some(wgpu::Color::BLACK), // Emissive luminance
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
    (0, 1), // Previous auxiliary luminance
    (0, 1), // Ambient reflected luminance
    (0, 1), // Occlusion
    (0, 1), // Emissive luminance
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

    /// Whether the texture used for this render attachment quantity will have
    /// mipmaps.
    pub const fn is_mipmapped(&self) -> bool {
        RENDER_ATTACHMENT_MIPMAPPED[self.index()]
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

    /// Returns how the render attachment should be sampled.
    pub fn sampler(&self) -> RenderAttachmentSampler {
        self.sampler
    }

    /// Returns the shader stages where the render attachment texture and its
    /// sampler should be visible.
    pub fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }
}

impl Default for RenderAttachmentInputDescription {
    fn default() -> Self {
        Self {
            sampler: RenderAttachmentSampler::NonFiltering,
            visibility: wgpu::ShaderStages::FRAGMENT,
        }
    }
}

impl RenderAttachmentOutputDescription {
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
}

impl Default for RenderAttachmentOutputDescription {
    fn default() -> Self {
        Self {
            blending: Blending::Replace,
            write_mask: wgpu::ColorWrites::ALL,
        }
    }
}

impl<D> RenderAttachmentDescriptionSet<D> {
    /// Creates a new set of descriptions for the render attachments for the
    /// given quantities. Descriptions that are not specified will use the
    /// default description.
    pub fn new(
        quantities: RenderAttachmentQuantitySet,
        descriptions: HashMap<RenderAttachmentQuantity, D>,
    ) -> Self {
        let described_quantities = descriptions
            .keys()
            .cloned()
            .fold(RenderAttachmentQuantitySet::empty(), |set, quantity| {
                set | quantity.flag()
            });
        assert!(quantities.contains(described_quantities));
        Self {
            quantities,
            descriptions,
        }
    }

    /// Creates a new empty set of descriptions for render attachments.
    pub fn empty() -> Self {
        Self {
            quantities: RenderAttachmentQuantitySet::empty(),
            descriptions: HashMap::new(),
        }
    }

    /// Creates a new set of descriptions for the render attachments for the
    /// given quantities, using the default description for all of them.
    pub fn with_defaults(quantities: RenderAttachmentQuantitySet) -> Self {
        Self::new(quantities, HashMap::new())
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.quantities.is_empty()
    }

    /// Returns the render attachment quantities in the set.
    pub fn quantities(&self) -> RenderAttachmentQuantitySet {
        self.quantities
    }
}

impl<D> RenderAttachmentDescriptionSet<D>
where
    D: Clone + Default,
{
    /// Creates a set of descriptions for a single render attachment quantity.
    pub fn single(quantity: RenderAttachmentQuantity, description: D) -> Self {
        Self::new(
            quantity.flag(),
            [(quantity, description)].into_iter().collect(),
        )
    }

    /// Returns the description for the given quantity, or the default
    /// description if the quantity does not have a specific description.
    /// Returns [`None`] if the quantity is not in the set.
    pub fn get_description(&self, quantity: RenderAttachmentQuantity) -> Option<Cow<'_, D>> {
        if !self.quantities.contains(quantity.flag()) {
            return None;
        }
        Some(
            self.descriptions
                .get(&quantity)
                .map_or_else(|| Cow::Owned(D::default()), Cow::Borrowed),
        )
    }

    /// Inserts the given quantities in the set, using the default description
    /// for all of them.
    pub fn insert_with_defaults(&mut self, quantities: RenderAttachmentQuantitySet) {
        self.quantities |= quantities;
    }

    /// Inserts the given quantity in the set, using the given description.
    ///
    /// # Panics
    /// If the quantity is already in the set.
    pub fn insert_description(&mut self, quantity: RenderAttachmentQuantity, description: D) {
        assert!(
            !self.quantities.contains(quantity.flag()),
            "Tried to insert a description for an existing quantity"
        );
        self.quantities |= quantity.flag();
        self.descriptions.insert(quantity, description);
    }
}

impl RenderAttachmentTextureManager {
    /// Creates a new manager for render attachment textures, initializing
    /// all render attachment textures.
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        mipmapper_generator: &MipmapperGenerator,
    ) -> Self {
        let samplers = [
            Sampler::create(
                graphics_device,
                RenderAttachmentSampler::NonFiltering.config(),
            ),
            Sampler::create(graphics_device, RenderAttachmentSampler::Filtering.config()),
        ];

        let mut manager = Self {
            quantity_textures: [
                None, None, None, None, None, None, None, None, None, None, None, None,
            ],
            samplers,
            bind_groups_and_layouts: HashMap::new(),
        };

        manager.recreate_textures(graphics_device, rendering_surface, mipmapper_generator);

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
    ) -> impl Iterator<Item = &RenderAttachmentTexture> {
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

    /// Returns an iterator over the render attachment texture and sampler bind
    /// group layouts for the requested set of input descriptions, in the order
    /// in which the quantities are returned from the
    /// [`RenderAttachmentQuantity`] methods. Any layout that does not already
    /// exist will be created.
    pub fn create_and_get_render_attachment_texture_bind_group_layouts(
        &mut self,
        graphics_device: &GraphicsDevice,
        input_descriptions: &RenderAttachmentInputDescriptionSet,
    ) -> impl Iterator<Item = &wgpu::BindGroupLayout> {
        let descriptions =
            self.create_missing_bind_groups_and_layouts(graphics_device, input_descriptions);

        descriptions.into_iter().map(|description| {
            let (layout, _) = self.bind_groups_and_layouts.get(&description).unwrap();
            layout
        })
    }

    /// Returns an iterator over the render attachment texture and sampler bind
    /// groups for the requested set of input descriptions, in the order in
    /// which the quantities are returned from the [`RenderAttachmentQuantity`]
    /// methods.
    ///
    /// This method should only be called after
    /// [`Self::create_and_get_render_attachment_texture_bind_group_layouts`]
    /// has been called with the same input descriptions, otherwise the
    /// requested bind groups may not exist yet.
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
        RenderAttachmentQuantity::all()
            .iter()
            .filter_map(|&quantity| {
                let full_description = FullRenderAttachmentInputDescription {
                    quantity,
                    description: input_descriptions.get_description(quantity)?.into_owned(),
                };

                let (_, bind_group) = self
                    .bind_groups_and_layouts
                    .get(&full_description)
                    .expect("Missing bind group for render attachment input description");

                Some(bind_group)
            })
    }

    /// Recreates all render attachment textures for the current state of the
    /// core system.
    pub fn recreate_textures(
        &mut self,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        mipmapper_generator: &MipmapperGenerator,
    ) {
        for &quantity in RenderAttachmentQuantity::all() {
            self.recreate_render_attachment_texture(
                graphics_device,
                rendering_surface,
                mipmapper_generator,
                quantity,
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
            RenderAttachmentQuantity::LuminanceAux,
            RenderAttachmentQuantity::PreviousLuminanceAux,
        );
    }

    fn recreate_render_attachment_texture(
        &mut self,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        mipmapper_generator: &MipmapperGenerator,
        quantity: RenderAttachmentQuantity,
    ) {
        let quantity_texture = RenderAttachmentTexture::new(
            graphics_device,
            rendering_surface,
            mipmapper_generator,
            quantity,
        );

        self.quantity_textures[quantity.index()] = Some(quantity_texture);
    }

    fn recreate_bind_groups(&mut self, graphics_device: &GraphicsDevice) {
        for (full_description, (bind_group_layout, bind_group)) in &mut self.bind_groups_and_layouts
        {
            let quantity = full_description.quantity;

            let quantity_texture = self.quantity_textures[quantity.index()].as_ref().unwrap();
            let sampler = &self.samplers[full_description.description.sampler as usize];

            let label = format!(
                "{} render attachment with {:?} sampler in stages {:?}",
                quantity,
                full_description.description.sampler(),
                full_description.description.visibility()
            );

            *bind_group = Self::create_bind_group(
                graphics_device.device(),
                quantity.texture_binding(),
                quantity.sampler_binding(),
                bind_group_layout,
                quantity_texture.texture(),
                sampler,
                &label,
            );
        }
    }

    fn create_missing_bind_groups_and_layouts(
        &mut self,
        graphics_device: &GraphicsDevice,
        input_descriptions: &RenderAttachmentInputDescriptionSet,
    ) -> Vec<FullRenderAttachmentInputDescription> {
        RenderAttachmentQuantity::all()
            .iter()
            .filter_map(|&quantity| {
                let full_description = FullRenderAttachmentInputDescription {
                    quantity,
                    description: input_descriptions.get_description(quantity)?.into_owned(),
                };

                self.bind_groups_and_layouts
                    .entry(full_description.clone())
                    .or_insert_with(|| {
                        Self::create_bind_group_and_layout_from_description(
                            graphics_device,
                            &self.quantity_textures,
                            &self.samplers,
                            &full_description,
                        )
                    });

                Some(full_description)
            })
            .collect()
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
            .filter_map(|full_description| {
                if full_description.quantity == first {
                    Some(full_description.description)
                } else {
                    None
                }
            })
            .collect();

        let input_descriptions_for_second: HashSet<_> = self
            .bind_groups_and_layouts
            .keys()
            .filter_map(|full_description| {
                if full_description.quantity == second {
                    Some(full_description.description)
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
                let missing_full_description = FullRenderAttachmentInputDescription {
                    quantity,
                    description: *description,
                };
                let missing_bind_group_and_layout =
                    Self::create_bind_group_and_layout_from_description(
                        graphics_device,
                        &self.quantity_textures,
                        &self.samplers,
                        &missing_full_description,
                    );
                self.bind_groups_and_layouts
                    .insert(missing_full_description, missing_bind_group_and_layout);
            }
        }

        // Now that both quantities have all the same bind groups and layouts,
        // we can swap them all
        input_descriptions_for_first
            .union(&input_descriptions_for_second)
            .for_each(|description| {
                let description_for_first = FullRenderAttachmentInputDescription {
                    quantity: first,
                    description: *description,
                };
                let description_for_second = FullRenderAttachmentInputDescription {
                    quantity: second,
                    description: *description,
                };

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

        let mut first_attachment = self.quantity_textures[first.index()].take().unwrap();
        let mut second_attachment = self.quantity_textures[second.index()].take().unwrap();
        first_attachment.swap(&mut second_attachment);
        self.quantity_textures[first.index()] = Some(first_attachment);
        self.quantity_textures[second.index()] = Some(second_attachment);
    }

    fn create_bind_group_and_layout_from_description(
        graphics_device: &GraphicsDevice,
        quantity_textures: &[Option<RenderAttachmentTexture>; RenderAttachmentQuantity::count()],
        samplers: &[Sampler; 2],
        full_description: &FullRenderAttachmentInputDescription,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let quantity_texture = quantity_textures[full_description.quantity.index()]
            .as_ref()
            .unwrap();

        let sampler = &samplers[full_description.description.sampler as usize];

        let texture_binding = full_description.quantity.texture_binding();
        let sampler_binding = full_description.quantity.sampler_binding();

        let label = format!(
            "{} render attachment with {:?} sampler in stages {:?}",
            full_description.quantity,
            full_description.description.sampler(),
            full_description.description.visibility()
        );

        let bind_group_layout = Self::create_bind_group_layout(
            graphics_device.device(),
            texture_binding,
            sampler_binding,
            quantity_texture.texture(),
            sampler,
            full_description.description.visibility,
            &label,
        );

        let bind_group = Self::create_bind_group(
            graphics_device.device(),
            texture_binding,
            sampler_binding,
            &bind_group_layout,
            quantity_texture.texture(),
            sampler,
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
        texture: &Texture,
        sampler: &Sampler,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                texture.create_bind_group_entry(texture_binding),
                sampler.create_bind_group_entry(sampler_binding),
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
        mipmapper_generator: &MipmapperGenerator,
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

        let mip_level_count = if quantity.is_mipmapped() {
            texture_size.max_mips(wgpu::TextureDimension::D2)
        } else {
            1
        };

        let texture = Self::create_empty_render_attachment_texture(
            device,
            texture_size,
            format,
            mip_level_count,
            1,
            &format!(
                "Render attachment texture for {} (format = {:?})",
                quantity, format
            ),
        );

        let mipmapper = mipmapper_generator.generate_mipmapper(
            graphics_device,
            &texture,
            Cow::Owned(format!("{} render attachment", quantity)),
        );

        let attachment_view = texture.create_view(&wgpu::TextureViewDescriptor {
            mip_level_count: Some(1),
            ..Default::default()
        });

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

        let texture = Texture::new(texture, binding_view, wgpu::TextureViewDimension::D2, None);

        Self {
            quantity,
            texture,
            attachment_view,
            mipmapper,
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

    /// Returns a view into the texture for use as a render attachment.
    pub fn attachment_view(&self) -> &wgpu::TextureView {
        &self.attachment_view
    }

    /// Returns a view into the texture for use as a binding.
    pub fn binding_view(&self) -> &wgpu::TextureView {
        self.texture.view()
    }

    /// Returns a reference to the mipmapper for the render attachment texture,
    /// or [`None`] if the texture has no mipmaps.
    pub fn mipmapper(&self) -> Option<&Mipmapper> {
        self.mipmapper.as_ref()
    }

    fn swap(&mut self, other: &mut Self) {
        assert_eq!(
            self.format(),
            other.format(),
            "Cannot swap render attachments with different formats"
        );
        assert_eq!(
            self.mipmapper().is_some(),
            other.mipmapper().is_some(),
            "Cannot swap render attachments with and without mipmapping"
        );
        std::mem::swap(self, other);
        // We want to keep the old quantity, so we swap it back
        std::mem::swap(&mut self.quantity, &mut other.quantity);
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
