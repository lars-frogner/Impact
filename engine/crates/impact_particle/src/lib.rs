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
    pub remaining_time: f32,
    pub velocity: Vector3C,
    pub extent: f32,
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
    pub spawn_position: Point3C,
    pub cos_min_polar_spawn_angle: f32,
    pub spawn_direction: UnitVector3C, // Aligned to 16 bytes
    pub cos_max_polar_spawn_angle: f32,
    pub min_spawn_extent: f32,
    pub max_spawn_extent: f32,
    pub min_spawn_speed: f32,
    pub max_spawn_speed: f32,
    pub min_lifetime: f32,
    pub max_lifetime: f32,
    _padding: [u8; 8],
}

type BasicSimulationUniformBuffer = UniformBuffer<BasicParticleEmitterID, BasicParticleSimulation>;

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
    pub materials: ParticleMaterialConfig,
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
        self.materials.resolve_paths(root_path);
    }
}
