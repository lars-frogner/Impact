//! Forces from springs attached to two rigid bodies.

use crate::{
    anchor::{AnchorManager, DynamicRigidBodyAnchorID, KinematicRigidBodyAnchorID},
    force::ForceGeneratorRegistry,
    quantities::Position,
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager},
};
use approx::abs_diff_eq;
use bytemuck::{Pod, Zeroable};
use nalgebra::UnitVector3;
use roc_integration::roc;

/// Manages all [`DynamicDynamicSpringForceGenerator`]s.
pub type DynamicDynamicSpringForceRegistry = ForceGeneratorRegistry<
    DynamicDynamicSpringForceGeneratorID,
    DynamicDynamicSpringForceGenerator,
>;

/// Manages all [`DynamicKinematicSpringForceGenerator`]s.
pub type DynamicKinematicSpringForceRegistry = ForceGeneratorRegistry<
    DynamicKinematicSpringForceGeneratorID,
    DynamicKinematicSpringForceGenerator,
>;

define_component_type! {
    /// Identifier for a [`DynamicDynamicSpringForceGenerator`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct DynamicDynamicSpringForceGeneratorID(u64);
}

define_component_type! {
    /// Identifier for a [`DynamicKinematicSpringForceGenerator`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct DynamicKinematicSpringForceGeneratorID(u64);
}

/// Generator for a spring force between two dynamic rigid bodies.
#[derive(Copy, Clone, Debug)]
pub struct DynamicDynamicSpringForceGenerator {
    /// The anchor the first end of the spring is attached to.
    pub anchor_1: DynamicRigidBodyAnchorID,
    /// The anchor the second end of the spring is attached to.
    pub anchor_2: DynamicRigidBodyAnchorID,
    /// The spring connecting the two anchors.
    pub spring: Spring,
}

/// Generator for a spring force between two dynamic rigid bodies.
#[derive(Copy, Clone, Debug)]
pub struct DynamicKinematicSpringForceGenerator {
    /// The anchor the first end of the spring is attached to.
    pub anchor_1: DynamicRigidBodyAnchorID,
    /// The anchor the second end of the spring is attached to.
    pub anchor_2: KinematicRigidBodyAnchorID,
    /// The spring connecting the two anchors.
    pub spring: Spring,
}

define_setup_type! {
    target = DynamicDynamicSpringForceGeneratorID;
    /// Generator for a spring force between two dynamic rigid bodies.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DynamicDynamicSpringForceProperties {
        /// The first dynamic rigid body the spring is attached to.
        pub rigid_body_1: DynamicRigidBodyID,
        /// The second dynamic rigid body the spring is attached to.
        pub rigid_body_2: DynamicRigidBodyID,
        /// The point where the spring is attached to the first body, in that
        /// body's model space.
        pub attachment_point_1: Position,
        /// The point where the spring is attached to the second body, in that
        /// body's model space.
        pub attachment_point_2: Position,
        /// The spring connecting the bodies.
        pub spring: Spring,
    }
}

define_setup_type! {
    target = DynamicKinematicSpringForceGeneratorID;
    /// Generator for a spring force between two dynamic rigid bodies.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct DynamicKinematicSpringForceProperties {
        /// The dynamic rigid body the spring is attached to.
        pub rigid_body_1: DynamicRigidBodyID,
        /// The kinematic rigid body the spring is attached to.
        pub rigid_body_2: KinematicRigidBodyID,
        /// The point where the spring is attached to the first (dynamic) body,
        /// in that body's model space.
        pub attachment_point_1: Position,
        /// The point where the spring is attached to the second (kinematic)
        /// body, in that body's model space.
        pub attachment_point_2: Position,
        /// The spring connecting the bodies.
        pub spring: Spring,
    }
}

/// A spring or elastic band.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Spring {
    /// The spring constant representing the stiffness of the spring.
    pub stiffness: f32,
    /// The spring damping coefficient.
    pub damping: f32,
    /// The length for which the spring is in equilibrium.
    pub rest_length: f32,
    /// The length below which the spring force is always zero.
    pub slack_length: f32,
}

impl From<u64> for DynamicDynamicSpringForceGeneratorID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<u64> for DynamicKinematicSpringForceGeneratorID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl DynamicDynamicSpringForceGenerator {
    /// Applies the force to the appropriate dynamic rigid bodies.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager, anchor_manager: &AnchorManager) {
        let Some(anchor_1) = anchor_manager.dynamic().get(self.anchor_1) else {
            return;
        };
        let Some(anchor_2) = anchor_manager.dynamic().get(self.anchor_2) else {
            return;
        };

        let Some([rigid_body_1, rigid_body_2]) = rigid_body_manager
            .get_two_dynamic_rigid_bodies_mut(anchor_1.rigid_body_id, anchor_2.rigid_body_id)
        else {
            return;
        };

        let attachment_point_1 =
            rigid_body_1.transform_point_from_body_to_world_space(&anchor_1.point);
        let attachment_point_2 =
            rigid_body_2.transform_point_from_body_to_world_space(&anchor_2.point);

        let Some((spring_direction, length)) =
            UnitVector3::try_new_and_get(attachment_point_2 - attachment_point_1, f32::EPSILON)
        else {
            return;
        };

