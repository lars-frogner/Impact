//! Management of particle emitters for entities.

use crate::{
    lock_order::OrderedRwLock, physics::PhysicsSimulator, resource::ResourceManager, scene::Scene,
};
use anyhow::{Result, bail};
use impact_ecs::{setup, world::PrototypeEntities};
use impact_geometry::ReferenceFrame;
use impact_id::EntityID;
use impact_math::vector::UnitVector3;
use impact_particle::{
    BasicParticleEmission, BasicParticleEmitterID, EmitterAnchoring, EmitterFrame,
    HasBasicParticleEmitter, RigidBodyEmitterAnchoring, WorldEmitterAnchoring,
};
use impact_physics::{
    anchor::{
        AnchorManager, DynamicRigidBodyAnchor, KinematicRigidBodyAnchor, TypedRigidBodyAnchorID,
    },
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager},
};
use impact_scene::ParentEntity;
use parking_lot::RwLock;

pub fn setup_particle_emitters_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    scene: &RwLock<Scene>,
    simulator: &RwLock<PhysicsSimulator>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let resource_manager = resource_manager.oread();
            let scene = scene.oread();
            let particle_emitter_manager = &mut **scene.particle_emitter_manager().owrite();
            let simulator = simulator.oread();
            let rigid_body_manager = simulator.rigid_body_manager().oread();
            let mut anchor_manager = simulator.anchor_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         particle_emission: &BasicParticleEmission,
         frame: Option<&ReferenceFrame>,
         parent: Option<&ParentEntity>|
         -> Result<HasBasicParticleEmitter> {
            let emitter_id = BasicParticleEmitterID::from_entity_id(entity_id);

            if !resource_manager
                .particle_materials
                .has_particle_material(particle_emission.material_id)
            {
                bail!("Got unknown material in `BasicParticleEmission`");
            }

            let (anchoring, emitter_frame) = determine_emitter_anchoring_and_frame(
                &rigid_body_manager,
                &mut anchor_manager,
                frame,
                parent,
            );

            particle_emitter_manager.add_basic_emitter(
                emitter_id,
                *particle_emission,
                anchoring,
                emitter_frame,
            )?;

            Ok(HasBasicParticleEmitter)
        },
        ![HasBasicParticleEmitter]
    )?;

    Ok(())
}

fn determine_emitter_anchoring_and_frame(
    rigid_body_manager: &RigidBodyManager,
    anchor_manager: &mut AnchorManager,
    frame: Option<&ReferenceFrame>,
    parent: Option<&ParentEntity>,
) -> (EmitterAnchoring, EmitterFrame) {
    let reference_frame = frame.copied().unwrap_or_default();

    let position = reference_frame.position;

    // Use the z-axis of the emitter's reference frame as the emission
    // direction
    let direction = reference_frame
        .orientation
        .aligned()
        .rotate_unit_vector(&UnitVector3::unit_z())
        .compact();

    let world_anchoring = WorldEmitterAnchoring {
        position,
        direction,
    };
    let mut anchoring = EmitterAnchoring::World(world_anchoring);

    // If the emitter has a parent entity, check if it is a rigid body,
    // and if so, create an anchor and update `anchoring`. Also compute
    // the appropriate `EmitterFrame`.
    let emitter_frame = if let Some(parent_entity_id) = parent.map(|parent| parent.0) {
        let parent_dynamic_rigid_body_id = DynamicRigidBodyID::from_entity_id(parent_entity_id);
        let parent_kinematic_rigid_body_id = KinematicRigidBodyID::from_entity_id(parent_entity_id);

        if let Some(rigid_body) =
            rigid_body_manager.get_dynamic_rigid_body(parent_dynamic_rigid_body_id)
        {
            let anchor_id = TypedRigidBodyAnchorID::Dynamic(anchor_manager.dynamic_mut().insert(
                DynamicRigidBodyAnchor {
                    rigid_body_id: parent_dynamic_rigid_body_id,
                    point: position,
                },
            ));
            anchoring = EmitterAnchoring::RigidBody(RigidBodyEmitterAnchoring {
                anchor_id,
                direction,
            });
            EmitterFrame::anchored_to_dynamic_rigid_body(rigid_body, &position, &direction)
        } else if let Some(rigid_body) =
            rigid_body_manager.get_kinematic_rigid_body(parent_kinematic_rigid_body_id)
        {
            let anchor_id =
                TypedRigidBodyAnchorID::Kinematic(anchor_manager.kinematic_mut().insert(
                    KinematicRigidBodyAnchor {
                        rigid_body_id: parent_kinematic_rigid_body_id,
                        point: reference_frame.position,
                    },
                ));
            anchoring = EmitterAnchoring::RigidBody(RigidBodyEmitterAnchoring {
                anchor_id,
                direction,
            });
            EmitterFrame::anchored_to_kinematic_rigid_body(rigid_body, &position, &direction)
        } else {
            EmitterFrame::world(&world_anchoring)
        }
    } else {
        EmitterFrame::world(&world_anchoring)
    };

    (anchoring, emitter_frame)
}

pub fn cleanup_particle_emission_for_removed_entity(
    scene: &RwLock<Scene>,
    simulator: &RwLock<PhysicsSimulator>,
    entity_id: EntityID,
    entity: &impact_ecs::world::EntityEntry<'_>,
) {
    if entity.has_component::<HasBasicParticleEmitter>() {
        let scene = scene.oread();
        let mut particle_emitter_manager = scene.particle_emitter_manager().owrite();
        let emitter_id = BasicParticleEmitterID::from_entity_id(entity_id);
        let metadata = particle_emitter_manager.remove_basic_emitter(emitter_id);
        drop(particle_emitter_manager);
        drop(scene);

        if let EmitterAnchoring::RigidBody(anchoring) = metadata.anchoring {
            let simulator = simulator.oread();
            let mut anchor_manager = simulator.anchor_manager().owrite();
            match anchoring.anchor_id {
                TypedRigidBodyAnchorID::Dynamic(id) => anchor_manager.dynamic_mut().remove(id),
                TypedRigidBodyAnchorID::Kinematic(id) => anchor_manager.kinematic_mut().remove(id),
            }
        }
    }
}
