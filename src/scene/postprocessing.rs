//! Management of postprocessing.

use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        fre, DepthMapUsage, MaterialShaderInput, OutputAttachmentSampling, PassthroughShaderInput,
        RenderAttachmentQuantity, RenderPassHints, RenderPassSpecification, RenderPassState,
    },
    scene::{
        create_ambient_occlusion_application_material,
        create_ambient_occlusion_computation_material, create_gaussian_blur_material,
        create_tone_mapping_material, GaussianBlurDirection, GaussianBlurSamples, MaterialID,
        MaterialLibrary, MaterialSpecification, ToneMapping, SCREEN_FILLING_QUAD_MESH_ID,
    },
};
use impact_utils::hash64;
use std::iter;

/// Manager of materials and render passes for postprocessing effects.
#[derive(Clone, Debug)]
pub struct Postprocessor {
    ambient_occlusion_enabled: bool,
    ambient_occlusion_passes: Vec<RenderPassSpecification>,
    bloom_enabled: bool,
    bloom_passes: Vec<RenderPassSpecification>,
    tone_mapping: ToneMapping,
    tone_mapping_passes: Vec<RenderPassSpecification>,
    exposure: fre,
}

/// Configuration options for ambient occlusion.
#[derive(Clone, Debug)]
pub struct AmbientOcclusionConfig {
    /// Whether ambient occlusion should be enabled when the scene loads.
    pub initially_enabled: bool,
    /// The number of samples to use for computing ambient occlusion.
    pub sample_count: u32,
    /// The sampling radius to use when computing ambient occlusion.
    pub sample_radius: fre,
}

#[derive(Clone, Debug)]
pub struct BloomConfig {
    /// Whether bloom should be enabled when the scene loads.
    pub initially_enabled: bool,
    /// The number of successive applications of Gaussian blur to perform.
    pub n_iterations: usize,
    /// The number of samples to use on each side of the center of the 1D
    /// Gaussian filtering kernel. Higher values will result in a wider blur.
    pub samples_per_side: u32,
    /// The number of samples to truncate from each tail of the 1D Gaussian
    /// distribution (this can be used to avoid computing samples with very
    /// small weights).
    pub tail_samples_to_truncate: u32,
}

impl Postprocessor {
    /// Creates a new postprocessor along with the associated materials and
    /// render passes according to the given configuration.
    pub fn new(
        material_library: &mut MaterialLibrary,
        ambient_occlusion_config: &AmbientOcclusionConfig,
        bloom_config: &BloomConfig,
        tone_mapping: ToneMapping,
        exposure: fre,
    ) -> Self {
        let ambient_occlusion_passes = setup_ambient_occlusion_materials_and_render_passes(
            material_library,
            ambient_occlusion_config,
        );

        let bloom_passes = setup_bloom_materials_and_render_passes(material_library, bloom_config);

        let tone_mapping_passes = setup_tone_mapping_materials_and_render_passes(material_library);

        Self {
            ambient_occlusion_enabled: ambient_occlusion_config.initially_enabled,
            ambient_occlusion_passes,
            bloom_enabled: bloom_config.initially_enabled,
            bloom_passes,
            tone_mapping,
            tone_mapping_passes,
            exposure,
        }
    }

    /// Returns the exposure value.
    pub fn exposure(&self) -> fre {
        self.exposure
    }

    /// Returns an iterator over the specifications for all postprocessing
    /// render passes, in the order in which they are to be performed.
    pub fn render_passes(&self) -> impl Iterator<Item = RenderPassSpecification> + '_ {
        assert_eq!(self.ambient_occlusion_passes.len(), 3);
        assert_eq!(self.tone_mapping_passes.len(), ToneMapping::all().len());
        self.ambient_occlusion_passes
            .iter()
            .cloned()
            .chain(self.bloom_passes.iter().cloned())
            .chain(self.tone_mapping_passes.iter().cloned())
    }

    /// Returns an iterator over the current states of all postprocessing render
    /// passes, in the same order as from [`Self::render_passes`].
    pub fn render_pass_states(&self) -> impl Iterator<Item = RenderPassState> + '_ {
        assert_eq!(self.ambient_occlusion_passes.len(), 3);
        assert_eq!(self.tone_mapping_passes.len(), ToneMapping::all().len());
        [
            !self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
        ]
        .into_iter()
        .chain(iter::once(!self.bloom_enabled))
        .chain(iter::repeat(self.bloom_enabled).take(self.bloom_passes.len() - 1))
        .chain(ToneMapping::all().map(|mapping| mapping == self.tone_mapping))
        .map(RenderPassState::active_if)
    }

    /// Toggles ambient occlusion.
    pub fn toggle_ambient_occlusion(&mut self) {
        self.ambient_occlusion_enabled = !self.ambient_occlusion_enabled;
    }

    /// Toggles bloom.
    pub fn toggle_bloom(&mut self) {
        self.bloom_enabled = !self.bloom_enabled;
    }

    /// Cycles tone mapping.
    pub fn cycle_tone_mapping(&mut self) {
        self.tone_mapping = match self.tone_mapping {
            ToneMapping::None => ToneMapping::ACES,
            ToneMapping::ACES => ToneMapping::KhronosPBRNeutral,
            ToneMapping::KhronosPBRNeutral => ToneMapping::None,
        };
    }

    /// Increases the exposure by a small multiplicative factor.
    pub fn increase_exposure(&mut self) {
        self.exposure *= 1.1;
    }

    /// Decreases the exposure by a small multiplicative factor.
    pub fn decrease_exposure(&mut self) {
        self.exposure /= 1.1;
    }
}

