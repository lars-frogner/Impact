//! Constraint solving based on the sequential impulse method.

use super::{
    AnchoredTwoBodyConstraint, ConstrainedBody, PreparedTwoBodyConstraint, TwoBodyConstraint,
    contact::ContactWithID,
};
use crate::{
    anchor::AnchorManager,
    constraint::{
        ConstraintID,
        contact::{ContactID, PreparedContact},
        spherical_joint::{PreparedSphericalJoint, SphericalJoint},
    },
    quantities::AngularVelocity,
    rigid_body::{RigidBodyManager, TypedRigidBodyID},
};
use bitflags::bitflags;
use impact_containers::KeyIndexMapper;
use num_traits::Zero;
use std::{
    fmt,
    hash::Hash,
    ops::{Deref, DerefMut},
};

/// A Sequential Impulse constraint solver.
#[derive(Clone, Debug)]
pub struct ConstraintSolver {
    bodies: Vec<ConstrainedBody>,
    body_index_map: KeyIndexMapper<TypedRigidBodyID>,
    contacts: ConstraintCache<ContactID, PreparedContact>,
    spherical_joints: ConstraintCache<ConstraintID, PreparedSphericalJoint>,
    config: ConstraintSolverConfig,
}

/// Configuration parameters for the [`ConstraintSolver`].
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct ConstraintSolverConfig {
    /// Whether constraints will be solved.
    pub enabled: bool,
    /// The number of sequential impulse iterations to perform for solving the
    /// velocity constraints.
    pub n_iterations: u32,
    /// How to scale the still-valid accumulated impulses from the previous
    /// frame before using them as the initial impulses for the current frame.
    /// Set to zero to disable warm starting.
    pub old_impulse_weight: f32,
    /// The number of iterations to use for positional correction after the
    /// velocity constraints have been solved.
    pub n_positional_correction_iterations: u32,
    /// The fraction of the current positional error the solver should try to
    /// correct.
    pub positional_correction_factor: f32,
}

/// Container for constraints of a specific type that manages their lifetime
/// over multiple frames.
#[derive(Clone, Debug)]
struct ConstraintCache<K, C: PreparedTwoBodyConstraint> {
    constraints: Vec<BodyPairConstraint<C>>,
    constraint_index_map: KeyIndexMapper<K>,
}

/// Wrapper for an arbitrary two-body constraint that manages general
/// information like the indices of the involved [`ConstrainedBody`]s
/// in the [`ConstraintSolver`] and the current accumulated impulses.
#[derive(Clone, Debug)]
struct BodyPairConstraint<C: PreparedTwoBodyConstraint> {
    body_a_idx: usize,
    body_b_idx: usize,
    constraint: C,
    accumulated_impulses: C::Impulses,
    flags: ConstraintFlags,
}

bitflags! {
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    struct ConstraintFlags: u8 {
        /// Whether this constraint was prepared for use in the current frame.
        const WAS_PREPARED = 1 << 0;
    }
}

impl ConstraintSolver {
    /// Creates a new constraint solver with the given configuration
    /// parameters.
    pub fn new(config: ConstraintSolverConfig) -> Self {
        Self {
            bodies: Vec::new(),
            body_index_map: KeyIndexMapper::new(),
            contacts: ConstraintCache::new(),
            spherical_joints: ConstraintCache::new(),
            config,
        }
    }

    pub fn prepared_contact_count(&self) -> usize {
        self.contacts.constraints().len()
    }

    pub fn prepared_spherical_joint_count(&self) -> usize {
        self.spherical_joints.constraints().len()
    }

    pub fn prepared_body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn config(&self) -> &ConstraintSolverConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut ConstraintSolverConfig {
        &mut self.config
    }

