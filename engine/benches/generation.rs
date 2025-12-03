use impact::benchmark::benchmarks::generation;
use impact_profiling::{
    benchmark::criterion::{self},
    define_criterion_target,
};

define_criterion_target!(generation, generate_box);
define_criterion_target!(generation, generate_sphere_union);
define_criterion_target!(generation, generate_complex_object);
define_criterion_target!(generation, generate_object_with_multifractal_noise);
define_criterion_target!(generation, generate_object_with_multiscale_spheres);
define_criterion_target!(generation, generate_box_with_gradient_noise_voxel_types);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        generate_box,
        generate_sphere_union,
        generate_complex_object,
        generate_object_with_multifractal_noise,
        generate_object_with_multiscale_spheres,
        generate_box_with_gradient_noise_voxel_types,
);
criterion::criterion_main!(benches);
