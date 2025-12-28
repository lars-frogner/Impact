use crate::quaternion::{Quaternion, QuaternionP};
use impact_profiling::benchmark::Benchmarker;

pub fn unpack_quaternion(benchmarker: impl Benchmarker) {
    let a = create_quaternionp();
    benchmarker.benchmark(&mut || a.unpack());
}

pub fn pack_quaternion(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    benchmarker.benchmark(&mut || a.pack());
}

pub fn mul_quaternion_unpacked(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    let b = create_quaternion();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_quaternion_both_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_quaternionp();
    let b = create_quaternionp();
    benchmarker.benchmark(&mut || a.unpack() * b.unpack());
}

pub fn mul_quaternion_one_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    let b = create_quaternionp();
    benchmarker.benchmark(&mut || a * b.unpack());
}

pub fn mul_quaternion_one_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_quaternion();
    let b = create_quaternionp();
    benchmarker.benchmark(&mut || (a * b.unpack()).pack());
}

pub fn mul_quaternion_both_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_quaternionp();
    let b = create_quaternionp();
    benchmarker.benchmark(&mut || (a.unpack() * b.unpack()).pack());
}

fn create_quaternion() -> Quaternion {
    Quaternion::from_real(1.0)
}

fn create_quaternionp() -> QuaternionP {
    QuaternionP::from_real(1.0)
}
