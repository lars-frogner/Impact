//! Setup of bounding volumes for new entities.

use crate::{lock_order::OrderedRwLock, resource::ResourceManager, scene::Scene};
use anyhow::Result;
use impact_ecs::{setup, world::PrototypeEntities};
use impact_id::EntityID;
use impact_intersection::bounding_volume::{BoundingVolumeID, HasBoundingVolume};
use impact_mesh::TriangleMeshID;
use impact_voxel::HasVoxelObject;
use parking_lot::RwLock;

pub fn setup_bounding_volumes_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let mut intersection_manager = scene.intersection_manager().owrite();
        },
        entities,
        |entity_id: EntityID, mesh_id: &TriangleMeshID| -> Result<HasBoundingVolume> {
            impact_scene::setup::setup_bounding_volume_for_mesh(
                &resource_manager.triangle_meshes,
                &mut intersection_manager.bounding_volume_manager,
                entity_id,
                *mesh_id,
            )?;
            Ok(HasBoundingVolume)
        },
        ![HasBoundingVolume]
    )?;

    setup!(
        {
            let scene = scene.oread();
            let voxel_manager = scene.voxel_manager().oread();
            let mut intersection_manager = scene.intersection_manager().owrite();
        },
        entities,
        |entity_id: EntityID| -> Result<HasBoundingVolume> {
            impact_voxel::setup::setup_bounding_volume_for_voxel_object(
                &voxel_manager.object_manager,
                &mut intersection_manager.bounding_volume_manager,
                entity_id,
            )?;
            Ok(HasBoundingVolume)
        },
        [HasVoxelObject],
        ![HasBoundingVolume]
    )
}

pub fn cleanup_bounding_volume_for_removed_entity(
    scene: &RwLock<Scene>,
    entity_id: EntityID,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    if entity.has_component::<HasBoundingVolume>() {
        let scene = scene.oread();
        let mut intersection_manager = scene.intersection_manager().owrite();
        let bounding_volume_id = BoundingVolumeID::from_entity_id(entity_id);
        intersection_manager
            .bounding_volume_manager
            .remove_bounding_volume(bounding_volume_id);
    }
}
