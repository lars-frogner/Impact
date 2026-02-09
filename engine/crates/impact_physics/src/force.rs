//! Calculation of forces and torques.

pub mod alignment_torque;
pub mod constant_acceleration;
pub mod detailed_drag;
pub mod dynamic_gravity;
pub mod local_force;
pub mod setup;
pub mod spring_force;

use crate::{UniformMedium, anchor::AnchorManager, rigid_body::RigidBodyManager};
use alignment_torque::AlignmentTorqueRegistry;
use anyhow::{Result, bail};
use constant_acceleration::ConstantAccelerationRegistry;
use detailed_drag::{DetailedDragForceRegistry, DragLoadMapConfig};
use dynamic_gravity::{DynamicGravityConfig, DynamicGravityManager};
use impact_containers::HashMap;
use local_force::LocalForceRegistry;
use spring_force::{DynamicDynamicSpringForceRegistry, DynamicKinematicSpringForceRegistry};
use std::{fmt, hash::Hash, path::Path};

/// Manager of all generators of forces and torques on rigid bodies.
#[derive(Debug)]
pub struct ForceGeneratorManager {
    constant_accelerations: ConstantAccelerationRegistry,
    local_forces: LocalForceRegistry,
    dynamic_dynamic_spring_forces: DynamicDynamicSpringForceRegistry,
    dynamic_kinematic_spring_forces: DynamicKinematicSpringForceRegistry,
    detailed_drag_forces: DetailedDragForceRegistry,
    dynamic_gravity_manager: DynamicGravityManager,
    alignment_torques: AlignmentTorqueRegistry,
}

/// Configuration parameters for rigid body force generation.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct ForceGenerationConfig {
    /// Configuration parameters for the generation of drag load maps.
    pub drag_load_map_config: DragLoadMapConfig,
    /// Configuration parameters for computing dynamic gravity.
    pub dynamic_gravity_config: DynamicGravityConfig,
}

/// Manages all instances of a specific type of force generator.
#[derive(Clone, Debug)]
pub struct ForceGeneratorRegistry<Id, G> {
    generators: HashMap<Id, G>,
}

impl ForceGeneratorManager {
    /// Creates a new force manager with the given configuration parameters.
    ///
    /// # Errors
    /// Returns an error if any of the configuration parameters are invalid.
    pub fn new(config: ForceGenerationConfig) -> Result<Self> {
        Ok(Self {
            constant_accelerations: ConstantAccelerationRegistry::new(),
            local_forces: LocalForceRegistry::new(),
            dynamic_dynamic_spring_forces: DynamicDynamicSpringForceRegistry::new(),
            dynamic_kinematic_spring_forces: DynamicKinematicSpringForceRegistry::new(),
            detailed_drag_forces: DetailedDragForceRegistry::new(config.drag_load_map_config)?,
            dynamic_gravity_manager: DynamicGravityManager::new(config.dynamic_gravity_config),
            alignment_torques: AlignmentTorqueRegistry::new(),
        })
    }

    pub fn constant_accelerations(&self) -> &ConstantAccelerationRegistry {
        &self.constant_accelerations
    }

    pub fn constant_accelerations_mut(&mut self) -> &mut ConstantAccelerationRegistry {
        &mut self.constant_accelerations
    }

    pub fn local_forces(&self) -> &LocalForceRegistry {
        &self.local_forces
    }

    pub fn local_forces_mut(&mut self) -> &mut LocalForceRegistry {
        &mut self.local_forces
    }

    pub fn dynamic_dynamic_spring_forces(&self) -> &DynamicDynamicSpringForceRegistry {
        &self.dynamic_dynamic_spring_forces
    }

    pub fn dynamic_dynamic_spring_forces_mut(&mut self) -> &mut DynamicDynamicSpringForceRegistry {
        &mut self.dynamic_dynamic_spring_forces
    }

    pub fn dynamic_kinematic_spring_forces(&self) -> &DynamicKinematicSpringForceRegistry {
        &self.dynamic_kinematic_spring_forces
    }

