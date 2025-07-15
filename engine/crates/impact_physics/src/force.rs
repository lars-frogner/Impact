//! Calculation of forces and torques.

pub mod constant_acceleration;
pub mod detailed_drag;
pub mod local_force;
pub mod setup;
pub mod spring_force;

use crate::{UniformMedium, rigid_body::RigidBodyManager};
use anyhow::Result;
use constant_acceleration::ConstantAccelerationRegistry;
use detailed_drag::{DetailedDragForceRegistry, DragLoadMapConfig};
use impact_containers::IndexMap;
use local_force::LocalForceRegistry;
use spring_force::{DynamicDynamicSpringForceRegistry, DynamicKinematicSpringForceRegistry};
use std::hash::Hash;

/// Manager of all generators of forces and torques on rigid bodies.
#[derive(Debug)]
pub struct ForceGeneratorManager {
    constant_accelerations: ConstantAccelerationRegistry,
    local_forces: LocalForceRegistry,
    dynamic_dynamic_spring_forces: DynamicDynamicSpringForceRegistry,
    dynamic_kinematic_spring_forces: DynamicKinematicSpringForceRegistry,
    detailed_drag_forces: DetailedDragForceRegistry,
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
}

/// Manages all instances of a specific type of force generator.
#[derive(Clone, Debug)]
pub struct ForceGeneratorRegistry<Id, G> {
    generators: IndexMap<Id, G>,
    id_counter: u64,
}

impl ForceGeneratorManager {
    /// Creates a new force manager with the given configuration parameters.
    ///
    /// # Errors
    /// Returns an error if any of the configuration parameters are invalid.
    pub fn new(config: ForceGenerationConfig) -> Result<Self> {
        Ok(Self {
            constant_accelerations: ForceGeneratorRegistry::new(),
            local_forces: ForceGeneratorRegistry::new(),
            dynamic_dynamic_spring_forces: ForceGeneratorRegistry::new(),
            dynamic_kinematic_spring_forces: ForceGeneratorRegistry::new(),
            detailed_drag_forces: DetailedDragForceRegistry::new(config.drag_load_map_config)?,
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

    /// Applies all forces of torques to the rigid bodies.
    pub fn apply_forces_and_torques(
        &self,
        medium: &UniformMedium,
        rigid_body_manager: &mut RigidBodyManager,
    ) {
        rigid_body_manager.reset_all_forces_and_torques();

        for generator in self.constant_accelerations.generators() {
            generator.apply(rigid_body_manager);
        }
        for generator in self.local_forces.generators() {
            generator.apply(rigid_body_manager);
        }
        for generator in self.dynamic_dynamic_spring_forces.generators() {
            generator.apply(rigid_body_manager);
        }
        for generator in self.dynamic_kinematic_spring_forces.generators() {
            generator.apply(rigid_body_manager);
        }
        self.detailed_drag_forces.apply(rigid_body_manager, medium);
    }

    /// Removes all stored force generators.
    pub fn clear(&mut self) {
        self.constant_accelerations.clear();
        self.local_forces.clear();
        self.dynamic_dynamic_spring_forces.clear();
        self.dynamic_kinematic_spring_forces.clear();
        self.detailed_drag_forces.clear();
    }
}

impl<Id: Copy + Eq + Hash + From<u64>, G> ForceGeneratorRegistry<Id, G> {
    fn new() -> Self {
        Self {
            generators: IndexMap::default(),
            id_counter: 0,
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

    /// Adds the given force generator to the map.
    ///
    /// # Returns
    /// A new ID representing the added force generator.
    pub fn insert_generator(&mut self, generator: G) -> Id {
        let id = self.create_new_id();
        self.generators.insert(id, generator);
        id
    }

    /// Removes the force generator with the given ID from the map if it exists.
    pub fn remove_generator(&mut self, id: Id) {
        self.generators.swap_remove(&id);
    }

    fn clear(&mut self) {
        self.generators.clear();
    }

    fn create_new_id(&mut self) -> Id {
        let id = Id::from(self.id_counter);
        self.id_counter = self.id_counter.checked_add(1).unwrap();
        id
    }
}
