//! ECS systems related to spring forces.

use crate::{
    control::{
        motion::components::MotionControlComp, orientation::components::OrientationControlComp,
    },
    physics::{
        fph,
        motion::{
            AngularVelocity, Position, Velocity,
            components::{ReferenceFrameComp, Static, VelocityComp},
        },
        rigid_body::{components::RigidBodyComp, forces::spring::components::SpringComp},
    },
    scene::components::SceneEntityFlagsComp,
};
use approx::abs_diff_eq;
use impact_ecs::{
    query,
    world::{EntityEntry, EntityID, World as ECSWorld},
};
use nalgebra::UnitVector3;

/// The outcome of applying the forces from a spring.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SpringForceApplicationOutcome {
    Ok,
    EntityMissing,
}

/// Applies spring forces to all applicable entities with a [`SpringComp`].
pub fn apply_spring_forces(ecs_world: &ECSWorld, entities_to_remove: &mut Vec<EntityID>) {
    query!(ecs_world, |entity_id: EntityID, spring: &mut SpringComp| {
        let outcome = apply_forces(spring, ecs_world);
        if outcome == SpringForceApplicationOutcome::EntityMissing {
            entities_to_remove.push(entity_id);
        }
    });
}

pub fn synchronize_spring_positions_and_orientations(ecs_world: &ECSWorld) {
    query!(
        ecs_world,
        |frame: &mut ReferenceFrameComp, spring: &SpringComp| {
            frame.position = *spring.spring_state.center();
            frame.orientation = spring.spring_state.compute_orientation();
        },
        ![
            Static,
            OrientationControlComp,
            MotionControlComp,
            VelocityComp
        ]
    );
}

fn apply_forces(spring: &mut SpringComp, ecs_world: &ECSWorld) -> SpringForceApplicationOutcome {
    let (entity_1, entity_2) = if let (Some(entity_1_id), Some(entity_2_id)) = (
        ecs_world.get_entity(spring.entity_1_id),
        ecs_world.get_entity(spring.entity_2_id),
    ) {
        (entity_1_id, entity_2_id)
    } else {
        log::debug!("Missing spring attachment entity: spring component will be removed");
        return SpringForceApplicationOutcome::EntityMissing;
    };

    let entity_1_is_disabled = entity_1
        .get_component::<SceneEntityFlagsComp>()
        .is_some_and(|comp| comp.access().is_disabled());

    let entity_2_is_disabled = entity_2
        .get_component::<SceneEntityFlagsComp>()
        .is_some_and(|comp| comp.access().is_disabled());

    if entity_1_is_disabled || entity_2_is_disabled {
        // We need both entities in order to apply a force
        return SpringForceApplicationOutcome::Ok;
    }

    let entity_1_is_static = entity_1.has_component::<Static>();
    let entity_2_is_static = entity_2.has_component::<Static>();

    if (!entity_1.has_component::<RigidBodyComp>() && !entity_2.has_component::<RigidBodyComp>())
        || (entity_1_is_static && entity_2_is_static)
    {
        // Nothing to apply the force to
        return SpringForceApplicationOutcome::Ok;
    }

    let frame_1 = determine_reference_frame(&entity_1);
    let frame_2 = determine_reference_frame(&entity_2);

    let attachment_point_1 =
        compute_attachment_point_in_world_space(&spring.attachment_point_1, &frame_1);

    let attachment_point_2 =
        compute_attachment_point_in_world_space(&spring.attachment_point_2, &frame_2);

    if let Some((spring_direction, length)) =
        UnitVector3::try_new_and_get(attachment_point_2 - attachment_point_1, fph::EPSILON)
    {
        spring
            .spring_state
            .update(&attachment_point_1, spring_direction, length);

        let rate_of_length_change = if abs_diff_eq!(spring.spring.damping, 0.0) {
            // The velocities are irrelevant if there is zero damping
            0.0
        } else {
            let attachment_velocity_1 =
                determine_attachment_velocity(&entity_1, &frame_1.position, &attachment_point_1);
            let attachment_velocity_2 =
                determine_attachment_velocity(&entity_2, &frame_2.position, &attachment_point_2);

            attachment_velocity_2.dot(&spring_direction)
                - attachment_velocity_1.dot(&spring_direction)
        };

        let force_on_2 =
            spring.spring.scalar_force(length, rate_of_length_change) * spring_direction.as_ref();

        if !entity_1_is_static {
            if let Some(mut rigid_body_1) = entity_1.get_component_mut::<RigidBodyComp>() {
                rigid_body_1.access().0.apply_force(
                    &frame_1.position,
                    &(-force_on_2),
                    &attachment_point_1,
                );

                // To prevent a potential deadlock, the entry to the
                // `RigidBodyComp` storage for entity 1 must have been dropped
                // before trying to access the storage for entity 2
                drop(rigid_body_1);
            }
        }
        if !entity_2_is_static {
            if let Some(mut rigid_body_2) = entity_2.get_component_mut::<RigidBodyComp>() {
                rigid_body_2.access().0.apply_force(
                    &frame_2.position,
                    &force_on_2,
                    &attachment_point_2,
                );
            }
        }
    } else {
        spring
            .spring_state
            .update_with_zero_length(attachment_point_1);
    }

    SpringForceApplicationOutcome::Ok
}

fn determine_reference_frame(entity: &EntityEntry<'_>) -> ReferenceFrameComp {
    entity
        .get_component::<ReferenceFrameComp>()
        .map_or_else(ReferenceFrameComp::default, |frame| *frame.access())
}

fn compute_attachment_point_in_world_space(
    attachment_point_in_entity_frame: &Position,
    frame: &ReferenceFrameComp,
) -> Position {
    frame.position
        + frame
            .orientation
            .transform_vector(&attachment_point_in_entity_frame.coords)
}

fn determine_attachment_velocity(
    entity: &EntityEntry<'_>,
    position: &Position,
    attachment_point: &Position,
) -> Velocity {
    if let Some(velocity) = entity.get_component::<VelocityComp>() {
        let velocity = velocity.access();
        compute_attachment_velocity(
            attachment_point,
            position,
            &velocity.linear,
            &velocity.angular,
        )
    } else {
        Velocity::zeros()
    }
}

fn compute_attachment_velocity(
    attachment_point: &Position,
    center_of_rotation: &Position,
    velocity: &Velocity,
    angular_velocity: &AngularVelocity,
) -> Velocity {
    velocity
        + angular_velocity
            .as_vector()
            .cross(&(attachment_point - center_of_rotation))
}
