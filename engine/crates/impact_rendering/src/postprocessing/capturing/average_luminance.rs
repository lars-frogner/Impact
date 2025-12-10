//! Computation of the average luminance of the captured scene.

use crate::{
    attachment::RenderAttachmentTextureManager,
    compute::ComputePass,
    postprocessing::Postprocessor,
    render_command::storage_buffer_result_copy_command::StorageBufferResultCopyCommand,
    shader_templates::{
        luminance_histogram::LuminanceHistogramShaderTemplate,
        luminance_histogram_average::LuminanceHistogramAverageShaderTemplate,
    },
    surface::RenderingSurface,
};
use anyhow::{Result, bail};
use approx::abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_gpu::{
    assert_uniform_valid,
    device::GraphicsDevice,
    resource_group::{GPUResourceGroup, GPUResourceGroupID, GPUResourceGroupManager},
    shader::ShaderManager,
    storage::{StorageBufferID, StorageGPUBuffer, StorageGPUBufferManager},
    timestamp_query::TimestampQueryRegistry,
    uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
    wgpu,
};
use impact_math::{
    bounds::{Bounds, UpperExclusiveBounds},
    hash::ConstStringHash64,
    hash64,
};
use std::{borrow::Cow, mem, sync::LazyLock};

/// Configuration options for computing average captured luminance.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
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
    /// Map the computed histogram to the CPU for debugging.
    pub fetch_histogram: bool,
}

#[derive(Debug)]
pub(super) struct AverageLuminanceComputeCommands {
    histogram_compute_pass: ComputePass,
    histogram_copy_command: Option<StorageBufferResultCopyCommand>,
    average_compute_pass: ComputePass,
    result_copy_command: StorageBufferResultCopyCommand,
    config: AverageLuminanceComputationConfig,
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

pub static LUMINANCE_HISTOGRAM_STORAGE_BUFFER_ID: LazyLock<StorageBufferID> = LazyLock::new(|| {
    StorageBufferID(hash64!(format!(
        "LuminanceHistogramBuffer{{ bin_count: {} }}",
        HISTOGRAM_BIN_COUNT
    )))
});
pub static AVERAGE_LUMINANCE_STORAGE_BUFFER_ID: LazyLock<StorageBufferID> =
    LazyLock::new(|| StorageBufferID(hash64!(format!("AverageLuminanceBuffer"))));

impl AverageLuminanceComputationConfig {
    fn new_config_requires_parameters_update(&self, other: &Self) -> bool {
        abs_diff_ne!(
            self.luminance_bounds.lower(),
            other.luminance_bounds.lower()
        ) || abs_diff_ne!(
            self.luminance_bounds.upper(),
            other.luminance_bounds.upper()
        )
    }

