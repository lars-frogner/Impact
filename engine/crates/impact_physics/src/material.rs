//! Material properties for physics simulation.

use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

/// Parameters quantifying the physical response of a body in contact with
/// another body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct ContactResponseParameters {
    /// The elasticity of collisions with the body, typically between 0 (fully
    /// inelastic, the bodies stay together) and 1 (elastic, the bodies bounce
    /// maximally apart).
    pub restitution_coef: f32,
    /// The strength of friction at the contact when the touching surfaces are
    /// not sliding across each other.
    pub static_friction_coef: f32,
    /// The strength of friction at the contact when the touching surfaces are
    /// sliding across each other.
    pub dynamic_friction_coef: f32,
}

#[roc]
impl ContactResponseParameters {
    #[roc(body = "{ restitution_coef, static_friction_coef, dynamic_friction_coef }")]
    pub fn new(
        restitution_coef: f32,
        static_friction_coef: f32,
        dynamic_friction_coef: f32,
    ) -> Self {
        Self {
            restitution_coef,
            static_friction_coef,
            dynamic_friction_coef,
        }
    }

    /// Computes the effective response parameters to use when resolving a
    /// contact between two bodies, given the reponse parameters of each of
    /// them (the physical response depends on the material properties of both
    /// bodies).
    pub fn combined(&self, other: &Self) -> Self {
        Self {
            restitution_coef: f32::max(self.restitution_coef, other.restitution_coef),
            static_friction_coef: f32::sqrt(self.static_friction_coef * other.static_friction_coef),
            dynamic_friction_coef: f32::sqrt(
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
