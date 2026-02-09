//! Setup of voxel collidables.

use crate::collidable::{CollisionWorld, LocalCollidable, LocalVoxelObjectCollidable};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_id::EntityID;
use impact_math::vector::Vector3C;
use impact_physics::{
    collision::{CollidableID, CollidableKind},
    material::ContactResponseParameters,
    rigid_body::TypedRigidBodyID,
};
use roc_integration::roc;

define_setup_type! {
    target = CollidableID;
    /// A voxel object-based collidable.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct VoxelCollidable {
        kind: u32,
        response_params: ContactResponseParameters,
    }
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
            kind: kind.to_u32(),
            response_params,
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u32(self.kind).unwrap()
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }
}

pub fn setup_voxel_collidable(
    collision_world: &mut CollisionWorld,
    entity_id: EntityID,
    rigid_body_id: TypedRigidBodyID,
    origin_offset: Vector3C,
    collidable: &VoxelCollidable,
) -> Result<()> {
    let collidable_id = CollidableID::from_entity_id(entity_id);
    collision_world.add_collidable(
        collidable_id,
        rigid_body_id,
        collidable.kind(),
        LocalCollidable::VoxelObject(LocalVoxelObjectCollidable {
            entity_id,
            response_params: *collidable.response_params(),
            origin_offset,
        }),
    )
}
