//! Management of postprocessing.

use crate::{
    rendering::{
        fre, DepthMapUsage, OutputAttachmentSampling, RenderAttachmentQuantity, RenderPassHints,
        RenderPassSpecification, RenderPassState,
    },
    scene::{
        create_ambient_occlusion_application_material,
        create_ambient_occlusion_computation_material, create_gaussian_blur_material,
        create_unoccluded_ambient_color_application_material, GaussianBlurDirection,
        GaussianBlurSamples, MaterialID, MaterialLibrary, SCREEN_FILLING_QUAD_MESH_ID,
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
    ) -> Self {
        let ambient_occlusion_passes = setup_ambient_occlusion_materials_and_render_passes(
            material_library,
            ambient_occlusion_config,
        );

        let bloom_passes = setup_bloom_materials_and_render_passes(material_library, bloom_config);

        Self {
            ambient_occlusion_enabled: ambient_occlusion_config.initially_enabled,
            ambient_occlusion_passes,
            bloom_enabled: bloom_config.initially_enabled,
            bloom_passes,
        }
    }

    /// Returns an iterator over the specifications for all postprocessing
    /// render passes, in the order in which they are to be performed.
    pub fn render_passes(&self) -> impl Iterator<Item = RenderPassSpecification> + '_ {
        assert_eq!(self.ambient_occlusion_passes.len(), 3);
        self.ambient_occlusion_passes
            .iter()
            .cloned()
            .chain(self.bloom_passes.iter().cloned())
    }

    /// Returns an iterator over the current states of all postprocessing render
    /// passes, in the same order as from [`Self::render_passes`].
    pub fn render_pass_states(&self) -> impl Iterator<Item = RenderPassState> + '_ {
        assert_eq!(self.ambient_occlusion_passes.len(), 3);
        [
            !self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
        ]
        .into_iter()
        .chain(iter::once(!self.bloom_enabled))
        .chain(iter::repeat(self.bloom_enabled).take(self.bloom_passes.len() - 1))
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
        setup_unoccluded_ambient_color_application_material_and_render_pass(material_library),
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

    render_passes.push(setup_gaussian_blur_material_and_render_pass(
        material_library,
        RenderAttachmentQuantity::EmissiveColor,
        RenderAttachmentQuantity::Surface,
        GaussianBlurDirection::Horizontal,
        &GaussianBlurSamples::new(0, 0),
    ));

    if bloom_config.n_iterations > 0 {
        let bloom_sample_uniform = GaussianBlurSamples::new(
            bloom_config.samples_per_side,
            bloom_config.tail_samples_to_truncate,
        );
        for _ in 1..bloom_config.n_iterations {
            render_passes.push(setup_gaussian_blur_material_and_render_pass(
                material_library,
                RenderAttachmentQuantity::EmissiveColor,
                RenderAttachmentQuantity::EmissiveColorAux,
                GaussianBlurDirection::Horizontal,
                &bloom_sample_uniform,
            ));
            render_passes.push(setup_gaussian_blur_material_and_render_pass(
                material_library,
                RenderAttachmentQuantity::EmissiveColorAux,
                RenderAttachmentQuantity::EmissiveColor,
                GaussianBlurDirection::Vertical,
                &bloom_sample_uniform,
            ));
        }
        render_passes.push(setup_gaussian_blur_material_and_render_pass(
            material_library,
            RenderAttachmentQuantity::EmissiveColor,
            RenderAttachmentQuantity::EmissiveColorAux,
            GaussianBlurDirection::Horizontal,
            &bloom_sample_uniform,
        ));
        // For the last pass, we write to the surface attachment
        render_passes.push(setup_gaussian_blur_material_and_render_pass(
            material_library,
            RenderAttachmentQuantity::EmissiveColorAux,
            RenderAttachmentQuantity::Surface,
            GaussianBlurDirection::Vertical,
            &bloom_sample_uniform,
        ));
    }

    render_passes
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

fn setup_unoccluded_ambient_color_application_material_and_render_pass(
    material_library: &mut MaterialLibrary,
) -> RenderPassSpecification {
    let material_id = MaterialID(hash64!("UnoccludedAmbientColorApplicationMaterial"));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(create_unoccluded_ambient_color_application_material);
    define_unoccluded_ambient_color_application_pass(material_id, specification.render_pass_hints())
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

fn define_unoccluded_ambient_color_application_pass(
    material_id: MaterialID,
    hints: RenderPassHints,
) -> RenderPassSpecification {
    RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        hints,
        label: "Unoccluded ambient color application pass".to_string(),
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
