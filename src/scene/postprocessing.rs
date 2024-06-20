//! Management of postprocessing.

use crate::{
    rendering::{fre, DepthMapUsage, RenderPassHints, RenderPassSpecification, RenderPassState},
    scene::{
        create_ambient_occlusion_application_material,
        create_ambient_occlusion_computation_material,
        create_unoccluded_ambient_color_application_material, MaterialID, MaterialLibrary,
        SCREEN_FILLING_QUAD_MESH_ID,
    },
};
use impact_utils::hash64;

/// Manager of materials and render passes for postprocessing effects.
#[derive(Clone, Debug)]
pub struct Postprocessor {
    ambient_occlusion_enabled: bool,
    ambient_occlusion_passes: Vec<RenderPassSpecification>,
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

impl Postprocessor {
    /// Creates a new postprocessor along with the associated materials and
    /// render passes according to the given configuration.
    pub fn new(
        material_library: &mut MaterialLibrary,
        ambient_occlusion_config: &AmbientOcclusionConfig,
    ) -> Self {
        let ambient_occlusion_passes = vec![
            setup_ambient_occlusion_computation_material_and_render_pass(
                material_library,
                ambient_occlusion_config,
            ),
            setup_ambient_occlusion_application_material_and_render_pass(material_library),
            setup_unoccluded_ambient_color_application_material_and_render_pass(material_library),
        ];
        Self {
            ambient_occlusion_enabled: ambient_occlusion_config.initially_enabled,
            ambient_occlusion_passes,
        }
    }

    /// Returns an iterator over the specifications for all postprocessing
    /// render passes, in the order in which they are to be performed.
    pub fn render_passes(&self) -> impl Iterator<Item = RenderPassSpecification> + '_ {
        assert_eq!(self.ambient_occlusion_passes.len(), 3);
        self.ambient_occlusion_passes.iter().cloned()
    }

    /// Returns an iterator over the current states of all postprocessing render
    /// passes, in the same order as from [`Self::render_passes`].
    pub fn render_pass_states(&self) -> impl Iterator<Item = RenderPassState> + '_ {
        assert_eq!(self.ambient_occlusion_passes.len(), 3);
        [
            self.ambient_occlusion_enabled,
            self.ambient_occlusion_enabled,
            !self.ambient_occlusion_enabled,
        ]
        .into_iter()
        .map(RenderPassState::active_if)
    }

    /// Toggles ambient occlusion.
    pub fn toggle_ambient_occlusion(&mut self) {
        self.ambient_occlusion_enabled = !self.ambient_occlusion_enabled;
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

fn setup_ambient_occlusion_computation_material_and_render_pass(
    material_library: &mut MaterialLibrary,
    config: &AmbientOcclusionConfig,
) -> RenderPassSpecification {
    let material_id = MaterialID(hash64!(format!(
        "AmbientOcclusionComputationMaterial{{ sample_count: {}, sampling_radius: {} }}",
        config.sample_count, config.sample_radius,
    )));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            create_ambient_occlusion_computation_material(config.sample_count, config.sample_radius)
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
