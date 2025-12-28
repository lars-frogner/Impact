//! Setup of voxel collidables.

use crate::{
    VoxelObjectID,
    collidable::{CollisionWorld, LocalCollidable, LocalVoxelObjectCollidable},
};
use bytemuck::{Pod, Zeroable};
use impact_math::vector::Vector3P;
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
    object_id: VoxelObjectID,
    rigid_body_id: TypedRigidBodyID,
    origin_offset: Vector3P,
    collidable: &VoxelCollidable,
) -> CollidableID {
    collision_world.add_collidable(
        rigid_body_id,
        collidable.kind(),
        LocalCollidable::VoxelObject(LocalVoxelObjectCollidable {
            object_id,
            response_params: *collidable.response_params(),
            origin_offset,
        }),
    )
}
