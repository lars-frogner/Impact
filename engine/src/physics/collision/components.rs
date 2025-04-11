//! [`Component`](impact_ecs::component::Component)s related to collisions.

use crate::{
    geometry::{Plane, Sphere},
    physics::{
        collision::{CollidableID, CollidableKind},
        fph,
        motion::Position,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, SetupComponent};
use nalgebra::UnitVector3;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a spherical collidable.
///
/// The purpose of this component is to aid in constructing a [`CollidableComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct SphereCollidableComp {
    kind: u64,
    center: Position,
    radius: fph,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a planar collidable.
///
/// The purpose of this component is to aid in constructing a [`CollidableComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct PlaneCollidableComp {
    kind: u64,
    unit_normal: UnitVector3<fph>,
    displacement: fph,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that use their voxel object as a collidable.
///
/// The purpose of this component is to aid in constructing a [`CollidableComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct VoxelObjectCollidableComp {
    kind: u64,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// collidable in the [`CollisionWorld`](super::CollisionWorld).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct CollidableComp {
    /// The ID of the entity's collidable.
    pub collidable_id: CollidableID,
}

impl SphereCollidableComp {
    pub fn new(kind: CollidableKind, sphere: &Sphere<fph>) -> Self {
        Self {
            kind: kind.to_u64(),
            center: *sphere.center(),
            radius: sphere.radius(),
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }

    pub fn sphere(&self) -> Sphere<fph> {
        Sphere::new(self.center, self.radius)
    }
}

impl PlaneCollidableComp {
    pub fn new(kind: CollidableKind, plane: &Plane<fph>) -> Self {
        Self {
            kind: kind.to_u64(),
            unit_normal: *plane.unit_normal(),
            displacement: plane.displacement(),
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }

    pub fn plane(&self) -> Plane<fph> {
        Plane::new(self.unit_normal, self.displacement)
    }
}

impl VoxelObjectCollidableComp {
    pub fn new(kind: CollidableKind) -> Self {
        Self {
            kind: kind.to_u64(),
        }
    }

    pub fn kind(&self) -> CollidableKind {
        CollidableKind::from_u64(self.kind).unwrap()
    }
}
