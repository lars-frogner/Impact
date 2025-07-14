//! Setup of collidables.

use crate::{
    physics::collision::collidable::voxel::{CollisionWorld, LocalCollidable},
    voxel::VoxelObjectID,
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::SetupComponent;
use impact_physics::{
    collision::{CollidableID, CollidableKind},
    material::ContactResponseParameters,
    rigid_body::RigidBodyID,
};
use roc_integration::roc;

/// A voxel object-based collidable.
///
/// This is a [`SetupComponent`](impact_ecs::component::SetupComponent) whose
/// purpose is to aid in constructing a `CollidableID` component for an entity.
/// It is therefore not kept after entity creation.
#[roc(parents = "Setup")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct VoxelCollidable {
    kind: u64,
    response_params: ContactResponseParameters,
}

#[roc(dependencies=[CollidableKind])]
impl VoxelCollidable {
    #[roc(body = r#"
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        response_params,
    }"#)]
    pub fn new(kind: CollidableKind, response_params: ContactResponseParameters) -> Self {
        Self {
            kind: kind.to_u64(),
            response_params,
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }
}

pub fn setup_voxel_collidable(
    collision_world: &mut CollisionWorld,
    object_id: VoxelObjectID,
    rigid_body_id: RigidBodyID,
    collidable: &VoxelCollidable,
) -> CollidableID {
    collision_world.add_collidable(
        rigid_body_id,
        collidable.kind(),
        LocalCollidable::VoxelObject {
            object_id,
            response_params: *collidable.response_params(),
        },
    )
}
