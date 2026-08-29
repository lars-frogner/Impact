//! Particle emitters. An emitter has a particle effect as well as information
//! anchoring it to the world.

use crate::{
    effect::{BasicParticleSimulation, ParticleEffect, ParticleSimulationType},
    gpu_resource::ParticleSimulationGPUResources,
    material::{ParticleMaterialID, ParticleMaterialRegistry},
};
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_containers::NoHashMap;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice, uniform::UniformBuffer,
    wgpu,
};
use impact_id::define_entity_id_newtype;
use impact_math::{
    point::Point3C,
    vector::{UnitVector3C, Vector3C},
};
use impact_physics::{
    anchor::{AnchorManager, TypedRigidBodyAnchorID},
    rigid_body::{DynamicRigidBody, KinematicRigidBody, RigidBodyManager},
};
use roc_integration::roc;

/// Manager of all particle emitters in a scene.
#[derive(Debug)]
pub struct ParticleEmitterManager {
    emitter_metadata: NoHashMap<ParticleEmitterID, ParticleEmitterMetadata>,
    basic_particle_emitters: UniformBuffer<BasicParticleEmitterID, BasicParticleSimulation>,
}

define_entity_id_newtype! {
    /// Identifier for a particle emitter in the [`ParticleEmitterManager`].
    [pub] ParticleEmitterID
}

define_component_type! {
    /// Marks that an entity has a particle emitter in the
    /// [`ParticleEmitterManager`].
    ///
    /// Use [`ParticleEmitterID::from_entity_id`] to obtain the particle emitter
    /// ID from the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HasParticleEmitter;
}

/// Metadata for a generic particle emitter.
#[derive(Clone, Debug)]
pub struct ParticleEmitterMetadata {
    pub material_id: ParticleMaterialID,
    pub log2_particle_count: u32,
    pub anchoring: ParticleEmitterAnchoring,
    pub simulation_type: ParticleSimulationType,
}

/// How a particle emitter is anchored, either directly to world space or to a
/// rigid body.
#[derive(Clone, Copy, Debug)]
pub enum ParticleEmitterAnchoring {
    World(WorldParticleEmitterAnchoring),
    RigidBody(RigidBodyParticleEmitterAnchoring),
}

/// The world space position and emission direction of a particle emitter.
#[derive(Clone, Copy, Debug)]
pub struct WorldParticleEmitterAnchoring {
    pub position: Point3C,
    pub direction: UnitVector3C,
}

/// The anchor tying a particle emitter to a rigid body and the emission
/// direction in body space.
#[derive(Clone, Copy, Debug)]
pub struct RigidBodyParticleEmitterAnchoring {
    pub anchor_id: TypedRigidBodyAnchorID,
    pub direction: UnitVector3C,
}

/// The world space position, velocity and orientation (encoded only in an
/// emission direction) of a particle emitter. Ultimately derived from an
/// [`EmitterAnchoring`].
#[derive(Clone, Debug)]
pub struct ParticleEmitterFrame {
    pub(crate) position: Point3C,
    pub(crate) velocity: Vector3C,
    pub(crate) direction: UnitVector3C,
}

define_entity_id_newtype! {
    /// Identifier for a particle emitter in the [`ParticleEmitterManager`]
    /// using a [`BasicParticleSimulation`].
    [pub] BasicParticleEmitterID
}

impl ParticleEmitterManager {
    /// By creating simulation uniform buffers with a small initial capacity, we
    /// avoid excessive buffer reallocation when the first few emitters are
    /// added.
    pub const INITIAL_SIMULATION_CAPACITY: usize = 5;

    /// Creates a new particle emitter manager with no emitters.
    pub fn new() -> Self {
        Self {
            basic_particle_emitters: UniformBuffer::with_capacity(
                Self::INITIAL_SIMULATION_CAPACITY,
            ),
            emitter_metadata: NoHashMap::default(),
        }
    }

    /// Returns a reference to the map of particle emitter metadata.
    pub fn emitter_metadata(&self) -> &NoHashMap<ParticleEmitterID, ParticleEmitterMetadata> {
        &self.emitter_metadata
    }

    /// Returns a reference to the [`UniformBuffer`] holding all
    /// [`BasicParticleSimulation`]s.
    pub fn basic_simulation_buffer(
        &self,
    ) -> &UniformBuffer<BasicParticleEmitterID, BasicParticleSimulation> {
        &self.basic_particle_emitters
    }

    /// Whether the emitter with the given ID is present in the manager.
    pub fn has_emitter(&self, emitter_id: ParticleEmitterID) -> bool {
        self.emitter_metadata.contains_key(&emitter_id)
    }

    /// Adds a particle emitter with the given [`ParticleEffect`], anchoring and
    /// current frame (derived from the anchoring) to the manager under the
    /// given ID.
    ///
    /// # Errors
    /// Returns an error if the ID is already present.
    pub fn add_emitter(
        &mut self,
        id: ParticleEmitterID,
        effect: ParticleEffect,
        anchoring: ParticleEmitterAnchoring,
        frame: ParticleEmitterFrame,
    ) -> Result<()> {
        if self.emitter_metadata.contains_key(&id) {
            bail!("A particle emitter with ID {id} already exists");
        }

        match effect {
            ParticleEffect::Basic(effect) => {
                self.emitter_metadata
                    .insert(id, effect.emitter_metadata(anchoring));

                self.basic_particle_emitters
                    .add_uniform(id.into(), effect.simulation(frame));
            }
        }

        Ok(())
    }

