//! Spring force.

use crate::physics::{
    fph, AngularVelocity, AngularVelocityComp, Direction, DrivenAngularVelocityComp, Orientation,
    OrientationComp, Position, PositionComp, RigidBodyComp, Velocity, VelocityComp,
};
use approx::abs_diff_eq;
use bytemuck::{Pod, Zeroable};
use impact_ecs::{
    query,
    world::{Entity, EntityEntry, World as ECSWorld},
    Component,
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

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// spring connecting two other entities.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SpringComp {
    /// The first entity the spring is attached to.
    pub entity_1: Entity,
    /// The second entity the spring is attached to.
    pub entity_2: Entity,
    /// The point where the spring is attached to the first entity, in that
    /// entity's reference frame.
    pub attachment_point_1: Position,
    /// The point where the spring is attached to the second entity, in that
    /// entity's reference frame.
    pub attachment_point_2: Position,
    /// The spring connecting the entities.
    pub spring: Spring,
    /// The current state of the spring.
    pub spring_state: SpringState,
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

impl SpringComp {
    /// Creates a new component for a spring connecting two entities.
    pub fn new(
        entity_1: Entity,
        entity_2: Entity,
        attachment_point_1: Position,
        attachment_point_2: Position,
        spring: Spring,
    ) -> Self {
        Self {
            entity_1,
            entity_2,
            attachment_point_1,
            attachment_point_2,
            spring,
            spring_state: SpringState::new(),
        }
    }

    /// Creates a new component for a spring connecting the origins of two
    /// entities' reference frames.
    pub fn attached_to_origins(entity_1: Entity, entity_2: Entity, spring: Spring) -> Self {
        Self::new(
            entity_1,
            entity_2,
            Position::origin(),
            Position::origin(),
            spring,
        )
    }

    fn apply_forces(&mut self, ecs_world: &ECSWorld) -> SpringForceApplicationOutcome {
        let (entity_1, entity_2) = match (
            ecs_world.get_entity(&self.entity_1),
            ecs_world.get_entity(&self.entity_2),
        ) {
            (Some(entity_1), Some(entity_2)) => (entity_1, entity_2),
            _ => {
                log::debug!("Missing spring attachment entity: spring component will be removed");
                return SpringForceApplicationOutcome::EntityMissing;
            }
        };

        if !entity_1.has_component::<RigidBodyComp>() && !entity_2.has_component::<RigidBodyComp>()
        {
            // Nothing to apply the force to
            return SpringForceApplicationOutcome::Ok;
        }

        let attachment_point_1 =
            Self::determine_attachment_point_in_world_space(&entity_1, &self.attachment_point_1);
        let attachment_point_2 =
            Self::determine_attachment_point_in_world_space(&entity_2, &self.attachment_point_2);

        if let Some((spring_direction, length)) =
            UnitVector3::try_new_and_get(attachment_point_2 - attachment_point_1, fph::EPSILON)
        {
            self.spring_state
                .update(&attachment_point_1, spring_direction, length);

            let rate_of_length_change = if abs_diff_eq!(self.spring.damping, 0.0) {
                // The velocities are irrelevant if there is zero damping
                0.0
            } else {
                let attachment_velocity_1 =
                    Self::determine_attachment_velocity(&entity_1, &attachment_point_1);
                let attachment_velocity_2 =
                    Self::determine_attachment_velocity(&entity_2, &attachment_point_2);

                attachment_velocity_2.dot(&spring_direction)
                    - attachment_velocity_1.dot(&spring_direction)
            };

            let force_on_2 =
                self.spring.scalar_force(length, rate_of_length_change) * spring_direction.as_ref();

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
            if let Some(mut rigid_body_2) = entity_2.get_component_mut::<RigidBodyComp>() {
                rigid_body_2
                    .access()
                    .0
                    .apply_force(&force_on_2, &attachment_point_2);
            }
        } else {
            self.spring_state
                .update_with_zero_length(attachment_point_1);
        }

        SpringForceApplicationOutcome::Ok
    }

    fn determine_attachment_point_in_world_space(
        entity: &EntityEntry<'_>,
        attachment_point_in_entity_frame: &Position,
    ) -> Position {
        let position = entity
            .get_component::<PositionComp>()
            .map_or_else(Position::origin, |p| p.access().0);

        let orientation = entity
            .get_component::<OrientationComp>()
            .map(|o| o.access().0);

        Self::compute_attachment_point_in_world_space(
            attachment_point_in_entity_frame,
            &position,
            orientation,
        )
    }

    fn compute_attachment_point_in_world_space(
        attachment_point_in_entity_frame: &Position,
        position: &Position,
        orientation: Option<Orientation>,
    ) -> Position {
        if let Some(orientation) = orientation {
            position + orientation.transform_vector(&attachment_point_in_entity_frame.coords)
        } else {
            position + attachment_point_in_entity_frame.coords
        }
    }

    fn determine_attachment_velocity(
        entity: &EntityEntry<'_>,
        attachment_point: &Position,
    ) -> Velocity {
        let velocity = entity
            .get_component::<VelocityComp>()
            .map_or_else(Velocity::zeros, |v| v.access().0);

        if let (Some(rigid_body), Some(angular_velocity)) = (
            entity.get_component::<RigidBodyComp>(),
            entity.get_component::<AngularVelocityComp>(),
        ) {
            let center_of_mass = rigid_body.access().0.center_of_mass();
            Self::compute_attachment_velocity(
                attachment_point,
                &center_of_mass,
                &velocity,
                &angular_velocity.access().0,
            )
        } else if let Some(driven_angular_velocity) =
            entity.get_component::<DrivenAngularVelocityComp>()
        {
            let center_of_rotation = driven_angular_velocity.access().center_of_rotation;
            let angular_velocity = driven_angular_velocity.access().angular_velocity;
            Self::compute_attachment_velocity(
                attachment_point,
                &center_of_rotation,
                &velocity,
                &angular_velocity,
            )
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
}

/// Applies spring forces to all applicable rigid bodies.
pub fn apply_spring_forces(ecs_world: &ECSWorld, entities_to_remove: &mut LinkedList<Entity>) {
    query!(ecs_world, |entity: Entity, spring: &mut SpringComp| {
        let outcome = spring.apply_forces(&ecs_world);
        if outcome == SpringForceApplicationOutcome::EntityMissing {
            entities_to_remove.push_back(entity);
        }
    });
}

pub fn synchronize_spring_positions_and_orientations(ecs_world: &ECSWorld) {
    query!(ecs_world, |position: &mut PositionComp,
                       orientation: &mut OrientationComp,
                       spring: &SpringComp| {
        position.0 = *spring.spring_state.center();
        orientation.0 = spring.spring_state.compute_orientation();
    });
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
