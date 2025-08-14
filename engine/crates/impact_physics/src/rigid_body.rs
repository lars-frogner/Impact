//! Rigid body simulation.

pub mod setup;

use crate::{
    fph,
    inertia::InertiaTensor,
    quantities::{
        self, AngularMomentum, AngularVelocity, Force, Momentum, Motion, Orientation, Position,
        Torque, Velocity,
    },
};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use impact_containers::KeyIndexMapper;
use impact_geometry::ReferenceFrame;
use impact_math::Angle;
use nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3};
use roc_integration::roc;

define_component_type! {
    /// Identifier for a [`DynamicRigidBody`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct DynamicRigidBodyID(u64);
}

define_component_type! {
    /// Identifier for a [`KinematicRigidBody`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct KinematicRigidBodyID(u64);
}

/// Identifier for a [`DynamicRigidBody`] or [`KinematicRigidBody`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypedRigidBodyID {
    Dynamic(DynamicRigidBodyID),
    Kinematic(KinematicRigidBodyID),
}

/// Manages and operates on dynamic and kinematic rigid bodies.
#[derive(Clone, Debug)]
pub struct RigidBodyManager {
    // TODO: separate vectors for disabled bodies
    dynamic_bodies: Vec<DynamicRigidBody>,
    kinematic_bodies: Vec<KinematicRigidBody>,
    dynamic_body_indices_by_id: KeyIndexMapper<DynamicRigidBodyID>,
    kinematic_body_indices_by_id: KeyIndexMapper<KinematicRigidBodyID>,
    dynamic_body_id_counter: u64,
    kinematic_body_id_counter: u64,
}

/// A rigid body whose motion is affected by the force and torque it experiences
/// as well as its inertial properties.
///
/// The body stores its linear and angular momentum rather than its linear and
/// angular velocity. The reason for this is that these are the conserved
/// quantities in free motion and thus should be the primary variables in the
/// simulation, with linear and angular velocity being derived from them (even
/// when left to rotate freely without torqe, the angular velocity will change
/// over time, while the angular momentum stays constant). This means that the
/// body's linear or angular momentum has to be updated whenever something
/// manually modifies the linear or angular velocity, respectively.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct DynamicRigidBody {
    mass: fph,
    inertia_tensor: InertiaTensor,
    position: Position,
    orientation: Orientation,
    momentum: Momentum,
    angular_momentum: AngularMomentum,
    total_force: Force,
    total_torque: Torque,
}

/// A rigid body whose linear and angular velocity only change when explicitly
/// modified. It does not have any inertial properties, and is not affected by
/// forces or torques.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct KinematicRigidBody {
    position: Position,
    orientation: Orientation,
    velocity: Velocity,
    angular_velocity: AngularVelocity,
}

impl From<DynamicRigidBodyID> for TypedRigidBodyID {
    fn from(id: DynamicRigidBodyID) -> Self {
        Self::Dynamic(id)
    }
}

impl From<KinematicRigidBodyID> for TypedRigidBodyID {
    fn from(id: KinematicRigidBodyID) -> Self {
        Self::Kinematic(id)
    }
}

impl RigidBodyManager {
    pub fn new() -> Self {
        Self {
            dynamic_bodies: Vec::new(),
            kinematic_bodies: Vec::new(),
            dynamic_body_indices_by_id: KeyIndexMapper::default(),
            kinematic_body_indices_by_id: KeyIndexMapper::default(),
            dynamic_body_id_counter: 0,
            kinematic_body_id_counter: 0,
        }
    }

    /// Returns a reference to the [`DynamicRigidBody`] with the given
    /// ID, or [`None`] if it does not exist.
    pub fn get_dynamic_rigid_body(&self, id: DynamicRigidBodyID) -> Option<&DynamicRigidBody> {
        let idx = self.dynamic_body_indices_by_id.get(id)?;
        Some(&self.dynamic_bodies[idx])
    }

    /// Returns a mutable reference to the [`DynamicRigidBody`] with the given
    /// ID, or [`None`] if it does not exist.
    pub fn get_dynamic_rigid_body_mut(
        &mut self,
        id: DynamicRigidBodyID,
    ) -> Option<&mut DynamicRigidBody> {
        let idx = self.dynamic_body_indices_by_id.get(id)?;
        Some(&mut self.dynamic_bodies[idx])
    }

