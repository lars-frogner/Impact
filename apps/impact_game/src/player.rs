//! Player character.

pub mod inventory;
pub mod tools;

use anyhow::{Context, Result, anyhow};
use impact::{
    engine::Engine, impact_ecs::world::EntityID, impact_physics::rigid_body::DynamicRigidBody,
};
use inventory::Inventory;

#[derive(Clone, Debug)]
pub struct Player {
    pub inventory: Inventory,
}

#[derive(Clone, Debug)]
pub struct EntityIDs {
    pub player: EntityID,
    pub player_body: EntityID,
    pub player_head: EntityID,
}

impl Player {
    pub const fn entity_ids() -> EntityIDs {
        EntityIDs {
            player: EntityID::hashed_from_str("player"),
            player_body: EntityID::hashed_from_str("player_body"),
            player_head: EntityID::hashed_from_str("player_head"),
        }
    }

    pub fn new() -> Self {
        Self {
            inventory: Inventory::empty(),
        }
    }

    pub fn with_rigid_body<R>(
        engine: &Engine,
        f: impl FnOnce(&DynamicRigidBody) -> Result<R>,
    ) -> Result<R> {
        let rigid_body_id = engine
            .get_component_copy(Self::entity_ids().player)
            .with_context(|| anyhow!("Failed to get `DynamicRigidBodyID` component for player"))?;

        engine.with_dynamic_rigid_body(rigid_body_id, f)
    }
}