    /// Prepares the given contact between the given bodies for solution. The
    /// states of the involved rigid bodies will be fetched and cached.
    pub fn prepare_contact(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        rigid_body_a_id: TypedRigidBodyID,
        rigid_body_b_id: TypedRigidBodyID,
        contact: &ContactWithID,
    ) {
        if let Some(prepared_contact) = self.prepare_constraint_for_body_pair(
            rigid_body_manager,
            rigid_body_a_id,
            rigid_body_b_id,
            &contact.contact,
        ) {
            self.contacts.register_prepared_constraint(
                contact.id,
                prepared_contact,
                self.config.old_impulse_weight,
            );
        }
    }

    /// Prepares the given [`SphericalJoint`] constraint for solution. The
    /// states of the involved rigid bodies will be fetched and cached.
    pub fn prepare_spherical_joint(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        anchor_manager: &AnchorManager,
        id: ConstraintID,
        joint: &SphericalJoint,
    ) {
        if let Some(prepared_joint) = self.prepare_anchored_constraint_for_body_pair(
            rigid_body_manager,
            anchor_manager,
            joint,
        ) {
            self.spherical_joints.register_prepared_constraint(
                id,
                prepared_joint,
                self.config.old_impulse_weight,
            );
        }
    }

    /// Removes any constraints cached from the previous solve that have not
    /// been re-prepared for the next solve. This should always be called
    /// before [`Self::compute_constrained_velocities`] after preparing all
    /// active constraints.
    pub fn remove_unprepared_constraints(&mut self) {
        self.contacts
            .remove_unprepared_constraints_and_reset_flags();
        self.spherical_joints
            .remove_unprepared_constraints_and_reset_flags();
    }

    /// Updates the velocities of all prepared constrained bodies to match those
    /// of their associated rigid body in the [`RigidBodyManager`]. This should
    /// be called after advancing the rigid body velocities (but not
    /// configurations) based on the non-constraint forces.
    pub fn synchronize_prepared_constrained_body_velocities(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
    ) {
        for (rigid_body_id, constrained_body) in
            self.body_index_map.key_at_each_idx().zip(&mut self.bodies)
        {
            synchronize_prepared_constrained_body_velocities(
                rigid_body_manager,
                rigid_body_id,
                constrained_body,
            );
        }
    }

    /// Tries to solve all prepared velocity constraints as follows:
    /// - Go though each constraint.
    /// - Compute the impulses that must be applied to the involved bodies for
    ///   their velocities to satisfy that constraint in isolation.
    /// - Update the velocities of the involved bodies with these impulses.
    /// - After doing this for each constraint, go back and repeat for a fixed
    ///   number of iterations and hope that the final velocities of all bodies
    ///   satisfy all the constraints.
    ///
    /// To speed up convergence, the final impulses of surviving constraints
    /// from the previous solve are applied to the involved bodies before
    /// starting the above procedure.
    pub fn compute_constrained_velocities(&mut self) {
        apply_warm_impulses_for_body_pair_constraints(
            &mut self.bodies,
            self.contacts.constraints(),
        );
        apply_warm_impulses_for_body_pair_constraints(
            &mut self.bodies,
            self.spherical_joints.constraints(),
        );

        for _ in 0..self.config.n_iterations {
            apply_impulses_sequentially_for_body_pair_constraints(
                &mut self.bodies,
                self.contacts.constraints_mut(),
            );
            apply_impulses_sequentially_for_body_pair_constraints(
                &mut self.bodies,
                self.spherical_joints.constraints_mut(),
            );
        }
    }

    /// Tries to correct the configurations of the bodies for all prepared
    /// constraints as follows:
    /// - Go though each constraint.
    /// - Compute the pseudo impulses (changes in position and orientation)
    ///   that must be applied to the involved bodies to improve their
    ///   configuration according to the criteria of that constraint in
    ///   isolation.
    /// - Update the configurations of the involved bodies with these pseudo
    ///   impulses.
    /// - After doing this for each constraint, go back and repeat for a fixed
    ///   number of iterations and hope that the final configurations of all
    ///   bodies are satisfactory for all the constraints.
    pub fn compute_corrected_configurations(&mut self) {
        for _ in 0..self.config.n_positional_correction_iterations {
            apply_positional_corrections_sequentially_for_body_pair_constraints(
                &mut self.bodies,
                self.contacts.constraints(),
                self.config.positional_correction_factor,
            );
            apply_positional_corrections_sequentially_for_body_pair_constraints(
                &mut self.bodies,
                self.spherical_joints.constraints(),
                self.config.positional_correction_factor,
            );
        }
    }

