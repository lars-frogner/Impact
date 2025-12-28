pub mod benchmarks;

impact_profiling::define_target_enum! {
    Target,
    crate::benchmark::benchmarks,
    vector => {
        align_vector3,
        unalign_vector3,
        add_vector3_unaligned,
        add_vector3_aligned,
        add_vector3_both_unaligned_as_aligned,
        add_vector3_one_unaligned_as_aligned,
        add_vector3_one_unaligned_as_aligned_to_unaligned,
        add_vector3_both_unaligned_as_aligned_to_unaligned,
        add_vector4_unaligned,
        add_vector4_aligned,
        add_vector4_both_unaligned_as_aligned,
        add_vector4_one_unaligned_as_aligned,
        add_vector4_one_unaligned_as_aligned_to_unaligned,
        add_vector4_both_unaligned_as_aligned_to_unaligned,
    },
    matrix => {
        align_matrix3,
        unalign_matrix3,
        mul_matrix3_aligned,
        mul_matrix3_both_unaligned_as_aligned,
        mul_matrix3_one_unaligned_as_aligned,
        mul_matrix3_one_unaligned_as_aligned_to_unaligned,
        mul_matrix3_both_unaligned_as_aligned_to_unaligned,
        align_matrix4,
        unalign_matrix4,
        mul_matrix4_aligned,
        mul_matrix4_both_unaligned_as_aligned,
        mul_matrix4_one_unaligned_as_aligned,
        mul_matrix4_one_unaligned_as_aligned_to_unaligned,
        mul_matrix4_both_unaligned_as_aligned_to_unaligned,
    },
    quaternion => {
        align_quaternion,
        unalign_quaternion,
        mul_quaternion_aligned,
        mul_quaternion_both_unaligned_as_aligned,
        mul_quaternion_one_unaligned_as_aligned,
        mul_quaternion_one_unaligned_as_aligned_to_unaligned,
        mul_quaternion_both_unaligned_as_aligned_to_unaligned,
    },
    isometry => {
        align_isometry,
        unalign_isometry,
        mul_isometry_aligned,
        mul_isometry_both_unaligned_as_aligned,
        mul_isometry_one_unaligned_as_aligned,
        mul_isometry_one_unaligned_as_aligned_to_unaligned,
        mul_isometry_both_unaligned_as_aligned_to_unaligned,
    }
}

pub fn benchmark(target: Target, duration: f64, delay: f64) {
    impact_profiling::benchmark::benchmark(
        |benchmarker| target.execute(benchmarker),
        duration,
        delay,
    );
}
