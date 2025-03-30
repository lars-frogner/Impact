//! Implementation of an [Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system) engine.

pub mod archetype;
pub mod component;
pub mod world;

pub use impact_ecs_macros::Component;
pub use impact_ecs_macros::archetype_of;
pub use impact_ecs_macros::query;
pub use impact_ecs_macros::setup;
