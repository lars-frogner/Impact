//! Particle effects.

use crate::{
    emitter::{ParticleEmitterAnchoring, ParticleEmitterFrame, ParticleEmitterMetadata},
    material::ParticleMaterialID,
};
use bytemuck::{Pod, Zeroable};
use impact_math::{
    angle::{Angle, Radians},
    consts::f32::PI,
    point::Point3C,
    vector::{UnitVector3C, Vector3C, Vector4C},
};
use roc_integration::roc;

/// A particle effect governed by a specific simulation type.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParticleEffect {
    Basic(BasicParticleEffect),
}

define_setup_type! {
    /// A particle effect governed by the basic particle simulation.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct BasicParticleEffect {
        pub material_id: ParticleMaterialID,
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

/// Parameters for a basic particle simulation.
///
/// This struct is intended to be stored in a [`ParticleEmitterManager`]
/// (wrapped in a [`ParticleEmitter`]), and its data will be passed directly to
/// the GPU in a uniform buffer. Importantly, its size is a multiple of 16 bytes
/// as required for uniform array items, and the fields that will be accessed on
/// the GPU are aligned to their required byte boundaries.
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct BasicParticleSimulation {
    rate_over_time: RateOverTime,
    color_over_lifetime: ColorOverLifetime,
    scale_over_lifetime: ScaleOverLifetime,
    spawn_position: Point3C,
    _padding_0: [u8; 4],
    spawn_velocity: Vector3C, // Aligned to 16 bytes
    _padding_1: [u8; 4],
    spawn_direction: UnitVector3C, // Aligned to 16 bytes
    cos_polar_spawn_angle: SpawnDistribution,
    spawn_extent: SpawnDistribution,
    spawn_distance: SpawnDistribution,
    spawn_speed: SpawnDistribution,
    spawn_angle: SpawnDistribution,
    spawn_angular_velocity: SpawnDistribution,
    lifetime: SpawnDistribution,
    _padding_end: [u8; 12],
}

/// Piecewise linear rate of particle emission as a function of time. The rate
/// is considered zero before `t0` and `r3` after `t3`. The times are assumed
/// monotonic.
#[roc(parents = "Particle")]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct RateOverTime {
    pub t0: f32,
    pub t1: f32,
    pub t2: f32,
    pub t3: f32,
    pub r0: f32,
    pub r1: f32,
    pub r2: f32,
    pub r3: f32,
}

/// Piecewise linear RGBA base color as a function of fractional time relative
/// to the lifetime of the particle (so the full lifetime spans `t = [0.0,
/// 1.0)`). The color is considered `c0` before `t0` and `c3` after `t3`. The
/// fractional times are assumed non-negative and monotonic.
#[roc(parents = "Particle")]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct ColorOverLifetime {
    pub t0: f32,
    pub t1: f32,
    pub t2: f32,
    pub t3: f32,
    pub c0: Vector4C,
    pub c1: Vector4C,
    pub c2: Vector4C,
    pub c3: Vector4C,
}

/// Piecewise linear scale as a function of fractional time relative to the
/// lifetime of the particle (so the full lifetime spans `t = [0.0, 1.0)`). The
/// scale is considered `s0` before `t0` and `s3` after `t3`. The fractional
/// times are assumed non-negative and monotonic.
#[roc(parents = "Particle")]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct ScaleOverLifetime {
    pub t0: f32,
    pub t1: f32,
    pub t2: f32,
    pub t3: f32,
    pub s0: f32,
    pub s1: f32,
    pub s2: f32,
    pub s3: f32,
}

/// A distribution of initial values to sample from when a particle is spawned.
#[roc(parents = "Particle")]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct SpawnDistribution {
    pub min_value: f32,
    pub max_value: f32,
}

impl BasicParticleEffect {
    pub(crate) fn emitter_metadata(
        &self,
        anchoring: ParticleEmitterAnchoring,
    ) -> ParticleEmitterMetadata {
        ParticleEmitterMetadata {
            log2_particle_count: self.log2_particle_count,
            material_id: self.material_id,
            anchoring,
        }
    }

    pub(crate) fn simulation(
        &self,
        emitter_frame: ParticleEmitterFrame,
    ) -> BasicParticleSimulation {
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
}

impl BasicParticleSimulation {
    pub(crate) fn set_emitter_frame(&mut self, emitter_frame: ParticleEmitterFrame) {
        self.spawn_position = emitter_frame.position;
        self.spawn_velocity = emitter_frame.velocity;
        self.spawn_direction = emitter_frame.direction;
    }
}

#[roc]
impl RateOverTime {
    const MIN_DURATION: f32 = 1e-6;

