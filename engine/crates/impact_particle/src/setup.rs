//! Setup of particle emitters.

use crate::{
    BasicParticleSimulation,
    material::{ParticleMaterialID, ParticleMaterialRegistry},
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_math::angle::Radians;
use impact_math::{
    angle::Angle, consts::f32::PI, hash::Hash32, point::Point3C, vector::UnitVector3C,
};
use roc_integration::roc;

define_setup_type! {
    /// Emission of particles governed by the basic particle simulation.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct BasicParticleEmission {
        pub material_name_hash: Hash32,
        pub log2_particle_count: u32,
        pub spawn_position: Point3C,
        pub spawn_direction: UnitVector3C,
        pub cos_min_polar_spawn_angle: f32,
        pub cos_max_polar_spawn_angle: f32,
        pub min_spawn_extent: f32,
        pub max_spawn_extent: f32,
        pub min_spawn_speed: f32,
        pub max_spawn_speed: f32,
        pub min_lifetime: f32,
        pub max_lifetime: f32,
    }
}

#[roc(dependencies = [Radians])]
impl BasicParticleEmission {
    #[roc(body = r#"
    material_name_hash = Hashing.hash_str_32(material_name)
    cos_min_polar_spawn_angle = Num.cos(Num.max(0, Num.min(Num.pi, Radians.radians(min_polar_spawn_angle))))
    cos_max_polar_spawn_angle = Num.cos(Num.max(Radians.radians(min_polar_spawn_angle), Num.min(Num.pi, Radians.radians(max_polar_spawn_angle))))
    {
        material_name_hash,
        log2_particle_count,
        spawn_position,
        spawn_direction,
        cos_min_polar_spawn_angle,
        cos_max_polar_spawn_angle,
        min_spawn_extent,
        max_spawn_extent: Num.max(min_spawn_extent, max_spawn_extent),
        min_spawn_speed,
        max_spawn_speed: Num.max(min_spawn_speed, max_spawn_speed),
        min_lifetime,
        max_lifetime: Num.max(min_lifetime, max_lifetime),
    }
    "#)]
    pub fn new(
        material_name: &str,
        log2_particle_count: u32,
        spawn_position: Point3C,
        spawn_direction: UnitVector3C,
        min_polar_spawn_angle: Radians,
        max_polar_spawn_angle: Radians,
        min_spawn_extent: f32,
        max_spawn_extent: f32,
        min_spawn_speed: f32,
        max_spawn_speed: f32,
        min_lifetime: f32,
        max_lifetime: f32,
    ) -> Self {
        Self {
            material_name_hash: Hash32::from_str(material_name),
            log2_particle_count,
            spawn_position,
            spawn_direction,
            cos_min_polar_spawn_angle: min_polar_spawn_angle.radians().clamp(0.0, PI).cos(),
            cos_max_polar_spawn_angle: max_polar_spawn_angle
                .radians()
                .clamp(min_polar_spawn_angle.radians(), PI)
                .cos(),
            min_spawn_extent,
            max_spawn_extent: max_spawn_extent.max(min_spawn_extent),
            min_spawn_speed,
            max_spawn_speed: max_spawn_speed.max(min_spawn_speed),
            min_lifetime,
            max_lifetime: max_lifetime.max(min_lifetime),
        }
    }

    pub fn simulation(&self) -> BasicParticleSimulation {
        BasicParticleSimulation {
            spawn_position: self.spawn_position,
            spawn_direction: self.spawn_direction,
            cos_min_polar_spawn_angle: self.cos_min_polar_spawn_angle,
            cos_max_polar_spawn_angle: self.cos_max_polar_spawn_angle,
            min_spawn_extent: self.min_spawn_extent,
            max_spawn_extent: self.max_spawn_extent,
            min_spawn_speed: self.min_spawn_speed,
            max_spawn_speed: self.max_spawn_speed,
            min_lifetime: self.min_lifetime,
            max_lifetime: self.max_lifetime,
            _padding: Default::default(),
        }
    }

    pub fn material_id(
        &self,
        particle_material_registry: &ParticleMaterialRegistry,
    ) -> Result<ParticleMaterialID> {
        particle_material_registry
            .particle_material_for_name_hash(self.material_name_hash)
            .ok_or_else(|| anyhow!("Missing particle material for name in `BasicParticleEmission`"))
    }
}
