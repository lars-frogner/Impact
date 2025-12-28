use crate::matrix::{Matrix3, Matrix3P, Matrix4, Matrix4P};
use impact_profiling::benchmark::Benchmarker;

pub fn unpack_matrix3(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    benchmarker.benchmark(&mut || a.unpack());
}

pub fn pack_matrix3(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    benchmarker.benchmark(&mut || a.pack());
}

pub fn mul_matrix3_unpacked(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3a();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_matrix3_both_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || a.unpack() * b.unpack());
}

pub fn mul_matrix3_one_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || a * b.unpack());
}

pub fn mul_matrix3_one_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_matrix3a();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || (a * b.unpack()).pack());
}

pub fn mul_matrix3_both_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_matrix3();
    let b = create_matrix3();
    benchmarker.benchmark(&mut || (a.unpack() * b.unpack()).pack());
}

pub fn unpack_matrix4(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    benchmarker.benchmark(&mut || a.unpack());
}

pub fn pack_matrix4(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    benchmarker.benchmark(&mut || a.pack());
}

pub fn mul_matrix4_unpacked(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4a();
    benchmarker.benchmark(&mut || a * b);
}

pub fn mul_matrix4_both_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || a.unpack() * b.unpack());
}

pub fn mul_matrix4_one_packed_as_unpacked(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || a * b.unpack());
}

pub fn mul_matrix4_one_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_matrix4a();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || (a * b.unpack()).pack());
}

pub fn mul_matrix4_both_packed_as_unpacked_to_packed(benchmarker: impl Benchmarker) {
    let a = create_matrix4();
    let b = create_matrix4();
    benchmarker.benchmark(&mut || (a.unpack() * b.unpack()).pack());
}

fn create_matrix3() -> Matrix3P {
    Matrix3P::identity()
}

fn create_matrix3a() -> Matrix3 {
    Matrix3::identity()
}

fn create_matrix4() -> Matrix4P {
    Matrix4P::identity()
}

fn create_matrix4a() -> Matrix4 {
    Matrix4::identity()
}
