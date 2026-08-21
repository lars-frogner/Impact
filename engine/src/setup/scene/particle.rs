//! Management of particle emitters for entities.

use crate::{lock_order::OrderedRwLock, resource::ResourceManager, scene::Scene};
use anyhow::Result;
use impact_ecs::{setup, world::PrototypeEntities};
use impact_id::EntityID;
use impact_particle::{
    BasicParticleEmitterID, HasBasicParticleEmitter, ParticleEmitter, setup::BasicParticleEmission,
};
use parking_lot::RwLock;

pub fn setup_particle_emitters_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let particle_emitter_manager = &mut **scene.particle_emitter_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         particle_emission: &BasicParticleEmission|
         -> Result<HasBasicParticleEmitter> {
            let emitter_id = BasicParticleEmitterID::from_entity_id(entity_id);

            let emitter = ParticleEmitter {
                material_id: particle_emission.material_id(&resource_manager.particle_materials)?,
                log2_particle_count: particle_emission.log2_particle_count,
                simulation: particle_emission.simulation(),
            };

            particle_emitter_manager.add_basic_emitter(emitter_id, emitter)?;

            Ok(HasBasicParticleEmitter)
        },
        ![HasBasicParticleEmitter]
    )?;

    Ok(())
}

pub fn cleanup_particle_emission_for_removed_entity(
    scene: &RwLock<Scene>,
    entity_id: EntityID,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    if entity.has_component::<HasBasicParticleEmitter>() {
        let scene = scene.oread();
        let mut particle_emitter_manager = scene.particle_emitter_manager().owrite();
        let emitter_id = BasicParticleEmitterID::from_entity_id(entity_id);
        particle_emitter_manager.remove_basic_emitter(emitter_id);
    }
}
