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
use impact_alloc::{AVec, arena::ArenaPool};
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
use impact_physics::{
    anchor::{AnchorManager, TypedRigidBodyAnchorID},
    rigid_body::{DynamicRigidBody, KinematicRigidBody, RigidBodyManager},
};
use material::{ParticleMaterialConfig, ParticleMaterialID};
use roc_integration::roc;
use std::path::Path;

/// A particle. These live exclusively in the particle GPU buffer and are only
/// updated and read on the GPU.
///
/// If you add fields, remember that the total size must be a multiple of the
/// alignment on the GPU, which is 16 bytes (due to vec3f and vec4f fields).
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct Particle {
    pub color: Vector4C,
    pub position: Vector3C,
    pub age: f32,
    pub velocity: Vector3C,
    pub lifetime: f32,
    pub extent: f32,
    pub scale: f32,
    pub angle: f32,
    pub angular_velocity: f32,
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

/// A generic particle emitter.
#[derive(Clone, Debug)]
pub struct ParticleEmitter<S> {
    pub material_id: ParticleMaterialID,
    pub log2_particle_count: u32,
    pub anchoring: EmitterAnchoring,
    pub simulation: S,
}

/// ID for a generic particle emitter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParticleEmitterID {
    entity_id: EntityID,
    simulation_type: ParticleSimulationType,
}

/// Metadata for a generic particle emitter.
#[derive(Clone, Debug)]
pub struct ParticleEmitterMetadata {
    pub material_id: ParticleMaterialID,
    pub log2_particle_count: u32,
    pub anchoring: EmitterAnchoring,
}

/// How a particle emitter is anchored, either directly to world space or to a
/// rigid body.
#[derive(Clone, Copy, Debug)]
pub enum EmitterAnchoring {
    World(WorldEmitterAnchoring),
    RigidBody(RigidBodyEmitterAnchoring),
}

/// The world space position and emission direction of a particle emitter.
#[derive(Clone, Copy, Debug)]
pub struct WorldEmitterAnchoring {
    pub position: Point3C,
    pub direction: UnitVector3C,
}

/// The anchor tying a particle emitter to a rigid body and the emission
/// direction in body space.
#[derive(Clone, Copy, Debug)]
pub struct RigidBodyEmitterAnchoring {
    pub anchor_id: TypedRigidBodyAnchorID,
    pub direction: UnitVector3C,
}

/// The world space position, velocity and orientation (encoded only in an
/// emission direction) of a particle emitter. Ultimately derived from an
/// [`EmitterAnchoring`].
#[derive(Clone, Debug)]
pub struct EmitterFrame {
    pub position: Point3C,
    pub velocity: Vector3C,
    pub direction: UnitVector3C,
}

/// The type of particle simulation used by a particle emitter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParticleSimulationType {
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
    pub color_over_lifetime: ColorOverLifetime,
    pub scale_over_lifetime: ScaleOverLifetime,
    pub spawn_position: Point3C,
    _padding_0: [u8; 4],
    pub spawn_velocity: Vector3C, // Aligned to 16 bytes
    _padding_1: [u8; 4],
    pub spawn_direction: UnitVector3C, // Aligned to 16 bytes
    pub cos_polar_spawn_angle: SpawnDistribution,
    pub spawn_extent: SpawnDistribution,
    pub spawn_distance: SpawnDistribution,
    pub spawn_speed: SpawnDistribution,
    pub spawn_angle: SpawnDistribution,
    pub spawn_angular_velocity: SpawnDistribution,
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

/// Piecewise linear RGBA base color as a function of fractional time relative
/// to the lifetime of the particle (so the full lifetime spans `t = [0.0,
/// 1.0)`). The color is considered `c0` before `t0` and `c3` after `t3`. The
/// fractional times are assumed non-negative and monotonic.
#[roc(parents = "Particle")]
#[repr(C)]
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

#[roc(parents = "Particle")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct SpawnDistribution {
    pub min_value: f32,
    pub max_value: f32,
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
            if let EmitterAnchoring::RigidBody(anchoring) = &metadata.anchoring {
                let Some(emitter_frame) =
                    anchoring.obtain_current_emitter_frame(rigid_body_manager, anchor_manager)
                else {
                    // The anchor or associated rigid body no longer exists, so
                    // we remove the emitter
                    emitter_ids_to_remove.push(*emitter_id);
                    continue;
                };

                match emitter_id.simulation_type {
                    ParticleSimulationType::Basic => {
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
                ParticleSimulationType::Basic => {
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

impl<S> ParticleEmitter<S> {
    fn metadata(&self) -> ParticleEmitterMetadata {
        ParticleEmitterMetadata {
            log2_particle_count: self.log2_particle_count,
            material_id: self.material_id,
            anchoring: self.anchoring,
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

impl RigidBodyEmitterAnchoring {
    fn obtain_current_emitter_frame(
        &self,
        rigid_body_manager: &RigidBodyManager,
        anchor_manager: &AnchorManager,
    ) -> Option<EmitterFrame> {
        match self.anchor_id {
            TypedRigidBodyAnchorID::Dynamic(id) => {
                let anchor = anchor_manager.dynamic().get(id)?;
                let rigid_body = rigid_body_manager.get_dynamic_rigid_body(anchor.rigid_body_id)?;

                Some(EmitterFrame::anchored_to_dynamic_rigid_body(
                    rigid_body,
                    &anchor.point,
                    &self.direction,
                ))
            }
            TypedRigidBodyAnchorID::Kinematic(id) => {
                let anchor = anchor_manager.kinematic().get(id)?;
                let rigid_body =
                    rigid_body_manager.get_kinematic_rigid_body(anchor.rigid_body_id)?;

                Some(EmitterFrame::anchored_to_kinematic_rigid_body(
                    rigid_body,
                    &anchor.point,
                    &self.direction,
                ))
            }
        }
    }
}

impl EmitterFrame {
    pub fn world(anchoring: &WorldEmitterAnchoring) -> Self {
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

impl BasicParticleSimulation {
    fn set_emitter_frame(&mut self, emitter_frame: EmitterFrame) {
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
