//! Gravitational forces in a collective gravitational field.

use crate::{
    quantities::ForceC,
    rigid_body::{DynamicRigidBodyID, RigidBodyManager},
};
use bytemuck::{Pod, Zeroable};
use impact_containers::KeyIndexMapper;
use impact_math::point::Point3C;
use roc_integration::roc;

define_component_type! {
    /// Marks a body that contributes to and is affected by a collective
    /// gravitational field.
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct DynamicGravity;
}

/// Manages dynamic rigid bodies that contribute to and are affected by a
/// collective gravitational field.
#[derive(Clone, Debug)]
pub struct DynamicGravityManager {
    body_ids: KeyIndexMapper<DynamicRigidBodyID>,
    bodies: Vec<GravitationalBody>,
    forces: Vec<ForceC>,
    config: DynamicGravityConfig,
}

/// Configuration parameters for computing dynamic gravity.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct DynamicGravityConfig {
    /// The gravitational constant for Newton's law of gravity (m³/kg/s²).
    pub gravitational_constant: f32,
}

#[derive(Clone, Debug, Default)]
pub struct GravitationalBody {
    pub mass: f32,
    /// Position of the center of mass in world space.
    pub position: Point3C,
}

impl DynamicGravityManager {
    pub fn new(config: DynamicGravityConfig) -> Self {
        Self {
            body_ids: KeyIndexMapper::new(),
            bodies: Vec::new(),
            forces: Vec::new(),
            config,
        }
    }

    /// Includes the given dynamic rigid body in the collective gravitational
    /// field.
    ///
    /// # Panics
    /// If the body is already included in the field.
    pub fn include_body(&mut self, rigid_body_id: DynamicRigidBodyID) {
        self.body_ids.push_key(rigid_body_id);
        self.bodies.push(GravitationalBody::default());
        self.forces.push(ForceC::zeros());
    }

    /// Removes the given dynamic rigid body from the collective gravitational
    /// field.
    pub fn remove_body(&mut self, rigid_body_id: DynamicRigidBodyID) {
        let idx = self.body_ids.swap_remove_key(rigid_body_id);
        self.bodies.swap_remove(idx);
        self.forces.swap_remove(idx);
    }

    /// Returns the current force of gravity on the given body, or [`None`] if
    /// the body is not included in the field.
    pub fn get_force_on_body(&self, rigid_body_id: DynamicRigidBodyID) -> Option<ForceC> {
        let idx = self.body_ids.get(rigid_body_id)?;
        Some(self.forces[idx])
    }

    /// Computes and applies the gravitational forces to the appropriate dynamic
    /// rigid bodies.
    pub fn compute_and_apply(&mut self, rigid_body_manager: &mut RigidBodyManager) {
        self.synchronize_bodies(rigid_body_manager);
        self.forces.fill(ForceC::zeros());

        if self.bodies.len() < 2 {
            return;
        }

        self.compute_forces();
        self.apply_forces(rigid_body_manager);
    }

    pub fn set_gravitational_constant(&mut self, gravitational_constant: f32) {
        self.config.gravitational_constant = gravitational_constant;
    }

    pub fn clear(&mut self) {
        self.body_ids.clear();
        self.bodies.clear();
        self.forces.clear();
    }

    fn synchronize_bodies(&mut self, rigid_body_manager: &RigidBodyManager) {
        for (body_id, gravitational_body) in
            self.body_ids.key_at_each_idx().zip(self.bodies.iter_mut())
        {
            let body = rigid_body_manager.dynamic_rigid_body(body_id);
            gravitational_body.mass = body.mass();
            gravitational_body.position = *body.position();
        }
    }

    fn compute_forces(&mut self) {
        let n_bodies = self.bodies.len();
        assert!(n_bodies >= 2);

        for (i, body_i) in (0..n_bodies - 1).zip(&self.bodies[0..n_bodies - 1]) {
            for (j, body_j) in (i + 1..n_bodies).zip(&self.bodies[i + 1..n_bodies]) {
                let displacement_i_to_j = body_j.position - body_i.position;
                let distance_squared = displacement_i_to_j.norm_squared();
                let distance = distance_squared.sqrt();

                let force_j_on_i = (self.config.gravitational_constant * body_i.mass * body_j.mass
                    / (distance_squared * distance))
                    * displacement_i_to_j;

                self.forces[i] += force_j_on_i;
                self.forces[j] -= force_j_on_i;
            }
        }
    }

    fn apply_forces(&self, rigid_body_manager: &mut RigidBodyManager) {
        for (body_id, force) in self.body_ids.key_at_each_idx().zip(&self.forces) {
            rigid_body_manager
                .dynamic_rigid_body_mut(body_id)
                .apply_force_at_center_of_mass_compact(force);
        }
    }
}

impl Default for DynamicGravityConfig {
    fn default() -> Self {
        Self {
            gravitational_constant: 1.0,
        }
    }
}