    #[roc(body = r#"
    clamped_duration = Num.max(duration, 1e-6)
    rate = Num.to_f32(particle_count) / clamped_duration
    start = Num.max(delay, 0.0)
    end = start + clamped_duration
    {
        t0: start,
        t1: end,
        t2: end,
        t3: end,
        r0: rate,
        r1: rate,
        r2: 0.0,
        r3: 0.0,
    }
    "#)]
    pub fn constant(particle_count: u32, delay: f32, duration: f32) -> Self {
        let duration = duration.max(Self::MIN_DURATION);
        let rate = particle_count as f32 / duration;
        let start = delay.max(0.0);
        let end = start + duration;
        Self {
            t0: start,
            t1: end,
            t2: end,
            t3: end,
            r0: rate,
            r1: rate,
            r2: 0.0,
            r3: 0.0,
        }
    }

    #[roc(body = "constant(particle_count, delay, 0.0)")]
    pub fn burst(particle_count: u32, delay: f32) -> Self {
        Self::constant(particle_count, delay, 0.0)
    }
}

#[roc(dependencies = [Vector3C])]
impl ColorOverLifetime {
    #[roc(body = r#"
    {
        t0: 0.0,
        t1: 1.0,
        t2: 1.0,
        t3: 1.0,
        c0: start_color,
        c1: end_color,
        c2: end_color,
        c3: end_color,
    }
    "#)]
    pub fn linear(start_color: Vector4C, end_color: Vector4C) -> Self {
        Self {
            t0: 0.0,
            t1: 1.0,
            t2: 1.0,
            t3: 1.0,
            c0: start_color,
            c1: end_color,
            c2: end_color,
            c3: end_color,
        }
    }

    #[roc(body = r#"
    start_color = (color.0, color.1, color.2, start_alpha)
    end_color = (color.0, color.1, color.2, end_alpha)
    linear(start_color, end_color)
    "#)]
    pub fn linear_alpha(color: Vector3C, start_alpha: f32, end_alpha: f32) -> Self {
        Self::linear(color.extended(start_alpha), color.extended(end_alpha))
    }

    #[roc(body = "linear_alpha(color, 1.0, 0.0)")]
    pub fn linear_fade(color: Vector3C) -> Self {
        Self::linear_alpha(color, 1.0, 0.0)
    }

    #[roc(body = "linear(color, color)")]
    pub fn constant(color: Vector4C) -> Self {
        Self::linear(color, color)
    }
}

#[roc]
impl ScaleOverLifetime {
    #[roc(body = r#"
    {
        t0: 0.0,
        t1: 1.0,
        t2: 1.0,
        t3: 1.0,
        s0: start_scale,
        s1: end_scale,
        s2: end_scale,
        s3: end_scale,
    }
    "#)]
    pub fn linear(start_scale: f32, end_scale: f32) -> Self {
        Self {
            t0: 0.0,
            t1: 1.0,
            t2: 1.0,
            t3: 1.0,
            s0: start_scale,
            s1: end_scale,
            s2: end_scale,
            s3: end_scale,
        }
    }

    #[roc(body = "linear(1.0, 0.0)")]
    pub fn linear_shrink() -> Self {
        Self::linear(1.0, 0.0)
    }

    #[roc(body = "linear(scale, scale)")]
    pub fn constant(scale: f32) -> Self {
        Self::linear(scale, scale)
    }
}

#[roc(dependencies = [Radians])]
impl SpawnDistribution {
    #[roc(body = "{ min_value: value, max_value: value }")]
    pub fn same(value: f32) -> Self {
        Self {
            min_value: value,
            max_value: value,
        }
    }

    #[roc(body = "{ min_value, max_value: Num.max(min_value, max_value) }")]
    pub fn uniform(min_value: f32, max_value: f32) -> Self {
        Self {
            min_value,
            max_value: max_value.max(min_value),
        }
    }

    #[roc(body = r#"
    {
        min_value: Num.cos(Num.max(Radians.radians(min_angle), Num.min(Num.pi, Radians.radians(max_angle)))),
        max_value: Num.cos(Num.max(0, Num.min(Num.pi, Radians.radians(min_angle))))
    }
    "#)]
    pub fn uniform_in_cos(min_angle: Radians, max_angle: Radians) -> Self {
        Self {
            min_value: max_angle.radians().clamp(min_angle.radians(), PI).cos(),
            max_value: min_angle.radians().clamp(0.0, PI).cos(),
        }
    }
}
