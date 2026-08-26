//! Particle system.

#[macro_use]
mod macros;

pub mod gpu_resource;
pub mod material;
pub mod render_commands;
pub mod setup;
pub mod shader_templates;
pub mod simulation_commands;

use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use gpu_resource::ParticleSimulationGPUResources;
use impact_containers::HashMap;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice, uniform::UniformBuffer,
    wgpu,
};
use impact_id::{EntityID, define_entity_id_newtype};
use impact_math::{
    angle::{Angle, Radians},
    consts::f32::PI,
    point::Point3C,
    vector::{UnitVector3C, Vector3C, Vector4C},
};
use material::{ParticleMaterialConfig, ParticleMaterialID};
use roc_integration::roc;
use std::path::Path;

define_entity_id_newtype! {
    /// Identifier for a particle emitter in the [`ParticleEmitterManager`]
    /// using a [`BasicParticleSimulation`].
    [pub] BasicParticleEmitterID
}

define_component_type! {
    /// Marks that an entity has a particle emitter using a
    /// [`BasicParticleSimulation`], identified by a [`BasicParticleEmitterID`].
    ///
    /// Use [`BasicParticleEmitterID::from_entity_id`] to obtain the particle
    /// emitter ID from the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HasBasicParticleEmitter;
}

/// The type of particle simulation used by a particle emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleSimulationType {
    Basic,
}

/// A particle emitter, specified by a particle material, particle count and
/// simulation parameters.
#[derive(Debug, Clone)]
pub struct ParticleEmitter<S> {
    pub material_id: ParticleMaterialID,
    pub log2_particle_count: u32,
    pub simulation: S,
}

/// ID for a generic particle emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleEmitterID {
    entity_id: EntityID,
    simulation_type: ParticleSimulationType,
}

/// Metadata for a generic particle emitter.
#[derive(Debug, Clone)]
pub struct ParticleEmitterMetadata {
    material_id: ParticleMaterialID,
    log2_particle_count: u32,
}

/// A particle. These live exclusively in the particle GPU buffer and are only
/// updated and read on the GPU.
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct Particle {
    pub rgba_base_color: Vector4C,
    pub position: Vector3C,
    pub age: f32,
    pub velocity: Vector3C,
    pub time_to_live: f32,
    pub extent: f32,
    _padding: [u8; 12], // Total size must be a multiple of alignment, which is 16
}

/// Manager of all particle emitters in a scene.
#[derive(Debug)]
pub struct ParticleEmitterManager {
    emitter_metadata: HashMap<ParticleEmitterID, ParticleEmitterMetadata>,
    basic_particle_emitters: BasicSimulationUniformBuffer,
}

/// Configuration parameters for the particle system.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct ParticleConfig {
    /// If [`None`], the particle system will be disabled.
    pub materials: Option<ParticleMaterialConfig>,
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
    pub rate_over_time: RateOverTime,
    pub spawn_position: Point3C,
    _padding_0: [u8; 4],
    pub spawn_velocity: Vector3C, // Aligned to 16 bytes
    _padding_1: [u8; 4],
    pub spawn_direction: UnitVector3C, // Aligned to 16 bytes
    pub cos_polar_spawn_angle: SpawnDistribution,
    pub spawn_extent: SpawnDistribution,
    pub spawn_distance: SpawnDistribution,
    pub spawn_speed: SpawnDistribution,
    pub lifetime: SpawnDistribution,
    _padding_end: [u8; 12],
}

type BasicSimulationUniformBuffer = UniformBuffer<BasicParticleEmitterID, BasicParticleSimulation>;

/// Piecewise linear rate of particle emission as a function of time. The rate
/// is considered zero before `t0` and `r3` after `t3`. The times are assumed
/// monotonic.
#[roc(parents = "Particle")]
#[repr(C)]
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

#[roc(parents = "Particle")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct SpawnDistribution {
    pub min_value: f32,
    pub max_value: f32,
}