    /// Returns mutable references to the two dynamic rigid bodies with the
    /// given IDs, or [`None`] if either of them does not exist.
    ///
    /// # Panics
    /// If the two IDs are equal.
    pub fn get_two_dynamic_rigid_bodies_mut(
        &mut self,
        id_1: DynamicRigidBodyID,
        id_2: DynamicRigidBodyID,
    ) -> Option<[&mut DynamicRigidBody; 2]> {
        assert_ne!(id_1, id_2);
        let idx_1 = self.dynamic_body_indices_by_id.get(id_1)?;
        let idx_2 = self.dynamic_body_indices_by_id.get(id_2)?;
        self.dynamic_bodies.get_disjoint_mut([idx_1, idx_2]).ok()
    }

    /// Returns a reference to the [`KinematicRigidBody`] with the given
    /// ID, or [`None`] if it does not exist.
    pub fn get_kinematic_rigid_body(
        &self,
        id: KinematicRigidBodyID,
    ) -> Option<&KinematicRigidBody> {
        let idx = self.kinematic_body_indices_by_id.get(id)?;
        Some(&self.kinematic_bodies[idx])
    }

    /// Returns a mutable reference to the [`KinematicRigidBody`] with the given
    /// ID, or [`None`] if it does not exist.
    pub fn get_kinematic_rigid_body_mut(
        &mut self,
        id: KinematicRigidBodyID,
    ) -> Option<&mut KinematicRigidBody> {
        let idx = self.kinematic_body_indices_by_id.get(id)?;
        Some(&mut self.kinematic_bodies[idx])
    }

    /// Returns a mutable reference to the specified dynamic rigid body along
    /// with an immutable reference to the specified kinematic rigid body, or
    /// [`None`] if either of them does not exist.
    pub fn get_dynamic_rigid_body_mut_and_kinematic_rigid_body(
        &mut self,
        dynamic_body_id: DynamicRigidBodyID,
        kinematic_body_id: KinematicRigidBodyID,
    ) -> Option<(&mut DynamicRigidBody, &KinematicRigidBody)> {
        let dynamic_body_idx = self.dynamic_body_indices_by_id.get(dynamic_body_id)?;
        let kinematic_body_idx = self.kinematic_body_indices_by_id.get(kinematic_body_id)?;
        Some((
            &mut self.dynamic_bodies[dynamic_body_idx],
            &self.kinematic_bodies[kinematic_body_idx],
        ))
    }

    /// Returns a reference to the [`DynamicRigidBody`] with the given ID.
    ///
    /// # Panics
    /// If no dynamic rigid body with the given ID exists.
    pub fn dynamic_rigid_body(&self, id: DynamicRigidBodyID) -> &DynamicRigidBody {
        self.get_dynamic_rigid_body(id)
            .expect("Requested missing dynamic rigid body")
    }

    /// Returns a mutable reference to the [`DynamicRigidBody`] with the given
    /// ID.
    ///
    /// # Panics
    /// If no dynamic rigid body with the given ID exists.
    pub fn dynamic_rigid_body_mut(&mut self, id: DynamicRigidBodyID) -> &mut DynamicRigidBody {
        self.get_dynamic_rigid_body_mut(id)
            .expect("Requested missing dynamic rigid body")
    }

    /// Returns a reference to the [`KinematicRigidBody`] with the given ID.
    ///
    /// # Panics
    /// If no kinematic rigid body with the given ID exists.
    pub fn kinematic_rigid_body(&self, id: KinematicRigidBodyID) -> &KinematicRigidBody {
        self.get_kinematic_rigid_body(id)
            .expect("Requested missing kinematic rigid body")
    }

    /// Returns a mutable reference to the [`KinematicRigidBody`] with the given
    /// ID.
    ///
    /// # Panics
    /// If no kinematic rigid body with the given ID exists.
    pub fn kinematic_rigid_body_mut(
        &mut self,
        id: KinematicRigidBodyID,
    ) -> &mut KinematicRigidBody {
        self.get_kinematic_rigid_body_mut(id)
            .expect("Requested missing kinematic rigid body")
    }

    /// Returns the slice of all dynamic rigid bodies.
    pub fn dynamic_rigid_bodies(&self) -> &[DynamicRigidBody] {
        &self.dynamic_bodies
    }

