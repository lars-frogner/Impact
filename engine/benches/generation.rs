use impact::benchmark::benchmarks::generation;
use impact_profiling::{
    benchmark::criterion::{self},
    define_criterion_target,
};

define_criterion_target!(generation, generate_box);
define_criterion_target!(generation, generate_sphere_union);
define_criterion_target!(generation, generate_complex_object);
define_criterion_target!(generation, generate_object_with_multifractal_noise);
define_criterion_target!(generation, generate_box_with_gradient_noise_voxel_types);
define_criterion_target!(generation, compile_complex_meta_graph);
define_criterion_target!(generation, build_complex_atomic_graph);
define_criterion_target!(generation, generate_object_from_complex_graph, 10);
define_criterion_target!(generation, update_signed_distances_for_block);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        generate_box,
        generate_sphere_union,
        generate_complex_object,
        generate_object_with_multifractal_noise,
        generate_box_with_gradient_noise_voxel_types,
        compile_complex_meta_graph,
        build_complex_atomic_graph,
        generate_object_from_complex_graph,
        update_signed_distances_for_block,
);
criterion::criterion_main!(benches);
