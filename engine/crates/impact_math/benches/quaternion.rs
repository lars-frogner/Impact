use impact_math::benchmark::benchmarks::quaternion;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(quaternion, align_quaternion);
define_criterion_target!(quaternion, unalign_quaternion);
define_criterion_target!(quaternion, mul_quaternion_aligned);
define_criterion_target!(quaternion, mul_quaternion_both_unaligned_as_aligned);
define_criterion_target!(quaternion, mul_quaternion_one_unaligned_as_aligned);
define_criterion_target!(
    quaternion,
    mul_quaternion_one_unaligned_as_aligned_to_unaligned
);
define_criterion_target!(
    quaternion,
    mul_quaternion_both_unaligned_as_aligned_to_unaligned
);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        align_quaternion,
        unalign_quaternion,
        mul_quaternion_aligned,
        mul_quaternion_both_unaligned_as_aligned,
        mul_quaternion_one_unaligned_as_aligned,
        mul_quaternion_one_unaligned_as_aligned_to_unaligned,
        mul_quaternion_both_unaligned_as_aligned_to_unaligned,
);
criterion::criterion_main!(benches);
