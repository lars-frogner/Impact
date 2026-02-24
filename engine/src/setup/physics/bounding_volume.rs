//! Setup of bounding volumes for new entities.

use crate::{lock_order::OrderedRwLock, scene::Scene};
use impact_ecs::{setup, world::PrototypeEntities};
use impact_id::EntityID;
use impact_intersection::bounding_volume::HasBoundingVolume;
use impact_physics::collision::{self, setup::SphericalCollidable};
use parking_lot::RwLock;

pub fn setup_bounding_volumes_for_new_entities(
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) {
    // Ensure that any entity with a spherical collidable has a bounding volume
    // encompassing it, even if it has no mesh or a mesh with a smaller bounding
    // volume.
    setup!(
        {
            let scene = scene.oread();
            let mut intersection_manager = scene.intersection_manager().owrite();
        },
        entities,
        |entity_id: EntityID, spherical_collidable: &SphericalCollidable| -> HasBoundingVolume {
            collision::setup::setup_bounding_volume_for_spherical_collidable(
                &mut intersection_manager.bounding_volume_manager,
                entity_id,
                spherical_collidable,
            );
            HasBoundingVolume
        }
    );
}
