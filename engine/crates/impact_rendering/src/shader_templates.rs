pub mod ambient_light;
pub mod ambient_occlusion_application;
pub mod ambient_occlusion_computation;
pub mod bloom_blending;
pub mod bloom_downsampling;
pub mod bloom_upsampling_blur;
pub mod dynamic_range_compression;
pub mod fixed_color;
pub mod gaussian_blur;
pub mod luminance_histogram;
pub mod luminance_histogram_average;
pub mod model_depth_prepass;
pub mod model_geometry;
pub mod omnidirectional_light;
pub mod omnidirectional_light_shadow_map;
pub mod passthrough;
pub mod render_attachment_visualization;
pub mod shadowable_omnidirectional_light;
pub mod shadowable_unidirectional_light;
pub mod skybox;
pub mod temporal_anti_aliasing;
pub mod unidirectional_light;
pub mod unidirectional_light_shadow_map;

#[macro_export]
macro_rules! compute_template_source {
    ($name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../shaders/compute/",
            $name,
            ".template.wgsl"
        ))
    }};
}

#[macro_export]
macro_rules! rendering_template_source {
    ($name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../shaders/rendering/",
            $name,
            ".template.wgsl"
        ))
    }};
}