    /// Returns the slice of all kinematic rigid bodies.
    pub fn kinematic_rigid_bodies(&self) -> &[KinematicRigidBody] {
        &self.kinematic_bodies
    }

    /// Returns the mutable slice of all dynamic rigid bodies.
    pub fn dynamic_rigid_bodies_mut(&mut self) -> &mut [DynamicRigidBody] {
        &mut self.dynamic_bodies
    }

    /// Returns the mutable slice of all kinematic rigid bodies.
    pub fn kinematic_rigid_bodies_mut(&mut self) -> &mut [KinematicRigidBody] {
        &mut self.kinematic_bodies
    }

    /// Adds the given [`DynamicRigidBody`] to the manager.
    ///
    /// # Returns
    /// A new [`DynamicRigidBodyID`] referring to the added body.
    pub fn add_dynamic_rigid_body(&mut self, body: DynamicRigidBody) -> DynamicRigidBodyID {
        let id = self.create_new_dynamic_rigid_body_id();

        self.dynamic_bodies.push(body);
        self.dynamic_body_indices_by_id.push_key(id);

        id
    }

    /// Adds the given [`KinematicRigidBody`] to the manager.
    ///
    /// # Returns
    /// A new [`KinematicRigidBodyID`] referring to the added body.
    pub fn add_kinematic_rigid_body(&mut self, body: KinematicRigidBody) -> KinematicRigidBodyID {
        let id = self.create_new_kinematic_rigid_body_id();

        self.kinematic_bodies.push(body);
        self.kinematic_body_indices_by_id.push_key(id);

        id
    }

    /// Removes the [`DynamicRigidBody`] with the given ID from the m if it
    /// exists.
    pub fn remove_dynamic_rigid_body(&mut self, id: DynamicRigidBodyID) {
        if let Ok(idx) = self.dynamic_body_indices_by_id.try_swap_remove_key(id) {
            self.dynamic_bodies.swap_remove(idx);
        }
    }

    /// Removes the [`KinematicRigidBody`] with the given ID from the manager if
    /// it exists.
    pub fn remove_kinematic_rigid_body(&mut self, id: KinematicRigidBodyID) {
        if let Ok(idx) = self.kinematic_body_indices_by_id.try_swap_remove_key(id) {
            self.kinematic_bodies.swap_remove(idx);
        }
    }

    /// Resets the total applied force and torque on all dynamic rigid bodies to
    /// zero.
    pub fn reset_all_forces_and_torques(&mut self) {
        for body in &mut self.dynamic_bodies {
            body.reset_force_and_torque();
        }
    }

    /// Advances the linear and angular momentum of all dynamic rigid bodies.
    pub fn advance_dynamic_rigid_body_momenta(&mut self, step_duration: fph) {
        for body in &mut self.dynamic_bodies {
            body.advance_momentum(step_duration);
            body.advance_angular_momentum(step_duration);
        }
    }

    /// Advances the position and orientation of all dynamic rigid bodies.
    pub fn advance_dynamic_rigid_body_configurations(&mut self, step_duration: fph) {
        for body in &mut self.dynamic_bodies {
            body.advance_position(step_duration);
            body.advance_orientation(step_duration);
        }
    }

    /// Advances the position and orientation of all kinematic rigid bodies.
    pub fn advance_kinematic_rigid_body_configurations(&mut self, step_duration: fph) {
        for body in &mut self.kinematic_bodies {
            body.advance_position(step_duration);
            body.advance_orientation(step_duration);
        }
    }

    /// Removes all stored rigid bodies.
    pub fn clear(&mut self) {
        self.dynamic_bodies.clear();
        self.dynamic_body_indices_by_id.clear();
        self.kinematic_bodies.clear();
        self.kinematic_body_indices_by_id.clear();
    }

    fn create_new_dynamic_rigid_body_id(&mut self) -> DynamicRigidBodyID {
        let id = DynamicRigidBodyID(self.dynamic_body_id_counter);
        self.dynamic_body_id_counter = self.dynamic_body_id_counter.checked_add(1).unwrap();
        id
    }

    fn create_new_kinematic_rigid_body_id(&mut self) -> KinematicRigidBodyID {
        let id = KinematicRigidBodyID(self.kinematic_body_id_counter);
        self.kinematic_body_id_counter = self.kinematic_body_id_counter.checked_add(1).unwrap();
        id
    }
}

