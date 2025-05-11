//! [`Component`](impact_ecs::component::Component)s related to collisions.

use crate::{
    geometry::{Plane, Sphere},
    physics::{
        collision::{CollidableID, CollidableKind},
        fph,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, SetupComponent};
use roc_codegen::roc;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a spherical collidable.
///
/// The purpose of this component is to aid in constructing a [`CollidableComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "SphereCollidable")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct SphereCollidableComp {
    kind: u64,
    sphere: Sphere<fph>,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a planar collidable.
///
/// The purpose of this component is to aid in constructing a [`CollidableComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "PlaneCollidable")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct PlaneCollidableComp {
    kind: u64,
    plane: Plane<fph>,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that use their voxel object as a collidable.
///
/// The purpose of this component is to aid in constructing a [`CollidableComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc(parents = "Comp", name = "VoxelObjectCollidable")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct VoxelObjectCollidableComp {
    kind: u64,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// collidable in the [`CollisionWorld`](super::CollisionWorld).
#[roc(parents = "Comp", name = "Collidable")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct CollidableComp {
    /// The ID of the entity's collidable.
    pub collidable_id: CollidableID,
}

#[roc(dependencies=[CollidableKind])]
impl SphereCollidableComp {
    #[roc(body = r#"
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        sphere,
    }"#)]
    pub fn new(kind: CollidableKind, sphere: Sphere<fph>) -> Self {
        Self {
            kind: kind.to_u64(),
            sphere,
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }

    pub fn sphere(&self) -> &Sphere<fph> {
        &self.sphere
    }
}

#[roc(dependencies=[CollidableKind])]
impl PlaneCollidableComp {
    #[roc(body = r#"
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
        plane,
    }"#)]
    pub fn new(kind: CollidableKind, plane: Plane<fph>) -> Self {
        Self {
            kind: kind.to_u64(),
            plane,
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }

    pub fn plane(&self) -> &Plane<fph> {
        &self.plane
    }
}

#[roc(dependencies=[CollidableKind])]
impl VoxelObjectCollidableComp {
    #[roc(body = r#"
    {
        kind:
        when kind is
            Dynamic -> 0
            Static -> 1
            Phantom -> 2,
    }"#)]
    pub fn new(kind: CollidableKind) -> Self {
        Self {
            kind: kind.to_u64(),
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }
}
