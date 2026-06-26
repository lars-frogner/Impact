//! Benchmarks for tesselation.

use impact_alloc::Global;
use impact_geometry::{AxisAlignedBox, AxisAlignedBoxC};
use impact_math::{
    point::{Point3, Point3C},
    random::Rng,
    vector::Vector3C,
};
use impact_profiling::benchmark::Benchmarker;
use impact_tesselation::{delaunay::DelaunayTetrahedralization, voronoi::VoronoiPolyhedron};
use std::hint::black_box;

const N_POINTS_PER_DIM: usize = 5;

pub fn delaunay_tetrahedralize_randomized_grid_points(benchmarker: impl Benchmarker) {
    let points = create_randomized_grid_points(
        N_POINTS_PER_DIM,
        &AxisAlignedBoxC::new(Point3C::origin(), Point3C::same(10.0)),
    );
    benchmarker.benchmark(&mut || DelaunayTetrahedralization::construct(&points).unwrap());
}

pub fn delaunay_tetrahedralize_regular_grid_points(benchmarker: impl Benchmarker) {
    let points = create_regular_grid_points(
        N_POINTS_PER_DIM,
        &AxisAlignedBoxC::new(Point3C::origin(), Point3C::same(10.0)),
    );
    benchmarker.benchmark(&mut || DelaunayTetrahedralization::construct(&points).unwrap());
}

pub fn voronoi_diagram_from_randomized_delaunay_tetrahedralization(benchmarker: impl Benchmarker) {
    let points = create_randomized_grid_points(
        N_POINTS_PER_DIM,
        &AxisAlignedBoxC::new(Point3C::origin(), Point3C::same(10.0)),
    );
    let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

    let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

    benchmarker.benchmark(&mut || {
        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);
        }
    });
}

pub fn voronoi_diagram_from_regular_delaunay_tetrahedralization(benchmarker: impl Benchmarker) {
    let points = create_regular_grid_points(
        N_POINTS_PER_DIM,
        &AxisAlignedBoxC::new(Point3C::origin(), Point3C::same(10.0)),
    );
    let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

    let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

    benchmarker.benchmark(&mut || {
        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);
        }
    });
}

pub fn voronoi_diagram_with_aabbs_from_delaunay_tetrahedralization(benchmarker: impl Benchmarker) {
    let points = create_randomized_grid_points(
        N_POINTS_PER_DIM,
        &AxisAlignedBoxC::new(Point3C::origin(), Point3C::same(10.0)),
    );
    let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

    let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

    let bounding_aabb = AxisAlignedBox::new(
        Point3::new(-100.0, -100.0, -100.0),
        Point3::new(100.0, 100.0, 100.0),
    );

    benchmarker.benchmark(&mut || {
        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);
            black_box(polyhedron.compute_bounded_aabb(&bounding_aabb));
        }
    });
}

pub fn create_regular_grid_points(points_per_dim: usize, aabb: &AxisAlignedBoxC) -> Vec<Point3C> {
    if points_per_dim == 0 {
        return Vec::new();
    }

    let start = aabb.lower_corner();
    let scale = aabb.extents() / ((points_per_dim - 1) as f32);

    let mut points = Vec::new();
    for i in 0..points_per_dim {
        for j in 0..points_per_dim {
            for k in 0..points_per_dim {
                points.push(
                    start + Vector3C::new(i as f32, j as f32, k as f32).component_mul(&scale),
                );
            }
        }
    }
    points
}

pub fn create_randomized_grid_points(
    points_per_dim: usize,
    aabb: &AxisAlignedBoxC,
) -> Vec<Point3C> {
    let mut rng = Rng::with_seed(0);

    let start = aabb.lower_corner();
    let scale = aabb.extents() / (points_per_dim as f32);

    let mut points = Vec::new();
    for i in 0..points_per_dim {
        for j in 0..points_per_dim {
            for k in 0..points_per_dim {
                points.push(
                    start
                        + Vector3C::new(
                            i as f32 + rng.random_f32_fraction(),
                            j as f32 + rng.random_f32_fraction(),
                            k as f32 + rng.random_f32_fraction(),
                        )
                        .component_mul(&scale),
                );
            }
        }
    }
    points
}
