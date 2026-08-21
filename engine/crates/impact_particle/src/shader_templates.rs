pub mod basic_simulation;
pub mod particle_geometry;

use crate::gpu_resource::ParticleImmediateGroup;

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

pub trait ParticleSimulationShaderTemplate {
    /// Creates a new particle simulation shader template for the given maximum
    /// number of simulations.
    fn new(max_simulation_count: usize) -> Self;

    /// Returns the workgroup size for the shader (the workgroup is 1D).
    fn workgroup_size() -> u32;

    /// Returns the workgroup counts to use when invoking the compute shader
    /// based on the given number of particles to simulate in that invocation.
    fn determine_workgroup_counts(particle_count: u32) -> [u32; 3];

    /// Returns the group of immediates used by the shader.
    fn immediates() -> ParticleImmediateGroup;
}
