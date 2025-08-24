//! Setup of collidables.

use crate::{
    collision::{
        Collidable, CollidableID, CollidableKind, CollisionWorld,
        collidable::{plane::PlaneCollidable, sphere::SphereCollidable},
    },
    fph,
    material::ContactResponseParameters,
    rigid_body::TypedRigidBodyID,
};
use bytemuck::{Pod, Zeroable};
use impact_geometry::{Plane, Sphere};
use roc_integration::roc;

define_setup_type! {
    target = CollidableID;
    /// A spherical collidable.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SphericalCollidable {
        kind: u64,
        sphere: Sphere<fph>,
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
        kind: u64,
        plane: Plane<fph>,
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
        sphere: Sphere<fph>,
        response_params: ContactResponseParameters,
    ) -> Self {
        Self {
            kind: kind.to_u64(),
            sphere,
            response_params,
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }

    pub fn sphere(&self) -> &Sphere<fph> {
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
        plane: Plane<fph>,
        response_params: ContactResponseParameters,
    ) -> Self {
        Self {
            kind: kind.to_u64(),
            plane,
            response_params,
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }

    pub fn plane(&self) -> &Plane<fph> {
        &self.plane
    }

    pub fn response_params(&self) -> &ContactResponseParameters {
        &self.response_params
    }
}

pub fn setup_spherical_collidable<C: Collidable>(
    collision_world: &mut CollisionWorld<C>,
    rigid_body_id: TypedRigidBodyID,
    collidable: &SphericalCollidable,
    get_local: impl FnOnce(SphereCollidable) -> C::Local,
) -> CollidableID {
    collision_world.add_collidable(
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
    rigid_body_id: TypedRigidBodyID,
    collidable: &PlanarCollidable,
    get_local: impl FnOnce(PlaneCollidable) -> C::Local,
) -> CollidableID {
    collision_world.add_collidable(
        rigid_body_id,
        collidable.kind(),
        get_local(PlaneCollidable::new(
            *collidable.plane(),
            *collidable.response_params(),
        )),
    )
}
