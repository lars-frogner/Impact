//! [`Component`](impact_ecs::component::Component)s related to rotation with a
//! constant angular velocity.

use crate::{
    component::ComponentRegistry,
    physics::{
        fph,
        motion::{AngularVelocity, Orientation},
    },
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that rotate
/// with a constant angular velocity over time.
///
/// For this component to have an effect, the entity also needs a
/// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ConstantRotationComp {
    /// When (in simulation time) the entity should have the initial
    /// orientation.
    pub initial_time: fph,
    /// The orientation of the entity at the initial time.
    pub initial_orientation: Orientation,
    /// The angular velocity of the entity.
    pub angular_velocity: AngularVelocity,
}

impl ConstantRotationComp {
    /// Creates a new component for constant rotation defined by the given
    /// initial time and orientation and angular velocity.
    pub fn new(
        initial_time: fph,
        initial_orientation: Orientation,
        angular_velocity: AngularVelocity,
    ) -> Self {
        Self {
            initial_time,
            initial_orientation,
            angular_velocity,
        }
    }
}

/// Registers all constant rotation motion
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_constant_rotation_motion_components(
    registry: &mut ComponentRegistry,
) -> Result<()> {
    register_component!(registry, ConstantRotationComp)
}