impl Default for AmbientOcclusionConfig {
    fn default() -> Self {
        Self {
            initially_enabled: true,
            sample_count: 4,
            sample_radius: 0.5,
        }
    }
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            initially_enabled: true,
            n_iterations: 3,
            samples_per_side: 4,
            tail_samples_to_truncate: 2,
        }
    }
}

fn setup_ambient_occlusion_materials_and_render_passes(
    material_library: &mut MaterialLibrary,
    ambient_occlusion_config: &AmbientOcclusionConfig,
) -> Vec<RenderPassSpecification> {
    vec![
        setup_unoccluded_ambient_reflected_luminance_application_material_and_render_pass(
            material_library,
        ),
        setup_ambient_occlusion_computation_material_and_render_pass(
            material_library,
            ambient_occlusion_config.sample_count,
            ambient_occlusion_config.sample_radius,
        ),
        setup_ambient_occlusion_application_material_and_render_pass(material_library),
    ]
}

fn setup_bloom_materials_and_render_passes(
    material_library: &mut MaterialLibrary,
    bloom_config: &BloomConfig,
) -> Vec<RenderPassSpecification> {
    let mut render_passes = Vec::with_capacity(1 + 2 * bloom_config.n_iterations);

    render_passes.push(setup_passthrough_material_and_render_pass(
        material_library,
        RenderAttachmentQuantity::EmissiveLuminance,
        RenderAttachmentQuantity::Luminance,
    ));

    if bloom_config.n_iterations > 0 {
        let bloom_sample_uniform = GaussianBlurSamples::new(
            bloom_config.samples_per_side,
            bloom_config.tail_samples_to_truncate,
        );
        for _ in 1..bloom_config.n_iterations {
            render_passes.push(setup_gaussian_blur_material_and_render_pass(
                material_library,
                RenderAttachmentQuantity::EmissiveLuminance,
                RenderAttachmentQuantity::EmissiveLuminanceAux,
                GaussianBlurDirection::Horizontal,
                &bloom_sample_uniform,
            ));
            render_passes.push(setup_gaussian_blur_material_and_render_pass(
                material_library,
                RenderAttachmentQuantity::EmissiveLuminanceAux,
                RenderAttachmentQuantity::EmissiveLuminance,
                GaussianBlurDirection::Vertical,
                &bloom_sample_uniform,
            ));
        }
        render_passes.push(setup_gaussian_blur_material_and_render_pass(
            material_library,
            RenderAttachmentQuantity::EmissiveLuminance,
            RenderAttachmentQuantity::EmissiveLuminanceAux,
            GaussianBlurDirection::Horizontal,
            &bloom_sample_uniform,
        ));
        // For the last pass, we write to the luminance attachment
        render_passes.push(setup_gaussian_blur_material_and_render_pass(
            material_library,
            RenderAttachmentQuantity::EmissiveLuminanceAux,
            RenderAttachmentQuantity::Luminance,
            GaussianBlurDirection::Vertical,
            &bloom_sample_uniform,
        ));
    }

    render_passes
}

fn setup_tone_mapping_materials_and_render_passes(
    material_library: &mut MaterialLibrary,
) -> Vec<RenderPassSpecification> {
    ToneMapping::all()
        .map(|mapping| {
            setup_tone_mapping_material_and_render_pass(
                material_library,
                RenderAttachmentQuantity::Luminance,
                mapping,
            )
        })
        .to_vec()
}

fn setup_passthrough_material_and_render_pass(
    material_library: &mut MaterialLibrary,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
) -> RenderPassSpecification {
    let (material_id, material_specification) = setup_passthrough_material(
        material_library,
        input_render_attachment_quantity,
        output_render_attachment_quantity,
    );
    define_passthrough_pass(
        material_id,
        material_specification.render_pass_hints(),
        input_render_attachment_quantity,
        output_render_attachment_quantity,
    )
}

fn setup_passthrough_material(
    material_library: &mut MaterialLibrary,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
) -> (MaterialID, &MaterialSpecification) {
    let material_id = MaterialID(hash64!(format!(
        "PassthroughMaterial{{ input: {}, output: {} }}",
        input_render_attachment_quantity, output_render_attachment_quantity,
    )));
    (
        material_id,
        material_library
            .material_specification_entry(material_id)
            .or_insert_with(|| {
                create_passthrough_material(
                    input_render_attachment_quantity,
                    output_render_attachment_quantity,
                )
            }),
    )
}

