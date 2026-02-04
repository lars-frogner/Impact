//! Setup and cleanup of new and removed entities.

use crate::Game;
use anyhow::Result;
use impact::impact_ecs::archetype::ArchetypeComponentStorage;

pub fn perform_setup_for_new_entities(
    _game: &Game,
    _components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    Ok(())
}
