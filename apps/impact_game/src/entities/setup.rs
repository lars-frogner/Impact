//! Setup and cleanup of new and removed entities.

use crate::Game;
use anyhow::Result;
use impact::impact_ecs::world::PrototypeEntities;

pub fn perform_setup_for_new_entities(
    _game: &Game,
    _entities: &mut PrototypeEntities,
) -> Result<()> {
    Ok(())
}
