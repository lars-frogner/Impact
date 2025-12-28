use crate::vector::{Vector3, Vector3P, Vector4, Vector4P};
use impact_profiling::benchmark::Benchmarker;

pub fn unpack_vector3(benchmarker: impl Benchmarker) {
    let a = create_vector3p();
    benchmarker.benchmark(&mut || a.unpack());
}

pub fn pack_vector3(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    benchmarker.benchmark(&mut || a.pack());
}

pub fn add_vector3_packed(benchmarker: impl Benchmarker) {
    let a = create_vector3p();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector3_unpacked(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector3_both_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_vector3p();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || a.unpack() + b.unpack());
}

pub fn add_vector3_one_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || a + b.unpack());
}

pub fn add_vector3_one_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_vector3();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || (a + b.unpack()).pack());
}

pub fn add_vector3_both_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_vector3p();
    let b = create_vector3p();
    benchmarker.benchmark(&mut || (a.unpack() + b.unpack()).pack());
}

pub fn unpack_vector4(benchmarker: impl Benchmarker) {
    let a = create_vector4p();
    benchmarker.benchmark(&mut || a.unpack());
}

pub fn pack_vector4(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    benchmarker.benchmark(&mut || a.pack());
}

pub fn add_vector4_packed(benchmarker: impl Benchmarker) {
    let a = create_vector4p();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector4_unpacked(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4();
    benchmarker.benchmark(&mut || a + b);
}

pub fn add_vector4_both_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_vector4p();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || a.unpack() + b.unpack());
}

pub fn add_vector4_one_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || a + b.unpack());
}

pub fn add_vector4_one_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_vector4();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || (a + b.unpack()).pack());
}

pub fn add_vector4_both_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_vector4p();
    let b = create_vector4p();
    benchmarker.benchmark(&mut || (a.unpack() + b.unpack()).pack());
}

fn create_vector3() -> Vector3 {
    Vector3::zeros()
}

fn create_vector3p() -> Vector3P {
    Vector3P::zeros()
}

fn create_vector4() -> Vector4 {
    Vector4::zeros()
}

fn create_vector4p() -> Vector4P {
    Vector4P::zeros()
}
