//! Torques for aligning a body axis with an external direction.

use crate::{
    force::{ForceGeneratorRegistry, dynamic_gravity::DynamicGravityManager},
    quantities::{Direction, DirectionC},
    rigid_body::{DynamicRigidBodyID, RigidBodyManager},
};
use bytemuck::{Pod, Zeroable};
use impact_id::define_entity_id_newtype;
use impact_math::vector::UnitVector3C;
use roc_integration::roc;

/// Manages all [`AlignmentTorqueGenerator`]s.
pub type AlignmentTorqueRegistry =
    ForceGeneratorRegistry<AlignmentTorqueGeneratorID, AlignmentTorqueGenerator>;

define_entity_id_newtype! {
    /// Identifier for a [`AlignmentTorqueGenerator`].
    [pub] AlignmentTorqueGeneratorID
}

define_component_type! {
    /// Marks that an entity has an alignment torque generator identified by a
    /// [`AlignmentTorqueGeneratorID`].
    ///
    /// Use [`AlignmentTorqueGeneratorID::from_entity_id`] to obtain the
    /// generator ID from the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HasAlignmentTorqueGenerator;
}

/// Generator for a torque working to align a body axis with an external
/// direction.
#[derive(Clone, Debug)]
pub struct AlignmentTorqueGenerator {
    /// The dynamic rigid body experiencing the torque.
    pub rigid_body_id: DynamicRigidBodyID,
    /// The local axis of the body to align.
    pub axis_to_align: DirectionC,
    /// The external direction to align with.
    pub alignment_direction: AlignmentDirection,
    /// The approximate time the torque should take to achieve the alignment.
    pub settling_time: f32,
    /// The frequency factor to multiply with the negative component of angular
    /// velocity around the axis to align, in order to damp it.
    pub spin_damping_frequency: f32,
    /// The frequency factor to multiply with the negative component of angular
    /// velocity causing precession around the alignement direction, in order to
    /// damp it.
    pub precession_damping_frequency: f32,
}

/// An external direction a body can be aligned with.
#[roc(parents = "Physics")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq)]
pub enum AlignmentDirection {
    Fixed(DirectionC),
    GravityForce,
}

define_setup_type! {
    /// A torque working to align an axis of the body with a fixed external
    /// direction.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct FixedDirectionAlignmentTorque {
        /// The local axis of the body to align.
        pub axis_to_align: DirectionC,
        /// The external direction to align with.
        pub alignment_direction: DirectionC,
        /// The approximate time the torque should take to achieve the alignment.
        pub settling_time: f32,
        /// The strength with which to damp the component of angular velocity
        /// around the axis to align.
        pub spin_damping: f32,
        /// The strength with which to damp the component of angular velocity
        /// causing precession around the alignement direction.
        pub precession_damping: f32,
    }
}

define_setup_type! {
    target = AlignmentTorqueGeneratorID;
    /// A torque working to align an axis of the body with the direction of the
    /// total gravitational force it is experiencing.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct GravityAlignmentTorque {
        /// The local axis of the body to align.
        pub axis_to_align: DirectionC,
        /// The approximate time the torque should take to achieve the alignment.
        pub settling_time: f32,
        /// The strength with which to damp the component of angular velocity
        /// around the axis to align.
        pub spin_damping: f32,
        /// The strength with which to damp the component of angular velocity
        /// causing precession around the alignement direction.
        pub precession_damping: f32,
    }
}

impl AlignmentTorqueGenerator {
    pub fn for_fixed_direction(
        rigid_body_id: DynamicRigidBodyID,
        torque: FixedDirectionAlignmentTorque,
    ) -> Self {
        Self {
            rigid_body_id,
            axis_to_align: torque.axis_to_align,
            alignment_direction: AlignmentDirection::Fixed(torque.alignment_direction),
            settling_time: torque.settling_time,
            spin_damping_frequency: torque.spin_damping / torque.settling_time,
            precession_damping_frequency: torque.precession_damping / torque.settling_time,
        }
    }

    pub fn for_gravity_direction(
        rigid_body_id: DynamicRigidBodyID,
        torque: GravityAlignmentTorque,
    ) -> Self {
        Self {
            rigid_body_id,
            axis_to_align: torque.axis_to_align,
            alignment_direction: AlignmentDirection::GravityForce,
            settling_time: torque.settling_time,
            spin_damping_frequency: torque.spin_damping / torque.settling_time,
            precession_damping_frequency: torque.precession_damping / torque.settling_time,
        }
    }