    fn new_config_requires_average_parameters_update(&self, other: &Self) -> bool {
        self.new_config_requires_parameters_update(other)
            || abs_diff_ne!(self.current_frame_weight, other.current_frame_weight)
    }
}

impl Default for AverageLuminanceComputationConfig {
    fn default() -> Self {
        Self {
            luminance_bounds: UpperExclusiveBounds::new(1e2, 1e7),
            current_frame_weight: 0.02,
            fetch_histogram: false,
        }
    }
}

impl AverageLuminanceComputeCommands {
    pub(super) fn new(
        config: AverageLuminanceComputationConfig,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    ) -> Result<Self> {
        let histogram_compute_pass = create_luminance_histogram_compute_pass(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            &config.luminance_bounds,
        )?;

        let histogram_copy_command = if config.fetch_histogram {
            Some(StorageBufferResultCopyCommand::new(
                *LUMINANCE_HISTOGRAM_STORAGE_BUFFER_ID,
            ))
        } else {
            None
        };

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
            histogram_copy_command,
            average_compute_pass,
            result_copy_command,
            config,
        })
    }

    pub(super) fn config(&self) -> &AverageLuminanceComputationConfig {
        &self.config
    }

    pub(super) fn set_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        config: AverageLuminanceComputationConfig,
    ) {
        if self.config.new_config_requires_parameters_update(&config) {
            let parameters_uniform = LuminanceHistogramParameters::new(&config.luminance_bounds);
            update_luminance_histogram_parameters_uniform(
                graphics_device,
                gpu_resource_group_manager,
                &parameters_uniform,
            );
        }
        if self
            .config
            .new_config_requires_average_parameters_update(&config)
        {
            let average_parameters_uniform = LuminanceHistogramAverageParameters::new(
                &config.luminance_bounds,
                config.current_frame_weight,
            );
            update_luminance_histogram_average_parameters_uniform(
                graphics_device,
                gpu_resource_group_manager,
                &average_parameters_uniform,
            );
        }
        self.config = config;
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

            if let Some(command) = &self.histogram_copy_command {
                command.record(storage_gpu_buffer_manager, command_encoder)?;
            }

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

    #[allow(dead_code)]
    pub(super) fn debug_print_luminance_histogram(
        &self,
        graphics_device: &GraphicsDevice,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
    ) -> Result<()> {
        if self.histogram_copy_command.is_none() {
            bail!(
                "Use `fetch_histogram = true` in `AverageLuminanceComputationConfig` to enable debug printing of luminance histogram"
            );
        }

        let histogram =
            load_computed_luminance_histogram(graphics_device, storage_gpu_buffer_manager)
                .unwrap()?;

        let params = LuminanceHistogramParameters::new(&self.config.luminance_bounds);

        let luminances = (0..histogram.len()).map(|bin_idx| {
            let normalized_log2_luminance =
                bin_idx.saturating_sub(1) as f32 / (HISTOGRAM_BIN_COUNT - 2) as f32;
            let luminance = f32::exp2(
                normalized_log2_luminance / params.inverse_log2_luminance_range
                    + params.min_log2_luminance,
            );
            luminance
        });

        let mut weighted_sum = 0.0;
        let mut sum = 0.0;

        println!("******* Luminance histogram ******");
        for (luminance, &count) in luminances.zip(&histogram).skip(1) {
            weighted_sum += luminance * count as f32;
            sum += count as f32;
            println!("{luminance}: {count}");
        }
        let average = weighted_sum / sum;
        println!("----------------------------------");
        println!("Below threshold: {}", histogram[0]);
        println!("Total count: {sum}");
        println!("Average: {average}");
        println!("**********************************");

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

pub(super) fn load_computed_luminance_histogram(
    graphics_device: &GraphicsDevice,
    storage_gpu_buffer_manager: &StorageGPUBufferManager,
) -> Option<Result<Vec<u32>>> {
    storage_gpu_buffer_manager
        .get_storage_buffer(*LUMINANCE_HISTOGRAM_STORAGE_BUFFER_ID)
        .map(|buffer| {
            buffer
                .load_result(graphics_device, |bytes| {
                    bytemuck::cast_slice(bytes).to_vec()
                })
                .unwrap()
        })
}

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
    let resource_group_id = luminance_histogram_resource_group_id();
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
    let resource_group_id = luminance_histogram_average_resource_group_id();
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
            StorageGPUBuffer::new_read_write_with_result_on_cpu(
                graphics_device,
                HISTOGRAM_BIN_COUNT * mem::size_of::<f32>(),
                Cow::Borrowed("Luminance histogram buffer"),
            )
        })
}

fn update_luminance_histogram_parameters_uniform(
    graphics_device: &GraphicsDevice,
    gpu_resource_group_manager: &GPUResourceGroupManager,
    uniform: &LuminanceHistogramParameters,
) {
    let resource_group_id = luminance_histogram_resource_group_id();
    let resource_group = gpu_resource_group_manager
        .get_resource_group(resource_group_id)
        .expect(
            "Luminance histogram parameters resource group should not be missing during update",
        );
    let buffer = resource_group
        .single_uniform_buffer(0)
        .expect("Luminance histogram parameters resource group should have single uniform buffer");
    buffer.update_uniform(graphics_device, uniform);
}

fn update_luminance_histogram_average_parameters_uniform(
    graphics_device: &GraphicsDevice,
    gpu_resource_group_manager: &GPUResourceGroupManager,
    uniform: &LuminanceHistogramAverageParameters,
) {
    let resource_group_id = luminance_histogram_average_resource_group_id();
    let resource_group = gpu_resource_group_manager
        .get_resource_group(resource_group_id)
        .expect(
            "Luminance histogram average parameters resource group should not be missing during update",
        );
    let buffer = resource_group.single_uniform_buffer(0).expect(
        "Luminance histogram average parameters resource group should have single uniform buffer",
    );
    buffer.update_uniform(graphics_device, uniform);
}

fn luminance_histogram_resource_group_id() -> GPUResourceGroupID {
    GPUResourceGroupID(hash64!("LuminanceHistogramResources"))
}

fn luminance_histogram_average_resource_group_id() -> GPUResourceGroupID {
    GPUResourceGroupID(hash64!("LuminanceHistogramAverageResources"))
}
