//! Materials for computing and applying ambient occlusion.

use crate::{
    assert_uniform_valid,
    geometry::VertexAttributeSet,
    gpu::{
        rendering::{
            create_uniform_buffer_bind_group_layout_entry, fre, RenderAttachmentQuantitySet,
            RenderPassHints, SingleUniformRenderBuffer, UniformBufferable,
        },
        shader::{
            AmbientOcclusionCalculationShaderInput, AmbientOcclusionShaderInput,
            MaterialShaderInput,
        },
        GraphicsDevice,
    },
    num::Float,
    scene::{MaterialSpecificResourceGroup, MaterialSpecification},
};
use bytemuck::{Pod, Zeroable};
use impact_utils::ConstStringHash64;
use nalgebra::Vector4;
use rand::{
    self,
    distributions::{Distribution, Uniform},
};
use std::borrow::Cow;

/// The maximum number of samples that can be used for computing ambient
/// occlusion.
pub const MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT: usize = 256;

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

/// Creates a [`MaterialSpecification`] for a material that computes ambient
/// occlusion and writes it to the occlusion attachment.
///
/// # Panics
/// - If the sample count is zero or exceeds
///   [`MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT`].
/// - If the sample radius does not exceed zero.
pub fn create_ambient_occlusion_computation_material(
    graphics_device: &GraphicsDevice,
    sample_count: u32,
    sample_radius: fre,
) -> MaterialSpecification {
    let sample_uniform = AmbientOcclusionSamples::new(sample_count, sample_radius, 1.0, 1.0);

    let sample_uniform_buffer = SingleUniformRenderBuffer::for_uniform(
        graphics_device,
        &sample_uniform,
        wgpu::ShaderStages::FRAGMENT,
        Cow::Borrowed("Ambient occlusion samples"),
    );
    let material_specific_resources = MaterialSpecificResourceGroup::new(
        graphics_device,
        vec![sample_uniform_buffer],
        &[],
        "Ambient occlusion samples",
    );

    let shader_input = MaterialShaderInput::AmbientOcclusion(
        AmbientOcclusionShaderInput::Calculation(AmbientOcclusionCalculationShaderInput {
            sample_uniform_binding: 0,
        }),
    );

    MaterialSpecification::new(
        VertexAttributeSet::POSITION,
        VertexAttributeSet::empty(),
        RenderAttachmentQuantitySet::POSITION | RenderAttachmentQuantitySet::NORMAL_VECTOR,
        RenderAttachmentQuantitySet::OCCLUSION,
        Some(material_specific_resources),
        Vec::new(),
        RenderPassHints::NO_DEPTH_PREPASS,
        shader_input,
    )
}

/// Creates a [`MaterialSpecification`] for a material that combines occlusion
/// and ambient reflected luminance from their respective attachments and adds
/// the resulting occluded ambient reflected luminance to the luminance
/// attachment.
pub fn create_ambient_occlusion_application_material() -> MaterialSpecification {
    MaterialSpecification::new(
        VertexAttributeSet::POSITION,
        VertexAttributeSet::empty(),
        RenderAttachmentQuantitySet::POSITION
            | RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE
            | RenderAttachmentQuantitySet::OCCLUSION,
        RenderAttachmentQuantitySet::LUMINANCE,
        None,
        Vec::new(),
        RenderPassHints::NO_DEPTH_PREPASS.union(RenderPassHints::NO_CAMERA),
        MaterialShaderInput::AmbientOcclusion(AmbientOcclusionShaderInput::Application),
    )
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

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(AmbientOcclusionSamples);