impl Default for RigidBodyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicRigidBody {
    /// Creates a new dynamic rigid body with the given properties.
    pub fn new(
        mass: fph,
        inertia_tensor: InertiaTensor,
        position: Position,
        orientation: Orientation,
        velocity: Velocity,
        angular_velocity: AngularVelocity,
    ) -> Self {
        let momentum = velocity * mass;
        let angular_momentum =
            quantities::compute_angular_momentum(&inertia_tensor, &orientation, &angular_velocity);
        Self {
            mass,
            inertia_tensor,
            position,
            orientation,
            momentum,
            angular_momentum,
            total_force: Force::zeros(),
            total_torque: Torque::zeros(),
        }
    }

    /// Returns the mass of the body.
    pub fn mass(&self) -> fph {
        self.mass
    }

    /// Returns the inertia tensor of the body.
    pub fn inertia_tensor(&self) -> &InertiaTensor {
        &self.inertia_tensor
    }

    /// Returns the position of the body.
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// Returns the orientation of the body.
    pub fn orientation(&self) -> &Orientation {
        &self.orientation
    }

    /// Returns the linear momentum of the body.
    pub fn momentum(&self) -> &Momentum {
        &self.momentum
    }

    /// Returns the angular momentum of the body.
    pub fn angular_momentum(&self) -> &AngularMomentum {
        &self.angular_momentum
    }

    /// Computes the velocity of the body.
    pub fn compute_velocity(&self) -> Velocity {
        self.momentum / self.mass
    }

    /// Computes the angular velocity of the body.
    pub fn compute_angular_velocity(&self) -> AngularVelocity {
        quantities::compute_angular_velocity(
            &self.inertia_tensor,
            &self.orientation,
            &self.angular_momentum,
        )
    }

    /// Returns the current total force on the body.
    pub fn total_force(&self) -> &Force {
        &self.total_force
    }

    /// Returns the current total torque on the body around the center of mass.
    pub fn total_torque(&self) -> &Torque {
        &self.total_torque
    }

    /// Transforms a vector from the body-fixed frame to world space.
    pub fn transform_vector_from_body_to_world_space(&self, vector: &Vector3<fph>) -> Vector3<fph> {
        transform_vector_from_body_to_world_space(&self.orientation, vector)
    }

    /// Transforms a vector from world space to the body-fixed frame.
    pub fn transform_vector_from_world_to_body_space(&self, vector: &Vector3<fph>) -> Vector3<fph> {
        transform_vector_from_world_to_body_space(&self.orientation, vector)
    }

    /// Transforms a point from the body-fixed frame to world space.
    pub fn transform_point_from_body_to_world_space(&self, point: &Point3<fph>) -> Point3<fph> {
        transform_point_from_body_to_world_space(&self.position, &self.orientation, point)
    }

    /// Transforms a point from world space to the body-fixed frame.
    pub fn transform_point_from_world_to_body_space(&self, point: &Point3<fph>) -> Point3<fph> {
        transform_point_from_world_to_body_space(&self.position, &self.orientation, point)
    }

    /// Computes the velocity of the given world space point on the body due to the
    /// body's linear and rotational motion.
    pub fn compute_velocity_of_attached_world_space_point(&self, point: &Point3<fph>) -> Velocity {
        compute_velocity_of_world_space_point_on_body(
            &self.position,
            &self.compute_velocity(),
            &self.compute_angular_velocity(),
            point,
        )
    }

    /// Returns the body's [`ReferenceFrame`].
    pub fn reference_frame(&self) -> ReferenceFrame {
        ReferenceFrame {
            position: self.position,
            orientation: self.orientation,
        }
    }

    /// Computes the body's [`Motion`].
    pub fn compute_motion(&self) -> Motion {
        Motion {
            linear_velocity: self.compute_velocity(),
            angular_velocity: self.compute_angular_velocity(),
        }
    }

    /// Applies the given force at the body's center of mass.
    pub fn apply_force_at_center_of_mass(&mut self, force: &Force) {
        self.total_force += force;
    }

    /// Applies the given torque around the body's center of mass.
    pub fn apply_torque(&mut self, torque: &Torque) {
        self.total_torque += torque;
    }

