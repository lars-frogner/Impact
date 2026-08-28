//! Setup of particle emitters.

use crate::{
    BasicParticleSimulation, ColorOverLifetime, EmitterFrame, RateOverTime, ScaleOverLifetime,
    SpawnDistribution,
    material::{ParticleMaterialID, ParticleMaterialRegistry},
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_math::hash::Hash32;
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
        pub cos_polar_spawn_angle: SpawnDistribution,
        pub spawn_extent: SpawnDistribution,
        pub spawn_distance: SpawnDistribution,
        pub spawn_speed: SpawnDistribution,
        pub spawn_angle: SpawnDistribution,
        pub spawn_angular_velocity: SpawnDistribution,
        pub lifetime: SpawnDistribution,
        pub color_over_lifetime: ColorOverLifetime,
        pub scale_over_lifetime: ScaleOverLifetime,
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
        cos_polar_spawn_angle,
        spawn_extent,
        spawn_distance,
        spawn_speed,
        spawn_angle,
        spawn_angular_velocity,
        lifetime,
        color_over_lifetime,
        scale_over_lifetime,
    }
    "#)]
    pub fn new(
        material_name: &str,
        log2_particle_count: u32,
        rate_over_time: RateOverTime,
        cos_polar_spawn_angle: SpawnDistribution,
        spawn_extent: SpawnDistribution,
        spawn_distance: SpawnDistribution,
        spawn_speed: SpawnDistribution,
        spawn_angle: SpawnDistribution,
        spawn_angular_velocity: SpawnDistribution,
        lifetime: SpawnDistribution,
        color_over_lifetime: ColorOverLifetime,
        scale_over_lifetime: ScaleOverLifetime,
    ) -> Self {
        Self {
            material_name_hash: Hash32::from_str(material_name),
            log2_particle_count,
            rate_over_time,
            cos_polar_spawn_angle,
            spawn_extent,
            spawn_distance,
            spawn_speed,
            spawn_angle,
            spawn_angular_velocity,
            lifetime,
            color_over_lifetime,
            scale_over_lifetime,
        }
    }

    pub fn simulation(&self, emitter_frame: EmitterFrame) -> BasicParticleSimulation {
        BasicParticleSimulation {
            rate_over_time: self.rate_over_time,
            color_over_lifetime: self.color_over_lifetime,
            scale_over_lifetime: self.scale_over_lifetime,
            spawn_position: emitter_frame.position,
            spawn_velocity: emitter_frame.velocity,
            spawn_direction: emitter_frame.direction,
            cos_polar_spawn_angle: self.cos_polar_spawn_angle,
            spawn_extent: self.spawn_extent,
            spawn_distance: self.spawn_distance,
            spawn_speed: self.spawn_speed,
            spawn_angle: self.spawn_angle,
            spawn_angular_velocity: self.spawn_angular_velocity,
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
