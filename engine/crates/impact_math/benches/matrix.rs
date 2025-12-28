use impact_math::benchmark::benchmarks::matrix;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(matrix, unpack_matrix3);
define_criterion_target!(matrix, pack_matrix3);
define_criterion_target!(matrix, mul_matrix3_unpacked);
define_criterion_target!(matrix, mul_matrix3_both_packed_as_unpacked);
define_criterion_target!(matrix, mul_matrix3_one_packed_as_unpacked);
define_criterion_target!(matrix, mul_matrix3_one_packed_as_unpacked_to_packed);
define_criterion_target!(matrix, mul_matrix3_both_packed_as_unpacked_to_packed);
define_criterion_target!(matrix, unpack_matrix4);
define_criterion_target!(matrix, pack_matrix4);
define_criterion_target!(matrix, mul_matrix4_unpacked);
define_criterion_target!(matrix, mul_matrix4_both_packed_as_unpacked);
define_criterion_target!(matrix, mul_matrix4_one_packed_as_unpacked);
define_criterion_target!(matrix, mul_matrix4_one_packed_as_unpacked_to_packed);
define_criterion_target!(matrix, mul_matrix4_both_packed_as_unpacked_to_packed);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
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
);
criterion::criterion_main!(benches);
