//! ECS systems related to voxels.

use crate::{
    physics::{motion::components::ReferenceFrameComp, PhysicsSimulator},
    voxel::{
        components::{VoxelAbsorbingSphereComp, VoxelObjectComp},
        VoxelManager,
    },
};
use impact_ecs::{
    query,
    world::{Entity, World as ECSWorld},
};

/// Applies each voxel-absorbing sphere to the affected voxel objects.
pub fn apply_sphere_absorption(
    simulator: &PhysicsSimulator,
    voxel_manager: &mut VoxelManager,
    ecs_world: &ECSWorld,
) {
    query!(
        ecs_world,
        |entity: Entity,
         voxel_object: &VoxelObjectComp,
         voxel_object_frame: &ReferenceFrameComp| {
            let voxel_object = voxel_manager
                .get_voxel_object_mut(voxel_object.voxel_object_id)
                .expect("Missing voxel object for entity with VoxelObjectComp")
                .object_mut();

            let world_to_voxel_object_transform = voxel_object_frame
                .create_transform_to_parent_space::<f64>()
                .inverse();

            query!(
                ecs_world,
                |absorbing_sphere: &VoxelAbsorbingSphereComp, sphere_frame: &ReferenceFrameComp| {
                    let sphere_to_world_transform =
                        sphere_frame.create_transform_to_parent_space::<f64>();

                    let sphere_in_voxel_object_space = absorbing_sphere
                        .sphere()
                        .transformed(&sphere_to_world_transform)
                        .transformed(&world_to_voxel_object_transform);

                    let inverse_radius_squared =
                        sphere_in_voxel_object_space.radius_squared().recip();

                    let absorption_rate_per_frame =
                        absorbing_sphere.rate() * simulator.time_step_duration();

                    voxel_object.modify_voxels_within_sphere(
                        &sphere_in_voxel_object_space,
                        &mut |_, squared_distance, voxel| {
                            let signed_distance_delta = absorption_rate_per_frame
                                * (1.0 - squared_distance * inverse_radius_squared);
                            voxel.increase_signed_distance(signed_distance_delta as f32);
                        },
                    );
                }
            );

            if voxel_object.invalidated_mesh_chunk_indices().len() > 0 {
                // The object could have become empty
                if voxel_object.is_effectively_empty() {
                    log::debug!("Marked voxel object as empty");
                    voxel_manager.mark_voxel_object_as_empty_for_entity(entity);
                } else {
                    // If the object has not become empty, we must resolve the connected region
                    // information
                    voxel_object.resolve_connected_regions_between_all_chunks();

                    // It could also have been split into multiple disconnected regions
                    if let Some(disconnected_voxel_object) =
                        voxel_object.split_off_any_disconnected_region()
                    {
                        // It could have become empty after splitting
                        if voxel_object.is_effectively_empty() {
                            log::debug!("Marked voxel object as empty");
                            voxel_manager.mark_voxel_object_as_empty_for_entity(entity);
                        }
                        log::debug!("Disconnected voxel object");
                        voxel_manager
                            .push_disconnected_voxel_object(entity, disconnected_voxel_object);
                    }
                }
            }
        }
    );
}
