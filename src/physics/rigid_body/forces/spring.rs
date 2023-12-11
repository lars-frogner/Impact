//! Spring force.

mod components;

pub use components::SpringComp;

use crate::{
    control::{MotionControlComp, OrientationControlComp},
    physics::{
        fph, AngularVelocity, AngularVelocityComp, Direction, Orientation, Position,
        ReferenceFrameComp, RigidBodyComp, Static, Velocity, VelocityComp,
    },
};
use approx::abs_diff_eq;
use bytemuck::{Pod, Zeroable};
use impact_ecs::{
    query,
    world::{Entity, EntityEntry, World as ECSWorld},
};
use nalgebra::{UnitVector3, Vector3};
use std::collections::LinkedList;

/// A spring or elastic band.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Spring {
    /// The spring constant representing the stiffness of the spring.
    pub stiffness: fph,
    /// The spring damping coefficient.
    pub damping: fph,
    /// The length for which the spring is in equilibrium.
    pub rest_length: fph,
    /// The length below which the spring force is always zero.
    pub slack_length: fph,
}

/// The current state of a spring.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct SpringState {
    /// The direction from the first to the second attachment point.
    direction: Direction,
    /// The position of the center of the spring.
    center: Position,
}

/// The outcome of applying the forces from a spring.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpringForceApplicationOutcome {
    Ok,
    EntityMissing,
}

impl Spring {
    /// Creates a new spring.
    pub fn new(stiffness: fph, damping: fph, rest_length: fph, slack_length: fph) -> Self {
        Self {
            stiffness,
            damping,
            rest_length,
            slack_length,
        }
    }

    /// Creates a standard spring (no slack).
    pub fn standard(stiffness: fph, damping: fph, rest_length: fph) -> Self {
        Self::new(stiffness, damping, rest_length, 0.0)
    }

    /// Creates an elastic band that is slack below a given length.
    pub fn elastic_band(stiffness: fph, damping: fph, slack_length: fph) -> Self {
        Self::new(stiffness, damping, slack_length, slack_length)
    }

    /// Computes the force along the spring axis for the given length and rate
    /// of change in length. A positive force is directed outward.
    pub fn scalar_force(&self, length: fph, rate_of_length_change: fph) -> fph {
        if length <= self.slack_length {
            0.0
        } else {
            self.compute_spring_force(length) + self.compute_damping_force(rate_of_length_change)
        }
    }

    fn compute_spring_force(&self, length: fph) -> fph {
        -self.stiffness * (length - self.rest_length)
    }

    fn compute_damping_force(&self, rate_of_length_change: fph) -> fph {
        -self.damping * rate_of_length_change
    }
}

impl SpringState {
    /// Creates a new spring state (with dummy values).
    pub fn new() -> Self {
        Self {
            direction: Vector3::y_axis(),
            center: Position::origin(),
        }
    }

    /// Returns the direction from the first to the second attachment point.
    pub fn direction(&self) -> &Direction {
        &self.direction
    }

    /// Returns the position of the center of the spring.
    pub fn center(&self) -> &Position {
        &self.center
    }

    /// Computes an orientation for the spring based on its direction.
    pub fn compute_orientation(&self) -> Orientation {
        Orientation::rotation_between_axis(&Vector3::y_axis(), &self.direction)
            .unwrap_or_else(|| Orientation::from_axis_angle(&Vector3::y_axis(), 0.0))
    }

    fn update(&mut self, attachment_point_1: &Position, direction: Direction, length: fph) {
        self.center = attachment_point_1 + direction.as_ref() * (0.5 * length);
        self.direction = direction;
    }

    fn update_with_zero_length(&mut self, attachment_point_1: Position) {
        self.center = attachment_point_1;
    }
}

/// Applies spring forces to all applicable rigid bodies.
pub fn apply_spring_forces(ecs_world: &ECSWorld, entities_to_remove: &mut LinkedList<Entity>) {
    query!(ecs_world, |entity: Entity, spring: &mut SpringComp| {
        let outcome = apply_forces(spring, &ecs_world);
        if outcome == SpringForceApplicationOutcome::EntityMissing {
            entities_to_remove.push_back(entity);
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
            VelocityComp,
            AngularVelocityComp
        ]
    );
}

fn apply_forces(spring: &mut SpringComp, ecs_world: &ECSWorld) -> SpringForceApplicationOutcome {
    let (entity_1, entity_2) = match (
        ecs_world.get_entity(&spring.entity_1),
        ecs_world.get_entity(&spring.entity_2),
    ) {
        (Some(entity_1), Some(entity_2)) => (entity_1, entity_2),
        _ => {
            log::debug!("Missing spring attachment entity: spring component will be removed");
            return SpringForceApplicationOutcome::EntityMissing;
        }
    };

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
                rigid_body_1
                    .access()
                    .0
                    .apply_force(&(-force_on_2), &attachment_point_1);

                // To prevent a potential deadlock, the entry to the
                // `RigidBodyComp` storage for entity 1 must have been dropped
                // before trying to access the storage for entity 2
                drop(rigid_body_1);
            }
        }
        if !entity_2_is_static {
            if let Some(mut rigid_body_2) = entity_2.get_component_mut::<RigidBodyComp>() {
                rigid_body_2
                    .access()
                    .0
                    .apply_force(&force_on_2, &attachment_point_2);
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
        .map_or_else(ReferenceFrameComp::default, |frame| frame.access().clone())
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
    let velocity = entity
        .get_component::<VelocityComp>()
        .map_or_else(Velocity::zeros, |v| v.access().0);

    if let Some(angular_velocity) = entity.get_component::<AngularVelocityComp>() {
        let angular_velocity = angular_velocity.access().0;
        compute_attachment_velocity(attachment_point, position, &velocity, &angular_velocity)
    } else {
        velocity
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

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn should_get_zero_undamped_force_at_rest_length() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 0.0, rest_length);
        assert_abs_diff_eq!(spring.scalar_force(rest_length, 0.0), 0.0);
    }

    #[test]
    fn should_get_positive_undamped_force_below_rest_length() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 0.0, rest_length);
        assert!(spring.scalar_force(0.5 * rest_length, 0.0) > 0.0);
    }

    #[test]
    fn should_get_negative_undamped_force_above_rest_length() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 0.0, rest_length);
        assert!(spring.scalar_force(2.0 * rest_length, 0.0) < 0.0);
    }

    #[test]
    fn should_get_zero_force_below_slack_length() {
        let slack_length = 1.0;
        let spring = Spring::elastic_band(1.0, 1.0, slack_length);
        assert_abs_diff_eq!(spring.scalar_force(0.5 * slack_length, -1.0), 0.0);
    }

    #[test]
    fn should_get_positive_damping_force_for_contracting_spring() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 1.0, rest_length);
        assert!(spring.scalar_force(rest_length, -1.0) > 0.0);
    }

    #[test]
    fn should_get_negative_damping_force_for_expanding_spring() {
        let rest_length = 1.0;
        let spring = Spring::standard(1.0, 1.0, rest_length);
        assert!(spring.scalar_force(rest_length, 1.0) < 0.0);
    }
}
