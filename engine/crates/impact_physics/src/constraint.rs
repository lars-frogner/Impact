//! Constraints on rigid bodies.

pub mod contact;
pub mod solver;
pub mod spherical_joint;

use crate::{
    collision::{Collidable, Collision, CollisionWorld},
    fph,
    quantities::{Orientation, Position, Velocity},
    rigid_body::{DynamicRigidBody, KinematicRigidBody, RigidBodyID, RigidBodyManager},
};
use bytemuck::{Pod, Zeroable};
use contact::ContactWithID;
use impact_containers::HashMap;
use nalgebra::{Matrix3, Vector3};
use num_traits::Zero;
use solver::{ConstraintSolver, ConstraintSolverConfig};
use spherical_joint::SphericalJoint;
use std::{
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
    fn prepare(&self, body_a: &ConstrainedBody, body_b: &ConstrainedBody) -> Self::Prepared;
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

    /// Whether the accumulated [`Self::Impulses`] from the other constraint can
    /// be used to kick start the solution of this constraint. It should be
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
            spherical_joints: HashMap::default(),
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
    /// quantities. Should be called before advancing dynamic rigid body
    /// velocities and configurations for the frame.
    pub fn prepare_constraints<C: Collidable>(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        collision_world: &CollisionWorld<C>,
        collidable_context: &C::Context,
    ) {
        // The cached states of the bodies from the previous frame are stale
        // and must be removed. Up-to-date body state will be gathered as
        // required for the constraints of this frame.
        self.solver.clear_prepared_bodies();

        collision_world.for_each_non_phantom_collision_involving_dynamic_collidable(
            collidable_context,
            &mut |Collision {
                      collider_a,
                      collider_b,
                      contact_manifold,
                  }| {
                for contact in contact_manifold.contacts() {
                    self.solver.prepare_contact(
                        rigid_body_manager,
                        collider_a.rigid_body_id(),
                        collider_b.rigid_body_id(),
                        contact,
                    );
                }
            },
        );

        for (id, joint) in &self.spherical_joints {
            self.solver
                .prepare_spherical_joint(rigid_body_manager, *id, joint);
        }

        // Any constraints left over from the previous frame that were not
        // prepared again for this frame must be removed
        self.solver.remove_unprepared_constraints();
    }

    /// For testing and benchmarking contact resolution.
    pub fn prepare_specific_contacts_only<'a>(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        contacts: impl IntoIterator<Item = (RigidBodyID, RigidBodyID, &'a ContactWithID)>,
    ) {
        self.solver.clear_prepared_bodies();

        for (rigid_body_a_id, rigid_body_b_id, contact) in contacts {
            self.solver.prepare_contact(
                rigid_body_manager,
                rigid_body_a_id,
                rigid_body_b_id,
                contact,
            );
        }

        self.solver.remove_unprepared_constraints();
    }

    /// Executes constraint solving. As opposed to
    /// [`Self::prepare_constraints`], this method should be called after
    /// advancing all rigid body velocities (but not configurations) based on
    /// the non-constraint forces. The prepared constrained bodies will be
    /// updated with the advanced velocities before constraint solving. After
    /// computing the resolved velocities and configurations, these will then be
    /// applied to the rigid bodies.
    pub fn compute_and_apply_constrained_state(
        &mut self,
        rigid_body_manager: &mut RigidBodyManager,
    ) {
        self.solver
            .synchronize_prepared_constrained_body_velocities(rigid_body_manager);
        self.solver.compute_constrained_velocities();
        self.solver.compute_corrected_configurations();
        self.solver
            .apply_constrained_velocities_and_corrected_configurations(rigid_body_manager);
    }

    /// Removes all stored constraint state.
    pub fn clear(&mut self) {
        self.solver.clear();
        self.spherical_joints.clear();
    }

    fn create_new_constraint_id(&mut self) -> ConstraintID {
        let constraint_id = ConstraintID(self.constraint_id_counter);
        self.constraint_id_counter = self.constraint_id_counter.checked_add(1).unwrap();
        constraint_id
    }
}

impl ConstrainedBody {
    fn from_dynamic_rigid_body(body: &DynamicRigidBody) -> Self {
        let inverse_inertia_tensor = body
            .inertia_tensor()
            .inverse_rotated_matrix(body.orientation());

        Self {
            inverse_mass: body.mass().recip(),
            inverse_inertia_tensor,
            position: *body.position(),
            orientation: *body.orientation(),
            velocity: body.compute_velocity(),
            angular_velocity: body.compute_angular_velocity().as_vector(),
        }
    }

    /// We can treat kinematic bodies as having infinite mass.
    fn from_kinematic_rigid_body(body: &KinematicRigidBody) -> Self {
        Self {
            inverse_mass: 0.0,
            inverse_inertia_tensor: Matrix3::zeros(),
            position: *body.position(),
            orientation: *body.orientation(),
            velocity: *body.velocity(),
            angular_velocity: body.angular_velocity().as_vector(),
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
