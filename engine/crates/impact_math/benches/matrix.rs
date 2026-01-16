use impact_math::benchmark::benchmarks::matrix;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(matrix, unpack_matrix3);
define_criterion_target!(matrix, pack_matrix3);
define_criterion_target!(matrix, mul_matrix3_uncompact);
define_criterion_target!(matrix, mul_matrix3_both_compact_as_uncompact);
define_criterion_target!(matrix, mul_matrix3_one_compact_as_uncompact);
define_criterion_target!(matrix, mul_matrix3_one_compact_as_uncompact_to_compact);
define_criterion_target!(matrix, mul_matrix3_both_compact_as_uncompact_to_compact);
define_criterion_target!(matrix, unpack_matrix4);
define_criterion_target!(matrix, pack_matrix4);
define_criterion_target!(matrix, mul_matrix4_uncompact);
define_criterion_target!(matrix, mul_matrix4_both_compact_as_uncompact);
define_criterion_target!(matrix, mul_matrix4_one_compact_as_uncompact);
define_criterion_target!(matrix, mul_matrix4_one_compact_as_uncompact_to_compact);
define_criterion_target!(matrix, mul_matrix4_both_compact_as_uncompact_to_compact);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
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
);
criterion::criterion_main!(benches);
