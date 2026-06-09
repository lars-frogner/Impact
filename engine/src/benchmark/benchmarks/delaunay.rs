//! Benchmarks for Delaunay tetrahedralization.

use impact_math::{point::Point3C, random::Rng};
use impact_profiling::benchmark::Benchmarker;
use impact_tesselation::delaunay::DelaunayTetrahedralization;

const N_POINTS_PER_DIM: usize = 5;

pub fn construct_from_randomized_grid_points(benchmarker: impl Benchmarker) {
    let mut rng = Rng::with_seed(0);
    let mut points = Vec::new();
    for i in 0..N_POINTS_PER_DIM {
        for j in 0..N_POINTS_PER_DIM {
            for k in 0..N_POINTS_PER_DIM {
                points.push(Point3C::new(
                    i as f32 + (rng.random_f32_fraction() - 0.5),
                    j as f32 + (rng.random_f32_fraction() - 0.5),
                    k as f32 + (rng.random_f32_fraction() - 0.5),
                ));
            }
        }
    }

    benchmarker.benchmark(&mut || DelaunayTetrahedralization::construct(&points).unwrap());
}

pub fn construct_from_regular_grid_points(benchmarker: impl Benchmarker) {
    let mut points = Vec::new();
    for i in 0..N_POINTS_PER_DIM {
        for j in 0..N_POINTS_PER_DIM {
            for k in 0..N_POINTS_PER_DIM {
                points.push(Point3C::new(i as f32, j as f32, k as f32));
            }
        }
    }

    benchmarker.benchmark(&mut || DelaunayTetrahedralization::construct(&points).unwrap());
}
