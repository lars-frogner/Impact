//! Hard constraints on rigid bodies.

pub(super) mod contact;
mod solver;
pub mod spherical_joint;

use crate::physics::{
    collision::CollisionWorld,
    fph,
    motion::{
        Orientation, Position, Velocity,
        components::{ReferenceFrameComp, VelocityComp},
    },
    rigid_body::components::RigidBodyComp,
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::world::World as ECSWorld;
use nalgebra::{Matrix3, Vector3};
use solver::ConstraintSolver;
use spherical_joint::SphericalJoint;
use std::{collections::HashMap, sync::RwLock};

use super::collision::Collision;

/// Identifier for a constraint in a [`ConstraintManager`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct ConstraintID(u32);

#[derive(Debug)]
pub struct ConstraintManager {
    solver: RwLock<ConstraintSolver>,
    spherical_joints: HashMap<ConstraintID, SphericalJoint>,
    constraint_id_counter: u32,
}

trait TwoBodyConstraint {
    type Prepared: PreparedTwoBodyConstraint;

    fn prepare(&self, body_a: &ConstrainedBody, body_b: &ConstrainedBody) -> Self::Prepared;
}

trait PreparedTwoBodyConstraint {
    fn compute_scalar_impulse(&self, body_a: &ConstrainedBody, body_b: &ConstrainedBody) -> fph;

    fn clamp_scalar_impulse(&self, scalar_impulse: fph) -> fph;

    fn apply_scalar_impulse_to_body_pair(
        &self,
        body_a: &mut ConstrainedBody,
        body_b: &mut ConstrainedBody,
        scalar_impulse: fph,
    );
}

/// All quantities are in world space.
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
    pub fn new() -> Self {
        Self {
            solver: RwLock::new(ConstraintSolver::new()),
            spherical_joints: HashMap::new(),
            constraint_id_counter: 0,
        }
    }

    pub fn add_spherical_joint(&mut self, joint: SphericalJoint) -> ConstraintID {
        let id = self.create_new_constraint_id();
        self.spherical_joints.insert(id, joint);
        id
    }

    pub(super) fn prepare_constraints(
        &self,
        ecs_world: &ECSWorld,
        collision_world: &CollisionWorld,
    ) {
        let mut solver = self.solver.write().unwrap();
        solver.clear_prepared_state();

        for joint in self.spherical_joints.values() {
            solver.prepare_spherical_joint(ecs_world, joint);
        }

        collision_world.for_each_non_phantom_collision_involving_dynamic_collidable(
            &mut |Collision {
                      collider_a,
                      collider_b,
                      contact_set,
                  }| {
                for contact in contact_set.contacts() {
                    solver.prepare_contact(
                        ecs_world,
                        collider_a.entity(),
                        collider_b.entity(),
                        contact,
                    );
                }
            },
        );
    }

    pub(super) fn compute_and_apply_constrained_velocities(&self, ecs_world: &ECSWorld) {
        let mut solver = self.solver.write().unwrap();
        solver.compute_constrained_velocities();
        solver.apply_constrained_velocities(ecs_world);
    }

    fn create_new_constraint_id(&mut self) -> ConstraintID {
        let constraint_id = ConstraintID(self.constraint_id_counter);
        self.constraint_id_counter = self.constraint_id_counter.checked_add(1).unwrap();
        constraint_id
    }
}

impl Default for ConstraintManager {
    fn default() -> Self {
        Self::new()
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

    fn from_static_body_components(frame: &ReferenceFrameComp) -> Self {
        Self {
            inverse_mass: 0.0,
            inverse_inertia_tensor: Matrix3::zeros(),
            position: frame.position,
            orientation: frame.orientation,
            velocity: Velocity::zeros(),
            angular_velocity: Vector3::zeros(),
        }
    }
}
