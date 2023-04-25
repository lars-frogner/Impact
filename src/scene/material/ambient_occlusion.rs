//! Material for computing ambient occlusion.

use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        create_uniform_buffer_bind_group_layout_entry, fre, AmbientOcclusionCalculationShaderInput,
        AmbientOcclusionShaderInput, MaterialShaderInput, RenderAttachmentQuantitySet,
        RenderPassHints, UniformBufferable,
    },
    scene::{FixedMaterialResources, MaterialID, MaterialLibrary, MaterialSpecification},
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64};
use lazy_static::lazy_static;
use nalgebra::Vector2;

/// The maximum number of samples that can be used for computing ambient
/// occlusion (if this is changed, `ambient_occlusion.wgsl` should be changed
/// accordingly).
pub const MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT: usize = 64;

/// Render pass hints for the ambient occlusion computation material.
pub const AMBIENT_OCCLUSION_COMPUTATION_RENDER_PASS_HINTS: RenderPassHints =
    RenderPassHints::NO_DEPTH_PREPASS;

/// Render pass hints for the ambient occlusion application material.
pub const AMBIENT_OCCLUSION_APPLICATION_RENDER_PASS_HINTS: RenderPassHints =
    RenderPassHints::NO_DEPTH_PREPASS
        .union(RenderPassHints::NO_CAMERA)
        .union(RenderPassHints::RENDERS_TO_SURFACE);

lazy_static! {
    /// ID of the ambient occlusion computation material in the
    /// [`MaterialLibrary`].
    pub static ref AMBIENT_OCCLUSION_COMPUTATION_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("AmbientOcclusionComputationMaterial"));

    /// ID of the ambient occlusion application material in the
    /// [`MaterialLibrary`].
    pub static ref AMBIENT_OCCLUSION_APPLICATION_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("AmbientOcclusionApplicationMaterial"));
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct AmbientOcclusionSamples {
    sample_offsets: [Vector2<fre>; MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT],
    sample_count: u32,
    _padding: [fre; 3],
}

/// Adds the material specifications for ambient occlusion materials with the
/// given parameters to the material library, overwriting any existing ambient
/// occlusion materials.
pub fn register_ambient_occlusion_materials(
    material_library: &mut MaterialLibrary,
    sample_count: u32,
    sample_radius: fre,
) {
    let vertex_attribute_requirements_for_mesh = VertexAttributeSet::POSITION;
    let vertex_attribute_requirements_for_shader = VertexAttributeSet::empty();

    let sample_uniform = AmbientOcclusionSamples::new(sample_count, sample_radius);

    material_library.add_material_specification(
        *AMBIENT_OCCLUSION_COMPUTATION_MATERIAL_ID,
        MaterialSpecification::new(
            vertex_attribute_requirements_for_mesh,
            vertex_attribute_requirements_for_shader,
            RenderAttachmentQuantitySet::POSITION | RenderAttachmentQuantitySet::NORMAL_VECTOR,
            RenderAttachmentQuantitySet::OCCLUSION,
            Some(FixedMaterialResources::new(&sample_uniform)),
            Vec::new(),
            AMBIENT_OCCLUSION_COMPUTATION_RENDER_PASS_HINTS,
            MaterialShaderInput::AmbientOcclusion(AmbientOcclusionShaderInput::Calculation(
                AmbientOcclusionCalculationShaderInput {
                    sample_uniform_binding: FixedMaterialResources::UNIFORM_BINDING,
                },
            )),
        ),
    );

    material_library.add_material_specification(
        *AMBIENT_OCCLUSION_APPLICATION_MATERIAL_ID,
        MaterialSpecification::new(
            vertex_attribute_requirements_for_mesh,
            vertex_attribute_requirements_for_shader,
            RenderAttachmentQuantitySet::COLOR | RenderAttachmentQuantitySet::OCCLUSION,
            RenderAttachmentQuantitySet::empty(),
            None,
            Vec::new(),
            AMBIENT_OCCLUSION_APPLICATION_RENDER_PASS_HINTS,
            MaterialShaderInput::AmbientOcclusion(AmbientOcclusionShaderInput::Application),
        ),
    );
}

impl AmbientOcclusionSamples {
    fn new(sample_count: u32, sample_radius: fre) -> Self {
        Self {
            sample_offsets: [Vector2::zeroed(); MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT],
            sample_count,
            _padding: [0.0; 3],
        }
    }
}

impl UniformBufferable for AmbientOcclusionSamples {
    const ID: ConstStringHash64 = ConstStringHash64::new("Ambient occlusion samples");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        create_uniform_buffer_bind_group_layout_entry(binding, wgpu::ShaderStages::FRAGMENT)
    }
}
