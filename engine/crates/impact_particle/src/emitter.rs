//! Particle emitters. An emitter has a particle effect as well as information
//! anchoring it to the world.

use crate::{
    effect::{BasicParticleEffect, BasicParticleSimulation},
    gpu_resource::ParticleSimulationGPUResources,
    material::{ParticleMaterialID, ParticleMaterialRegistry},
};
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_containers::HashMap;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice, uniform::UniformBuffer,
    wgpu,
};
use impact_id::{EntityID, define_entity_id_newtype};
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
    emitter_metadata: HashMap<ParticleEmitterID, ParticleEmitterMetadata>,
    basic_particle_emitters: BasicSimulationUniformBuffer,
}

/// ID for a generic particle emitter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParticleEmitterID {
    entity_id: EntityID,
    simulation_type: ParticleEmitterType,
}

/// Metadata for a generic particle emitter.
#[derive(Clone, Debug)]
pub struct ParticleEmitterMetadata {
    pub material_id: ParticleMaterialID,
    pub log2_particle_count: u32,
    pub anchoring: ParticleEmitterAnchoring,
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

/// The type of a particle emitter, determined by its simulation type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParticleEmitterType {
    Basic,
}

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

type BasicSimulationUniformBuffer = UniformBuffer<BasicParticleEmitterID, BasicParticleSimulation>;

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

    /// Adds a particle emitter with the given [`BasicParticleEffect`],
    /// anchoring and current frame (derived from the anchoring) to the manager
    /// under the given ID.
    ///
    /// # Errors
    /// Returns an error if the ID is already present.
    pub fn add_basic_emitter(
        &mut self,
        id: BasicParticleEmitterID,
        effect: BasicParticleEffect,
        anchoring: ParticleEmitterAnchoring,
        frame: ParticleEmitterFrame,
    ) -> Result<()> {
        if self.basic_particle_emitters.has_uniform(id) {
            bail!("A basic particle emitter with ID {id} already exists");
        }

        self.basic_particle_emitters
            .add_uniform(id, effect.simulation(frame));

        self.emitter_metadata.insert(
            ParticleEmitterID::from(id),
            effect.emitter_metadata(anchoring),
        );

        Ok(())
    }

    /// Removes the basic particle emitter with the given ID from the manager.
    ///
    /// # Returns
    /// The metadata of the removed emitter.
    ///
    /// # Panics
    /// If no emitter with the given ID exists.
    pub fn remove_basic_emitter(&mut self, id: BasicParticleEmitterID) -> ParticleEmitterMetadata {
        self.basic_particle_emitters.remove_uniform(id);
        self.emitter_metadata
            .remove(&ParticleEmitterID::from(id))
            .unwrap()
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

        for (emitter_id, metadata) in &self.emitter_metadata {
            if let ParticleEmitterAnchoring::RigidBody(anchoring) = &metadata.anchoring {
                let Some(emitter_frame) =
                    anchoring.obtain_current_emitter_frame(rigid_body_manager, anchor_manager)
                else {
                    // The anchor or associated rigid body no longer exists, so
                    // we remove the emitter
                    emitter_ids_to_remove.push(*emitter_id);
                    continue;
                };

                match emitter_id.simulation_type {
                    ParticleEmitterType::Basic => {
                        let simulation = self
                            .basic_particle_emitters
                            .get_uniform_mut(BasicParticleEmitterID::from_entity_id(
                                emitter_id.entity_id,
                            ))
                            .unwrap();

                        simulation.set_emitter_frame(emitter_frame);
                    }
                }
            }
        }

        for emitter_id in emitter_ids_to_remove {
            match emitter_id.simulation_type {
                ParticleEmitterType::Basic => {
                    self.remove_basic_emitter(BasicParticleEmitterID::from_entity_id(
                        emitter_id.entity_id,
                    ));
                }
            }
        }
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
        Self {
            entity_id: id.as_entity_id(),
            simulation_type: ParticleEmitterType::Basic,
        }
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
