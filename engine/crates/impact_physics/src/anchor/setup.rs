//! Anchor setup and cleanup.

#[cfg(feature = "ecs")]
pub fn remove_anchors_for_entity(
    anchor_manager: &parking_lot::RwLock<crate::anchor::AnchorManager>,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    use crate::rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID};

    if let Some(rigid_body_id) = entity.get_component::<DynamicRigidBodyID>() {
        anchor_manager
            .write()
            .dynamic_mut()
            .remove_all_anchors_for_body(*rigid_body_id.access());
    }
    if let Some(rigid_body_id) = entity.get_component::<KinematicRigidBodyID>() {
        anchor_manager
            .write()
            .kinematic_mut()
            .remove_all_anchors_for_body(*rigid_body_id.access());
    }
}
