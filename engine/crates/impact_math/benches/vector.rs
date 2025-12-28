use impact_math::benchmark::benchmarks::vector;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(vector, unpack_vector3);
define_criterion_target!(vector, pack_vector3);
define_criterion_target!(vector, add_vector3_packed);
define_criterion_target!(vector, add_vector3_unpacked);
define_criterion_target!(vector, add_vector3_both_packed_as_unpacked);
define_criterion_target!(vector, add_vector3_one_packed_as_unpacked);
define_criterion_target!(vector, add_vector3_one_packed_as_unpacked_to_packed);
define_criterion_target!(vector, add_vector3_both_packed_as_unpacked_to_packed);
define_criterion_target!(vector, unpack_vector4);
define_criterion_target!(vector, pack_vector4);
define_criterion_target!(vector, add_vector4_packed);
define_criterion_target!(vector, add_vector4_unpacked);
define_criterion_target!(vector, add_vector4_both_packed_as_unpacked);
define_criterion_target!(vector, add_vector4_one_packed_as_unpacked);
define_criterion_target!(vector, add_vector4_one_packed_as_unpacked_to_packed);
define_criterion_target!(vector, add_vector4_both_packed_as_unpacked_to_packed);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
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
);
criterion::criterion_main!(benches);
