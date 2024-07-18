//! Computation of the average luminance of the captured scene.

use crate::{
    assert_uniform_valid,
    gpu::{
        compute::ComputePassSpecification,
        push_constant::{PushConstant, PushConstantVariant},
        rendering::{fre, render_command::RenderCommandSpecification},
        resource_group::{GPUResourceGroup, GPUResourceGroupID, GPUResourceGroupManager},
        shader::{template::SpecificShaderTemplate, ShaderManager},
        storage::{StorageBufferID, StorageGPUBuffer, StorageGPUBufferManager},
        texture::{
            attachment::{RenderAttachmentQuantity, RenderAttachmentTextureManager},
            Texture,
        },
        uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
        GraphicsDevice,
    },
    util::bounds::{Bounds, UpperExclusiveBounds},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64};
use lazy_static::lazy_static;
use std::{borrow::Cow, mem};

/// Configuration options for computing average captured luminance.
#[derive(Clone, Debug)]
pub struct AverageLuminanceComputationConfig {
    /// The range of luminance values that the histogram used for computing
    /// average luminance should cover (luminance values outside these bounds
    /// will be clipped).
    ///
    /// # Unit
    /// Nit (cd/m²)
    pub luminance_bounds: UpperExclusiveBounds<fre>,
    /// How much the average luminance computed for the current frame will be
    /// weighted compared to the average luminance computed for the previous
    /// frame. A value of 0.0 reuses the previous luminance without
    /// modification, while a value of 1.0 uses the current luminance without
    /// any contribution from the previous frame.
    pub current_frame_weight: fre,
}

/// Uniform holding parameters needed in the shader for computing the luminance
/// histogram.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct LuminanceHistogramParameters {
    min_log2_luminance: fre,
    inverse_log2_luminance_range: fre,
    _pad: [u8; 8],
}

/// Uniform holding parameters needed in the shader for computing the average of
/// the luminance histogram.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct LuminanceHistogramAverageParameters {
    min_log2_luminance: fre,
    log2_luminance_range: fre,
    current_frame_weight: fre,
    _pad: [u8; 4],
}

const LOG2_HISTOGRAM_THREADS_PER_SIDE: usize = 4;

const HISTOGRAM_THREADS_PER_SIDE: usize = 1 << LOG2_HISTOGRAM_THREADS_PER_SIDE;

const HISTOGRAM_BIN_COUNT: usize = HISTOGRAM_THREADS_PER_SIDE * HISTOGRAM_THREADS_PER_SIDE;

lazy_static! {
    pub static ref LUMINANCE_HISTOGRAM_STORAGE_BUFFER_ID: StorageBufferID =
        StorageBufferID(hash64!(format!(
            "LuminanceHistogramBuffer{{ bin_count: {} }}",
            HISTOGRAM_BIN_COUNT
        )));
    pub static ref AVERAGE_LUMINANCE_STORAGE_BUFFER_ID: StorageBufferID =
        StorageBufferID(hash64!(format!("AverageLuminanceBuffer")));
}

impl Default for AverageLuminanceComputationConfig {
    fn default() -> Self {
        Self {
            luminance_bounds: UpperExclusiveBounds::new(1.0, 1e9),
            current_frame_weight: 0.5,
        }
    }
}

impl LuminanceHistogramParameters {
    fn new(luminance_bounds: &UpperExclusiveBounds<fre>) -> Self {
        let log2_lower_bound = fre::log2(luminance_bounds.lower());
        let log2_upper_bound = fre::log2(luminance_bounds.upper());
        Self {
            min_log2_luminance: log2_lower_bound,
            inverse_log2_luminance_range: (log2_upper_bound - log2_lower_bound).recip(),
            _pad: [0; 8],
        }
    }
}

