use crate::transform::{Isometry3, Isometry3A};
use impact_profiling::benchmark::Benchmarker;

pub fn align_isometry(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn unalign_isometry(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    benchmarker.benchmark(&mut || a.unaligned());
}

pub fn mul_isometry_aligned(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometrya();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_isometry_both_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    let b = create_isometry();
    benchmarker.benchmark(&mut || a.aligned() * b.aligned());
}

pub fn mul_isometry_one_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometry();
    benchmarker.benchmark(&mut || a * b.aligned());
}

pub fn mul_isometry_one_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometry();
    benchmarker.benchmark(&mut || (a * b.aligned()).unaligned());
}

pub fn mul_isometry_both_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    let b = create_isometry();
    benchmarker.benchmark(&mut || (a.aligned() * b.aligned()).unaligned());
}

fn create_isometry() -> Isometry3 {
    Isometry3::identity()
}

fn create_isometrya() -> Isometry3A {
    Isometry3A::identity()
}
