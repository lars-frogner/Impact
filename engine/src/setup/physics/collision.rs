//! Setup of collidables for new entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator};
use anyhow::Result;
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_geometry::ModelTransform;
use impact_id::EntityID;
use impact_physics::{
    collision::{
        self, CollidableID, HasCollidable,
        setup::{PlanarCollidable, SphericalCollidable},
    },
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID},
};
use impact_voxel::{
    HasVoxelObject,
    collidable::{LocalCollidable, setup::VoxelCollidable},
};
use parking_lot::RwLock;

/// Checks if the given entities have a component representing a collidable, and
/// if so, creates the corresponding collidables and adds the [`CollidableID`]s
/// to the entity.
pub fn setup_collidables_for_new_entities(
    simulator: &RwLock<PhysicsSimulator>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
        |entity_id: EntityID,
         spherical_collidable: &SphericalCollidable,
         rigid_body_id: &DynamicRigidBodyID|
         -> Result<HasCollidable> {
            collision::setup::setup_spherical_collidable(
                &mut collision_world,
                entity_id,
                (*rigid_body_id).into(),
                spherical_collidable,
                LocalCollidable::Sphere,
            )?;
            Ok(HasCollidable)
        },
        ![HasCollidable]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
        |entity_id: EntityID,
         spherical_collidable: &SphericalCollidable,
         rigid_body_id: &KinematicRigidBodyID|
         -> Result<HasCollidable> {
            collision::setup::setup_spherical_collidable(
                &mut collision_world,
                entity_id,
                (*rigid_body_id).into(),
                spherical_collidable,
                LocalCollidable::Sphere,
            )?;
            Ok(HasCollidable)
        },
        ![HasCollidable]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
        |entity_id: EntityID,
         planar_collidable: &PlanarCollidable,
         rigid_body_id: &DynamicRigidBodyID|
         -> Result<HasCollidable> {
            collision::setup::setup_planar_collidable(
                &mut collision_world,
                entity_id,
                (*rigid_body_id).into(),
                planar_collidable,
                LocalCollidable::Plane,
            )?;
            Ok(HasCollidable)
        },
        ![HasCollidable]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
        |entity_id: EntityID,
         planar_collidable: &PlanarCollidable,
         rigid_body_id: &KinematicRigidBodyID|
         -> Result<HasCollidable> {
            collision::setup::setup_planar_collidable(
                &mut collision_world,
                entity_id,
                (*rigid_body_id).into(),
                planar_collidable,
                LocalCollidable::Plane,
            )?;
            Ok(HasCollidable)
        },
        ![HasCollidable]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_collidable: &VoxelCollidable,
         rigid_body_id: &DynamicRigidBodyID,
         model_transform: &ModelTransform|
         -> Result<HasCollidable> {
            impact_voxel::collidable::setup::setup_voxel_collidable(
                &mut collision_world,
                entity_id,
                (*rigid_body_id).into(),
                model_transform.offset,
                voxel_collidable,
            )?;
            Ok(HasCollidable)
        },
        [HasVoxelObject],
        ![HasCollidable]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut collision_world = simulator.collision_world().owrite();
        },
        entities,
        |entity_id: EntityID,
         voxel_collidable: &VoxelCollidable,
         rigid_body_id: &KinematicRigidBodyID,
         model_transform: Option<&ModelTransform>|
         -> Result<HasCollidable> {
            impact_voxel::collidable::setup::setup_voxel_collidable(
                &mut collision_world,
                entity_id,
                (*rigid_body_id).into(),
                model_transform
                    .map(|model_transform| model_transform.offset)
                    .unwrap_or_default(),
                voxel_collidable,
            )?;
            Ok(HasCollidable)
        },
        [HasVoxelObject],
        ![HasCollidable]
    )
}

pub fn remove_collidable_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    if entity.has_component::<HasCollidable>() {
        let simulator = simulator.oread();
        let mut collision_world = simulator.collision_world().owrite();
        let collidable_id = CollidableID::from_entity_id(entity_id);
        collision_world.remove_collidable(collidable_id);
    }
}
