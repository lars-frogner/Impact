pub mod benchmarks;

impact_profiling::define_target_enum! {
    Target,
    crate::benchmark::benchmarks,
    vector => {
        unpack_vector3,
        pack_vector3,
        add_vector3_packed,
        add_vector3_unpacked,
        add_vector3_both_packed_as_unpacked,
        add_vector3_one_packed_as_unpacked,
        add_vector3_one_packed_as_unpacked_to_packed,
        add_vector3_both_packed_as_unpacked_to_packed,
        unpack_vector4,
        pack_vector4,
        add_vector4_packed,
        add_vector4_unpacked,
        add_vector4_both_packed_as_unpacked,
        add_vector4_one_packed_as_unpacked,
        add_vector4_one_packed_as_unpacked_to_packed,
        add_vector4_both_packed_as_unpacked_to_packed,
    },
    matrix => {
        unpack_matrix3,
        pack_matrix3,
        mul_matrix3_unpacked,
        mul_matrix3_both_packed_as_unpacked,
        mul_matrix3_one_packed_as_unpacked,
        mul_matrix3_one_packed_as_unpacked_to_packed,
        mul_matrix3_both_packed_as_unpacked_to_packed,
        unpack_matrix4,
        pack_matrix4,
        mul_matrix4_unpacked,
        mul_matrix4_both_packed_as_unpacked,
        mul_matrix4_one_packed_as_unpacked,
        mul_matrix4_one_packed_as_unpacked_to_packed,
        mul_matrix4_both_packed_as_unpacked_to_packed,
    },
    quaternion => {
        unpack_quaternion,
        pack_quaternion,
        mul_quaternion_unpacked,
        mul_quaternion_both_packed_as_unpacked,
        mul_quaternion_one_packed_as_unpacked,
        mul_quaternion_one_packed_as_unpacked_to_packed,
        mul_quaternion_both_packed_as_unpacked_to_packed,
    },
    isometry => {
        unpack_isometry,
        pack_isometry,
        mul_isometry_unpacked,
        mul_isometry_both_packed_as_unpacked,
        mul_isometry_one_packed_as_unpacked,
        mul_isometry_one_packed_as_unpacked_to_packed,
        mul_isometry_both_packed_as_unpacked_to_packed,
    }
}

pub fn benchmark(target: Target, duration: f64, delay: f64) {
    impact_profiling::benchmark::benchmark(
        |benchmarker| target.execute(benchmarker),
        duration,
        delay,
    );
}