impl<S> ParticleEmitter<S> {
    pub fn metadata(&self) -> ParticleEmitterMetadata {
        ParticleEmitterMetadata {
            log2_particle_count: self.log2_particle_count,
            material_id: self.material_id,
        }
    }
}

impl From<BasicParticleEmitterID> for ParticleEmitterID {
    fn from(id: BasicParticleEmitterID) -> Self {
        Self {
            entity_id: id.as_entity_id(),
            simulation_type: ParticleSimulationType::Basic,
        }
    }
}

impl ParticleEmitterMetadata {
    pub fn particle_count(&self) -> usize {
        1 << self.log2_particle_count
    }
}

impl ParticleEmitterManager {
    /// By creating simulation uniform buffers with a small initial capacity, we
    /// avoid excessive buffer reallocation when the first few emitters are
    /// added.
    pub const INITIAL_SIMULATION_CAPACITY: usize = 5;

    /// Creates a new particle emitter manager with no emitters.
    pub fn new() -> Self {
        Self {
            basic_particle_emitters: BasicSimulationUniformBuffer::with_capacity(
                Self::INITIAL_SIMULATION_CAPACITY,
            ),
            emitter_metadata: HashMap::default(),
        }
    }

    /// Returns a reference to the map of particle emitter metadata.
    pub fn emitter_metadata(&self) -> &HashMap<ParticleEmitterID, ParticleEmitterMetadata> {
        &self.emitter_metadata
    }

    /// Whether the emitter with the given ID is present in the manager.
    pub fn has_emitter(&self, emitter_id: &ParticleEmitterID) -> bool {
        self.emitter_metadata.contains_key(emitter_id)
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`BasicParticleSimulation`]s.
    pub fn basic_simulation_buffer(&self) -> &BasicSimulationUniformBuffer {
        &self.basic_particle_emitters
    }

    /// Adds the given [`ParticleEmitter`] with a [`BasicParticleSimulation`] to
    /// the manager under the given ID.
    ///
    /// # Errors
    /// Returns an error if the ID is already present.
    pub fn add_basic_emitter(
        &mut self,
        id: BasicParticleEmitterID,
        emitter: ParticleEmitter<BasicParticleSimulation>,
    ) -> Result<()> {
        if self.basic_particle_emitters.has_uniform(id) {
            bail!("A basic particle emitter with ID {id} already exists");
        }
        self.basic_particle_emitters
            .add_uniform(id, emitter.simulation);
        self.emitter_metadata
            .insert(ParticleEmitterID::from(id), emitter.metadata());

        Ok(())
    }

    /// Removes the basic particle emitter with the given ID from the manager.
    ///
    /// # Panics
    /// If no emitter with the given ID exists.
    pub fn remove_basic_emitter(&mut self, id: BasicParticleEmitterID) {
        self.basic_particle_emitters.remove_uniform(id);
        self.emitter_metadata.remove(&ParticleEmitterID::from(id));
    }

    /// Removes all state in the manager and frees up all allocated memory.
    #[inline]
    pub fn reset_and_free(&mut self) {
        self.basic_particle_emitters
            .reset_and_free_to_capacity(Self::INITIAL_SIMULATION_CAPACITY);
        self.emitter_metadata = HashMap::default();
    }

    /// Performs any required updates for keeping the particle simulation GPU
    /// resources in sync with the particle emitter data.
    ///
    /// GPU resources whose source data no longer exists will be removed, and
    /// missing GPU resources for new source data will be created.
    pub fn sync_simulation_gpu_resources(
        &self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        effective_time_step_duration: f32,
        particle_simulation_gpu_resources: &mut ParticleSimulationGPUResources,
    ) {
        particle_simulation_gpu_resources.sync_buffers_with_manager(
            graphics_device,
            staging_belt,
            command_encoder,
            bind_group_layout_registry,
            effective_time_step_duration,
            self,
        );
    }
}

impl Default for ParticleEmitterManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let Some(materials) = &mut self.materials {
            materials.resolve_paths(root_path);
        }
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
