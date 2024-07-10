//! Material for applying a Gaussian blur.

use crate::{
    assert_uniform_valid,
    gpu::{
        rendering::{fre, render_command::RenderPassHints},
        shader::{GaussianBlurShaderInput, MaterialShaderInput},
        texture::attachment::RenderAttachmentQuantity,
        uniform::{self, SingleUniformRenderBuffer, UniformBufferable},
        GraphicsDevice,
    },
    material::{MaterialSpecificResourceGroup, MaterialSpecification},
    mesh::VertexAttributeSet,
};
use bytemuck::{Pod, Zeroable};
use impact_utils::ConstStringHash64;
use nalgebra::Vector4;
use std::{borrow::Cow, fmt::Display};

/// The maximum number of unique Gaussian weights that can be passed to the GPU
/// for computing Gaussian blur. The actual number of samples that will be
/// averaged along each direction is `2 * MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS - 1`.
pub const MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS: usize = 64;

/// Uniform holding offsets and weights for the Gaussian blur samples. Only the
/// first `sample_count` sets of offsets and weights in the array will be
/// computed. Since the weights and offsets are symmetrical around the center of
/// the 1D Gaussian kernel, only the values for the center and the positive
/// offset side are included.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct GaussianBlurSamples {
    /// Each entry stores an offset as the first vector component and a weight
    /// as the second component. The remaining vector components are ignored.
    /// The reason we need to use a `Vector4` is that arrays in uniforms must
    /// have elements aligned to 16 bytes.
    sample_offsets_and_weights: [Vector4<fre>; MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS],
    sample_count: u32,
    truncated_tail_samples: u32,
    _pad: [u8; 8],
}

/// The direction of a 1D Gaussian blur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GaussianBlurDirection {
    Horizontal,
    Vertical,
}

impl GaussianBlurSamples {
    /// Creates a new set of offsets and weights for Gaussian blur samples.
    ///
    /// # Panics
    /// - If `samples_per_side` is not smaller than
    ///   [`MAX_GAUSSIAN_BLUR_SAMPLE_COUNT`].
    /// - If `tail_samples_to_truncate` is larger than `samples_per_side`.
    pub fn new(samples_per_side: u32, tail_samples_to_truncate: u32) -> Self {
        assert!(samples_per_side < MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS as u32);
        assert!(tail_samples_to_truncate <= samples_per_side);

        // We will only compute offsets and weights for the center of the 1D
        // kernel as well as the positive offset side, since they are
        // symmetrical around the center
        let sample_count = 1 + samples_per_side;

        // The 1D Gaussian kernel weights can be computed from the binomial
        // coefficients in the appropriate row of Pascal's triangle
        let binomial_coefficients =
            compute_pascal_triangle_row(2 * (samples_per_side + tail_samples_to_truncate));

        // We drop the `tail_samples_to_truncate` coefficients on each side of
        // the row, in order to avoid including very small weights that make
        // little difference for the result
        let truncation_offset = tail_samples_to_truncate as usize;
        let truncated_binomial_coefficients = &binomial_coefficients
            [truncation_offset..binomial_coefficients.len() - truncation_offset];

        // To obtain the weight, we must normalize each coefficient by the sum
        // of all coefficients
        let coefficient_sum: u64 = truncated_binomial_coefficients.iter().copied().sum();
        let weight_normalization = (coefficient_sum as fre).recip();

        // Drop the coefficients on the negative offset side
        let coefficients_from_center =
            &truncated_binomial_coefficients[truncated_binomial_coefficients.len() / 2..];

        let mut sample_offsets_and_weights = [Vector4::zeroed(); MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS];

        for (sample_idx, offset_and_weight) in sample_offsets_and_weights[..(sample_count as usize)]
            .iter_mut()
            .enumerate()
        {
            // Offset
            offset_and_weight.x = sample_idx as fre;
            // Weight
            offset_and_weight.y =
                coefficients_from_center[sample_idx] as fre * weight_normalization;
        }

        Self {
            sample_offsets_and_weights,
            sample_count,
            truncated_tail_samples: tail_samples_to_truncate,
            _pad: [0; 8],
        }
    }

    /// Returns the number of samples.
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Returns the number of samples that were truncated from the tail of the
    /// 1D Gaussian distribution.
    pub fn truncated_tail_samples(&self) -> u32 {
        self.truncated_tail_samples
    }

