//! ECS systems related to motion and orientation control.

use crate::{
    MotionController, OrientationController,
    motion::VelocityControl,
    orientation::{AngularVelocityControl, AngularVelocityControlParent},
};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ReferenceFrame;
use impact_id::EntityID;
use impact_physics::{
    quantities::{AngularVelocity, Motion},
    rigid_body::{
        DynamicRigidBodyID, HasDynamicRigidBody, HasKinematicRigidBody, KinematicRigidBodyID,
        RigidBodyManager,
    },
};

/// Updates the world-space velocities of all entities controlled by the given
/// motion controller.
pub fn update_controlled_entity_velocities(
    ecs_world: &ECSWorld,
    rigid_body_manager: &mut RigidBodyManager,
    motion_controller: &(impl MotionController + ?Sized),
) {
    query!(
        ecs_world,
        |entity_id: EntityID, motion: &mut Motion, frame: &ReferenceFrame| {
            let orientation = frame.orientation.aligned();

            let new_velocity = motion_controller.compute_controlled_velocity(&orientation);

            motion.linear_velocity = new_velocity.compact();

            let rigid_body_id = KinematicRigidBodyID::from_entity_id(entity_id);
            if let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(rigid_body_id)
            {
                rigid_body.set_velocity(motion.linear_velocity);
            }
        },
        [VelocityControl, HasKinematicRigidBody]
    );

    query!(
        ecs_world,
        |entity_id: EntityID, motion: &mut Motion, frame: &ReferenceFrame| {
            let orientation = frame.orientation.aligned();

            let new_velocity = motion_controller.compute_controlled_velocity(&orientation);

            motion.linear_velocity = new_velocity.compact();

            let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
            if let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(rigid_body_id) {
                rigid_body.synchronize_momentum(&new_velocity);
            }
        },
        [VelocityControl, HasDynamicRigidBody]
    );
}

/// Updates the angular velocities of all entities controlled by the given
/// orientation controller.
pub fn update_controlled_entity_angular_velocities(
    ecs_world: &ECSWorld,
    rigid_body_manager: &mut RigidBodyManager,
    orientation_controller: &mut (impl OrientationController + ?Sized),
    time_step_duration: f32,
) {
    update_control_frame_orientations_from_parents(ecs_world);

    query!(
        ecs_world,
        |entity_id: EntityID,
         control: &AngularVelocityControl,
         frame: &ReferenceFrame,
         motion: &mut Motion| {
            let mut new_angular_velocity = motion.angular_velocity.aligned();

            if orientation_controller.orientation_has_changed() {
                let old_orientation = frame.orientation.aligned();
                let mut new_orientation = old_orientation;

                control.update_orientation(orientation_controller, &mut new_orientation);

                let new_controlled_angular_velocity =
                    AngularVelocity::from_consecutive_orientations(
                        &old_orientation,
                        &new_orientation,
                        time_step_duration,
                    );

                control.update_total_angular_velocity(
                    new_controlled_angular_velocity,
                    &mut new_angular_velocity,
                );
            } else {
                control
                    .update_total_angular_velocity_for_unchanged_control(&mut new_angular_velocity);
            }

            motion.angular_velocity = new_angular_velocity.compact();

            let rigid_body_id = KinematicRigidBodyID::from_entity_id(entity_id);
            if let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(rigid_body_id)
            {
                rigid_body.set_angular_velocity(motion.angular_velocity);
            }
        },
        [HasKinematicRigidBody]
    );

    query!(
        ecs_world,
        |entity_id: EntityID,
         control: &AngularVelocityControl,
         frame: &ReferenceFrame,
         motion: &mut Motion| {
            let mut new_angular_velocity = motion.angular_velocity.aligned();

            if orientation_controller.orientation_has_changed() {
                let old_orientation = frame.orientation.aligned();
                let mut new_orientation = old_orientation;

                control.update_orientation(orientation_controller, &mut new_orientation);

                let new_controlled_angular_velocity =
                    AngularVelocity::from_consecutive_orientations(
                        &old_orientation,
                        &new_orientation,
                        time_step_duration,
                    );

                control.update_total_angular_velocity(
                    new_controlled_angular_velocity,
                    &mut new_angular_velocity,
                );
            } else {
                control
                    .update_total_angular_velocity_for_unchanged_control(&mut new_angular_velocity);
            }

            motion.angular_velocity = new_angular_velocity.compact();

            let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
            if let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(rigid_body_id) {
                rigid_body.synchronize_angular_momentum(&new_angular_velocity);
            }
        },
        [HasDynamicRigidBody]
    );

    orientation_controller.reset_orientation_change();
}

fn update_control_frame_orientations_from_parents(ecs_world: &ECSWorld) {
    let arena = ArenaPool::get_arena();
    let mut parent_orientations = AVec::new_in(&arena);

    query!(
        ecs_world,
        |entity_id: EntityID, parent: &AngularVelocityControlParent| {
            if let Some(entity) = ecs_world.get_entity(parent.entity_id)
                && let Some(frame) = entity.get_component::<ReferenceFrame>()
            {
                let parent_orientation = frame.access().orientation;
                parent_orientations.push((entity_id, parent_orientation));
            }
        },
        [AngularVelocityControl]
    );

    for (entity_id, parent_orientation) in parent_orientations {
        let entity = ecs_world.entity(entity_id);
        let mut control = entity.component_mut::<AngularVelocityControl>();
        control.access().frame_orientation = parent_orientation;
    }
}