    /// Updates the velocities and configurations of the rigid bodies to match
    /// the solved velocities and configurations from
    /// [`Self::compute_constrained_velocities`] and
    /// [`Self::compute_corrected_configurations`].
    pub fn apply_constrained_velocities_and_corrected_configurations(
        &self,
        rigid_body_manager: &mut RigidBodyManager,
    ) {
        for (rigid_body_id, constrained_body) in
            self.body_index_map.key_at_each_idx().zip(&self.bodies)
        {
            apply_constrained_body_velocities_and_configuration_to_rigid_body(
                rigid_body_manager,
                rigid_body_id,
                constrained_body,
            );
        }
    }

    /// Clears all constrained bodies cached from the previous solve. This
    /// should always be done before starting to prepare constraints for the
    /// next solve.
    pub fn clear_prepared_bodies(&mut self) {
        self.bodies.clear();
        self.body_index_map.clear();
    }

    /// Removes all stored constraint solver state.
    pub fn clear(&mut self) {
        self.clear_prepared_bodies();
        self.contacts.clear();
        self.spherical_joints.clear();
    }

    fn prepare_constraint_for_body_pair<C: TwoBodyConstraint>(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        rigid_body_a_id: TypedRigidBodyID,
        rigid_body_b_id: TypedRigidBodyID,
        constraint: &C,
    ) -> Option<BodyPairConstraint<C::Prepared>> {
        let (body_a_idx, body_b_idx) =
            self.prepare_body_pair(rigid_body_manager, rigid_body_a_id, rigid_body_b_id)?;

        let prepared_constraint =
            constraint.prepare(&self.bodies[body_a_idx], &self.bodies[body_b_idx]);

        Some(BodyPairConstraint {
            body_a_idx,
            body_b_idx,
            constraint: prepared_constraint,
            accumulated_impulses: Zero::zero(),
            flags: ConstraintFlags::WAS_PREPARED,
        })
    }

    fn prepare_anchored_constraint_for_body_pair<C: AnchoredTwoBodyConstraint>(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        anchor_manager: &AnchorManager,
        constraint: &C,
    ) -> Option<BodyPairConstraint<C::Prepared>> {
        let (anchor_a, anchor_b) = constraint.resolve_anchors(anchor_manager)?;

        let (body_a_idx, body_b_idx) = self.prepare_body_pair(
            rigid_body_manager,
            anchor_a.rigid_body_id(),
            anchor_b.rigid_body_id(),
        )?;

        let prepared_constraint = constraint.prepare(
            &self.bodies[body_a_idx],
            &self.bodies[body_b_idx],
            anchor_a,
            anchor_b,
        );

        Some(BodyPairConstraint {
            body_a_idx,
            body_b_idx,
            constraint: prepared_constraint,
            accumulated_impulses: Zero::zero(),
            flags: ConstraintFlags::WAS_PREPARED,
        })
    }

    fn prepare_body_pair(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        rigid_body_a_id: TypedRigidBodyID,
        rigid_body_b_id: TypedRigidBodyID,
    ) -> Option<(usize, usize)> {
        let body_a_idx = self.prepare_body(rigid_body_manager, rigid_body_a_id)?;
        let body_b_idx = self.prepare_body(rigid_body_manager, rigid_body_b_id)?;
        Some((body_a_idx, body_b_idx))
    }

