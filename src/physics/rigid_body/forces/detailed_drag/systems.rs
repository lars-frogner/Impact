//! ECS systems related to the detailed drag model.

use super::DragLoadMapRepository;
use crate::{
    physics::{
        fph,
        medium::UniformMedium,
        motion::{
            components::{ReferenceFrameComp, Static, VelocityComp},
            Direction,
        },
        rigid_body::{
            components::RigidBodyComp, forces::detailed_drag::components::DragLoadMapComp,
        },
    },
    scene::components::SceneEntityFlagsComp,
};
use impact_ecs::{query, world::World as ECSWorld};

/// Applies the drag force and torque calculated from precomputed detailed
/// [`DragLoad`](super::DragLoad)s to all applicable entities with a
/// [`DragLoadMapComp`].
pub fn apply_detailed_drag(
    ecs_world: &ECSWorld,
    drag_load_map_repository: &DragLoadMapRepository<f32>,
    medium: &UniformMedium,
) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp,
         frame: &ReferenceFrameComp,
         velocity: &VelocityComp,
         drag: &DragLoadMapComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
            apply_detailed_drag_for_entity(
                drag_load_map_repository,
                medium,
                rigid_body,
                frame,
                velocity,
                drag,
            );
        },
        ![Static]
    );
}

fn apply_detailed_drag_for_entity(
    drag_load_map_repository: &DragLoadMapRepository<f32>,
    medium: &UniformMedium,
    rigid_body: &mut RigidBodyComp,
    frame: &ReferenceFrameComp,
    velocity: &VelocityComp,
    drag: &DragLoadMapComp,
) {
    let velocity_relative_to_medium = velocity.linear - medium.velocity;
    let squared_body_speed_relative_to_medium = velocity_relative_to_medium.norm_squared();

    if squared_body_speed_relative_to_medium > 0.0 {
        let body_space_velocity_relative_to_medium = frame
            .orientation
            .inverse_transform_vector(&velocity_relative_to_medium);

        let body_space_direction_of_motion_relative_to_medium = Direction::new_unchecked(
            body_space_velocity_relative_to_medium
                / fph::sqrt(squared_body_speed_relative_to_medium),
        );

        let phi = super::compute_phi(&body_space_direction_of_motion_relative_to_medium);
        let theta = super::compute_theta(&body_space_direction_of_motion_relative_to_medium);

        let drag_load_map = drag_load_map_repository.drag_load_map(drag.mesh_id);

        let drag_load = drag_load_map.value(phi, theta);

        let (force, torque) = drag_load.compute_world_space_drag_force_and_torque(
            frame.scaling,
            medium.mass_density,
            drag.drag_coefficient,
            &frame.orientation,
            squared_body_speed_relative_to_medium,
        );

        rigid_body.0.apply_force_at_center_of_mass(&force);
        rigid_body.0.apply_torque(&torque);
    }
}
