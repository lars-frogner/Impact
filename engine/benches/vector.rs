use impact::benchmark::benchmarks::vector;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(vector, unpack_vector3);
define_criterion_target!(vector, pack_vector3);
define_criterion_target!(vector, add_vector3_compact);
define_criterion_target!(vector, add_vector3_uncompact);
define_criterion_target!(vector, add_vector3_both_compact_as_uncompact);
define_criterion_target!(vector, add_vector3_one_compact_as_uncompact);
define_criterion_target!(vector, add_vector3_one_compact_as_uncompact_to_compact);
define_criterion_target!(vector, add_vector3_both_compact_as_uncompact_to_compact);
define_criterion_target!(vector, unpack_vector4);
define_criterion_target!(vector, pack_vector4);
define_criterion_target!(vector, add_vector4_compact);
define_criterion_target!(vector, add_vector4_uncompact);
define_criterion_target!(vector, add_vector4_both_compact_as_uncompact);
define_criterion_target!(vector, add_vector4_one_compact_as_uncompact);
define_criterion_target!(vector, add_vector4_one_compact_as_uncompact_to_compact);
define_criterion_target!(vector, add_vector4_both_compact_as_uncompact_to_compact);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
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
);
criterion::criterion_main!(benches);