    /// Removes the particle emitter with the given ID from the manager.
    ///
    /// # Returns
    /// The metadata of the removed emitter if it existed.
    pub fn remove_emitter(&mut self, id: ParticleEmitterID) -> Option<ParticleEmitterMetadata> {
        let metadata = self.emitter_metadata.remove(&id)?;
        match metadata.simulation_type {
            ParticleSimulationType::Basic => {
                self.basic_particle_emitters.remove_uniform(id.into());
            }
        }
        Some(metadata)
    }

    /// Synchronizes all particle emitters with the current state of the physics
    /// simulation.
    pub fn sync_dynamic_emitter_state(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
        anchor_manager: &AnchorManager,
    ) {
        let arena = ArenaPool::get_arena();
        let mut emitter_ids_to_remove = AVec::new_in(&arena);

        for (&emitter_id, metadata) in &self.emitter_metadata {
            if let ParticleEmitterAnchoring::RigidBody(anchoring) = &metadata.anchoring {
                let Some(emitter_frame) =
                    anchoring.obtain_current_emitter_frame(rigid_body_manager, anchor_manager)
                else {
                    // The anchor or associated rigid body no longer exists, so
                    // we remove the emitter
                    emitter_ids_to_remove.push(emitter_id);
                    continue;
                };

                match metadata.simulation_type {
                    ParticleSimulationType::Basic => {
                        let simulation = self
                            .basic_particle_emitters
                            .get_uniform_mut(emitter_id.into())
                            .unwrap();

                        simulation.set_emitter_frame(emitter_frame);
                    }
                }
            }
        }

        for emitter_id in emitter_ids_to_remove {
            self.remove_emitter(emitter_id);
        }
    }

    /// Removes all state in the manager and frees up all allocated memory.
    #[inline]
    pub fn reset_and_free(&mut self) {
        self.basic_particle_emitters
            .reset_and_free_to_capacity(Self::INITIAL_SIMULATION_CAPACITY);
        self.emitter_metadata = NoHashMap::default();
    }

    /// Performs any required updates for keeping the particle simulation GPU
    /// resources in sync with the particle emitter data.
    ///
    /// GPU resources whose source data no longer exists will be removed, and
    /// missing GPU resources for new source data will be created.
    pub fn sync_simulation_gpu_resources(
        &self,
        material_registry: &ParticleMaterialRegistry,
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
            material_registry,
            self,
        );
    }
}

impl Default for ParticleEmitterManager {
    fn default() -> Self {
        Self::new()
    }
}

impl From<BasicParticleEmitterID> for ParticleEmitterID {
    fn from(id: BasicParticleEmitterID) -> Self {
        Self::from_entity_id(id.as_entity_id())
    }
}

impl From<ParticleEmitterID> for BasicParticleEmitterID {
    fn from(id: ParticleEmitterID) -> Self {
        Self::from_entity_id(id.as_entity_id())
    }
}

impl ParticleEmitterMetadata {
    pub fn particle_count(&self) -> usize {
        1 << self.log2_particle_count
    }
}

impl RigidBodyParticleEmitterAnchoring {
    fn obtain_current_emitter_frame(
        &self,
        rigid_body_manager: &RigidBodyManager,
        anchor_manager: &AnchorManager,
    ) -> Option<ParticleEmitterFrame> {
        match self.anchor_id {
            TypedRigidBodyAnchorID::Dynamic(id) => {
                let anchor = anchor_manager.dynamic().get(id)?;
                let rigid_body = rigid_body_manager.get_dynamic_rigid_body(anchor.rigid_body_id)?;

                Some(ParticleEmitterFrame::anchored_to_dynamic_rigid_body(
                    rigid_body,
                    &anchor.point,
                    &self.direction,
                ))
            }
            TypedRigidBodyAnchorID::Kinematic(id) => {
                let anchor = anchor_manager.kinematic().get(id)?;
                let rigid_body =
                    rigid_body_manager.get_kinematic_rigid_body(anchor.rigid_body_id)?;

                Some(ParticleEmitterFrame::anchored_to_kinematic_rigid_body(
                    rigid_body,
                    &anchor.point,
                    &self.direction,
                ))
            }
        }
    }
}

impl ParticleEmitterFrame {
    pub fn world(anchoring: &WorldParticleEmitterAnchoring) -> Self {
        Self {
            position: anchoring.position,
            velocity: Vector3C::zeros(),
            direction: anchoring.direction,
        }
    }

    pub fn anchored_to_dynamic_rigid_body(
        rigid_body: &DynamicRigidBody,
        anchor_point: &Point3C,
        direction: &UnitVector3C,
    ) -> Self {
        let world_position =
            rigid_body.transform_point_from_body_to_world_space(&anchor_point.aligned());

        let world_direction =
            rigid_body.transform_direction_from_body_to_world_space(&direction.aligned());

        let world_velocity =
            rigid_body.compute_velocity_of_attached_world_space_point(&world_position);

        Self {
            position: world_position.compact(),
            velocity: world_velocity.compact(),
            direction: world_direction.compact(),
        }
    }

    pub fn anchored_to_kinematic_rigid_body(
        rigid_body: &KinematicRigidBody,
        anchor_point: &Point3C,
        direction: &UnitVector3C,
    ) -> Self {
        let world_position =
            rigid_body.transform_point_from_body_to_world_space(&anchor_point.aligned());

        let world_direction =
            rigid_body.transform_direction_from_body_to_world_space(&direction.aligned());

        let world_velocity =
            rigid_body.compute_velocity_of_attached_world_space_point(&world_position);

        Self {
            position: world_position.compact(),
            velocity: world_velocity.compact(),
            direction: world_direction.compact(),
        }
    }
}