        let rate_of_length_change = if abs_diff_eq!(self.spring.damping, 0.0) {
            // The velocities are irrelevant if there is zero damping
            0.0
        } else {
            let attachment_velocity_1 =
                rigid_body_1.compute_velocity_of_attached_world_space_point(&attachment_point_1);
            let attachment_velocity_2 =
                rigid_body_2.compute_velocity_of_attached_world_space_point(&attachment_point_2);

            attachment_velocity_2.dot(&spring_direction)
                - attachment_velocity_1.dot(&spring_direction)
        };

        let force_on_2 =
            self.spring.scalar_force(length, rate_of_length_change) * spring_direction.as_ref();

        rigid_body_1.apply_force(&(-force_on_2), &attachment_point_1);
        rigid_body_2.apply_force(&force_on_2, &attachment_point_2);
    }
}

impl DynamicKinematicSpringForceGenerator {
    /// Applies the force to the appropriate dynamic rigid body.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager, anchor_manager: &AnchorManager) {
        let Some(anchor_1) = anchor_manager.dynamic().get(self.anchor_1) else {
            return;
        };
        let Some(anchor_2) = anchor_manager.kinematic().get(self.anchor_2) else {
            return;
        };

        let Some((rigid_body_1, rigid_body_2)) = rigid_body_manager
            .get_dynamic_rigid_body_mut_and_kinematic_rigid_body(
                anchor_1.rigid_body_id,
                anchor_2.rigid_body_id,
            )
        else {
            return;
        };

        let attachment_point_1 =
            rigid_body_1.transform_point_from_body_to_world_space(&anchor_1.point);
        let attachment_point_2 =
            rigid_body_2.transform_point_from_body_to_world_space(&anchor_2.point);

        let Some((spring_direction, length)) =
            UnitVector3::try_new_and_get(attachment_point_2 - attachment_point_1, f32::EPSILON)
        else {
            return;
        };

        let rate_of_length_change = if abs_diff_eq!(self.spring.damping, 0.0) {
            // The velocities are irrelevant if there is zero damping
            0.0
        } else {
            let attachment_velocity_1 =
                rigid_body_1.compute_velocity_of_attached_world_space_point(&attachment_point_1);
            let attachment_velocity_2 =
                rigid_body_2.compute_velocity_of_attached_world_space_point(&attachment_point_2);

            attachment_velocity_2.dot(&spring_direction)
                - attachment_velocity_1.dot(&spring_direction)
        };

        let force_on_1 =
            -self.spring.scalar_force(length, rate_of_length_change) * spring_direction.as_ref();

        rigid_body_1.apply_force(&force_on_1, &attachment_point_1);
    }
}

#[roc]
impl DynamicDynamicSpringForceProperties {
    #[roc(body = "{ rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring }")]
    pub fn new(
        rigid_body_1: DynamicRigidBodyID,
        attachment_point_1: Position,
        rigid_body_2: DynamicRigidBodyID,
        attachment_point_2: Position,
        spring: Spring,
    ) -> Self {
        Self {
            rigid_body_1,
            attachment_point_1,
            rigid_body_2,
            attachment_point_2,
            spring,
        }
    }
}

#[roc]
impl DynamicKinematicSpringForceProperties {
    #[roc(body = "{ rigid_body_1, attachment_point_1, rigid_body_2, attachment_point_2, spring }")]
    pub fn new(
        rigid_body_1: DynamicRigidBodyID,
        attachment_point_1: Position,
        rigid_body_2: KinematicRigidBodyID,
        attachment_point_2: Position,
        spring: Spring,
    ) -> Self {
        Self {
            rigid_body_1,
            attachment_point_1,
            rigid_body_2,
            attachment_point_2,
            spring,
        }
    }
}

#[roc]
impl Spring {
    /// Creates a new spring.
    #[roc(body = r#"{
        stiffness,
        damping,
        rest_length,
        slack_length,
    }"#)]
    pub fn new(stiffness: f32, damping: f32, rest_length: f32, slack_length: f32) -> Self {
        Self {
            stiffness,
            damping,
            rest_length,
            slack_length,
        }
    }

    /// Creates a standard spring (no slack).
    #[roc(body = "new(stiffness, damping, rest_length, 0)")]
    pub fn standard(stiffness: f32, damping: f32, rest_length: f32) -> Self {
        Self::new(stiffness, damping, rest_length, 0.0)
    }

    /// Creates an elastic band that is slack below a given length.
    #[roc(body = "new(stiffness, damping, slack_length, slack_length)")]
    pub fn elastic_band(stiffness: f32, damping: f32, slack_length: f32) -> Self {
        Self::new(stiffness, damping, slack_length, slack_length)
    }

    /// Computes the force along the spring axis for the given length and rate
    /// of change in length. A positive force is directed outward.
    pub fn scalar_force(&self, length: f32, rate_of_length_change: f32) -> f32 {
        if length <= self.slack_length {
            0.0
        } else {
            self.compute_spring_force(length) + self.compute_damping_force(rate_of_length_change)
        }
    }

    fn compute_spring_force(&self, length: f32) -> f32 {
        -self.stiffness * (length - self.rest_length)
    }

    fn compute_damping_force(&self, rate_of_length_change: f32) -> f32 {
        -self.damping * rate_of_length_change
    }
}

#[cfg(test)]
mod tests {
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
