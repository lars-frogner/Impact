//! Simulation of physics.

pub mod collision;
pub mod driven_motion;
pub mod entity;
pub mod force;
pub mod rigid_body;
pub mod systems;

pub type PhysicsSimulator = impact_physics::PhysicsSimulator<impact_voxel::collidable::Collidable>;
