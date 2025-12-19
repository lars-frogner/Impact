use impact::benchmark::benchmarks::voxel_object;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(voxel_object, update_internal_adjacencies_for_all_chunks);
define_criterion_target!(voxel_object, update_connected_regions_for_all_chunks);
define_criterion_target!(voxel_object, update_all_chunk_boundary_adjacencies);
define_criterion_target!(voxel_object, resolve_connected_regions_between_all_chunks);
define_criterion_target!(voxel_object, update_occupied_voxel_ranges);
define_criterion_target!(voxel_object, compute_all_derived_state);
define_criterion_target!(voxel_object, initialize_inertial_properties);
define_criterion_target!(voxel_object, clone_object);
define_criterion_target!(voxel_object, create_mesh);
define_criterion_target!(voxel_object, get_each_voxel);
define_criterion_target!(
    voxel_object,
    obtain_surface_voxels_within_negative_halfspace_of_plane
);
define_criterion_target!(voxel_object, obtain_surface_voxels_within_sphere);
define_criterion_target!(voxel_object, for_each_exposed_chunk_with_sdf);
define_criterion_target!(voxel_object, modify_voxels_within_sphere);
define_criterion_target!(voxel_object, split_off_disconnected_region);
define_criterion_target!(
    voxel_object,
    split_off_disconnected_region_with_inertial_property_transfer
);
define_criterion_target!(voxel_object, update_mesh);
define_criterion_target!(voxel_object, obtain_sphere_voxel_object_contacts);
define_criterion_target!(voxel_object, obtain_plane_voxel_object_contacts);
define_criterion_target!(voxel_object, obtain_mutual_voxel_object_contacts);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        clone_object,
        update_internal_adjacencies_for_all_chunks,
        update_connected_regions_for_all_chunks,
        update_all_chunk_boundary_adjacencies,
        resolve_connected_regions_between_all_chunks,
        update_occupied_voxel_ranges,
        compute_all_derived_state,
        initialize_inertial_properties,
        for_each_exposed_chunk_with_sdf,
        clone_object,
        create_mesh,
        get_each_voxel,
        obtain_surface_voxels_within_negative_halfspace_of_plane,
        obtain_surface_voxels_within_sphere,
        for_each_exposed_chunk_with_sdf,
        modify_voxels_within_sphere,
        split_off_disconnected_region,
        split_off_disconnected_region_with_inertial_property_transfer,
        update_mesh,
        obtain_sphere_voxel_object_contacts,
        obtain_plane_voxel_object_contacts,
        obtain_mutual_voxel_object_contacts,
);
criterion::criterion_main!(benches);
