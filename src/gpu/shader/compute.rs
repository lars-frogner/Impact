//! Compute shaders.

use crate::gpu::shader::template::ShaderTemplate;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref LUMINANCE_HISTOGRAM_SHADER_TEMPLATE: ShaderTemplate<'static> =
        ShaderTemplate::new(include_str!(
            "../../../shader/compute/luminance_histogram.template.wgsl"
        ));
}
