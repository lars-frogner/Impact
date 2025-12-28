use impact_math::benchmark::benchmarks::matrix;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(matrix, align_matrix3);
define_criterion_target!(matrix, unalign_matrix3);
define_criterion_target!(matrix, mul_matrix3_aligned);
define_criterion_target!(matrix, mul_matrix3_both_unaligned_as_aligned);
define_criterion_target!(matrix, mul_matrix3_one_unaligned_as_aligned);
define_criterion_target!(matrix, mul_matrix3_one_unaligned_as_aligned_to_unaligned);
define_criterion_target!(matrix, mul_matrix3_both_unaligned_as_aligned_to_unaligned);
define_criterion_target!(matrix, align_matrix4);
define_criterion_target!(matrix, unalign_matrix4);
define_criterion_target!(matrix, mul_matrix4_aligned);
define_criterion_target!(matrix, mul_matrix4_both_unaligned_as_aligned);
define_criterion_target!(matrix, mul_matrix4_one_unaligned_as_aligned);
define_criterion_target!(matrix, mul_matrix4_one_unaligned_as_aligned_to_unaligned);
define_criterion_target!(matrix, mul_matrix4_both_unaligned_as_aligned_to_unaligned);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
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
);
criterion::criterion_main!(benches);