    fn prepare_body(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        rigid_body_id: TypedRigidBodyID,
    ) -> Option<usize> {
        if let Some(body_idx) = self.body_index_map.get(rigid_body_id) {
            return Some(body_idx);
        }

        let constrained_body = match rigid_body_id {
            TypedRigidBodyID::Dynamic(id) => {
                let rigid_body = rigid_body_manager.get_dynamic_rigid_body(id)?;
                ConstrainedBody::from_dynamic_rigid_body(rigid_body)
            }
            TypedRigidBodyID::Kinematic(id) => {
                let rigid_body = rigid_body_manager.get_kinematic_rigid_body(id)?;
                ConstrainedBody::from_kinematic_rigid_body(rigid_body)
            }
        };

        let body_idx = self.bodies.len();
        self.bodies.push(constrained_body);
        self.body_index_map.push_key(rigid_body_id);

        Some(body_idx)
    }
}

impl Default for ConstraintSolverConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            n_iterations: 8,
            old_impulse_weight: 0.4,
            n_positional_correction_iterations: 3,
            positional_correction_factor: 0.2,
        }
    }
}

impl<K, C> ConstraintCache<K, C>
where
    K: fmt::Debug + Copy + Eq + Hash,
    C: PreparedTwoBodyConstraint,
{
    fn new() -> Self {
        Self {
            constraints: Vec::new(),
            constraint_index_map: KeyIndexMapper::new(),
        }
    }

    fn constraints(&self) -> &[BodyPairConstraint<C>] {
        &self.constraints
    }

    fn constraints_mut(&mut self) -> &mut [BodyPairConstraint<C>] {
        &mut self.constraints
    }

    fn register_prepared_constraint(
        &mut self,
        key: K,
        prepared_constraint: BodyPairConstraint<C>,
        old_impulse_weight: f32,
    ) {
        if let Some(idx) = self.constraint_index_map.get(key) {
            // We know this constraint from the previous solve
            let old_constraint = &self.constraints[idx];

            // If the geometry has not changed significantly, the impulses
            // obtained from the previous solve are likely still close to the
            // solution, so we initialize the constraint with the old impulses
            // as an initial guess, but with a weight to mitigate overshoot
            if prepared_constraint.can_use_warm_impulses_from(old_constraint) {
                self.constraints[idx] = prepared_constraint.with_accumulated_impulses(
                    old_constraint.accumulated_impulses * old_impulse_weight,
                );
            }
        } else {
            self.constraints.push(prepared_constraint);
            self.constraint_index_map.push_key(key);
        }
    }

    fn remove_unprepared_constraints_and_reset_flags(&mut self) {
        let mut idx = 0;
        let mut len = self.constraints.len();
        while idx < len {
            let constraint = &mut self.constraints[idx];
            if constraint.flags.contains(ConstraintFlags::WAS_PREPARED) {
                // The constraint was prepared for the next solve, so we
                // keep it and clear its prepared flag
                constraint.flags.remove(ConstraintFlags::WAS_PREPARED);
                idx += 1;
            } else {
                // The constraint was not prepared for the next solve, so
                // we must remove it
                self.constraints.swap_remove(idx);
                self.constraint_index_map.swap_remove_key_at_idx(idx);
                len -= 1;
            }
        }
    }

    fn clear(&mut self) {
        self.constraints.clear();
        self.constraint_index_map.clear();
    }
}

impl<C: PreparedTwoBodyConstraint> BodyPairConstraint<C> {
    fn with_accumulated_impulses(mut self, accumulated_impulses: C::Impulses) -> Self {
        self.accumulated_impulses = accumulated_impulses;
        self
    }
}

impl<C: PreparedTwoBodyConstraint> Deref for BodyPairConstraint<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.constraint
    }
}

impl<C: PreparedTwoBodyConstraint> DerefMut for BodyPairConstraint<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.constraint
    }
}

fn apply_warm_impulses_for_body_pair_constraints<P: PreparedTwoBodyConstraint>(
    bodies: &mut [ConstrainedBody],
    constraints: &[BodyPairConstraint<P>],
) {
    for constraint in constraints {
        let (body_a, body_b) =
            two_mutable_elements(bodies, constraint.body_a_idx, constraint.body_b_idx);

        // The warm impulses from the previous solve are used as the initial
        // guess for this solve, so they must be pre-applied to the bodies
        // before we begin
        constraint.apply_impulses_to_body_pair(body_a, body_b, constraint.accumulated_impulses);
    }
}

