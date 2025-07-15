pub mod voxel_chunk_culling;
pub mod voxel_geometry;

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

#[macro_export]
macro_rules! rendering_template_source {
    ($name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/rendering/",
            $name,
            ".template.wgsl"
        ))
    }};
}