impl UniformBufferable for LuminanceHistogramParameters {
    const ID: ConstStringHash64 = ConstStringHash64::new("Luminance histogram parameters");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(LuminanceHistogramParameters);

impl LuminanceHistogramAverageParameters {
    fn new(luminance_bounds: &UpperExclusiveBounds<fre>, current_frame_weight: fre) -> Self {
        let log2_lower_bound = fre::log2(luminance_bounds.lower());
        let log2_upper_bound = fre::log2(luminance_bounds.upper());
        Self {
            min_log2_luminance: log2_lower_bound,
            log2_luminance_range: log2_upper_bound - log2_lower_bound,
            current_frame_weight,
            _pad: [0; 4],
        }
    }
}

impl UniformBufferable for LuminanceHistogramAverageParameters {
    const ID: ConstStringHash64 = ConstStringHash64::new("Luminance histogram average parameters");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(LuminanceHistogramAverageParameters);

pub(super) fn load_computed_average_luminance(
    graphics_device: &GraphicsDevice,
    storage_gpu_buffer_manager: &StorageGPUBufferManager,
) -> Option<Result<fre>> {
    storage_gpu_buffer_manager
        .get_storage_buffer(*AVERAGE_LUMINANCE_STORAGE_BUFFER_ID)
        .map(|buffer| {
            buffer
                .load_result(graphics_device, |bytes| *bytemuck::from_bytes(bytes))
                .unwrap()
        })
}

pub(super) fn setup_average_luminance_computations_and_render_commands(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &RenderAttachmentTextureManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    config: &AverageLuminanceComputationConfig,
) -> Vec<RenderCommandSpecification> {
    vec![
        create_luminance_histogram_compute_pass(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            &config.luminance_bounds,
            false,
        ),
        create_luminance_histogram_average_compute_pass(
            graphics_device,
            shader_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            &config.luminance_bounds,
            config.current_frame_weight,
        ),
        RenderCommandSpecification::StorageBufferResultCopyPass {
            buffer_id: *AVERAGE_LUMINANCE_STORAGE_BUFFER_ID,
        },
    ]
}

pub(super) fn create_luminance_histogram_compute_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &RenderAttachmentTextureManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    luminance_bounds: &UpperExclusiveBounds<fre>,
    render_attachment_textures_were_recreated: bool,
) -> RenderCommandSpecification {
    let luminance_texture = render_attachment_texture_manager
        .render_attachment_texture(RenderAttachmentQuantity::Luminance);

    let workgroup_counts =
        determine_luminance_histogram_workgroup_counts(luminance_texture.regular.texture());

    let resource_group_id = GPUResourceGroupID(hash64!(format!(
        "LuminanceHistogramResources{{ luminance_range: [{}, {}) }}",
        luminance_bounds.lower(),
        luminance_bounds.upper(),
    )));

    if render_attachment_textures_were_recreated {
        // If the textures were recreated, the resource bind group is
        // invalidated and must be recreated
        gpu_resource_group_manager.remove_resource_group(resource_group_id);
    }

    gpu_resource_group_manager
        .resource_group_entry(resource_group_id)
        .or_insert_with(|| {
            let parameter_uniform = LuminanceHistogramParameters::new(luminance_bounds);

            let parameter_uniform_buffer = SingleUniformGPUBuffer::for_uniform(
                graphics_device,
                &parameter_uniform,
                wgpu::ShaderStages::COMPUTE,
                Cow::Borrowed("Luminance histogram parameters"),
            );

            let histogram_buffer =
                get_or_create_histogram_storage_buffer(graphics_device, storage_gpu_buffer_manager);

            GPUResourceGroup::new(
                graphics_device,
                vec![parameter_uniform_buffer],
                &[histogram_buffer],
                &[luminance_texture.regular.texture()],
                &[],
                wgpu::ShaderStages::COMPUTE,
                "Luminance histogram resources",
            )
        });

    let shader_id = shader_manager
        .get_or_create_compute_shader_from_template(
            graphics_device,
            SpecificShaderTemplate::LuminanceHistogram,
            &[
                ("threads_per_side", HISTOGRAM_THREADS_PER_SIDE.to_string()),
                ("params_binding", "0".to_string()),
                ("histogram_binding", "1".to_string()),
                ("texture_binding", "2".to_string()),
            ],
        )
        .unwrap();

    let push_constants = PushConstant::new(
        PushConstantVariant::InverseExposure,
        wgpu::ShaderStages::COMPUTE,
    )
    .into();

    RenderCommandSpecification::ComputePass(ComputePassSpecification {
        shader_id,
        workgroup_counts,
        push_constants,
        resource_group_id: Some(resource_group_id),
        label: format!(
            "Luminance histogram compute pass (luminance range: [{}, {}), bin count: {}, workgroup counts: {:?})",
            luminance_bounds.lower(),
            luminance_bounds.upper(),
            HISTOGRAM_BIN_COUNT,
            workgroup_counts
        ),
    })
}

fn create_luminance_histogram_average_compute_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    luminance_bounds: &UpperExclusiveBounds<fre>,
    current_frame_weight: fre,
) -> RenderCommandSpecification {
    let resource_group_id = GPUResourceGroupID(hash64!(format!(
        "LuminanceHistogramAverageResources{{ luminance_range: [{}, {}), current_frame_weight: {} }}",
        luminance_bounds.lower(),
        luminance_bounds.upper(),
        current_frame_weight,
    )));

    gpu_resource_group_manager
        .resource_group_entry(resource_group_id)
        .or_insert_with(|| {
            let parameter_uniform =
                LuminanceHistogramAverageParameters::new(luminance_bounds, current_frame_weight);

            let parameter_uniform_buffer = SingleUniformGPUBuffer::for_uniform(
                graphics_device,
                &parameter_uniform,
                wgpu::ShaderStages::COMPUTE,
                Cow::Borrowed("Luminance histogram average parameters"),
            );

            get_or_create_histogram_storage_buffer(graphics_device, storage_gpu_buffer_manager);

            storage_gpu_buffer_manager
                .storage_buffer_entry(*AVERAGE_LUMINANCE_STORAGE_BUFFER_ID)
                .or_insert_with(|| {
                    StorageGPUBuffer::new_read_write_with_result_on_cpu(
                        graphics_device,
                        mem::size_of::<fre>(),
                        Cow::Borrowed("Average luminance buffer"),
                    )
                });

            let histogram_buffer = storage_gpu_buffer_manager
                .get_storage_buffer(*LUMINANCE_HISTOGRAM_STORAGE_BUFFER_ID)
                .unwrap();

            let average_buffer = storage_gpu_buffer_manager
                .get_storage_buffer(*AVERAGE_LUMINANCE_STORAGE_BUFFER_ID)
                .unwrap();

            GPUResourceGroup::new(
                graphics_device,
                vec![parameter_uniform_buffer],
                &[histogram_buffer, average_buffer],
                &[],
                &[],
                wgpu::ShaderStages::COMPUTE,
                "Luminance histogram average resources",
            )
        });

    let shader_id = shader_manager
        .get_or_create_compute_shader_from_template(
            graphics_device,
            SpecificShaderTemplate::LuminanceHistogramAverage,
            &[
                ("bin_count", HISTOGRAM_BIN_COUNT.to_string()),
                ("params_binding", "0".to_string()),
                ("histogram_binding", "1".to_string()),
                ("average_binding", "2".to_string()),
            ],
        )
        .unwrap();

    let push_constants =
        PushConstant::new(PushConstantVariant::PixelCount, wgpu::ShaderStages::COMPUTE).into();

    RenderCommandSpecification::ComputePass(ComputePassSpecification {
        shader_id,
        workgroup_counts: [1; 3],
        push_constants,
        resource_group_id: Some(resource_group_id),
        label: format!(
            "Luminance histogram average compute pass (luminance range: [{}, {}), current frame weight: {}, bin count: {})",
            luminance_bounds.lower(),
            luminance_bounds.upper(),
            current_frame_weight,
            HISTOGRAM_BIN_COUNT,
        ),
    })
}

