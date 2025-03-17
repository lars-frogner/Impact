//! Constraint solving.

use super::{ConstrainedBody, PreparedTwoBodyConstraint, TwoBodyConstraint};
use crate::physics::{
    constraint::{
        contact::{Contact, PreparedContact},
        spherical_joint::{PreparedSphericalJoint, SphericalJoint},
    },
    fph,
    motion::{
        AngularVelocity,
        components::{ReferenceFrameComp, Static, VelocityComp},
    },
    rigid_body::components::RigidBodyComp,
};
use impact_ecs::world::{Entity, World as ECSWorld};
use impact_utils::KeyIndexMapper;
use std::ops::Deref;

#[derive(Clone, Debug)]
pub struct ConstraintSolver {
    config: ConstraintSolverConfig,
    bodies: Vec<ConstrainedBody>,
    body_index_map: KeyIndexMapper<Entity>,
    spherical_joints: Vec<BodyPairConstraint<PreparedSphericalJoint>>,
    contacts: Vec<BodyPairConstraint<PreparedContact>>,
}

#[derive(Clone, Debug)]
pub struct ConstraintSolverConfig {
    pub n_iterations: u32,
}

#[derive(Clone, Debug)]
struct SingleBodyConstraint<T> {
    body_idx: usize,
    accumulated_impulse: fph,
    inner: T,
}

#[derive(Clone, Debug)]
struct BodyPairConstraint<T> {
    body_a_idx: usize,
    body_b_idx: usize,
    accumulated_impulse: fph,
    inner: T,
}

impl ConstraintSolver {
    pub fn new(config: ConstraintSolverConfig) -> Self {
        Self {
            config,
            bodies: Vec::new(),
            body_index_map: KeyIndexMapper::new(),
            spherical_joints: Vec::new(),
            contacts: Vec::new(),
        }
    }

    pub fn prepare_spherical_joint(&mut self, ecs_world: &ECSWorld, joint: &SphericalJoint) {
        if let Some(prepared_joint) = self.prepare_constraint_for_body_pair(
            ecs_world,
            joint.body_a_entity,
            joint.body_b_entity,
            joint,
        ) {
            self.spherical_joints.push(prepared_joint);
        }
    }

    pub fn prepare_contact(
        &mut self,
        ecs_world: &ECSWorld,
        body_a_entity: Entity,
        body_b_entity: Entity,
        contact: &Contact,
    ) {
        if let Some(prepared_contact) =
            self.prepare_constraint_for_body_pair(ecs_world, body_a_entity, body_b_entity, contact)
        {
            self.contacts.push(prepared_contact);
        }
    }

    pub fn compute_constrained_velocities(&mut self) {
        for _ in 0..self.config.n_iterations {
            apply_impulses_sequentially_for_body_pair_constraints(
                &mut self.bodies,
                &mut self.spherical_joints,
            );
            apply_impulses_sequentially_for_body_pair_constraints(
                &mut self.bodies,
                &mut self.contacts,
            );
        }
    }

    pub fn apply_constrained_velocities(&self, ecs_world: &ECSWorld) {
        for (body_entity, body) in self.body_index_map.key_at_each_idx().zip(&self.bodies) {
            apply_body_velocities(ecs_world, body_entity, body);
        }
    }

    pub fn clear_prepared_state(&mut self) {
        self.bodies.clear();
        self.body_index_map.clear();
        self.spherical_joints.clear();
        self.contacts.clear();
    }

    fn prepare_constraint_for_body_pair<C: TwoBodyConstraint>(
        &mut self,
        ecs_world: &ECSWorld,
        body_a_entity: Entity,
        body_b_entity: Entity,
        constraint: &C,
    ) -> Option<BodyPairConstraint<C::Prepared>> {
        let (body_a_idx, body_b_idx) =
            self.prepare_body_pair(ecs_world, body_a_entity, body_b_entity)?;

        let prepared_constraint = constraint.prepare(
            ecs_world,
            &body_a_entity,
            &body_b_entity,
            &self.bodies[body_a_idx],
            &self.bodies[body_b_idx],
        );

        Some(BodyPairConstraint {
            body_a_idx,
            body_b_idx,
            accumulated_impulse: 0.0,
            inner: prepared_constraint,
        })
    }