    /// Applies the given force at the given position. This may result in a
    /// torque around the center of mass.
    pub fn apply_force(&mut self, force: &Force, force_position: &Position) {
        self.apply_force_at_center_of_mass(force);
        self.apply_torque(&(force_position - self.position).cross(force));
    }

    /// Sets the given inertial properties for the body.
    pub fn set_inertial_properties(&mut self, mass: fph, inertia_tensor: InertiaTensor) {
        self.mass = mass;
        self.inertia_tensor = inertia_tensor;
    }

    /// Sets the given position for the body.
    pub fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    /// Sets the given orientation for the body.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;
    }

    /// Recomputes the body's linear momentum according to the given
    /// velocity.
    pub fn synchronize_momentum(&mut self, velocity: &Velocity) {
        self.momentum = velocity * self.mass();
    }

    /// Recomputes the body's angular momentum according to the given angular
    /// velocity.
    pub fn synchronize_angular_momentum(&mut self, angular_velocity: &AngularVelocity) {
        self.angular_momentum = quantities::compute_angular_momentum(
            &self.inertia_tensor,
            &self.orientation,
            angular_velocity,
        );
    }

    /// Advances the linear momentum of the body based on the total force
    /// applied to the body since
    /// [`reset_total_force`](Self::reset_total_force) was called.
    pub fn advance_momentum(&mut self, step_duration: fph) {
        self.momentum += self.total_force() * step_duration;
    }

    /// Advances the angular momentum of the body based on the total torque
    /// applied to the body since
    /// [`reset_total_torque`](Self::reset_total_torque) was called.
    pub fn advance_angular_momentum(&mut self, step_duration: fph) {
        self.angular_momentum += self.total_torque() * step_duration;
    }

    /// Advances the position of the body based on the current linear velocity.
    pub fn advance_position(&mut self, step_duration: fph) {
        let velocity = self.compute_velocity();
        self.position = advance_position(&self.position, &velocity, step_duration);
    }

    /// Advances the orientation of the body based on the current angular velocity.
    pub fn advance_orientation(&mut self, step_duration: fph) {
        let angular_velocity = self.compute_angular_velocity();
        self.orientation = advance_orientation(&self.orientation, &angular_velocity, step_duration);
    }

    /// Resets the total applied force and torque to zero.
    pub fn reset_force_and_torque(&mut self) {
        self.reset_total_force();
        self.reset_total_torque();
    }

    /// Resets the total applied force to zero.
    pub fn reset_total_force(&mut self) {
        self.total_force = Force::zeros();
    }

    /// Resets the total applied torque to zero.
    pub fn reset_total_torque(&mut self) {
        self.total_torque = Torque::zeros();
    }
}

impl AbsDiffEq for DynamicRigidBody {
    type Epsilon = <fph as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        fph::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        fph::abs_diff_eq(&self.mass, &other.mass, epsilon)
            && InertiaTensor::abs_diff_eq(&self.inertia_tensor, &other.inertia_tensor, epsilon)
            && Position::abs_diff_eq(&self.position, &other.position, epsilon)
            && Orientation::abs_diff_eq(&self.orientation, &other.orientation, epsilon)
            && Momentum::abs_diff_eq(&self.momentum, &other.momentum, epsilon)
            && AngularMomentum::abs_diff_eq(
                &self.angular_momentum,
                &other.angular_momentum,
                epsilon,
            )
            && Force::abs_diff_eq(&self.total_force, &other.total_force, epsilon)
            && Torque::abs_diff_eq(&self.total_torque, &other.total_torque, epsilon)
    }
}

impl KinematicRigidBody {
    /// Creates a new kinematic rigid body with the given properties.
    pub fn new(
        position: Position,
        orientation: Orientation,
        velocity: Velocity,
        angular_velocity: AngularVelocity,
    ) -> Self {
        Self {
            position,
            orientation,
            velocity,
            angular_velocity,
        }
    }

    /// Returns the position of the body.
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// Returns the orientation of the body.
    pub fn orientation(&self) -> &Orientation {
        &self.orientation
    }

    /// Returns the linear velocity of the body.
    pub fn velocity(&self) -> &Velocity {
        &self.velocity
    }

    /// Returns the angular velocity of the body.
    pub fn angular_velocity(&self) -> &AngularVelocity {
        &self.angular_velocity
    }

