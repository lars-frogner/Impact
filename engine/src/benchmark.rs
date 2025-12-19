pub mod benchmarks;

impact_profiling::define_target_enum! {
    Target,
    crate::benchmark::benchmarks,
    generation => {
        generate_box,
        generate_sphere_union,
        generate_complex_object,
        generate_object_with_multifractal_noise,
        generate_object_with_multiscale_spheres,
        generate_box_with_gradient_noise_voxel_types,
        compile_complex_meta_graph,
        build_complex_atomic_graph,
        generate_object_from_complex_graph,
        update_signed_distances_for_block,
    },
    voxel_object => {
        update_internal_adjacencies_for_all_chunks,
        update_connected_regions_for_all_chunks,
        update_all_chunk_boundary_adjacencies,
        resolve_connected_regions_between_all_chunks,
        compute_all_derived_state,
        initialize_inertial_properties,
        create_mesh,
        modify_voxels_within_sphere,
        split_off_disconnected_region,
        split_off_disconnected_region_with_inertial_property_transfer,
        update_mesh,
        obtain_sphere_voxel_object_contacts,
        obtain_plane_voxel_object_contacts,
        obtain_mutual_voxel_object_contacts,
    },
    model => {
        add_feature_to_dynamic_instance_buffer_from_storage,
        add_feature_to_dynamic_instance_buffer_from_storage_repeatedly,
    },
    constraint => {
        prepare_contacts,
        solve_contact_velocities,
        correct_contact_configurations,
    },
    lookup_table => {
        compute_specular_ggx_reflectance,
    }
}

pub fn benchmark(target: Target, duration: f64, delay: f64) {
    impact_profiling::benchmark::benchmark(
        |benchmarker| target.execute(benchmarker),
        duration,
        delay,
    );
}
