//! Benchmarks for quaternions.

use impact_math::quaternion::{Quaternion, QuaternionC};
use impact_profiling::benchmark::Benchmarker;

pub fn unpack_quaternion(benchmarker: impl Benchmarker) {
    let a = create_quaternionp();
    benchmarker.benchmark(&mut || a.aligned());
}

pub fn pack_quaternion(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    benchmarker.benchmark(&mut || a.compact());
}

pub fn mul_quaternion_uncompact(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    let b = create_quaternion();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_quaternion_both_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_quaternionp();
    let b = create_quaternionp();
    benchmarker.benchmark(&mut || a.aligned() * b.aligned());
}

pub fn mul_quaternion_one_compact_as_uncompact(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    let b = create_quaternionp();
    benchmarker.benchmark(&mut || a * b.aligned());
}

pub fn mul_quaternion_one_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    let b = create_quaternionp();
    benchmarker.benchmark(&mut || (a * b.aligned()).compact());
}

pub fn mul_quaternion_both_compact_as_uncompact_to_compact(benchmarker: impl Benchmarker) {
    let a = create_quaternionp();
    let b = create_quaternionp();
    benchmarker.benchmark(&mut || (a.aligned() * b.aligned()).compact());
}

fn create_quaternion() -> Quaternion {
    Quaternion::from_real(1.0)
}

fn create_quaternionp() -> QuaternionC {
    QuaternionC::from_real(1.0)
}
