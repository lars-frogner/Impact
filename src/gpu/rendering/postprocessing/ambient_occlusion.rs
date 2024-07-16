//! Materials and render passes for computing and applying ambient occlusion.

use crate::{
    assert_uniform_valid,
    gpu::{
        push_constant::{PushConstant, PushConstantVariant},
        rendering::{
            fre,
            render_command::{
                DepthMapUsage, RenderCommandSpecification, RenderPassHints, RenderPassSpecification,
            },
        },
        shader::{
            AmbientOcclusionCalculationShaderInput, AmbientOcclusionShaderInput,
            MaterialShaderInput,
        },
        texture::attachment::{RenderAttachmentQuantity, RenderAttachmentQuantitySet},
        uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
        GraphicsDevice,
    },
    material::{MaterialID, MaterialLibrary, MaterialSpecificResourceGroup, MaterialSpecification},
    mesh::{VertexAttributeSet, SCREEN_FILLING_QUAD_MESH_ID},
    num::Float,
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64};
use nalgebra::Vector4;
use rand::{
    self,
    distributions::{Distribution, Uniform},
};
use std::borrow::Cow;

/// The maximum number of samples that can be used for computing ambient
/// occlusion.
pub const MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT: usize = 256;

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

impl Default for AmbientOcclusionConfig {
    fn default() -> Self {
        Self {
            initially_enabled: true,
            sample_count: 4,
            sample_radius: 0.5,
        }
    }
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
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(AmbientOcclusionSamples);

pub(super) fn setup_ambient_occlusion_materials_and_render_commands(
    graphics_device: &GraphicsDevice,
    material_library: &mut MaterialLibrary,
    ambient_occlusion_config: &AmbientOcclusionConfig,
) -> Vec<RenderCommandSpecification> {
    vec![
        setup_unoccluded_ambient_reflected_luminance_application_material_and_render_pass(
            material_library,
        ),
        setup_ambient_occlusion_computation_material_and_render_pass(
            graphics_device,
            material_library,
            ambient_occlusion_config.sample_count,
            ambient_occlusion_config.sample_radius,
        ),
        setup_ambient_occlusion_application_material_and_render_pass(material_library),
    ]
}

fn setup_ambient_occlusion_computation_material_and_render_pass(
    graphics_device: &GraphicsDevice,
    material_library: &mut MaterialLibrary,
    sample_count: u32,
    sample_radius: fre,
) -> RenderCommandSpecification {
    let material_id = MaterialID(hash64!(format!(
        "AmbientOcclusionComputationMaterial{{ sample_count: {}, sampling_radius: {} }}",
        sample_count, sample_radius,
    )));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            create_ambient_occlusion_computation_material(
                graphics_device,
                sample_count,
                sample_radius,
            )
        });
    define_ambient_occlusion_computation_pass(material_id, specification)
}

fn setup_ambient_occlusion_application_material_and_render_pass(
    material_library: &mut MaterialLibrary,
) -> RenderCommandSpecification {
    let material_id = MaterialID(hash64!("AmbientOcclusionApplicationMaterial"));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(create_ambient_occlusion_application_material);
    define_ambient_occlusion_application_pass(material_id, specification)
}

fn setup_unoccluded_ambient_reflected_luminance_application_material_and_render_pass(
    material_library: &mut MaterialLibrary,
) -> RenderCommandSpecification {
    let (material_id, specification) = super::setup_passthrough_material(
        material_library,
        RenderAttachmentQuantity::AmbientReflectedLuminance,
        RenderAttachmentQuantity::Luminance,
    );
    define_unoccluded_ambient_reflected_luminance_application_pass(material_id, specification)
}

/// Creates a [`MaterialSpecification`] for a material that computes ambient
/// occlusion and writes it to the occlusion attachment.
///
/// # Panics
/// - If the sample count is zero or exceeds
///   [`MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT`].
/// - If the sample radius does not exceed zero.
fn create_ambient_occlusion_computation_material(
    graphics_device: &GraphicsDevice,
    sample_count: u32,
    sample_radius: fre,
) -> MaterialSpecification {
    let sample_uniform = AmbientOcclusionSamples::new(sample_count, sample_radius, 1.0, 1.0);

    let sample_uniform_buffer = SingleUniformGPUBuffer::for_uniform(
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
fn create_ambient_occlusion_application_material() -> MaterialSpecification {
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

fn define_ambient_occlusion_computation_pass(
    material_id: MaterialID,
    material_specification: &MaterialSpecification,
) -> RenderCommandSpecification {
    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        vertex_attribute_requirements: material_specification
            .vertex_attribute_requirements_for_shader(),
        input_render_attachment_quantities: material_specification
            .input_render_attachment_quantities(),
        output_render_attachment_quantities: material_specification
            .output_render_attachment_quantities(),
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints: material_specification.render_pass_hints(),
        label: "Ambient occlusion computation pass".to_string(),
        ..Default::default()
    })
}

fn define_ambient_occlusion_application_pass(
    material_id: MaterialID,
    material_specification: &MaterialSpecification,
) -> RenderCommandSpecification {
    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        vertex_attribute_requirements: material_specification
            .vertex_attribute_requirements_for_shader(),
        input_render_attachment_quantities: material_specification
            .input_render_attachment_quantities(),
        output_render_attachment_quantities: material_specification
            .output_render_attachment_quantities(),
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints: material_specification.render_pass_hints(),
        label: "Ambient occlusion application pass".to_string(),
        ..Default::default()
    })
}

fn define_unoccluded_ambient_reflected_luminance_application_pass(
    material_id: MaterialID,
    material_specification: &MaterialSpecification,
) -> RenderCommandSpecification {
    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        depth_map_usage: DepthMapUsage::StencilTest,
        vertex_attribute_requirements: material_specification
            .vertex_attribute_requirements_for_shader(),
        input_render_attachment_quantities: material_specification
            .input_render_attachment_quantities(),
        output_render_attachment_quantities: material_specification
            .output_render_attachment_quantities(),
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints: material_specification.render_pass_hints(),
        label: "Unoccluded ambient reflected luminance application pass".to_string(),
        ..Default::default()
    })
}
