pub mod luminance_histogram;
pub mod luminance_histogram_average;
pub mod voxel_chunk_culling;

#[macro_export]
macro_rules! compute_template_source {
    ($name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compute/",
            $name,
            ".template.wgsl"
        ))
    }};
}
