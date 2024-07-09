//! [`Component`](impact_ecs::component::Component)s related to motion.

use crate::{
    component::ComponentRegistry,
    num::Float,
    physics::{fph, AngularVelocity, Orientation, Position, Velocity},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::{Similarity3, Translation3, Vector3};

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// reference frame defined by position, orientation and scaling.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ReferenceFrameComp {
    /// The offset, expressed in the entity's reference frame (before scaling),
    /// from the original origin of the entity's reference frame to the point
    /// that should be used as the actual origin.
    pub origin_offset: Vector3<fph>,
    /// The coordinates of the origin of the entity's reference frame measured
    /// in the parent space.
    pub position: Position,
    /// The 3D orientation of the entity's reference frame in the parent space.
    pub orientation: Orientation,
    /// The uniform scale factor of the entity's reference frame (distance in
    /// world space per distance in the reference frame).
    pub scaling: fph,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// linear and/or angular velocity.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct VelocityComp {
    /// The linear velocity of the entity's reference frame in the parent space.
    pub linear: Velocity,
    /// The angular velocity of the entity's reference frame about its origin in
    /// the parent space.
    pub angular: AngularVelocity,
}

/// Marker [`Component`](impact_ecs::component::Component) for entities whose
/// position and orientation are not supposed to change.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct Static;

/// Marker [`Component`](impact_ecs::component::Component) for entities whose
/// translational and rotational kinetic energy should be written to the log at
/// each time step.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct LogsKineticEnergy;

/// Marker [`Component`](impact_ecs::component::Component) for entities whose
/// linear and angular momentum should be written to the log at each time step.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct LogsMomentum;

impl ReferenceFrameComp {
    /// Creates a new reference frame component with the given position,
    /// orientation and scaling, retaining the original origin of the entity's
    /// reference frame.
    pub fn new(position: Position, orientation: Orientation, scaling: fph) -> Self {
        Self::scaled_with_offset_origin(Vector3::zeros(), position, orientation, scaling)
    }

    /// Creates a new reference frame component with the given position and
    /// orientation, retaining the original origin of the entity's reference
    /// frame and no scaling.
    pub fn unscaled(position: Position, orientation: Orientation) -> Self {
        Self::new(position, orientation, 1.0)
    }

    /// Creates a new reference frame component with the given position,
    /// retaining the original origin of the entity's reference frame and the
    /// identity orientation and scaling.
    pub fn unoriented(position: Position) -> Self {
        Self::unoriented_scaled(position, 1.0)
    }

    /// Creates a new reference frame component with the given position and
    /// scaling, retaining the original origin of the entity's reference frame
    /// and the identity orientation.
    pub fn unoriented_scaled(position: Position, scaling: fph) -> Self {
        Self::new(position, Orientation::identity(), scaling)
    }

    /// Creates a new reference frame component with the given orientation,
    /// retaining the original origin of the entity's reference frame and
    /// located at the origin with no scaling.
    pub fn unlocated(orientation: Orientation) -> Self {
        Self::unlocated_scaled(orientation, 1.0)
    }

    /// Creates a new reference frame component with the given orientation and
    /// scaling, retaining the original origin of the entity's reference frame
    /// and located at the origin.
    pub fn unlocated_scaled(orientation: Orientation, scaling: fph) -> Self {
        Self::new(Position::origin(), orientation, scaling)
    }

    /// Creates a new reference frame component with the given scaling,
    /// retaining the original origin of the entity's reference frame and
    /// located at the origin with the identity orientation.
    pub fn scaled(scaling: fph) -> Self {
        Self::unoriented_scaled(Position::origin(), scaling)
    }

    /// Creates a new reference frame component with the given origin offset and
    /// position, and with the identity orientation and scaling.
    pub fn unoriented_with_offset_origin(origin_offset: Vector3<fph>, position: Position) -> Self {
        Self::unoriented_scaled_with_offset_origin(origin_offset, position, 1.0)
    }

    /// Creates a new reference frame component with the given origin offset,
    /// position and scaling, and with the identity orientation.
    pub fn unoriented_scaled_with_offset_origin(
        origin_offset: Vector3<fph>,
        position: Position,
        scaling: fph,
    ) -> Self {
        Self::scaled_with_offset_origin(origin_offset, position, Orientation::identity(), scaling)
    }

