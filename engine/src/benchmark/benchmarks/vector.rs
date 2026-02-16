//! Benchmarks for vectors.

use impact_math::vector::{Vector3, Vector3C, Vector4, Vector4C};
use impact_profiling::benchmark::Benchmarker;

pub fn unpack_vector3(benchmarker: impl Benchmarker) {
    let a = create_vector3p();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn pack_vector3(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    benchmarker.benchmark(&mut || a.compact());
}

pub fn add_vector3_compact(benchmarker: impl Benchmarker) {
    let a = create_vector3p();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector3_uncompact(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector3_both_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_vector3p();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || a.aligned() + b.aligned());
}

pub fn add_vector3_one_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || a + b.aligned());
}

pub fn add_vector3_one_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || (a + b.aligned()).compact());
}

pub fn add_vector3_both_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_vector3p();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || (a.aligned() + b.aligned()).compact());
}

pub fn unpack_vector4(benchmarker: impl Benchmarker) {
    let a = create_vector4p();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn pack_vector4(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    benchmarker.benchmark(&mut || a.compact());
}

pub fn add_vector4_compact(benchmarker: impl Benchmarker) {
    let a = create_vector4p();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector4_uncompact(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector4_both_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_vector4p();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || a.aligned() + b.aligned());
}

pub fn add_vector4_one_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || a + b.aligned());
}

pub fn add_vector4_one_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || (a + b.aligned()).compact());
}

pub fn add_vector4_both_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_vector4p();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || (a.aligned() + b.aligned()).compact());
}

fn create_vector3() -> Vector3 {
    Vector3::zeros()
}

fn create_vector3p() -> Vector3C {
    Vector3C::zeros()
}

fn create_vector4() -> Vector4 {
    Vector4::zeros()
}

fn create_vector4p() -> Vector4C {
    Vector4C::zeros()
}