    /// Returns an iterator over the 1D Gaussian kernel sample offsets starting
    /// at the center and proceeding along the positive offset side.
    pub fn sample_offsets(&self) -> impl Iterator<Item = fre> + '_ {
        self.sample_offsets_and_weights
            .iter()
            .take(self.sample_count as usize)
            .map(|offset_and_weight| offset_and_weight.x)
    }

    /// Returns an iterator over the 1D Gaussian kernel sample weights starting
    /// at the center and proceeding along the positive offset side
    pub fn sample_weights(&self) -> impl Iterator<Item = fre> + '_ {
        self.sample_offsets_and_weights
            .iter()
            .take(self.sample_count as usize)
            .map(|offset_and_weight| offset_and_weight.y)
    }
}

impl Display for GaussianBlurDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Horizontal => "horizontal",
                Self::Vertical => "vertical",
            }
        )
    }
}

/// Creates a [`MaterialSpecification`] for a material that applies a 1D
/// Gaussian blur in the given direction to the input attachment and writes the
/// result to the output attachment.
pub fn create_gaussian_blur_material(
    graphics_device: &GraphicsDevice,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    output_render_attachment_quantity: RenderAttachmentQuantity,
    direction: GaussianBlurDirection,
    sample_uniform: &GaussianBlurSamples,
) -> MaterialSpecification {
    let sample_uniform_buffer = SingleUniformRenderBuffer::for_uniform(
        graphics_device,
        sample_uniform,
        wgpu::ShaderStages::FRAGMENT,
        Cow::Borrowed("Gaussian blur samples"),
    );
    let material_specific_resources = MaterialSpecificResourceGroup::new(
        graphics_device,
        vec![sample_uniform_buffer],
        &[],
        "Gaussian blur samples",
    );

    let shader_input = MaterialShaderInput::GaussianBlur(GaussianBlurShaderInput {
        direction,
        sample_uniform_binding: 0,
        input_texture_and_sampler_bindings: input_render_attachment_quantity.bindings(),
    });

    MaterialSpecification::new(
        VertexAttributeSet::POSITION,
        VertexAttributeSet::empty(),
        input_render_attachment_quantity.flag(),
        output_render_attachment_quantity.flag(),
        Some(material_specific_resources),
        Vec::new(),
        RenderPassHints::NO_DEPTH_PREPASS.union(RenderPassHints::NO_CAMERA),
        shader_input,
    )
}

impl UniformBufferable for GaussianBlurSamples {
    const ID: ConstStringHash64 = ConstStringHash64::new("Gaussian blur samples");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(GaussianBlurSamples);

/// Computes the `k`'th row of Pascal's triangle, which contains the binomial
/// coefficients `(n k)` for `n = 0..=k`.
fn compute_pascal_triangle_row(k: u32) -> Vec<u64> {
    let final_row_length = k as usize + 1;

    let mut row = Vec::with_capacity(final_row_length);
    row.push(1);

    if k == 0 {
        return row;
    }

    let mut next_row = Vec::with_capacity(final_row_length);

    for _ in 1..=k {
        next_row.clear();
        next_row.push(1);
        next_row.extend(
            row.windows(2)
                .map(|pair| pair[0].checked_add(pair[1]).unwrap()),
        );
        next_row.push(1);
        std::mem::swap(&mut next_row, &mut row);
    }
    row
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn should_compute_correct_pascal_triangle_rows() {
        assert_eq!(compute_pascal_triangle_row(0), vec![1]);
        assert_eq!(compute_pascal_triangle_row(1), vec![1, 1]);
        assert_eq!(compute_pascal_triangle_row(2), vec![1, 2, 1]);
        assert_eq!(
            compute_pascal_triangle_row(7),
            vec![1, 7, 21, 35, 35, 21, 7, 1]
        );
    }

    #[test]
    fn shoud_compute_correct_sample_offsets_and_weights() {
        let samples_per_side = 4;
        let tail_samples_to_truncate = 2;
        let samples = GaussianBlurSamples::new(samples_per_side, tail_samples_to_truncate);
        assert_eq!(samples.sample_count(), 1 + samples_per_side);
        assert_eq!(samples.truncated_tail_samples(), tail_samples_to_truncate);
        assert_abs_diff_eq!(
            samples.sample_offsets().collect::<Vec<_>>().as_slice(),
            &[0.0_f32, 1.0, 2.0, 3.0, 4.0] as _
        );
        assert_abs_diff_eq!(
            samples.sample_weights().collect::<Vec<_>>().as_slice(),
            &[
                0.227_027_03_f32,
                0.194_594_59,
                0.121_621_62,
                0.054_054_055,
                0.016_216_217
            ] as _
        );
    }
}
