//! Benchmarks for isometry transforms.

use impact_math::transform::{Isometry3, Isometry3C};
use impact_profiling::benchmark::Benchmarker;

pub fn unpack_isometry(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn pack_isometry(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    benchmarker.benchmark(&mut || a.compact());
}

pub fn mul_isometry_uncompact(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometrya();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_isometry_both_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    let b = create_isometry();
    benchmarker.benchmark(&mut || a.aligned() * b.aligned());
}

pub fn mul_isometry_one_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometry();
    benchmarker.benchmark(&mut || a * b.aligned());
}

pub fn mul_isometry_one_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometry();
    benchmarker.benchmark(&mut || (a * b.aligned()).compact());
}

pub fn mul_isometry_both_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    let b = create_isometry();
    benchmarker.benchmark(&mut || (a.aligned() * b.aligned()).compact());
}

fn create_isometry() -> Isometry3C {
    Isometry3C::identity()
}

fn create_isometrya() -> Isometry3 {
    Isometry3::identity()
}
