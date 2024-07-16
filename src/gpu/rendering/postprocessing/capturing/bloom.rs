//! Materials and render passes for applying bloom.

use crate::{
    gpu::{
        rendering::{
            postprocessing::gaussian_blur::{self, GaussianBlurDirection, GaussianBlurSamples},
            render_command::RenderCommandSpecification,
        },
        texture::attachment::RenderAttachmentQuantity,
        GraphicsDevice,
    },
    material::MaterialLibrary,
};

/// Configuration options for bloom.
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

pub(super) fn setup_bloom_materials_and_render_commands(
    graphics_device: &GraphicsDevice,
    material_library: &mut MaterialLibrary,
    bloom_config: &BloomConfig,
) -> Vec<RenderCommandSpecification> {
    let mut render_passes = Vec::with_capacity(1 + 2 * bloom_config.n_iterations);

    render_passes.push(super::super::setup_passthrough_material_and_render_pass(
        material_library,
        RenderAttachmentQuantity::EmissiveLuminance,
        RenderAttachmentQuantity::Luminance,
    ));

    if bloom_config.n_iterations > 0 {
        let bloom_sample_uniform = GaussianBlurSamples::new(
            bloom_config.samples_per_side,
            bloom_config.tail_samples_to_truncate,
        );
        for _ in 1..bloom_config.n_iterations {
            render_passes.push(gaussian_blur::setup_gaussian_blur_material_and_render_pass(
                graphics_device,
                material_library,
                RenderAttachmentQuantity::EmissiveLuminance,
                RenderAttachmentQuantity::EmissiveLuminanceAux,
                GaussianBlurDirection::Horizontal,
                &bloom_sample_uniform,
            ));
            render_passes.push(gaussian_blur::setup_gaussian_blur_material_and_render_pass(
                graphics_device,
                material_library,
                RenderAttachmentQuantity::EmissiveLuminanceAux,
                RenderAttachmentQuantity::EmissiveLuminance,
                GaussianBlurDirection::Vertical,
                &bloom_sample_uniform,
            ));
        }
        render_passes.push(gaussian_blur::setup_gaussian_blur_material_and_render_pass(
            graphics_device,
            material_library,
            RenderAttachmentQuantity::EmissiveLuminance,
            RenderAttachmentQuantity::EmissiveLuminanceAux,
            GaussianBlurDirection::Horizontal,
            &bloom_sample_uniform,
        ));
        // For the last pass, we write to the luminance attachment
        render_passes.push(gaussian_blur::setup_gaussian_blur_material_and_render_pass(
            graphics_device,
            material_library,
            RenderAttachmentQuantity::EmissiveLuminanceAux,
            RenderAttachmentQuantity::Luminance,
            GaussianBlurDirection::Vertical,
            &bloom_sample_uniform,
        ));
    }

    render_passes
}
