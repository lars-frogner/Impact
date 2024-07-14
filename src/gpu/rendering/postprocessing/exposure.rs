//! Automatic exposure.

use crate::{
    assert_uniform_valid,
    gpu::{
        compute::{
            GPUComputationID, GPUComputationLibrary, GPUComputationResourceGroup,
            GPUComputationSpecification,
        },
        push_constant::{PushConstant, PushConstantVariant},
        rendering::{
            fre,
            render_command::{ComputePassSpecification, RenderCommandSpecification},
        },
        shader::{compute::LUMINANCE_HISTOGRAM_SHADER_TEMPLATE, template, ShaderManager},
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
use bytemuck::{Pod, Zeroable};
use impact_utils::{hash64, ConstStringHash64};
use std::{borrow::Cow, mem};

/// Configuration options for exposure computation.
#[derive(Clone, Debug)]
pub struct ExposureConfig {
    pub initial_exposure: fre,
    /// The range of luminance values to support when computing optimal exposure
    /// for a luminance map (luminance values outside these bounds will be
    /// clipped).
    ///
    /// # Unit
    /// Nit (cd/m²)
    pub luminance_bounds: UpperExclusiveBounds<fre>,
}

/// Uniform holding parameters for the range of luminance values supported by
/// the luminance histogram.
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

const LOG2_HISTOGRAM_THREADS_PER_SIDE: usize = 4;

const HISTOGRAM_THREADS_PER_SIDE: usize = 1 << LOG2_HISTOGRAM_THREADS_PER_SIDE;

const HISTOGRAM_BIN_COUNT: usize = HISTOGRAM_THREADS_PER_SIDE * HISTOGRAM_THREADS_PER_SIDE;

impl Default for ExposureConfig {
    fn default() -> Self {
        Self {
            initial_exposure: 1e-4,
            luminance_bounds: UpperExclusiveBounds::new(1.0, 1e9),
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

pub(super) fn setup_exposure_computations_and_render_commands(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &RenderAttachmentTextureManager,
    storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    gpu_computation_library: &mut GPUComputationLibrary,
    exposure_config: &ExposureConfig,
    render_attachment_textures_were_recreated: bool,
) -> Vec<RenderCommandSpecification> {
    let mut compute_passes = Vec::with_capacity(2);

    compute_passes.push(setup_luminance_histogram_computation_and_compute_pass(
        graphics_device,
        shader_manager,
        render_attachment_texture_manager,
        storage_gpu_buffer_manager,
        gpu_computation_library,
        &exposure_config.luminance_bounds,
        render_attachment_textures_were_recreated,
    ));

    compute_passes
}

fn setup_luminance_histogram_computation_and_compute_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    render_attachment_texture_manager: &RenderAttachmentTextureManager,
    storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    gpu_computation_library: &mut GPUComputationLibrary,
    luminance_bounds: &UpperExclusiveBounds<fre>,
    render_attachment_textures_were_recreated: bool,
) -> RenderCommandSpecification {
    let luminance_texture = render_attachment_texture_manager
        .render_attachment_texture(RenderAttachmentQuantity::Luminance);

    let workgroup_size =
        determine_luminance_histogram_workgroup_size(luminance_texture.regular.texture());

    let computation_id = GPUComputationID(hash64!(format!(
        "LuminanceHistogramComputation{{ luminance_range: [{}, {}), bin_count: {}, workgroup_size: {:?} }}",
        luminance_bounds.lower(),
        luminance_bounds.upper(),
        HISTOGRAM_BIN_COUNT,
        workgroup_size
    )));

    if render_attachment_textures_were_recreated {
        // If the textures were recreated, the resource bind group is
        // invalidated, so the computation must be recreated
        gpu_computation_library.remove_computation_specification(computation_id);
    }

    gpu_computation_library
        .computation_specification_entry(computation_id)
        .or_insert_with(|| {
            create_luminance_histogram_computation(
                graphics_device,
                shader_manager,
                storage_gpu_buffer_manager,
                workgroup_size,
                luminance_texture.regular.texture(),
                luminance_bounds,
            )
        });

    RenderCommandSpecification::ComputePass(ComputePassSpecification {
        computation_id,
        label: format!(
            "Luminance histogram compute pass (luminance range: [{}, {}), bin count: {}, workgroup size: {:?})",
            luminance_bounds.lower(),
            luminance_bounds.upper(),
            HISTOGRAM_BIN_COUNT,
            workgroup_size
        ),
    })
}

fn create_luminance_histogram_computation(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    workgroup_size: [u32; 3],
    luminance_texture: &Texture,
    luminance_bounds: &UpperExclusiveBounds<fre>,
) -> GPUComputationSpecification {
    let parameter_uniform = LuminanceHistogramParameters::new(luminance_bounds);

    let parameter_uniform_buffer = SingleUniformGPUBuffer::for_uniform(
        graphics_device,
        &parameter_uniform,
        wgpu::ShaderStages::COMPUTE,
        Cow::Borrowed("Luminance histogram parameters"),
    );

    let histogram_buffer_id = StorageBufferID(hash64!(format!(
        "LuminanceHistogramBuffer{{ bin_count: {} }}",
        HISTOGRAM_BIN_COUNT
    )));
    let histogram_buffer = storage_gpu_buffer_manager
        .storage_buffer_entry(histogram_buffer_id)
        .or_insert_with(|| {
            StorageGPUBuffer::new_read_write(
                graphics_device,
                HISTOGRAM_BIN_COUNT * mem::size_of::<fre>(),
                Cow::Borrowed("Luminance histogram buffer"),
            )
        });

    let push_constants = PushConstant::new(
        PushConstantVariant::InverseExposure,
        wgpu::ShaderStages::COMPUTE,
    )
    .into();

    let replacements = [
        ("threads_per_side", HISTOGRAM_THREADS_PER_SIDE.to_string()),
        ("params_binding", "0".to_string()),
        ("histogram_binding", "1".to_string()),
        ("texture_binding", "2".to_string()),
    ];

    let shader_id =
        template::create_shader_id_for_template("LuminanceHistogram", replacements.clone());

    shader_manager
        .compute_shaders
        .entry(shader_id)
        .or_insert_with(|| {
            LUMINANCE_HISTOGRAM_SHADER_TEMPLATE
                .resolve_and_compile_as_wgsl(
                    graphics_device,
                    replacements,
                    "Luminance histogram shader",
                )
                .unwrap()
        });

    GPUComputationSpecification::new(
        shader_id,
        workgroup_size,
        push_constants,
        Some(GPUComputationResourceGroup::new(
            graphics_device,
            vec![parameter_uniform_buffer],
            &[histogram_buffer],
            &[luminance_texture],
            "Luminance histogram computation",
        )),
    )
}

fn determine_luminance_histogram_workgroup_size(luminance_texture: &Texture) -> [u32; 3] {
    let size = luminance_texture.texture().size();

    let workgroup_width = min_workgroups_to_cover_texture_extent(size.width);
    let workgroup_height = min_workgroups_to_cover_texture_extent(size.height);

    [workgroup_width, workgroup_height, 1]
}

fn min_workgroups_to_cover_texture_extent(extent: u32) -> u32 {
    (f64::from(extent) / HISTOGRAM_THREADS_PER_SIDE as f64).ceil() as u32
}
