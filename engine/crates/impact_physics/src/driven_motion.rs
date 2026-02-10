//! Driven motion.

pub mod circular;
pub mod constant_acceleration;
pub mod constant_rotation;
pub mod harmonic_oscillation;
pub mod orbit;
pub mod setup;

use crate::rigid_body::RigidBodyManager;
use anyhow::{Result, bail};
use circular::CircularTrajectoryRegistry;
use constant_acceleration::ConstantAccelerationTrajectoryRegistry;
use constant_rotation::ConstantRotationRegistry;
use harmonic_oscillation::HarmonicOscillatorTrajectoryRegistry;
use impact_containers::HashMap;
use orbit::OrbitalTrajectoryRegistry;
use std::{fmt, hash::Hash};

/// Manager of all motion drivers for kinematic bodies.
#[derive(Debug)]
pub struct MotionDriverManager {
    circular_trajectories: CircularTrajectoryRegistry,
    constant_acceleration_trajectories: ConstantAccelerationTrajectoryRegistry,
    constant_rotations: ConstantRotationRegistry,
    harmonic_oscillator_trajectories: HarmonicOscillatorTrajectoryRegistry,
    orbital_trajectories: OrbitalTrajectoryRegistry,
}

/// Manages all instances of a specific type of analytical motion driver.
#[derive(Clone, Debug)]
pub struct MotionDriverRegistry<Id, D> {
    drivers: HashMap<Id, D>,
}

impl MotionDriverManager {
    pub fn new() -> Self {
        Self {
            circular_trajectories: MotionDriverRegistry::new(),
            constant_acceleration_trajectories: MotionDriverRegistry::new(),
            constant_rotations: MotionDriverRegistry::new(),
            harmonic_oscillator_trajectories: MotionDriverRegistry::new(),
            orbital_trajectories: MotionDriverRegistry::new(),
        }
    }

    /// Sets the positions, velocities, orientations and angular velocities of
    /// all driven kinematic rigid bodies to the values for the given simulation
    /// time.
    pub fn apply_motion(&self, rigid_body_manager: &mut RigidBodyManager, simulation_time: f32) {
        // By first resetting the properties and then applying them additively,
        // multiple drivers can affect the same body.

        for driver in self.circular_trajectories.drivers() {
            driver.reset(rigid_body_manager);
        }
        for driver in self.constant_acceleration_trajectories.drivers() {
            driver.reset(rigid_body_manager);
        }
        for driver in self.harmonic_oscillator_trajectories.drivers() {
            driver.reset(rigid_body_manager);
        }
        for driver in self.orbital_trajectories.drivers() {
            driver.reset(rigid_body_manager);
        }

        for driver in self.circular_trajectories.drivers() {
            driver.apply(rigid_body_manager, simulation_time);
        }
        for driver in self.constant_acceleration_trajectories.drivers() {
            driver.apply(rigid_body_manager, simulation_time);
        }
        for driver in self.harmonic_oscillator_trajectories.drivers() {
            driver.apply(rigid_body_manager, simulation_time);
        }
        for driver in self.orbital_trajectories.drivers() {
            driver.apply(rigid_body_manager, simulation_time);
        }
        for driver in self.constant_rotations.drivers() {
            driver.apply(rigid_body_manager, simulation_time);
        }
    }

    pub fn circular_trajectories(&self) -> &CircularTrajectoryRegistry {
        &self.circular_trajectories
    }

    pub fn circular_trajectories_mut(&mut self) -> &mut CircularTrajectoryRegistry {
        &mut self.circular_trajectories
    }

    pub fn constant_acceleration_trajectories(&self) -> &ConstantAccelerationTrajectoryRegistry {
        &self.constant_acceleration_trajectories
    }

    pub fn constant_acceleration_trajectories_mut(
        &mut self,
    ) -> &mut ConstantAccelerationTrajectoryRegistry {
        &mut self.constant_acceleration_trajectories
    }

    pub fn constant_rotations(&self) -> &ConstantRotationRegistry {
        &self.constant_rotations
    }

    pub fn constant_rotations_mut(&mut self) -> &mut ConstantRotationRegistry {
        &mut self.constant_rotations
    }

    pub fn harmonic_oscillator_trajectories(&self) -> &HarmonicOscillatorTrajectoryRegistry {
        &self.harmonic_oscillator_trajectories
    }

    pub fn harmonic_oscillator_trajectories_mut(
        &mut self,
    ) -> &mut HarmonicOscillatorTrajectoryRegistry {
        &mut self.harmonic_oscillator_trajectories
    }

    pub fn orbital_trajectories(&self) -> &OrbitalTrajectoryRegistry {
        &self.orbital_trajectories
    }

    pub fn orbital_trajectories_mut(&mut self) -> &mut OrbitalTrajectoryRegistry {
        &mut self.orbital_trajectories
    }

    /// Removes all stored motion drivers.
    pub fn clear(&mut self) {
        self.circular_trajectories.clear();
        self.constant_acceleration_trajectories.clear();
        self.constant_rotations.clear();
        self.harmonic_oscillator_trajectories.clear();
        self.orbital_trajectories.clear();
    }
}

impl Default for MotionDriverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl<Id: Copy + Eq + Hash + fmt::Display, D> MotionDriverRegistry<Id, D> {
    fn new() -> Self {
        Self {
            drivers: HashMap::default(),
        }
    }

    /// Returns a reference to the driver with the given ID, or [`None`] if it
    /// does not exist.
    pub fn get_driver(&self, id: &Id) -> Option<&D> {
        self.drivers.get(id)
    }

    /// Returns a mutable reference to the driver with the given ID, or [`None`]
    /// if it does not exist.
    pub fn get_driver_mut(&mut self, id: &Id) -> Option<&mut D> {
        self.drivers.get_mut(id)
    }

    /// Returns an iterator over all drivers.
    pub fn drivers(&self) -> impl Iterator<Item = &D> {
        self.drivers.values()
    }

    /// Adds the given motion driver to the map under the given ID.
    ///
    /// # Errors
    /// Returns an error if the given driver ID already exists.
    pub fn insert_driver(&mut self, id: Id, driver: D) -> Result<()> {
        if self.drivers.contains_key(&id) {
            bail!("A motion driver with ID {id} already exists");
        }
        self.drivers.insert(id, driver);
        Ok(())
    }

    /// Removes the motion driver with the given ID from the map if it exists.
    pub fn remove_driver(&mut self, id: Id) {
        self.drivers.remove(&id);
    }

    fn clear(&mut self) {
        self.drivers.clear();
    }
}
