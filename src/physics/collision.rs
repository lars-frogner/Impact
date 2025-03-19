//! Collision detection and resolution.

pub mod components;
pub mod entity;
pub mod systems;

use crate::{
    geometry::{Plane, Sphere},
    physics::{
        constraint::contact::{Contact, ContactSet},
        fph,
    },
    voxel::VoxelObjectID,
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::world::Entity;
use nalgebra::{Similarity3, UnitVector3, Vector3};
use std::collections::HashMap;

#[derive(Debug)]
pub struct CollisionWorld {
    collidable_descriptors: HashMap<CollidableID, CollidableDescriptor>,
    collidables: [Vec<Collidable>; 3],
    collidable_id_counter: u32,
}

#[derive(Clone, Debug)]
pub struct CollidableDescriptor {
    kind: CollidableKind,
    geometry: LocalCollidableGeometry,
    entity: Entity,
    idx: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollidableKind {
    Dynamic = 0,
    Static = 1,
    Phantom = 2,
}

#[derive(Clone, Debug)]
pub struct Collidable {
    id: CollidableID,
    geometry: WorldCollidableGeometry,
}

#[derive(Clone, Debug)]
pub enum LocalCollidableGeometry {
    Sphere(SphereCollidableGeometry),
    Plane(PlaneCollidableGeometry),
    VoxelObject(VoxelObjectID),
}

#[derive(Clone, Debug)]
pub enum WorldCollidableGeometry {
    Sphere(SphereCollidableGeometry),
    Plane(PlaneCollidableGeometry),
    VoxelObject(VoxelObjectCollidableGeometry),
}

#[derive(Clone, Debug)]
pub struct SphereCollidableGeometry {
    sphere: Sphere<fph>,
}

#[derive(Clone, Debug)]
pub struct PlaneCollidableGeometry {
    plane: Plane<fph>,
}

#[derive(Clone, Debug)]
pub struct VoxelObjectCollidableGeometry {
    object_id: VoxelObjectID,
    transform_to_world_space: Similarity3<fph>,
}

/// Identifier for a collidable in a [`CollisionWorld`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct CollidableID(u32);

#[derive(Clone, Debug)]
pub struct Collision<'a> {
    pub collider_a: &'a CollidableDescriptor,
    pub collider_b: &'a CollidableDescriptor,
    pub contact_set: &'a ContactSet,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CollidableOrder {
    Original,
    Swapped,
}

impl CollisionWorld {
    pub fn new() -> Self {
        Self {
            collidable_descriptors: HashMap::new(),
            collidables: [Vec::new(), Vec::new(), Vec::new()],
            collidable_id_counter: 0,
        }
    }

    pub fn add_sphere_collidable(
        &mut self,
        kind: CollidableKind,
        sphere: Sphere<fph>,
    ) -> CollidableID {
        self.add_collidable(
            kind,
            LocalCollidableGeometry::Sphere(SphereCollidableGeometry { sphere }),
        )
    }

    pub fn add_plane_collidable(
        &mut self,
        kind: CollidableKind,
        plane: Plane<fph>,
    ) -> CollidableID {
        self.add_collidable(
            kind,
            LocalCollidableGeometry::Plane(PlaneCollidableGeometry { plane }),
        )
    }

    pub fn synchronize_collidable(
        &mut self,
        collidable_id: CollidableID,
        entity: Entity,
        transform_to_world_space: Similarity3<fph>,
    ) {
        let descriptor = self
            .collidable_descriptors
            .get_mut(&collidable_id)
            .expect("Missing descriptor for collidable");

        descriptor.entity = entity;

        let collidable = Collidable::new(
            collidable_id,
            WorldCollidableGeometry::from_local(&descriptor.geometry, transform_to_world_space),
        );

        let collidables_of_kind = &mut self.collidables[descriptor.kind as usize];
        descriptor.idx = collidables_of_kind.len();
        collidables_of_kind.push(collidable);
    }

    pub fn clear_spatial_state(&mut self) {
        for collidables_of_kind in &mut self.collidables {
            collidables_of_kind.clear();
        }
        for descriptor in self.collidable_descriptors.values_mut() {
            descriptor.idx = usize::MAX;
        }
    }

    pub fn for_each_collision_with_collidable(
        &self,
        collidable_id: CollidableID,
        f: &mut impl FnMut(Collision<'_>),
    ) {
        let descriptor_a = self.collidable_descriptor(collidable_id);
        let collidable_a = self.collidable_with_descriptor(descriptor_a);

        let mut contact_set = ContactSet::new();

        for collidables_of_kind in &self.collidables {
            for collidable_b in collidables_of_kind {
                if collidable_b.id == collidable_a.id {
                    continue;
                }

                let order = collidable_a.determine_contact_set(collidable_b, &mut contact_set);

                if !contact_set.is_empty() {
                    let descriptor_b = self.collidable_descriptor(collidable_b.id);

                    let (collider_a, collider_b) =
                        order.swap_if_required(descriptor_a, descriptor_b);

                    f(Collision {
                        collider_a,
                        collider_b,
                        contact_set: &contact_set,
                    });

                    contact_set.clear();
                }
            }
        }
    }

    pub fn for_each_non_phantom_collision_involving_dynamic_collidable(
        &self,
        f: &mut impl FnMut(Collision<'_>),
    ) {
        let dynamic_collidables = self.collidables(CollidableKind::Dynamic);
        let static_collidables = self.collidables(CollidableKind::Static);

        let mut contact_set = ContactSet::new();

        for (idx, collidable_a) in dynamic_collidables.iter().enumerate() {
            let descriptor_a = self.collidable_descriptor(collidable_a.id);

            for collidable_b in dynamic_collidables[idx + 1..]
                .iter()
                .chain(static_collidables)
            {
                let order = collidable_a.determine_contact_set(collidable_b, &mut contact_set);

                if !contact_set.is_empty() {
                    let descriptor_b = self.collidable_descriptor(collidable_b.id);

                    let (collider_a, collider_b) =
                        order.swap_if_required(descriptor_a, descriptor_b);

                    f(Collision {
                        collider_a,
                        collider_b,
                        contact_set: &contact_set,
                    });

                    contact_set.clear();
                }
            }
        }
    }

    fn collidables(&self, kind: CollidableKind) -> &[Collidable] {
        &self.collidables[kind as usize]
    }

    fn collidable_descriptor(&self, collidable_id: CollidableID) -> &CollidableDescriptor {
        self.collidable_descriptors
            .get(&collidable_id)
            .expect("Missing descriptor for collidable")
    }

    fn collidable_with_descriptor(&self, descriptor: &CollidableDescriptor) -> &Collidable {
        &self.collidables(descriptor.kind)[descriptor.idx]
    }

    fn add_collidable(
        &mut self,
        kind: CollidableKind,
        geometry: LocalCollidableGeometry,
    ) -> CollidableID {
        let descriptor = CollidableDescriptor::new(kind, geometry);
        let collidable_id = self.create_new_collidable_id();
        self.collidable_descriptors
            .insert(collidable_id, descriptor);
        collidable_id
    }

    fn create_new_collidable_id(&mut self) -> CollidableID {
        let collidable_id = CollidableID(self.collidable_id_counter);
        self.collidable_id_counter = self.collidable_id_counter.checked_add(1).unwrap();
        collidable_id
    }
}

impl Default for CollisionWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl CollidableDescriptor {
    fn new(kind: CollidableKind, geometry: LocalCollidableGeometry) -> Self {
        Self {
            kind,
            geometry,
            entity: Entity::zeroed(),
            idx: usize::MAX,
        }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn kind(&self) -> CollidableKind {
        self.kind
    }
}

impl CollidableKind {
    fn to_u64(self) -> u64 {
        self as u64
    }

    fn from_u64(number: u64) -> Option<Self> {
        match number {
            0 => Some(Self::Dynamic),
            1 => Some(Self::Static),
            2 => Some(Self::Phantom),
            _ => None,
        }
    }
}

impl Collidable {
    fn new(id: CollidableID, geometry: WorldCollidableGeometry) -> Self {
        Self { id, geometry }
    }

    fn determine_contact_set(&self, other: &Self, contact_set: &mut ContactSet) -> CollidableOrder {
        self.geometry
            .determine_contact_set(&other.geometry, contact_set)
    }
}

impl WorldCollidableGeometry {
    fn from_local(
        geometry: &LocalCollidableGeometry,
        transform_to_world_space: Similarity3<fph>,
    ) -> Self {
        match geometry {
            LocalCollidableGeometry::Sphere(sphere) => {
                Self::Sphere(sphere.to_world(&transform_to_world_space))
            }
            LocalCollidableGeometry::Plane(plane) => {
                Self::Plane(plane.to_world(&transform_to_world_space))
            }
            LocalCollidableGeometry::VoxelObject(object_id) => Self::VoxelObject(
                VoxelObjectCollidableGeometry::new(*object_id, transform_to_world_space),
            ),
        }
    }

    fn determine_contact_set(&self, other: &Self, contact_set: &mut ContactSet) -> CollidableOrder {
        match (self, other) {
            (Self::VoxelObject(this), Self::VoxelObject(other)) => {
                this.determine_contact_set_with_other(other, contact_set);
                CollidableOrder::Original
            }
            (Self::VoxelObject(voxel_object), Self::Sphere(sphere)) => {
                voxel_object.determine_contact_set_with_sphere(sphere, contact_set);
                CollidableOrder::Original
            }
            (Self::VoxelObject(voxel_object), Self::Plane(plane)) => {
                voxel_object.determine_contact_set_with_plane(plane, contact_set);
                CollidableOrder::Original
            }
            (Self::Sphere(this), Self::Sphere(other)) => {
                this.determine_contact_set_with_other(other, contact_set);
                CollidableOrder::Original
            }
            (Self::Sphere(sphere), Self::VoxelObject(voxel_object)) => {
                voxel_object.determine_contact_set_with_sphere(sphere, contact_set);
                CollidableOrder::Swapped
            }
            (Self::Sphere(sphere), Self::Plane(plane)) => {
                sphere.determine_contact_set_with_plane(plane, contact_set);
                CollidableOrder::Original
            }
            (Self::Plane(_), Self::Plane(_)) => {
                // Not useful
                CollidableOrder::Original
            }
            (Self::Plane(plane), Self::Sphere(sphere)) => {
                sphere.determine_contact_set_with_plane(plane, contact_set);
                CollidableOrder::Swapped
            }
            (Self::Plane(plane), Self::VoxelObject(voxel_object)) => {
                voxel_object.determine_contact_set_with_plane(plane, contact_set);
                CollidableOrder::Swapped
            }
        }
    }
}

impl SphereCollidableGeometry {
    fn to_world(&self, transform_to_world_space: &Similarity3<fph>) -> Self {
        Self {
            sphere: self.sphere.transformed(transform_to_world_space),
        }
    }

    fn determine_contact_set_with_other(&self, other: &Self, contact_set: &mut ContactSet) {
        let center_displacement = self.sphere.center() - other.sphere.center();
        let squared_center_distance = center_displacement.norm_squared();

        if squared_center_distance
            <= self.sphere.radius_squared()
                + other.sphere.radius_squared()
                + 2.0 * self.sphere.radius() * other.sphere.radius()
        {
            let center_distance = squared_center_distance.sqrt();

            let surface_normal = if center_distance > 1e-8 {
                UnitVector3::new_unchecked(center_displacement.unscale(center_distance))
            } else {
                Vector3::z_axis()
            };

            let position = other.sphere.center() + surface_normal.scale(other.sphere.radius());

            let penetration_depth = fph::max(
                0.0,
                (self.sphere.radius() + other.sphere.radius()) - center_distance,
            );

            contact_set.add_contact(Contact {
                position,
                surface_normal,
                penetration_depth,
            });
        }
    }

    fn determine_contact_set_with_plane(
        &self,
        plane: &PlaneCollidableGeometry,
        contact_set: &mut ContactSet,
    ) {
        let signed_distance = plane.plane.compute_signed_distance(self.sphere.center());
        let penetration_depth = self.sphere.radius() - signed_distance;

        if penetration_depth >= 0.0 {
            let surface_normal = *plane.plane.unit_normal();
            let position = self.sphere.center() - surface_normal.scale(signed_distance);

            contact_set.add_contact(Contact {
                position,
                surface_normal,
                penetration_depth,
            });
        }
    }
}

impl PlaneCollidableGeometry {
    fn to_world(&self, transform_to_world_space: &Similarity3<fph>) -> Self {
        Self {
            plane: self.plane.transformed(transform_to_world_space),
        }
    }
}

impl VoxelObjectCollidableGeometry {
    fn new(object_id: VoxelObjectID, transform_to_world_space: Similarity3<fph>) -> Self {
        Self {
            object_id,
            transform_to_world_space,
        }
    }

    fn determine_contact_set_with_other(&self, other: &Self, contact_set: &mut ContactSet) {
        todo!()
    }

    fn determine_contact_set_with_sphere(
        &self,
        sphere: &SphereCollidableGeometry,
        contact_set: &mut ContactSet,
    ) {
        todo!()
    }

    fn determine_contact_set_with_plane(
        &self,
        plane: &PlaneCollidableGeometry,
        contact_set: &mut ContactSet,
    ) {
        todo!()
    }
}

impl CollidableOrder {
    fn swap_if_required<T>(self, a: T, b: T) -> (T, T) {
        match self {
            Self::Original => (a, b),
            Self::Swapped => (b, a),
        }
    }
}