    /// Applies the torque to the appropriate dynamic rigid body.
    pub fn apply(
        &self,
        rigid_body_manager: &mut RigidBodyManager,
        dynamic_gravity_manager: &DynamicGravityManager,
    ) {
        let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(self.rigid_body_id)
        else {
            return;
        };

        let alignment_direction = match self.alignment_direction {
            AlignmentDirection::Fixed(alignment_direction) => alignment_direction,
            AlignmentDirection::GravityForce => {
                let Some(gravity_force) =
                    dynamic_gravity_manager.get_force_on_body(self.rigid_body_id)
                else {
                    return;
                };
                let Some(gravity_direction) =
                    UnitVector3C::normalized_from_if_above(gravity_force, 1e-9)
                else {
                    return;
                };
                gravity_direction
            }
        }
        .aligned();

        let local_axis_to_align = self.axis_to_align.aligned();
        let axis_to_align =
            rigid_body.transform_direction_from_body_to_world_space(&local_axis_to_align);

        // Find the axis we can rotate around to align the vectors with the
        // shortest possible arc
        let rotation_axis =
            Direction::normalized_from_if_above(alignment_direction.cross(&axis_to_align), 1e-8)
                .unwrap_or_else(|| Direction::orthogonal_to(&axis_to_align));

        let orientation = rigid_body.orientation().aligned();
        let inertia_tensor_body_space = rigid_body.inertia_tensor().aligned();
        let inertia_tensor = inertia_tensor_body_space.rotated_matrix(&orientation);
        let angular_momentum = rigid_body.angular_momentum().aligned();
        let angular_velocity = rigid_body.compute_angular_velocity().as_vector();

        // Determine how fast we are rotating directly towards the alignment
        // direction
        let angular_speed_about_rotation_axis = angular_velocity.dot(&rotation_axis);
        let angular_velocity_about_rotation_axis =
            angular_speed_about_rotation_axis * rotation_axis;

        // Determine how fast we are rotating about the axis to align. We may
        // want to damp this to prevent excessive spin from building up due to
        // the other corrections.
        let angular_speed_about_axis_to_align = angular_velocity.dot(&axis_to_align);
        let angular_velocity_about_axis_to_align =
            angular_speed_about_axis_to_align * axis_to_align;

        // The remaining angular velocity represents precession/wobbling. We may
        // want to damp this to avoid circling around the alignment direction
        // forever.
        let precession_angular_velocity = angular_velocity
            - angular_velocity_about_rotation_axis
            - angular_velocity_about_axis_to_align;

        // We model the direct rotation towards the alignment direction as a
        // critically dampened harmonic oscillator expressed in terms of the
        // angle between the axis to align and the alignment direction. This
        // gives smooth and efficient stabilization without oscillations.

        let angle_about_rotation_axis =
            f32::acos(alignment_direction.dot(&axis_to_align).clamp(-1.0, 1.0));

        let angular_acceleration_about_rotation_axis =
            compute_critically_dampened_angular_acceleration(
                angle_about_rotation_axis,
                angular_speed_about_rotation_axis,
                self.settling_time,
            );

        // Include spin and precession damping terms in the final angular
        // acceleration
        let angular_acceleration = angular_acceleration_about_rotation_axis * rotation_axis
            - self.spin_damping_frequency * angular_velocity_about_axis_to_align
            - self.precession_damping_frequency * precession_angular_velocity;

        // Compute torque from Euler's equations
        let torque =
            inertia_tensor * angular_acceleration + angular_velocity.cross(&angular_momentum);

        rigid_body.apply_torque(&torque);
    }
}

fn compute_critically_dampened_angular_acceleration(
    angle: f32,
    angular_speed: f32,
    settling_time: f32,
) -> f32 {
    // We define "settled" as 4 time constants
    let time_constant = 0.25 * settling_time;

    let natural_frequency = time_constant.recip();
    let critical_damping = 2.0 * natural_frequency;

    -natural_frequency.powi(2) * angle - critical_damping * angular_speed
}

#[roc]
impl FixedDirectionAlignmentTorque {
    #[roc(
        body = "{ axis_to_align, alignment_direction, settling_time, spin_damping, precession_damping }"
    )]
    pub fn new(
        axis_to_align: DirectionC,
        alignment_direction: DirectionC,
        settling_time: f32,
        spin_damping: f32,
        precession_damping: f32,
    ) -> Self {
        Self {
            axis_to_align,
            alignment_direction,
            settling_time,
            spin_damping,
            precession_damping,
        }
    }
}

#[roc]
impl GravityAlignmentTorque {
    #[roc(body = "{ axis_to_align, settling_time, spin_damping, precession_damping }")]
    pub fn new(
        axis_to_align: DirectionC,
        settling_time: f32,
        spin_damping: f32,
        precession_damping: f32,
    ) -> Self {
        Self {
            axis_to_align,
            settling_time,
            spin_damping,
            precession_damping,
        }
    }
}
