use crate::quaternion::{Quaternion, QuaternionA};
use impact_profiling::benchmark::Benchmarker;

pub fn align_quaternion(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn unalign_quaternion(benchmarker: impl Benchmarker) {
    let a = create_quaterniona();
    benchmarker.benchmark(&mut || a.unaligned());
}

pub fn mul_quaternion_aligned(benchmarker: impl Benchmarker) {
    let a = create_quaterniona();
    let b = create_quaterniona();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_quaternion_both_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    let b = create_quaternion();
    benchmarker.benchmark(&mut || a.aligned() * b.aligned());
}

pub fn mul_quaternion_one_unaligned_as_aligned(benchmarker: impl Benchmarker) {
    let a = create_quaterniona();
    let b = create_quaternion();
    benchmarker.benchmark(&mut || a * b.aligned());
}

pub fn mul_quaternion_one_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_quaterniona();
    let b = create_quaternion();
    benchmarker.benchmark(&mut || (a * b.aligned()).unaligned());
}

pub fn mul_quaternion_both_unaligned_as_aligned_to_unaligned(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    let b = create_quaternion();
    benchmarker.benchmark(&mut || (a.aligned() * b.aligned()).unaligned());
}

fn create_quaternion() -> Quaternion {
    Quaternion::from_real(1.0)
}

fn create_quaterniona() -> QuaternionA {
    QuaternionA::from_real(1.0)
}
