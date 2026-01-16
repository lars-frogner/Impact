pub mod benchmarks;

impact_profiling::define_target_enum! {
    Target,
    crate::benchmark::benchmarks,
    vector => {
        unpack_vector3,
        pack_vector3,
        add_vector3_compact,
        add_vector3_uncompact,
        add_vector3_both_compact_as_uncompact,
        add_vector3_one_compact_as_uncompact,
        add_vector3_one_compact_as_uncompact_to_compact,
        add_vector3_both_compact_as_uncompact_to_compact,
        unpack_vector4,
        pack_vector4,
        add_vector4_compact,
        add_vector4_uncompact,
        add_vector4_both_compact_as_uncompact,
        add_vector4_one_compact_as_uncompact,
        add_vector4_one_compact_as_uncompact_to_compact,
        add_vector4_both_compact_as_uncompact_to_compact,
    },
    matrix => {
        unpack_matrix3,
        pack_matrix3,
        mul_matrix3_uncompact,
        mul_matrix3_both_compact_as_uncompact,
        mul_matrix3_one_compact_as_uncompact,
        mul_matrix3_one_compact_as_uncompact_to_compact,
        mul_matrix3_both_compact_as_uncompact_to_compact,
        unpack_matrix4,
        pack_matrix4,
        mul_matrix4_uncompact,
        mul_matrix4_both_compact_as_uncompact,
        mul_matrix4_one_compact_as_uncompact,
        mul_matrix4_one_compact_as_uncompact_to_compact,
        mul_matrix4_both_compact_as_uncompact_to_compact,
    },
    quaternion => {
        unpack_quaternion,
        pack_quaternion,
        mul_quaternion_uncompact,
        mul_quaternion_both_compact_as_uncompact,
        mul_quaternion_one_compact_as_uncompact,
        mul_quaternion_one_compact_as_uncompact_to_compact,
        mul_quaternion_both_compact_as_uncompact_to_compact,
    },
    isometry => {
        unpack_isometry,
        pack_isometry,
        mul_isometry_uncompact,
        mul_isometry_both_compact_as_uncompact,
        mul_isometry_one_compact_as_uncompact,
        mul_isometry_one_compact_as_uncompact_to_compact,
        mul_isometry_both_compact_as_uncompact_to_compact,
    }
}

pub fn benchmark(target: Target, duration: f64, delay: f64) {
    impact_profiling::benchmark::benchmark(
        |benchmarker| target.execute(benchmarker),
        duration,
        delay,
    );
}