fn create_passthrough_material(
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
) -> MaterialSpecification {
    MaterialSpecification::new(
        VertexAttributeSet::POSITION,
        VertexAttributeSet::empty(),
        input_render_attachment_quantity.flag(),
        output_render_attachment_quantity.flag(),
        None,
        Vec::new(),
        RenderPassHints::NO_DEPTH_PREPASS.union(RenderPassHints::NO_CAMERA),
        MaterialShaderInput::Passthrough(PassthroughShaderInput {
            input_texture_and_sampler_bindings: input_render_attachment_quantity.bindings(),
        }),
    )
}

fn setup_ambient_occlusion_computation_material_and_render_pass(
    material_library: &mut MaterialLibrary,
    sample_count: u32,
    sample_radius: fre,
) -> RenderPassSpecification {
    let material_id = MaterialID(hash64!(format!(
        "AmbientOcclusionComputationMaterial{{ sample_count: {}, sampling_radius: {} }}",
        sample_count, sample_radius,
    )));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            create_ambient_occlusion_computation_material(sample_count, sample_radius)
        });
    define_ambient_occlusion_computation_pass(material_id, specification.render_pass_hints())
}

fn setup_ambient_occlusion_application_material_and_render_pass(
    material_library: &mut MaterialLibrary,
) -> RenderPassSpecification {
    let material_id = MaterialID(hash64!("AmbientOcclusionApplicationMaterial"));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(create_ambient_occlusion_application_material);
    define_ambient_occlusion_application_pass(material_id, specification.render_pass_hints())
}

fn setup_unoccluded_ambient_reflected_luminance_application_material_and_render_pass(
    material_library: &mut MaterialLibrary,
) -> RenderPassSpecification {
    let (material_id, specification) = setup_passthrough_material(
        material_library,
        RenderAttachmentQuantity::AmbientReflectedLuminance,
        RenderAttachmentQuantity::Luminance,
    );
    define_unoccluded_ambient_reflected_luminance_application_pass(
        material_id,
        specification.render_pass_hints(),
    )
}

fn setup_gaussian_blur_material_and_render_pass(
    material_library: &mut MaterialLibrary,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
    direction: GaussianBlurDirection,
    sample_uniform: &GaussianBlurSamples,
) -> RenderPassSpecification {
    let material_id = MaterialID(hash64!(format!(
        "GaussianBlurMaterial{{ direction: {}, input: {}, output: {}, sample_count: {}, truncated_tail_samples: {} }}",
        direction,
        input_render_attachment_quantity,
        output_render_attachment_quantity,
        sample_uniform.sample_count(),
        sample_uniform.truncated_tail_samples(),
    )));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            create_gaussian_blur_material(
                input_render_attachment_quantity,
                output_render_attachment_quantity,
                direction,
                sample_uniform,
            )
        });
    define_gaussian_blur_pass(material_id, specification.render_pass_hints(), direction)
}

fn setup_tone_mapping_material_and_render_pass(
    material_library: &mut MaterialLibrary,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    mapping: ToneMapping,
) -> RenderPassSpecification {
    let material_id = MaterialID(hash64!(format!(
        "ToneMappingMaterial{{ mapping: {}, input: {} }}",
        mapping, input_render_attachment_quantity,
    )));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| create_tone_mapping_material(input_render_attachment_quantity, mapping));
    define_tone_mapping_pass(material_id, specification.render_pass_hints(), mapping)
}

fn define_passthrough_pass(
    material_id: MaterialID,
    hints: RenderPassHints,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
) -> RenderPassSpecification {
    RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        hints,
        output_attachment_sampling: OutputAttachmentSampling::Single,
        label: format!(
            "Passthrough pass: {} -> {}",
            input_render_attachment_quantity, output_render_attachment_quantity
        ),
        ..Default::default()
    }
}

fn define_ambient_occlusion_computation_pass(
    material_id: MaterialID,
    hints: RenderPassHints,
) -> RenderPassSpecification {
    RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        hints,
        label: "Ambient occlusion computation pass".to_string(),
        ..Default::default()
    }
}

fn define_ambient_occlusion_application_pass(
    material_id: MaterialID,
    hints: RenderPassHints,
) -> RenderPassSpecification {
    RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        hints,
        label: "Ambient occlusion application pass".to_string(),
        ..Default::default()
    }
}

fn define_unoccluded_ambient_reflected_luminance_application_pass(
    material_id: MaterialID,
    hints: RenderPassHints,
) -> RenderPassSpecification {
    RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        hints,
        label: "Unoccluded ambient reflected luminance application pass".to_string(),
        ..Default::default()
    }
}

fn define_gaussian_blur_pass(
    material_id: MaterialID,
    hints: RenderPassHints,
    direction: GaussianBlurDirection,
) -> RenderPassSpecification {
    RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        hints,
        output_attachment_sampling: OutputAttachmentSampling::Single,
        label: format!("1D Gaussian blur pass ({})", direction),
        ..Default::default()
    }
}

fn define_tone_mapping_pass(
    material_id: MaterialID,
    hints: RenderPassHints,
    mapping: ToneMapping,
) -> RenderPassSpecification {
    RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        hints,
        output_attachment_sampling: OutputAttachmentSampling::Single,
        label: format!("Tone mapping pass ({})", mapping),
        ..Default::default()
    }
}