    /// Creates a new reference frame component with the given origin offset,
    /// position orientation, and scaling.
    pub fn scaled_with_offset_origin(
        origin_offset: Vector3<fph>,
        position: Position,
        orientation: Orientation,
        scaling: fph,
    ) -> Self {
        Self {
            origin_offset,
            position,
            orientation,
            scaling,
        }
    }

    /// Creates a new reference frame component with the given origin offset,
    /// position and orientation and no scaling.
    pub fn with_offset_origin(
        origin_offset: Vector3<fph>,
        position: Position,
        orientation: Orientation,
    ) -> Self {
        Self::scaled_with_offset_origin(origin_offset, position, orientation, 1.0)
    }

    /// Creates a new reference frame component with the given position and
    /// orientation for a rigid body and no scaling. The origin offset will be
    /// set to the center of mass.
    pub fn for_rigid_body(position: Position, orientation: Orientation) -> Self {
        Self::for_scaled_rigid_body(position, orientation, 1.0)
    }

    /// Creates a new reference frame component with the given position,
    /// orientation and scaling for a rigid body. The origin offset will be set
    /// to the center of mass.
    pub fn for_scaled_rigid_body(
        position: Position,
        orientation: Orientation,
        scaling: fph,
    ) -> Self {
        Self::new(position, orientation, scaling)
    }

    /// Creates a new reference frame component with the given position for a
    /// rigid body with the identity orientation and scaling. The origin offset
    /// will be set to the center of mass.
    pub fn for_unoriented_rigid_body(position: Position) -> Self {
        Self::unoriented(position)
    }

    /// Creates a new reference frame component with the given position and
    /// scaling for a rigid body with the identity orientation. The origin
    /// offset will be set to the center of mass.
    pub fn for_scaled_unoriented_rigid_body(position: Position, scaling: fph) -> Self {
        Self::unoriented_scaled(position, scaling)
    }

    /// Creates a new reference frame component with the given position for an
    /// entity whose orientation will be evolved analytically (and thus should
    /// not be initialised in this component).
    pub fn for_driven_rotation(position: Position) -> Self {
        Self::unoriented(position)
    }

    /// Creates a new reference frame component with the given position and
    /// scaling for an entity whose orientation will be evolved analytically
    /// (and thus should not be initialised in this component).
    pub fn for_scaled_driven_rotation(position: Position, scaling: fph) -> Self {
        Self::unoriented_scaled(position, scaling)
    }

    /// Creates a new reference frame component with the given origin offset and
    /// position for an entity with no scaling whose orientation will be evolved
    /// analytically (and thus should not be initialised in this component).
    pub fn for_driven_rotation_around_offset_origin(
        origin_offset: Vector3<fph>,
        position: Position,
    ) -> Self {
        Self::unoriented_with_offset_origin(origin_offset, position)
    }

    /// Creates a new reference frame component with the given origin offset,
    /// position and scaling for an entity whose orientation will be evolved
    /// analytically (and thus should not be initialised in this component).
    pub fn for_scaled_driven_rotation_around_offset_origin(
        origin_offset: Vector3<fph>,
        position: Position,
        scaling: fph,
    ) -> Self {
        Self::unoriented_scaled_with_offset_origin(origin_offset, position, scaling)
    }

    /// Creates a new reference frame component with the given orientation for
    /// an entity with no scaling whose trajectory will be evolved analytically
    /// (and whose position should thus not be initialised in this component).
    pub fn for_driven_trajectory(orientation: Orientation) -> Self {
        Self::unlocated(orientation)
    }

    /// Creates a new reference frame component with the given orientation and
    /// scaling for an entity whose trajectory will be evolved analytically (and
    /// whose position should thus not be initialised in this component).
    pub fn for_scaled_driven_trajectory(orientation: Orientation, scaling: fph) -> Self {
        Self::unlocated_scaled(orientation, scaling)
    }

