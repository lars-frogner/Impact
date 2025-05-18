//! Constraints on rigid bodies.

pub mod contact;
pub mod solver;
pub mod spherical_joint;

use crate::{
    physics::{
        collision::{Collision, CollisionWorld},
        fph,
        motion::{
            Orientation, Position, Velocity,
            components::{ReferenceFrameComp, VelocityComp},
        },
        rigid_body::components::RigidBodyComp,
    },
    voxel::VoxelObjectManager,
};
use bytemuck::{Pod, Zeroable};
use contact::Contact;
use impact_ecs::world::{EntityID, World as ECSWorld};
use nalgebra::{Matrix3, Vector3};
use num_traits::Zero;
use solver::{ConstraintSolver, ConstraintSolverConfig};
use spherical_joint::SphericalJoint;
use std::{
    collections::HashMap,
    fmt,
    ops::{Add, Mul, Sub},
};

/// Identifier for a constraint in a [`ConstraintManager`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ConstraintID(u32);

/// Manages all constraints in the simulation.
#[derive(Debug, Clone)]
pub struct ConstraintManager {
    solver: ConstraintSolver,
    spherical_joints: HashMap<ConstraintID, SphericalJoint>,
    constraint_id_counter: u32,
}

/// Represents a constraint involving two rigid bodies.
trait TwoBodyConstraint {
    type Prepared: PreparedTwoBodyConstraint;

    /// Creates an instantiation of the constraint that has been prepared for
    /// constraint solving in the current frame.
    fn prepare(
        &self,
        ecs_world: &ECSWorld,
        body_a_entity_id: EntityID,
        body_b_entity_id: EntityID,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> Self::Prepared;
}

/// Represents a [`TwoBodyConstraint`] that has been prepared for constraint
/// solving in the current frame.
trait PreparedTwoBodyConstraint {
    type Impulses: fmt::Debug
        + Copy
        + Zero
        + Add<Output = Self::Impulses>
        + Sub<Output = Self::Impulses>
        + Mul<fph, Output = Self::Impulses>;

    /// Whether the accumulated [`Impulses`] from the other constraint can be
    /// used to kick start the solution of this constraint. It should be
    /// assumed that the given constraint involves the same entities as this
    /// constraint.
    fn can_use_warm_impulses_from(&self, other: &Self) -> bool;

    /// Computes the corrective impulses that should be applied to the bodies
    /// in order to satisfy the velocity constraint. This method should not
    /// perform clamping.
    fn compute_impulses(
        &self,
        body_a: &ConstrainedBody,
        body_b: &ConstrainedBody,
    ) -> Self::Impulses;

    /// Clamps the given impulses to satisfy the inequality velocity
    /// constraints.
    fn clamp_impulses(&self, impulses: Self::Impulses) -> Self::Impulses;

    /// Applies the given impulses to the velcities of the two bodies.
    fn apply_impulses_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        impulses: Self::Impulses,
    );

    /// Computes and applies pseudo impulses to the position and orientation
    /// of the bodies to satisfy the position constraint.
    fn apply_positional_correction_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        correction_factor: fph,
    );
}

/// The relevant properties and state of a rigid body required for constraint
/// solving. The state is updated iteratively as constraints are being solved.
#[derive(Clone, Debug)]
struct ConstrainedBody {
    /// Inverse of the body's mass.
    pub inverse_mass: fph,
    /// Inverse of the body's inertia tensor (in world space).
    pub inverse_inertia_tensor: Matrix3<fph>,
    /// Position of the body's center of mass (in world space).
    pub position: Position,
    /// Orientation of the body's reference frame (in world space).
    pub orientation: Orientation,
    /// Linear velocity of the body' center of mass (in world space).
    pub velocity: Velocity,
    /// Angular velocity of the body about its center of mass (in world space).
    pub angular_velocity: Vector3<fph>,
}

impl ConstraintManager {
    /// Creates a new constraint manager with the given configuration for the
    /// [`ConstraintSolver`].
    pub fn new(solver_config: ConstraintSolverConfig) -> Self {
        Self {
            solver: ConstraintSolver::new(solver_config),
            spherical_joints: HashMap::new(),
            constraint_id_counter: 0,
        }
    }

    pub fn solver(&self) -> &ConstraintSolver {
        &self.solver
    }

    pub fn solver_mut(&mut self) -> &mut ConstraintSolver {
        &mut self.solver
    }

    /// Registers the given spherical joint constraint and returns a new ID
    /// that can be used to refer to the constraint.
    pub fn add_spherical_joint(&mut self, joint: SphericalJoint) -> ConstraintID {
        let id = self.create_new_constraint_id();
        self.spherical_joints.insert(id, joint);
        id
    }