fn determine_luminance_histogram_workgroup_counts(luminance_texture: &Texture) -> [u32; 3] {
    let size = luminance_texture.texture().size();

    let workgroup_count_across_width = min_workgroups_to_cover_texture_extent(size.width);
    let workgroup_count_across_height = min_workgroups_to_cover_texture_extent(size.height);

    [
        workgroup_count_across_width,
        workgroup_count_across_height,
        1,
    ]
}

fn min_workgroups_to_cover_texture_extent(extent: u32) -> u32 {
    (f64::from(extent) / HISTOGRAM_THREADS_PER_SIDE as f64).ceil() as u32
}

fn get_or_create_histogram_storage_buffer<'a>(
    graphics_device: &GraphicsDevice,
    storage_gpu_buffer_manager: &'a mut StorageGPUBufferManager,
) -> &'a mut StorageGPUBuffer {
    storage_gpu_buffer_manager
        .storage_buffer_entry(*LUMINANCE_HISTOGRAM_STORAGE_BUFFER_ID)
        .or_insert_with(|| {
            StorageGPUBuffer::new_read_write(
                graphics_device,
                HISTOGRAM_BIN_COUNT * mem::size_of::<fre>(),
                Cow::Borrowed("Luminance histogram buffer"),
            )
        })
}
