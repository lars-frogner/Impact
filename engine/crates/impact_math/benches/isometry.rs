use impact_math::benchmark::benchmarks::isometry;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(isometry, align_isometry);
define_criterion_target!(isometry, unalign_isometry);
define_criterion_target!(isometry, mul_isometry_aligned);
define_criterion_target!(isometry, mul_isometry_both_unaligned_as_aligned);
define_criterion_target!(isometry, mul_isometry_one_unaligned_as_aligned);
define_criterion_target!(isometry, mul_isometry_one_unaligned_as_aligned_to_unaligned);
define_criterion_target!(
    isometry,
    mul_isometry_both_unaligned_as_aligned_to_unaligned
);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        align_isometry,
        unalign_isometry,
        mul_isometry_aligned,
        mul_isometry_both_unaligned_as_aligned,
        mul_isometry_one_unaligned_as_aligned,
        mul_isometry_one_unaligned_as_aligned_to_unaligned,
        mul_isometry_both_unaligned_as_aligned_to_unaligned,
);
criterion::criterion_main!(benches);
