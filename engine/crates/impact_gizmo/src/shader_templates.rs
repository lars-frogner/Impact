pub mod gizmo;

#[macro_export]
macro_rules! rendering_template_source {
    ($name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/",
            $name,
            ".template.wgsl"
        ))
    }};
}