    /// Creates a new reference frame component with the given origin offset and
    /// orientation for an entity with no scaling whose trajectory will be
    /// evolved analytically (and whose position should thus not be initialised
    /// in this component).
    pub fn for_driven_trajectory_with_offset_origin(
        origin_offset: Vector3<fph>,
        orientation: Orientation,
    ) -> Self {
        Self::for_scaled_driven_trajectory_with_offset_origin(origin_offset, orientation, 1.0)
    }

    /// Creates a new reference frame component with the given origin offset,
    /// orientation and scaling for an entity whose trajectory will be evolved
    /// analytically (and whose position should thus not be initialised in this
    /// component).
    pub fn for_scaled_driven_trajectory_with_offset_origin(
        origin_offset: Vector3<fph>,
        orientation: Orientation,
        scaling: fph,
    ) -> Self {
        Self::scaled_with_offset_origin(origin_offset, Position::origin(), orientation, scaling)
    }

    /// Creates a new reference frame component for an entity with no scaling
    /// whose trajectory and orientation will be evolved analytically (and whose
    /// position and orientation should thus not be initialised in this
    /// component).
    pub fn for_driven_trajectory_and_rotation() -> Self {
        Self::default()
    }

    /// Creates a new reference frame component for an entity with the given
    /// scaling whose trajectory and orientation will be evolved analytically
    /// (and whose position and orientation should thus not be initialised in
    /// this component).
    pub fn for_scaled_driven_trajectory_and_rotation(scaling: fph) -> Self {
        Self::scaled(scaling)
    }

    /// Creates a new reference frame component with the given origin offset for
    /// an entity with no scaling whose trajectory and orientation will be
    /// evolved analytically (and whose position and orientation should thus not
    /// be initialised in this component).
    pub fn for_driven_trajectory_and_rotation_with_offset_origin(
        origin_offset: Vector3<fph>,
    ) -> Self {
        Self::for_scaled_driven_trajectory_and_rotation_with_offset_origin(origin_offset, 1.0)
    }

    /// Creates a new reference frame component with the given origin offset and
    /// scaling for an entity whose trajectory and orientation will be evolved
    /// analytically (and whose position and orientation should thus not be
    /// initialised in this component).
    pub fn for_scaled_driven_trajectory_and_rotation_with_offset_origin(
        origin_offset: Vector3<fph>,
        scaling: fph,
    ) -> Self {
        Self::for_scaled_driven_trajectory_with_offset_origin(
            origin_offset,
            Orientation::identity(),
            scaling,
        )
    }

    /// Creates the [`Similarity3`] transform from the entity's reference frame
    /// to the parent space.
    pub fn create_transform_to_parent_space<F: Float>(&self) -> Similarity3<F> {
        Similarity3::from_parts(
            Translation3::from(self.position.cast::<F>()),
            self.orientation.cast::<F>(),
            F::from_f64(self.scaling).unwrap(),
        ) * Translation3::from(-self.origin_offset.cast::<F>())
    }
}

impl VelocityComp {
    /// Creates a new velocity component for an entity with the given linear and
    /// angular velocity.
    pub fn new(linear: Velocity, angular: AngularVelocity) -> Self {
        Self { linear, angular }
    }

    /// Creates a new velocity component for an entity with the given linear
    /// velocity and zero angular velocity.
    pub fn linear(velocity: Velocity) -> Self {
        Self::new(velocity, AngularVelocity::zero())
    }

    /// Creates a new velocity component for an entity with the given angular
    /// velocity and zero linear velocity.
    pub fn angular(velocity: AngularVelocity) -> Self {
        Self::new(Velocity::zeros(), velocity)
    }

    /// Creates a new velocity component for an entity with the zero linear and
    /// angular velocity.
    pub fn stationary() -> Self {
        Self::linear(Velocity::zeros())
    }
}

impl Default for ReferenceFrameComp {
    fn default() -> Self {
        Self {
            origin_offset: Vector3::zeros(),
            position: Position::origin(),
            orientation: Orientation::identity(),
            scaling: 1.0,
        }
    }
}

/// Registers all motion [`Component`](impact_ecs::component::Component)s.
pub fn register_motion_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_component!(registry, ReferenceFrameComp)?;
    register_component!(registry, VelocityComp)?;
    register_component!(registry, Static)?;
    register_component!(registry, LogsKineticEnergy)?;
    register_component!(registry, LogsMomentum)
}
