use crate::matrix::{Matrix3, Matrix3A, Matrix4, Matrix4A};
use impact_profiling::benchmark::Benchmarker;

pub fn align_matrix3(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn unalign_matrix3(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    benchmarker.benchmark(&mut || a.unaligned());
}

pub fn mul_matrix3_aligned(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3a();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_matrix3_both_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || a.aligned() * b.aligned());
}

pub fn mul_matrix3_one_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || a * b.aligned());
}

pub fn mul_matrix3_one_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || (a * b.aligned()).unaligned());
}

pub fn mul_matrix3_both_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || (a.aligned() * b.aligned()).unaligned());
}

pub fn align_matrix4(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn unalign_matrix4(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    benchmarker.benchmark(&mut || a.unaligned());
}

pub fn mul_matrix4_aligned(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4a();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_matrix4_both_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || a.aligned() * b.aligned());
}

pub fn mul_matrix4_one_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || a * b.aligned());
}

pub fn mul_matrix4_one_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || (a * b.aligned()).unaligned());
}

pub fn mul_matrix4_both_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || (a.aligned() * b.aligned()).unaligned());
}

fn create_matrix3() -> Matrix3 {
    Matrix3::identity()
}

fn create_matrix3a() -> Matrix3A {
    Matrix3A::identity()
}

fn create_matrix4() -> Matrix4 {
    Matrix4::identity()
}

fn create_matrix4a() -> Matrix4A {
    Matrix4A::identity()
}
