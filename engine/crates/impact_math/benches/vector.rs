use impact_math::benchmark::benchmarks::vector;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(vector, align_vector3);
define_criterion_target!(vector, unalign_vector3);
define_criterion_target!(vector, add_vector3_unaligned);
define_criterion_target!(vector, add_vector3_aligned);
define_criterion_target!(vector, add_vector3_both_unaligned_as_aligned);
define_criterion_target!(vector, add_vector3_one_unaligned_as_aligned);
define_criterion_target!(vector, add_vector3_one_unaligned_as_aligned_to_unaligned);
define_criterion_target!(vector, add_vector3_both_unaligned_as_aligned_to_unaligned);
define_criterion_target!(vector, add_vector4_unaligned);
define_criterion_target!(vector, add_vector4_aligned);
define_criterion_target!(vector, add_vector4_both_unaligned_as_aligned);
define_criterion_target!(vector, add_vector4_one_unaligned_as_aligned);
define_criterion_target!(vector, add_vector4_one_unaligned_as_aligned_to_unaligned);
define_criterion_target!(vector, add_vector4_both_unaligned_as_aligned_to_unaligned);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
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
);
criterion::criterion_main!(benches);