    /// Transforms a vector from the body-fixed frame to world space.
    pub fn transform_vector_from_body_to_world_space(&self, vector: &Vector3<fph>) -> Vector3<fph> {
        transform_vector_from_body_to_world_space(&self.orientation, vector)
    }

    /// Transforms a vector from world space to the body-fixed frame.
    pub fn transform_vector_from_world_to_body_space(&self, vector: &Vector3<fph>) -> Vector3<fph> {
        transform_vector_from_world_to_body_space(&self.orientation, vector)
    }

    /// Transforms a point from the body-fixed frame to world space.
    pub fn transform_point_from_body_to_world_space(&self, point: &Point3<fph>) -> Point3<fph> {
        transform_point_from_body_to_world_space(&self.position, &self.orientation, point)
    }

    /// Transforms a point from world space to the body-fixed frame.
    pub fn transform_point_from_world_to_body_space(&self, point: &Point3<fph>) -> Point3<fph> {
        transform_point_from_world_to_body_space(&self.position, &self.orientation, point)
    }

    /// Computes the velocity of the given world space point on the body due to the
    /// body's linear and rotational motion.
    pub fn compute_velocity_of_attached_world_space_point(&self, point: &Point3<fph>) -> Velocity {
        compute_velocity_of_world_space_point_on_body(
            &self.position,
            &self.velocity,
            &self.angular_velocity,
            point,
        )
    }

    /// Sets the given position for the body.
    pub fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    /// Sets the given orientation for the body.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;
    }

    /// Sets the given velocity for the body.
    pub fn set_velocity(&mut self, velocity: Velocity) {
        self.velocity = velocity;
    }

    /// Sets the given angular velocity for the body.
    pub fn set_angular_velocity(&mut self, angular_velocity: AngularVelocity) {
        self.angular_velocity = angular_velocity;
    }

    /// Advances the position of the body based on the current linear velocity.
    pub fn advance_position(&mut self, step_duration: fph) {
        self.position = advance_position(&self.position, &self.velocity, step_duration);
    }

    /// Advances the orientation of the body based on the current angular velocity.
    pub fn advance_orientation(&mut self, step_duration: fph) {
        self.orientation =
            advance_orientation(&self.orientation, &self.angular_velocity, step_duration);
    }
}

impl AbsDiffEq for KinematicRigidBody {
    type Epsilon = <fph as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        fph::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Position::abs_diff_eq(&self.position, &other.position, epsilon)
            && Orientation::abs_diff_eq(&self.orientation, &other.orientation, epsilon)
            && Velocity::abs_diff_eq(&self.velocity, &other.velocity, epsilon)
            && AngularVelocity::abs_diff_eq(
                &self.angular_velocity,
                &other.angular_velocity,
                epsilon,
            )
    }
}

/// Transforms a vector from the body-fixed frame to world space.
pub fn transform_vector_from_body_to_world_space(
    body_orientation: &Orientation,
    vector: &Vector3<fph>,
) -> Vector3<fph> {
    body_orientation.transform_vector(vector)
}

/// Transforms a vector from world space to the body-fixed frame.
pub fn transform_vector_from_world_to_body_space(
    body_orientation: &Orientation,
    vector: &Vector3<fph>,
) -> Vector3<fph> {
    body_orientation.inverse_transform_vector(vector)
}

/// Transforms a point from the body-fixed frame to world space.
pub fn transform_point_from_body_to_world_space(
    body_position: &Position,
    body_orientation: &Orientation,
    point: &Point3<fph>,
) -> Point3<fph> {
    body_position + body_orientation.transform_point(point).coords
}

/// Transforms a point from world space to the body-fixed frame.
pub fn transform_point_from_world_to_body_space(
    body_position: &Position,
    body_orientation: &Orientation,
    point: &Point3<fph>,
) -> Point3<fph> {
    body_orientation.inverse_transform_point(&Point3::from(point - body_position))
}

/// Computes the velocity of the given world space point on the body due to the
/// body's linear and rotational motion.
pub fn compute_velocity_of_world_space_point_on_body(
    body_position: &Position,
    body_velocity: &Velocity,
    body_angular_velocity: &AngularVelocity,
    point: &Position,
) -> Velocity {
    body_velocity
        + body_angular_velocity
            .as_vector()
            .cross(&(point - body_position))
}

