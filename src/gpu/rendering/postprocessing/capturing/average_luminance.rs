//! Computation of the average luminance of the captured scene.

use crate::{
    assert_uniform_valid,
    gpu::{
        compute::ComputePass,
        query::TimestampQueryRegistry,
        rendering::{
            postprocessing::Postprocessor, render_command::StorageBufferResultCopyCommand,
            surface::RenderingSurface,
        },
        resource_group::{GPUResourceGroup, GPUResourceGroupID, GPUResourceGroupManager},
        shader::{
            template::{
                luminance_histogram::LuminanceHistogramShaderTemplate,
                luminance_histogram_average::LuminanceHistogramAverageShaderTemplate,
            },
            ShaderManager,
        },
        storage::{StorageBufferID, StorageGPUBuffer, StorageGPUBufferManager},
        texture::attachment::RenderAttachmentTextureManager,
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
    pub luminance_bounds: UpperExclusiveBounds<f32>,
    /// How much the average luminance computed for the current frame will be
    /// weighted compared to the average luminance computed for the previous
    /// frame. A value of 0.0 reuses the previous luminance without
    /// modification, while a value of 1.0 uses the current luminance without
    /// any contribution from the previous frame.
    pub current_frame_weight: f32,
}

#[derive(Debug)]
pub(super) struct AverageLuminanceComputeCommands {
    histogram_compute_pass: ComputePass,
    average_compute_pass: ComputePass,
    result_copy_command: StorageBufferResultCopyCommand,
}

/// Uniform holding parameters needed in the shader for computing the luminance
/// histogram.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct LuminanceHistogramParameters {
    min_log2_luminance: f32,
    inverse_log2_luminance_range: f32,
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
    min_log2_luminance: f32,
    log2_luminance_range: f32,
    current_frame_weight: f32,
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
            luminance_bounds: UpperExclusiveBounds::new(1e-2, 1e9),
            current_frame_weight: 0.02,
        }
    }
}

impl AverageLuminanceComputeCommands {
    pub(super) fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
        config: &AverageLuminanceComputationConfig,
    ) -> Result<Self> {
        let histogram_compute_pass = create_luminance_histogram_compute_pass(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            &config.luminance_bounds,
        )?;

        let average_compute_pass = create_luminance_histogram_average_compute_pass(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            &config.luminance_bounds,
            config.current_frame_weight,
        )?;

        let result_copy_command =
            StorageBufferResultCopyCommand::new(*AVERAGE_LUMINANCE_STORAGE_BUFFER_ID);

        Ok(Self {
            histogram_compute_pass,
            average_compute_pass,
            result_copy_command,
        })
    }

    pub(super) fn record(
        &self,
        rendering_surface: &RenderingSurface,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        postprocessor: &Postprocessor,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        enabled: bool,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if enabled {
            self.histogram_compute_pass.record(
                rendering_surface,
                gpu_resource_group_manager,
                render_attachment_texture_manager,
                postprocessor,
                timestamp_recorder,
                command_encoder,
            )?;

            self.average_compute_pass.record(
                rendering_surface,
                gpu_resource_group_manager,
                render_attachment_texture_manager,
                postprocessor,
                timestamp_recorder,
                command_encoder,
            )?;

            self.result_copy_command
                .record(storage_gpu_buffer_manager, command_encoder)?;
        }
        Ok(())
    }
}

impl LuminanceHistogramParameters {
    fn new(luminance_bounds: &UpperExclusiveBounds<f32>) -> Self {
        let log2_lower_bound = f32::log2(luminance_bounds.lower());
        let log2_upper_bound = f32::log2(luminance_bounds.upper());
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
    fn new(luminance_bounds: &UpperExclusiveBounds<f32>, current_frame_weight: f32) -> Self {
        let log2_lower_bound = f32::log2(luminance_bounds.lower());
        let log2_upper_bound = f32::log2(luminance_bounds.upper());
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
) -> Option<Result<f32>> {
    storage_gpu_buffer_manager
        .get_storage_buffer(*AVERAGE_LUMINANCE_STORAGE_BUFFER_ID)
        .map(|buffer| {
            buffer
                .load_result(graphics_device, |bytes| *bytemuck::from_bytes(bytes))
                .unwrap()
        })
}

/// Creates a [`ComputePass`] that computes the histogram of the luminances in
/// the luminance attachment.
fn create_luminance_histogram_compute_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    luminance_bounds: &UpperExclusiveBounds<f32>,
) -> Result<ComputePass> {
    let resource_group_id = GPUResourceGroupID(hash64!(format!(
        "LuminanceHistogramResources{{ luminance_range: [{}, {}) }}",
        luminance_bounds.lower(),
        luminance_bounds.upper(),
    )));

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
                &[],
                &[],
                wgpu::ShaderStages::COMPUTE,
                "Luminance histogram resources",
            )
        });

    let shader_template =
        LuminanceHistogramShaderTemplate::new(LOG2_HISTOGRAM_THREADS_PER_SIDE, resource_group_id);

    ComputePass::new(
        graphics_device,
        shader_manager,
        gpu_resource_group_manager,
        render_attachment_texture_manager,
        shader_template,
        Cow::Borrowed("Luminance histogram compute pass"),
    )
}

/// Creates a [`ComputePass`] that computes the weighted average of the
/// luminances in the luminance attachment using the histogram computed by the
/// previous pass.
fn create_luminance_histogram_average_compute_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
    storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    luminance_bounds: &UpperExclusiveBounds<f32>,
    current_frame_weight: f32,
) -> Result<ComputePass> {
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
                        mem::size_of::<f32>(),
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

    let shader_template =
        LuminanceHistogramAverageShaderTemplate::new(HISTOGRAM_BIN_COUNT, resource_group_id);

    ComputePass::new(
        graphics_device,
        shader_manager,
        gpu_resource_group_manager,
        render_attachment_texture_manager,
        shader_template,
        Cow::Borrowed("Luminance histogram average compute pass"),
    )
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
                HISTOGRAM_BIN_COUNT * mem::size_of::<f32>(),
                Cow::Borrowed("Luminance histogram buffer"),
            )
        })
}
