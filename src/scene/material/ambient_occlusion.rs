//! Material for computing ambient occlusion.

use crate::{
    geometry::VertexAttributeSet,
    num::Float,
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
use nalgebra::Vector4;
use rand::{
    self,
    distributions::{Distribution, Uniform},
};

/// The maximum number of samples that can be used for computing ambient
/// occlusion.
pub const MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT: usize = 256;

/// Render pass hints for the ambient occlusion computation material.
pub const AMBIENT_OCCLUSION_COMPUTATION_RENDER_PASS_HINTS: RenderPassHints =
    RenderPassHints::NO_DEPTH_PREPASS;

/// Render pass hints for the ambient occlusion application material.
pub const AMBIENT_OCCLUSION_APPLICATION_RENDER_PASS_HINTS: RenderPassHints =
    RenderPassHints::NO_DEPTH_PREPASS
        .union(RenderPassHints::NO_CAMERA)
        .union(RenderPassHints::RENDERS_TO_SURFACE);

/// Render pass hints for the ambient occlusion disabled material.
pub const AMBIENT_OCCLUSION_DISABLED_RENDER_PASS_HINTS: RenderPassHints =
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

    /// ID of the material in the [`MaterialLibrary`] that should be used
    /// when ambient occlusion is disabled.
    pub static ref AMBIENT_OCCLUSION_DISABLED_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("AmbientOcclusionDisabledMaterial"));
}

/// Uniform holding horizontal offsets for the ambient occlusion samples. Only
/// the first `sample_count` offsets in the array will be computed. The uniform
/// also contains the ambient occlusion parameters needed in the shader.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct AmbientOcclusionSamples {
    sample_offsets: [Vector4<fre>; MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT],
    sample_count: u32,
    sample_radius: f32,
    sample_normalization: f32,
    contrast: fre,
}

/// Adds the material specifications for ambient occlusion materials with the
/// given parameters to the material library, overwriting any existing ambient
/// occlusion materials.
///
/// # Panics
/// - If the sample count is zero or exceeds [`MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT`].
/// - If the sample radius does not exceed zero.
pub fn register_ambient_occlusion_materials(
    material_library: &mut MaterialLibrary,
    sample_count: u32,
    sample_radius: fre,
) {
    let vertex_attribute_requirements_for_mesh = VertexAttributeSet::POSITION;
    let vertex_attribute_requirements_for_shader = VertexAttributeSet::empty();

    let sample_uniform = AmbientOcclusionSamples::new(sample_count, sample_radius, 1.0, 1.0);

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
            RenderAttachmentQuantitySet::POSITION
                | RenderAttachmentQuantitySet::COLOR
                | RenderAttachmentQuantitySet::OCCLUSION,
            RenderAttachmentQuantitySet::empty(),
            None,
            Vec::new(),
            AMBIENT_OCCLUSION_APPLICATION_RENDER_PASS_HINTS,
            MaterialShaderInput::AmbientOcclusion(AmbientOcclusionShaderInput::Application),
        ),
    );

    material_library.add_material_specification(
        *AMBIENT_OCCLUSION_DISABLED_MATERIAL_ID,
        MaterialSpecification::new(
            vertex_attribute_requirements_for_mesh,
            vertex_attribute_requirements_for_shader,
            RenderAttachmentQuantitySet::COLOR,
            RenderAttachmentQuantitySet::empty(),
            None,
            Vec::new(),
            AMBIENT_OCCLUSION_DISABLED_RENDER_PASS_HINTS,
            MaterialShaderInput::AmbientOcclusion(AmbientOcclusionShaderInput::Disabled),
        ),
    );
}

impl AmbientOcclusionSamples {
    fn new(sample_count: u32, sample_radius: fre, intensity_scale: f32, contrast: f32) -> Self {
        assert_ne!(sample_count, 0);
        assert!(sample_count <= MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT as u32);
        assert!(sample_radius > 0.0);

        let mut rng = rand::thread_rng();
        let unit_range = Uniform::from(0.0..1.0);
        let angle_range = Uniform::from(0.0..fre::TWO_PI);

        let mut sample_offsets = [Vector4::zeroed(); MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT];

        for offset in &mut sample_offsets[..(sample_count as usize)] {
            // Take square root of radius fraction to ensure uniform
            // distribution over the disk
            let radius_fraction = fre::sqrt(unit_range.sample(&mut rng));
            let radius = sample_radius * radius_fraction;

            let angle = angle_range.sample(&mut rng);
            let (sin_angle, cos_angle) = fre::sin_cos(angle);

            offset.x = radius * cos_angle;
            offset.y = radius * sin_angle;
        }

        let sample_normalization = 2.0 * intensity_scale / (fre::PI * (sample_count as fre));

        Self {
            sample_offsets,
            sample_count,
            sample_radius,
            sample_normalization,
            contrast,
        }
    }
}

impl UniformBufferable for AmbientOcclusionSamples {
    const ID: ConstStringHash64 = ConstStringHash64::new("Ambient occlusion samples");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        create_uniform_buffer_bind_group_layout_entry(binding, wgpu::ShaderStages::FRAGMENT)
    }
}