/// Evolves the given [`Position`] linearly with the given [`Velocity`] for the
/// given duration.
pub fn advance_position(position: &Position, velocity: &Velocity, duration: fph) -> Position {
    position + velocity * duration
}

/// Evolves the given [`Orientation`] with the given [`AngularVelocity`] for the
/// given duration.
pub fn advance_orientation(
    orientation: &Orientation,
    angular_velocity: &AngularVelocity,
    duration: fph,
) -> Orientation {
    let angle = angular_velocity.angular_speed().radians() * duration;
    let (sin_half_angle, cos_half_angle) = (0.5 * angle).sin_cos();

    let rotation = Quaternion::from_parts(
        cos_half_angle,
        angular_velocity.axis_of_rotation().scale(sin_half_angle),
    );

    UnitQuaternion::new_normalize(rotation * orientation.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::{abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne};
    use impact_math::{Float, Radians};
    use nalgebra::{Vector3, point, vector};
    use proptest::prelude::*;

    prop_compose! {
        fn position_strategy(max_position_coord: fph)(
            position_coord_x in -max_position_coord..max_position_coord,
            position_coord_y in -max_position_coord..max_position_coord,
            position_coord_z in -max_position_coord..max_position_coord,
        ) -> Position {
            point![position_coord_x, position_coord_y, position_coord_z]
        }
    }

    prop_compose! {
        fn orientation_strategy()(
            rotation_roll in 0.0..fph::TWO_PI,
            rotation_pitch in -fph::FRAC_PI_2..fph::FRAC_PI_2,
            rotation_yaw in 0.0..fph::TWO_PI,
        ) -> Orientation {
            Orientation::from_euler_angles(rotation_roll, rotation_pitch, rotation_yaw)
        }
    }

    prop_compose! {
        fn force_strategy(max_force_coord: fph)(
            force_coord_x in -max_force_coord..max_force_coord,
            force_coord_y in -max_force_coord..max_force_coord,
            force_coord_z in -max_force_coord..max_force_coord,
        ) -> Force {
            vector![force_coord_x, force_coord_y, force_coord_z]
        }
    }

    prop_compose! {
        fn torque_strategy(max_torque_coord: fph)(
            torque_coord_x in -max_torque_coord..max_torque_coord,
            torque_coord_y in -max_torque_coord..max_torque_coord,
            torque_coord_z in -max_torque_coord..max_torque_coord,
        ) -> Force {
            vector![torque_coord_x, torque_coord_y, torque_coord_z]
        }
    }

    fn dummy_dynamic_rigid_body() -> DynamicRigidBody {
        DynamicRigidBody::new(
            1.0,
            InertiaTensor::identity(),
            Position::origin(),
            Orientation::identity(),
            Velocity::zeros(),
            AngularVelocity::zero(),
        )
    }

    #[test]
    fn should_get_zero_force_and_torque_for_new_dynamic_body() {
        let body = dummy_dynamic_rigid_body();
        assert_abs_diff_eq!(body.total_force(), &Force::zeros());
        assert_abs_diff_eq!(body.total_torque(), &Torque::zeros());
    }

    proptest! {
        #[test]
        fn should_add_forces_applied_at_center_of_mass(
            force_1 in force_strategy(1e3),
            force_2 in force_strategy(1e3),
        ) {
            let mut body = dummy_dynamic_rigid_body();
            body.apply_force_at_center_of_mass(&force_1);
            body.apply_force_at_center_of_mass(&force_2);
            prop_assert!(abs_diff_eq!(body.total_force(), &(force_1 + force_2)));
        }
    }

    proptest! {
        #[test]
        fn should_add_forces_applied_anywhere(
            force_1 in force_strategy(1e3),
            force_2 in force_strategy(1e3),
            force_position_1 in position_strategy(1e3),
            force_position_2 in position_strategy(1e3),
        ) {
            let mut body = dummy_dynamic_rigid_body();
            body.apply_force(&force_1, &force_position_1);
            body.apply_force(&force_2, &force_position_2);
            prop_assert!(abs_diff_eq!(body.total_force(), &(force_1 + force_2)));
        }
    }

    proptest! {
        #[test]
        fn should_add_torques_applied_around_center_of_mass(
            torque_1 in torque_strategy(1e3),
            torque_2 in torque_strategy(1e3),
        ) {
            let mut body = dummy_dynamic_rigid_body();
            body.apply_torque(&torque_1);
            body.apply_torque(&torque_2);
            prop_assert!(abs_diff_eq!(body.total_torque(), &(torque_1 + torque_2)));
        }
    }

    proptest! {
        #[test]
        fn should_get_torque_from_applying_force_outside_center_of_mass(
            force in force_strategy(1e3),
            force_position in position_strategy(1e3),
        ) {
            let mut body = dummy_dynamic_rigid_body();
            body.apply_force(&force, &force_position);
            prop_assert!(abs_diff_eq!(
                body.total_torque(),
                &((force_position - body.position()).cross(&force))
            ));
        }
    }

    #[test]
    fn should_retain_dynamic_body_velocities_when_advancing_for_zero_time() {
        let velocity = Velocity::z();
        let angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = DynamicRigidBody::new(
            1.0,
            InertiaTensor::identity(),
            Position::origin(),
            Orientation::identity(),
            velocity,
            angular_velocity,
        );

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        body.advance_momentum(0.0);
        assert_abs_diff_eq!(body.compute_velocity(), velocity);

        body.advance_angular_momentum(0.0);
        assert_abs_diff_eq!(
            body.compute_angular_velocity(),
            angular_velocity,
            epsilon = 1e-9
        );
    }

    #[test]
    fn should_retain_dynamic_body_velocities_with_zero_force() {
        let velocity = Velocity::zeros();
        let angular_velocity = AngularVelocity::zero();

        let mut body = DynamicRigidBody::new(
            1.0,
            InertiaTensor::identity(),
            Position::origin(),
            Orientation::identity(),
            velocity,
            angular_velocity,
        );

        body.advance_momentum(1.0);
        assert_abs_diff_eq!(body.compute_velocity(), velocity);

        body.advance_angular_momentum(1.0);
        assert_abs_diff_eq!(
            body.compute_angular_velocity(),
            angular_velocity,
            epsilon = 1e-9
        );
    }

    #[test]
    fn should_change_dynamic_body_velocities_with_nonzero_force_and_torque() {
        let position = Position::origin();
        let orientation = Orientation::identity();
        let velocity = Velocity::z();
        let angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = DynamicRigidBody::new(
            1.0,
            InertiaTensor::identity(),
            position,
            orientation,
            velocity,
            angular_velocity,
        );

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        body.advance_momentum(1.0);
        assert_abs_diff_ne!(body.compute_velocity(), velocity);

        body.advance_angular_momentum(1.0);
        assert_abs_diff_ne!(
            body.compute_angular_velocity(),
            angular_velocity,
            epsilon = 1e-9
        );
    }

    #[test]
    fn advancing_orientation_with_zero_angular_speed_gives_same_orientation() {
        let orientation = Orientation::identity();
        let angular_velocity = AngularVelocity::new(Vector3::x_axis(), Radians(0.0));
        let advanced_orientation = advance_orientation(&orientation, &angular_velocity, 1.2);
        assert_abs_diff_eq!(advanced_orientation, orientation);
    }

    #[test]
    fn advancing_orientation_by_zero_duration_gives_same_orientation() {
        let orientation = Orientation::identity();
        let angular_velocity = AngularVelocity::new(Vector3::x_axis(), Radians(1.2));
        let advanced_orientation = advance_orientation(&orientation, &angular_velocity, 0.0);
        assert_abs_diff_eq!(advanced_orientation, orientation);
    }

    #[test]
    fn advancing_orientation_about_its_own_axis_works() {
        let angular_speed = 0.1;
        let duration = 2.0;
        let orientation = Orientation::from_axis_angle(&Vector3::y_axis(), 0.1);
        let angular_velocity = AngularVelocity::new(Vector3::y_axis(), Radians(angular_speed));
        let advanced_orientation = advance_orientation(&orientation, &angular_velocity, duration);
        assert_abs_diff_eq!(
            advanced_orientation.angle(),
            orientation.angle() + angular_speed * duration,
            epsilon = 1e-8,
        );
        assert_abs_diff_eq!(
            advanced_orientation.axis().unwrap(),
            orientation.axis().unwrap(),
            epsilon = 1e-8,
        );
    }
}
