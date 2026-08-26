//! Setup of particle emitters.

use crate::{
    BasicParticleSimulation, RateOverTime, SpawnDistribution,
    material::{ParticleMaterialID, ParticleMaterialRegistry},
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_math::{
    hash::Hash32,
    point::Point3C,
    vector::{UnitVector3C, Vector3C},
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
        pub rate_over_time: RateOverTime,
        pub spawn_position: Point3C,
        pub spawn_velocity: Vector3C,
        pub spawn_direction: UnitVector3C,
        pub cos_polar_spawn_angle: SpawnDistribution,
        pub spawn_extent: SpawnDistribution,
        pub spawn_distance: SpawnDistribution,
        pub spawn_speed: SpawnDistribution,
        pub lifetime: SpawnDistribution,
    }
}

#[roc]
impl BasicParticleEmission {
    #[roc(body = r#"
    material_name_hash = Hashing.hash_str_32(material_name)
    {
        material_name_hash,
        log2_particle_count,
        rate_over_time,
        spawn_position,
        spawn_velocity,
        spawn_direction,
        cos_polar_spawn_angle,
        spawn_extent,
        spawn_distance,
        spawn_speed,
        lifetime,
    }
    "#)]
    pub fn new(
        material_name: &str,
        log2_particle_count: u32,
        rate_over_time: RateOverTime,
        spawn_position: Point3C,
        spawn_velocity: Vector3C,
        spawn_direction: UnitVector3C,
        cos_polar_spawn_angle: SpawnDistribution,
        spawn_extent: SpawnDistribution,
        spawn_distance: SpawnDistribution,
        spawn_speed: SpawnDistribution,
        lifetime: SpawnDistribution,
    ) -> Self {
        Self {
            material_name_hash: Hash32::from_str(material_name),
            log2_particle_count,
            rate_over_time,
            spawn_position,
            spawn_velocity,
            spawn_direction,
            cos_polar_spawn_angle,
            spawn_extent,
            spawn_distance,
            spawn_speed,
            lifetime,
        }
    }

    pub fn simulation(&self) -> BasicParticleSimulation {
        BasicParticleSimulation {
            rate_over_time: self.rate_over_time,
            spawn_position: self.spawn_position,
            spawn_velocity: self.spawn_velocity,
            spawn_direction: self.spawn_direction,
            cos_polar_spawn_angle: self.cos_polar_spawn_angle,
            spawn_extent: self.spawn_extent,
            spawn_distance: self.spawn_distance,
            spawn_speed: self.spawn_speed,
            lifetime: self.lifetime,
            _padding_0: Default::default(),
            _padding_1: Default::default(),
            _padding_end: Default::default(),
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
