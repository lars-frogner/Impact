//! Benchmarks for lookup tables.

use impact_profiling::benchmark::Benchmarker;
use impact_rendering::brdf;

pub fn compute_specular_ggx_reflectance(benchmarker: impl Benchmarker) {
    benchmarker.benchmark(&mut || brdf::create_specular_ggx_reflectance_lookup_tables(256, 128));
}
