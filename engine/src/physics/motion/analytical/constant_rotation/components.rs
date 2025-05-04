//! [`Component`](impact_ecs::component::Component)s related to rotation with a
//! constant angular velocity.

use crate::physics::{
    fph,
    motion::{AngularVelocity, Orientation},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use roc_codegen::roc;

/// [`Component`](impact_ecs::component::Component) for entities that rotate
/// with a constant angular velocity over time.
///
/// For this component to have an effect, the entity also needs a
/// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp).
#[roc(prefix = "Comp", name = "ConstantRotation")]
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

#[roc]
impl ConstantRotationComp {
    /// Creates a new component for constant rotation defined by the given
    /// initial time and orientation and angular velocity.
    #[roc(body = r#"
    {
        initial_time,
        initial_orientation,
        angular_velocity,
    }
    "#)]
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
