//! Physics setup.

#[cfg(feature = "ecs")]
impl<C: crate::collision::Collidable> super::PhysicsSimulator<C> {
    /// Performs any modifications required to clean up the physics simulator
    /// when the given entity is removed.
    pub fn perform_cleanup_for_removed_entity(&self, entity: &impact_ecs::world::EntityEntry<'_>) {
        crate::collision::setup::remove_collidable_for_entity(&self.collision_world, entity);

        crate::driven_motion::setup::remove_motion_drivers_for_entity(
            &self.motion_driver_manager,
            entity,
        );

        crate::force::setup::remove_force_generators_for_entity(
            &self.force_generator_manager,
            entity,
        );

        crate::anchor::setup::remove_anchors_for_entity(&self.anchor_manager, entity);

        crate::rigid_body::setup::remove_rigid_body_for_entity(&self.rigid_body_manager, entity);
    }
}
