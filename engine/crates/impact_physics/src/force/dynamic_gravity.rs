//! Gravitational forces in a collective gravitational field.

use crate::{
    quantities::{ForceC, TorqueC},
    rigid_body::{DynamicRigidBodyID, RigidBodyManager},
};
use bytemuck::{Pod, Zeroable};
use impact_containers::KeyIndexMapper;
use impact_math::{matrix::Matrix3C, point::Point3C};
use roc_integration::roc;

define_component_type! {
    /// Marks a body that contributes to and is affected by a collective
    /// gravitational field.
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct DynamicGravity;
}

#[cfg(feature = "ecs")]
impact_ecs::declare_component_flags! {
    DynamicGravity => impact_ecs::component::ComponentFlags::INHERITABLE,
}

/// Manages dynamic rigid bodies that contribute to and are affected by a
/// collective gravitational field.
#[derive(Clone, Debug)]
pub struct DynamicGravityManager {
    body_ids: KeyIndexMapper<DynamicRigidBodyID>,
    bodies: Vec<GravitationalBody>,
    loads: Vec<GravitationalLoad>,
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
    /// Inertia tensor in world space.
    pub inertia_tensor: Matrix3C,
    /// Position of the center of mass in world space.
    pub position: Point3C,
}

#[derive(Clone, Copy, Debug)]
pub struct GravitationalLoad {
    /// World-space gravitational force on the center of mass.
    pub force: ForceC,
    /// World-space gravitational torque about the center of mass.
    pub torque: TorqueC,
}

impl DynamicGravityManager {
    pub fn new(config: DynamicGravityConfig) -> Self {
        Self {
            body_ids: KeyIndexMapper::new(),
            bodies: Vec::new(),
            loads: Vec::new(),
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
        self.loads.push(GravitationalLoad::zero());
    }

    /// Removes the given dynamic rigid body from the collective gravitational
    /// field.
    pub fn remove_body(&mut self, rigid_body_id: DynamicRigidBodyID) {
        let idx = self.body_ids.swap_remove_key(rigid_body_id);
        self.bodies.swap_remove(idx);
        self.loads.swap_remove(idx);
    }

    /// Returns the current gravitational load on the given body, or [`None`] if
    /// the body is not included in the field.
    pub fn get_load_on_body(&self, rigid_body_id: DynamicRigidBodyID) -> Option<GravitationalLoad> {
        let idx = self.body_ids.get(rigid_body_id)?;
        Some(self.loads[idx])
    }

    /// Computes and applies the gravitational forces and torques to the
    /// appropriate dynamic rigid bodies.
    pub fn compute_and_apply(&mut self, rigid_body_manager: &mut RigidBodyManager) {
        self.synchronize_bodies(rigid_body_manager);
        self.loads.fill(GravitationalLoad::zero());

        if self.bodies.len() < 2 {
            return;
        }

        self.compute_loads();
        self.apply_loads(rigid_body_manager);
    }

    pub fn set_gravitational_constant(&mut self, gravitational_constant: f32) {
        self.config.gravitational_constant = gravitational_constant;
    }

    pub fn clear(&mut self) {
        self.body_ids.clear();
        self.bodies.clear();
        self.loads.clear();
    }

    fn synchronize_bodies(&mut self, rigid_body_manager: &RigidBodyManager) {
        for (body_id, gravitational_body) in
            self.body_ids.key_at_each_idx().zip(self.bodies.iter_mut())
        {
            let body = rigid_body_manager.dynamic_rigid_body(body_id);

            let orientation = body.orientation().aligned();
            let body_space_inertia_tensor = body.inertia_tensor().aligned();
            let world_space_inertia_tensor = body_space_inertia_tensor.rotated_matrix(&orientation);

            gravitational_body.mass = body.mass();
            gravitational_body.inertia_tensor = world_space_inertia_tensor.compact();
            gravitational_body.position = *body.position();
        }
    }

    fn compute_loads(&mut self) {
        let n_bodies = self.bodies.len();
        assert!(n_bodies >= 2);

        for (i, body_i) in (0..n_bodies - 1).zip(&self.bodies[0..n_bodies - 1]) {
            for (j, body_j) in (i + 1..n_bodies).zip(&self.bodies[i + 1..n_bodies]) {
                let (force_i, torque_i, torque_j) =
                    compute_gravitational_force_and_torques(&self.config, body_i, body_j);

                let load_i = &mut self.loads[i];
                load_i.force += force_i;
                load_i.torque += torque_i;

                let load_j = &mut self.loads[j];
                load_j.force -= force_i;
                load_j.torque += torque_j;
            }
        }
    }

    fn apply_loads(&self, rigid_body_manager: &mut RigidBodyManager) {
        for (body_id, load) in self.body_ids.key_at_each_idx().zip(&self.loads) {
            let rigid_body = rigid_body_manager.dynamic_rigid_body_mut(body_id);
            rigid_body.apply_force_at_center_of_mass_compact(&load.force);
            rigid_body.apply_torque_compact(&load.torque);
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

impl GravitationalLoad {
    #[inline]
    const fn zero() -> Self {
        Self {
            force: ForceC::zeros(),
            torque: TorqueC::zeros(),
        }
    }
}

#[inline]
fn compute_gravitational_force_and_torques(
    config: &DynamicGravityConfig,
    body_i: &GravitationalBody,
    body_j: &GravitationalBody,
) -> (ForceC, TorqueC, TorqueC) {
    const EPS: f32 = 1e-6;

    let g_const = config.gravitational_constant;
    let m_i = body_i.mass;
    let m_j = body_j.mass;
    let imat_i = body_i.inertia_tensor.aligned();
    let imat_j = body_j.inertia_tensor.aligned();
    let p_i = body_i.position.aligned();
    let p_j = body_j.position.aligned();

    let r_vec = p_j - p_i;
    let r_squared = r_vec.norm_squared();
    let r = r_squared.sqrt();
    let r_cubed = r_squared * r;
    let r_to_fifth = r_cubed * r_squared;

    let force_i_mono = (g_const * m_i * m_j / (r_cubed + EPS)) * r_vec;

    let quad_scale = 1.5 * g_const / (r_to_fifth + EPS);

    let m_j_imat_i_r = m_j * (imat_i * r_vec);
    let m_i_imat_j_r = m_i * (imat_j * r_vec);
    let m_imat_r_sum = m_j_imat_i_r + m_i_imat_j_r;

    let force_i_quad = quad_scale
        * ((m_j * imat_i.trace() + m_i * imat_j.trace()
            - 5.0 * (r_vec.dot(&m_imat_r_sum)) / (r_squared + EPS))
            * r_vec
            + 2.0 * m_imat_r_sum);

    let force_i = force_i_mono + force_i_quad;

    let torque_scale = 2.0 * quad_scale;
    let torque_i = torque_scale * r_vec.cross(&m_j_imat_i_r);
    let torque_j = torque_scale * r_vec.cross(&m_i_imat_j_r);

    (force_i.compact(), torque_i.compact(), torque_j.compact())
}
