use crate::transform::{Isometry3, Isometry3P};
use impact_profiling::benchmark::Benchmarker;

pub fn unpack_isometry(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    benchmarker.benchmark(&mut || a.unpack());
}

pub fn pack_isometry(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    benchmarker.benchmark(&mut || a.pack());
}

pub fn mul_isometry_unpacked(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometrya();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_isometry_both_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    let b = create_isometry();
    benchmarker.benchmark(&mut || a.unpack() * b.unpack());
}

pub fn mul_isometry_one_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometry();
    benchmarker.benchmark(&mut || a * b.unpack());
}

pub fn mul_isometry_one_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_isometrya();
    let b = create_isometry();
    benchmarker.benchmark(&mut || (a * b.unpack()).pack());
}

pub fn mul_isometry_both_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_isometry();
    let b = create_isometry();
    benchmarker.benchmark(&mut || (a.unpack() * b.unpack()).pack());
}

fn create_isometry() -> Isometry3P {
    Isometry3P::identity()
}

fn create_isometrya() -> Isometry3 {
    Isometry3::identity()
}