fn apply_impulses_sequentially_for_body_pair_constraints<P: PreparedTwoBodyConstraint>(
    bodies: &mut [ConstrainedBody],
    constraints: &mut [BodyPairConstraint<P>],
) {
    for constraint in constraints {
        let (body_a, body_b) =
            two_mutable_elements(bodies, constraint.body_a_idx, constraint.body_b_idx);

        // These are the incremental impulses that must be applied to the
        // current velocities of the bodies to make them satisfy the constraint
        // as an equality constraint
        let corrective_impulses = constraint.compute_impulses(body_a, body_b);

        // But we are also tracking the accumulated impulses that must be
        // applied to the pre-solve (but after advancement due to forces)
        // velocities to make them satisfy the constraint as (potentially)
        // an inequality constraint. To update the accumulated impulse, we
        // add the incremental impulses and apply the clamping required to
        // make the constraint an inequality constraint.
        let old_accumulated_impulses = constraint.accumulated_impulses;
        constraint.accumulated_impulses =
            constraint.clamp_impulses(constraint.accumulated_impulses + corrective_impulses);

        // To update the current velocities to be consistent with the new
        // accumulated impulses, we compute the difference from the old
        // accumulated impulses and apply that
        let clamped_corrective_impulses =
            constraint.accumulated_impulses - old_accumulated_impulses;

        constraint.apply_impulses_to_body_pair(body_a, body_b, clamped_corrective_impulses);
    }
}

fn apply_positional_corrections_sequentially_for_body_pair_constraints<
    P: PreparedTwoBodyConstraint,
>(
    bodies: &mut [ConstrainedBody],
    constraints: &[BodyPairConstraint<P>],
    correction_factor: f32,
) {
    for constraint in constraints {
        let (body_a, body_b) =
            two_mutable_elements(bodies, constraint.body_a_idx, constraint.body_b_idx);

        constraint.apply_positional_correction_to_body_pair(body_a, body_b, correction_factor);
    }
}

fn synchronize_prepared_constrained_body_velocities(
    rigid_body_manager: &RigidBodyManager,
    rigid_body_id: TypedRigidBodyID,
    constrained_body: &mut ConstrainedBody,
) {
    match rigid_body_id {
        TypedRigidBodyID::Dynamic(id) => {
            let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(id) else {
                return;
            };
            constrained_body.velocity = rigid_body.compute_velocity();
            constrained_body.angular_velocity = rigid_body.compute_angular_velocity().as_vector();
        }
        TypedRigidBodyID::Kinematic(id) => {
            let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body(id) else {
                return;
            };
            constrained_body.velocity = *rigid_body.velocity();
            constrained_body.angular_velocity = rigid_body.angular_velocity().as_vector();
        }
    }
}

fn apply_constrained_body_velocities_and_configuration_to_rigid_body(
    rigid_body_manager: &mut RigidBodyManager,
    rigid_body_id: TypedRigidBodyID,
    constrained_body: &ConstrainedBody,
) {
    match rigid_body_id {
        TypedRigidBodyID::Dynamic(id) => {
            let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(id) else {
                return;
            };
            rigid_body.set_position(constrained_body.position);
            rigid_body.set_orientation(constrained_body.orientation);
            rigid_body.synchronize_momentum(&constrained_body.velocity);
            rigid_body.synchronize_angular_momentum(&AngularVelocity::from_vector(
                constrained_body.angular_velocity,
            ));
        }
        TypedRigidBodyID::Kinematic(id) => {
            let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(id) else {
                return;
            };
            rigid_body.set_position(constrained_body.position);
            rigid_body.set_orientation(constrained_body.orientation);
            rigid_body.set_velocity(constrained_body.velocity);
            rigid_body.set_angular_velocity(AngularVelocity::from_vector(
                constrained_body.angular_velocity,
            ));
        }
    }
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
