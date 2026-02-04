//! Benchmarks for lookup tables.

use impact_light::photometry;
use impact_profiling::benchmark::Benchmarker;
use impact_rendering::brdf;

pub fn compute_specular_ggx_reflectance(benchmarker: impl Benchmarker) {
    benchmarker.benchmark(&mut || brdf::create_specular_ggx_reflectance_lookup_tables(256, 128));
}

pub fn compute_black_body_luminance(benchmarker: impl Benchmarker) {
    benchmarker.benchmark(&mut || photometry::compute_black_body_luminance(5800.0));
}