    fn prepare_constraint_for_body<C>(
        &mut self,
        ecs_world: &ECSWorld,
        entity: Entity,
        prepare_constraint: impl FnOnce(&ConstrainedBody) -> C,
    ) -> Option<SingleBodyConstraint<C>> {
        let body_idx = self.prepare_body(ecs_world, entity)?;

        let constraint = prepare_constraint(&self.bodies[body_idx]);

        Some(SingleBodyConstraint {
            body_idx,
            accumulated_impulse: 0.0,
            inner: constraint,
        })
    }

    fn prepare_body_pair(
        &mut self,
        ecs_world: &ECSWorld,
        body_a_entity: Entity,
        body_b_entity: Entity,
    ) -> Option<(usize, usize)> {
        let body_a_idx = self.prepare_body(ecs_world, body_a_entity)?;
        let body_b_idx = self.prepare_body(ecs_world, body_b_entity)?;
        Some((body_a_idx, body_b_idx))
    }

    fn prepare_body(&mut self, ecs_world: &ECSWorld, body_entity: Entity) -> Option<usize> {
        if let Some(body_idx) = self.body_index_map.get(body_entity) {
            return Some(body_idx);
        }

        let entry = ecs_world.get_entity(&body_entity)?;

        let frame = entry.get_component::<ReferenceFrameComp>()?;

        let constrained_body = match entry.get_component::<RigidBodyComp>() {
            Some(rigid_body) if !entry.has_component::<Static>() => {
                let velocity = entry
                    .get_component::<VelocityComp>()
                    .map_or_else(VelocityComp::default, |velocity| *velocity.access());

                ConstrainedBody::from_rigid_body_components(
                    frame.access(),
                    &velocity,
                    rigid_body.access(),
                )
            }
            _ => ConstrainedBody::from_static_body_components(frame.access()),
        };

        let body_idx = self.bodies.len();
        self.bodies.push(constrained_body);
        self.body_index_map.push_key(body_entity);

        Some(body_idx)
    }
}

impl Default for ConstraintSolverConfig {
    fn default() -> Self {
        Self { n_iterations: 5 }
    }
}

impl<T> Deref for SingleBodyConstraint<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Deref for BodyPairConstraint<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

fn apply_impulses_sequentially_for_body_pair_constraints<P: PreparedTwoBodyConstraint>(
    bodies: &mut [ConstrainedBody],
    constraints: &mut [BodyPairConstraint<P>],
) {
    for constraint in constraints {
        let (body_a, body_b) =
            two_mutable_elements(bodies, constraint.body_a_idx, constraint.body_b_idx);

        let corrective_impulse = constraint.compute_scalar_impulse(body_a, body_b);

        let old_accumulated_impulse = constraint.accumulated_impulse;
        constraint.accumulated_impulse += corrective_impulse;
        constraint.accumulated_impulse =
            constraint.clamp_scalar_impulse(constraint.accumulated_impulse);
        let clamped_corrective_impulse = constraint.accumulated_impulse - old_accumulated_impulse;

        constraint.apply_scalar_impulse_to_body_pair(body_a, body_b, clamped_corrective_impulse);
    }
}

fn apply_body_velocities(ecs_world: &ECSWorld, body_entity: Entity, body: &ConstrainedBody) {
    let Some(entry) = ecs_world.get_entity(&body_entity) else {
        return;
    };

    let Some(frame) = entry.get_component::<ReferenceFrameComp>() else {
        return;
    };
    let frame = frame.access();

    let Some(mut velocity) = entry.get_component_mut::<VelocityComp>() else {
        return;
    };
    let velocity = velocity.access();
    velocity.linear = body.velocity;
    velocity.angular = AngularVelocity::from_vector(body.angular_velocity);

    let Some(mut rigid_body) = entry.get_component_mut::<RigidBodyComp>() else {
        return;
    };
    let rigid_body = rigid_body.access();
    rigid_body.0.synchronize_momentum(&velocity.linear);
    rigid_body
        .0
        .synchronize_angular_momentum(&frame.orientation, frame.scaling, &velocity.angular);
}

fn two_mutable_elements<T>(values: &mut [T], idx_a: usize, idx_b: usize) -> (&mut T, &mut T) {
    assert_ne!(idx_a, idx_b);

    if idx_b > idx_a {
        let (left, right) = values.split_at_mut(idx_b);
        (&mut left[idx_a], &mut right[0])
    } else {
        let (left, right) = values.split_at_mut(idx_a);
        (&mut right[0], &mut left[idx_b])
    }
}
