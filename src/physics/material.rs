//! Material properties for physics simulation.

pub mod components;

use crate::physics::fph;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct ContactResponseParameters {
    pub restitution_coef: fph,
    pub static_friction_coef: fph,
    pub dynamic_friction_coef: fph,
}

impl ContactResponseParameters {
    pub fn combined(&self, other: &Self) -> Self {
        Self {
            restitution_coef: fph::max(self.restitution_coef, other.restitution_coef),
            static_friction_coef: fph::sqrt(self.static_friction_coef * other.static_friction_coef),
            dynamic_friction_coef: fph::sqrt(
                self.dynamic_friction_coef * other.dynamic_friction_coef,
            ),
        }
    }
}

impl Default for ContactResponseParameters {
    fn default() -> Self {
        Self {
            restitution_coef: 0.0,
            static_friction_coef: 0.0,
            dynamic_friction_coef: 0.0,
        }
    }
}