    /// Prepares for solving the constraints for the current frame by gathering
    /// all relevant rigid body state and precomputing relevant constraint
    /// quantities. Should be called before advancing rigid body velocities and
    /// configurations for the frame.
    pub fn prepare_constraints(
        &mut self,
        ecs_world: &ECSWorld,
        voxel_object_manager: &VoxelObjectManager,
        collision_world: &CollisionWorld,
    ) {
        // The cached states of the bodies from the previous frame are stale
        // and must be removed. Up-to-date body state will be gathered as
        // required for the constraints of this frame.
        self.solver.clear_prepared_bodies();

        collision_world.for_each_non_phantom_collision_involving_dynamic_collidable(
            voxel_object_manager,
            &mut |Collision {
                      collider_a,
                      collider_b,
                      contact_manifold,
                  }| {
                for contact in contact_manifold.contacts() {
                    self.solver.prepare_contact(
                        ecs_world,
                        collider_a.entity_id(),
                        collider_b.entity_id(),
                        contact,
                    );
                }
            },
        );

        for (id, joint) in &self.spherical_joints {
            self.solver.prepare_spherical_joint(ecs_world, *id, joint);
        }

        // Any constraints left over from the previous frame that were not
        // prepared again for this frame must be removed
        self.solver.remove_unprepared_constraints();
    }

    /// For testing and benchmarking contact resolution.
    pub fn prepare_specific_contacts_only<'a>(
        &mut self,
        ecs_world: &ECSWorld,
        contacts: impl IntoIterator<Item = (EntityID, EntityID, &'a Contact)>,
    ) {
        self.solver.clear_prepared_bodies();

        for (body_a_entity_id, body_b_entity_id, contact) in contacts {
            self.solver
                .prepare_contact(ecs_world, body_a_entity_id, body_b_entity_id, contact);
        }

        self.solver.remove_unprepared_constraints();
    }

    /// Executes constraint solving. As opposed to
    /// [`Self::prepare_constraints`], this method should be called after
    /// advancing all rigid body velocities (but not configurations) based on
    /// the non-constraint forces. The prepared bodies will be updated with the
    /// advanced velocities before constraint solving. After computing the
    /// resolved velocities and configurations, these will then be applied to
    /// the rigid body entities.
    pub fn compute_and_apply_constrained_state(&mut self, ecs_world: &ECSWorld) {
        self.solver
            .synchronize_prepared_body_velocities_with_entity_velocities(ecs_world);
        self.solver.compute_constrained_velocities();
        self.solver.compute_corrected_configurations();
        self.solver
            .apply_constrained_velocities_and_corrected_configurations(ecs_world);
    }

    fn create_new_constraint_id(&mut self) -> ConstraintID {
        let constraint_id = ConstraintID(self.constraint_id_counter);
        self.constraint_id_counter = self.constraint_id_counter.checked_add(1).unwrap();
        constraint_id
    }
}

impl ConstrainedBody {
    fn from_rigid_body_components(
        frame: &ReferenceFrameComp,
        velocity: &VelocityComp,
        rigid_body: &RigidBodyComp,
    ) -> Self {
        let inverse_inertia_tensor = rigid_body
            .0
            .inertial_properties()
            .inertia_tensor()
            .inverse_rotated_matrix_with_scaled_extent(&frame.orientation, frame.scaling);

        Self {
            inverse_mass: rigid_body.0.mass().recip(),
            inverse_inertia_tensor,
            position: frame.position,
            orientation: frame.orientation,
            velocity: velocity.linear,
            angular_velocity: velocity.angular.as_vector(),
        }
    }

    /// These are bodies whose motion is not dictated by forces or rigid body
    /// constraints. We can treat them as having infinite mass.
    fn from_kinematic_body_components(frame: &ReferenceFrameComp, velocity: &VelocityComp) -> Self {
        Self {
            inverse_mass: 0.0,
            inverse_inertia_tensor: Matrix3::zeros(),
            position: frame.position,
            orientation: frame.orientation,
            velocity: velocity.linear,
            angular_velocity: velocity.angular.as_vector(),
        }
    }

    /// Transforms the given point to world space from the coordinate system
    /// that moves and rotates with the rigid body, with its origin at the
    /// body's center of mass.
    fn transform_point_from_body_to_world_frame(&self, point: &Position) -> Position {
        self.orientation.transform_point(point) + self.position.coords
    }

    /// Transforms the given point from world space to the coordinate system
    /// that moves and rotates with the rigid body, with its origin at the
    /// body's center of mass.
    fn transform_point_from_world_to_body_frame(&self, point: &Position) -> Position {
        self.orientation
            .inverse_transform_point(&(point - self.position.coords))
    }
}
