//! Setup of collidables.

use crate::{
    collision::{
        Collidable, CollidableID, CollidableKind, CollisionWorld,
        collidable::{plane::PlaneCollidable, sphere::SphereCollidable},
    },
    material::ContactResponseParameters,
    rigid_body::{RigidBodyType, TypedRigidBodyID},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_geometry::{PlaneC, SphereC};
use impact_id::EntityID;
use roc_integration::roc;

define_setup_type! {
    target = CollidableID;
    /// A spherical collidable.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SphericalCollidable {
        kind: u32,
        sphere: SphereC,
        response_params: ContactResponseParameters,
    }
}

define_setup_type! {
    target = CollidableID;
    /// A planar collidable.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct PlanarCollidable {
        kind: u32,
        plane: PlaneC,
        response_params: ContactResponseParameters,
    }
}

#[roc(dependencies=[CollidableKind])]
impl SphericalCollidable {
    #[roc(body = r#"
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        sphere,
        response_params,
    }"#)]
    pub fn new(
        kind: CollidableKind,
        sphere: SphereC,
        response_params: ContactResponseParameters,
    ) -> Self {
        Self {
            kind: kind.to_u32(),
            sphere,
            response_params,
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u32(self.kind).unwrap()
    }

    pub fn sphere(&self) -> &SphereC {
        &self.sphere
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }
}

#[roc(dependencies=[CollidableKind])]
impl PlanarCollidable {
    #[roc(body = r#"
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        plane,
        response_params,
    }"#)]
    pub fn new(
        kind: CollidableKind,
        plane: PlaneC,
        response_params: ContactResponseParameters,
    ) -> Self {
        Self {
            kind: kind.to_u32(),
            plane,
            response_params,
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u32(self.kind).unwrap()
    }

    pub fn plane(&self) -> &PlaneC {
        &self.plane
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }
}

pub fn setup_spherical_collidable<C: Collidable>(
    collision_world: &mut CollisionWorld<C>,
    entity_id: EntityID,
    rigid_body_type: RigidBodyType,
    collidable: &SphericalCollidable,
    get_local: impl FnOnce(SphereCollidable) -> C::Local,
) -> Result<()> {
    let collidable_id = CollidableID::from_entity_id(entity_id);
    let rigid_body_id = TypedRigidBodyID::from_entity_id_and_type(entity_id, rigid_body_type);
    collision_world.add_collidable(
        collidable_id,
        rigid_body_id,
        collidable.kind(),
        get_local(SphereCollidable::new(
            *collidable.sphere(),
            *collidable.response_params(),
        )),
    )
}

pub fn setup_planar_collidable<C: Collidable>(
    collision_world: &mut CollisionWorld<C>,
    entity_id: EntityID,
    rigid_body_type: RigidBodyType,
    collidable: &PlanarCollidable,
    get_local: impl FnOnce(PlaneCollidable) -> C::Local,
) -> Result<()> {
    let collidable_id = CollidableID::from_entity_id(entity_id);
    let rigid_body_id = TypedRigidBodyID::from_entity_id_and_type(entity_id, rigid_body_type);
    collision_world.add_collidable(
        collidable_id,
        rigid_body_id,
        collidable.kind(),
        get_local(PlaneCollidable::new(
            *collidable.plane(),
            *collidable.response_params(),
        )),
    )
}