    pub fn dynamic_kinematic_spring_forces_mut(
        &mut self,
    ) -> &mut DynamicKinematicSpringForceRegistry {
        &mut self.dynamic_kinematic_spring_forces
    }

    pub fn detailed_drag_forces(&self) -> &DetailedDragForceRegistry {
        &self.detailed_drag_forces
    }

    pub fn detailed_drag_forces_mut(&mut self) -> &mut DetailedDragForceRegistry {
        &mut self.detailed_drag_forces
    }

    pub fn dynamic_gravity_manager(&self) -> &DynamicGravityManager {
        &self.dynamic_gravity_manager
    }

    pub fn dynamic_gravity_manager_mut(&mut self) -> &mut DynamicGravityManager {
        &mut self.dynamic_gravity_manager
    }

    pub fn alignment_torques(&self) -> &AlignmentTorqueRegistry {
        &self.alignment_torques
    }

    pub fn alignment_torques_mut(&mut self) -> &mut AlignmentTorqueRegistry {
        &mut self.alignment_torques
    }

    /// Applies all forces of torques to the rigid bodies.
    pub fn apply_forces_and_torques(
        &mut self,
        medium: &UniformMedium,
        rigid_body_manager: &mut RigidBodyManager,
        anchor_manager: &AnchorManager,
    ) {
        rigid_body_manager.reset_all_forces_and_torques();

        for generator in self.constant_accelerations.generators() {
            generator.apply(rigid_body_manager);
        }
        for generator in self.local_forces.generators() {
            generator.apply(rigid_body_manager, anchor_manager);
        }
        for generator in self.dynamic_dynamic_spring_forces.generators() {
            generator.apply(rigid_body_manager, anchor_manager);
        }
        for generator in self.dynamic_kinematic_spring_forces.generators() {
            generator.apply(rigid_body_manager, anchor_manager);
        }

        self.detailed_drag_forces.apply(rigid_body_manager, medium);

        self.dynamic_gravity_manager
            .compute_and_apply(rigid_body_manager);

        for generator in self.alignment_torques.generators() {
            generator.apply(rigid_body_manager, &self.dynamic_gravity_manager);
        }
    }

    /// Removes all stored force generators.
    pub fn clear(&mut self) {
        self.constant_accelerations.clear();
        self.local_forces.clear();
        self.dynamic_dynamic_spring_forces.clear();
        self.dynamic_kinematic_spring_forces.clear();
        self.detailed_drag_forces.clear();
        self.dynamic_gravity_manager.clear();
        self.alignment_torques.clear();
    }
}

impl<Id: Copy + Eq + Hash + fmt::Display, G> ForceGeneratorRegistry<Id, G> {
    fn new() -> Self {
        Self {
            generators: HashMap::default(),
        }
    }

    /// Returns a reference to the generator with the given ID, or [`None`] if
    /// it does not exist.
    pub fn get_generator(&self, id: &Id) -> Option<&G> {
        self.generators.get(id)
    }

    /// Returns a mutable reference to the generator with the given ID, or
    /// [`None`] if it does not exist.
    pub fn get_generator_mut(&mut self, id: &Id) -> Option<&mut G> {
        self.generators.get_mut(id)
    }

    /// Returns an iterator over all generators.
    pub fn generators(&self) -> impl Iterator<Item = &G> {
        self.generators.values()
    }

    /// Adds the given force generator to the map under the given ID.
    ///
    /// # Errors
    /// Returns an error if the given generator ID already exists.
    pub fn insert_generator(&mut self, id: Id, generator: G) -> Result<()> {
        if self.generators.contains_key(&id) {
            bail!("A force generator with ID {id} already exists");
        }
        self.generators.insert(id, generator);
        Ok(())
    }

    /// Removes the force generator with the given ID from the map if it exists.
    pub fn remove_generator(&mut self, id: Id) {
        self.generators.remove(&id);
    }

    fn clear(&mut self) {
        self.generators.clear();
    }
}

impl ForceGenerationConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.drag_load_map_config.resolve_paths(root_path);
    }
}
