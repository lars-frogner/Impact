//! Benchmarks for matrices.

use impact_math::matrix::{Matrix3, Matrix3C, Matrix4, Matrix4C};
use impact_profiling::benchmark::Benchmarker;

pub fn unpack_matrix3(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn pack_matrix3(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    benchmarker.benchmark(&mut || a.compact());
}

pub fn mul_matrix3_uncompact(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3a();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_matrix3_both_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || a.aligned() * b.aligned());
}

pub fn mul_matrix3_one_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || a * b.aligned());
}

pub fn mul_matrix3_one_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || (a * b.aligned()).compact());
}

pub fn mul_matrix3_both_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || (a.aligned() * b.aligned()).compact());
}

pub fn unpack_matrix4(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn pack_matrix4(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    benchmarker.benchmark(&mut || a.compact());
}

pub fn mul_matrix4_uncompact(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4a();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_matrix4_both_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || a.aligned() * b.aligned());
}

pub fn mul_matrix4_one_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || a * b.aligned());
}

pub fn mul_matrix4_one_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || (a * b.aligned()).compact());
}

pub fn mul_matrix4_both_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || (a.aligned() * b.aligned()).compact());
}

fn create_matrix3() -> Matrix3C {
    Matrix3C::identity()
}

fn create_matrix3a() -> Matrix3 {
    Matrix3::identity()
}

fn create_matrix4() -> Matrix4C {
    Matrix4C::identity()
}

fn create_matrix4a() -> Matrix4 {
    Matrix4::identity()
}
