//! Simulation of physics.

pub mod collision;
pub mod command;
pub mod driven_motion;
pub mod entity;
pub mod force;
pub mod rigid_body;
pub mod systems;
pub mod tasks;

pub type PhysicsSimulator = impact_physics::PhysicsSimulator<impact_voxel::collidable::Collidable>;
