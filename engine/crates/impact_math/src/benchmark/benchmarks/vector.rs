use crate::vector::{Vector3, Vector3A, Vector4, Vector4A};
use impact_profiling::benchmark::Benchmarker;

pub fn align_vector3(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn unalign_vector3(benchmarker: impl Benchmarker) {
    let a = create_vector3a();
    benchmarker.benchmark(&mut || a.unaligned());
}

pub fn add_vector3_unaligned(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector3_aligned(benchmarker: impl Benchmarker) {
    let a = create_vector3a();
    let b = create_vector3a();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector3_both_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3();
    benchmarker.benchmark(&mut || a.aligned() + b.aligned());
}

pub fn add_vector3_one_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_vector3a();
    let b = create_vector3();
    benchmarker.benchmark(&mut || a + b.aligned());
}

pub fn add_vector3_one_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_vector3a();
    let b = create_vector3();
    benchmarker.benchmark(&mut || (a + b.aligned()).unaligned());
}

pub fn add_vector3_both_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3();
    benchmarker.benchmark(&mut || (a.aligned() + b.aligned()).unaligned());
}

pub fn add_vector4_unaligned(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector4_aligned(benchmarker: impl Benchmarker) {
    let a = create_vector4a();
    let b = create_vector4a();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector4_both_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4();
    benchmarker.benchmark(&mut || a.aligned() + b.aligned());
}

pub fn add_vector4_one_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_vector4a();
    let b = create_vector4();
    benchmarker.benchmark(&mut || a + b.aligned());
}

pub fn add_vector4_one_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_vector4a();
    let b = create_vector4();
    benchmarker.benchmark(&mut || (a + b.aligned()).unaligned());
}

pub fn add_vector4_both_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4();
    benchmarker.benchmark(&mut || (a.aligned() + b.aligned()).unaligned());
}

fn create_vector3() -> Vector3 {
    Vector3::zeros()
}

fn create_vector3a() -> Vector3A {
    Vector3A::zeros()
}

fn create_vector4() -> Vector4 {
    Vector4::zeros()
}

fn create_vector4a() -> Vector4A {
    Vector4A::zeros()
}
