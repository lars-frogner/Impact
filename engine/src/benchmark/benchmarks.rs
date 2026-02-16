//! Benchmarks.

pub mod constraint;
pub mod generation;
pub mod isometry;
pub mod lookup_table;
pub mod matrix;
pub mod model;
pub mod quaternion;
pub mod vector;
pub mod voxel_object;

use std::path::PathBuf;

pub fn benchmark_data_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches/data")
        .join(file_name)
}
