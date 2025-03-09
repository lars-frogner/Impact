//! Collision detection and resolution.
#![allow(clippy::all, dead_code)]

pub mod components;
pub mod response;
pub mod systems;

use crate::{
    geometry::{Plane, Sphere},
    physics::{fph, motion::Position},
    voxel::VoxelObjectID,
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::world::Entity;
use nalgebra::{Point3, Similarity3, UnitVector3, Vector3};
use std::collections::HashMap;
use tinyvec::TinyVec;

#[derive(Debug)]
pub struct CollisionWorld {
    collidable_descriptors: HashMap<CollidableID, CollidableDescriptor>,
    collidables: [Vec<Collidable>; 3],
}

#[derive(Clone, Debug)]
pub struct CollidableDescriptor {
    entity: Entity,
    kind: CollidableKind,
    geometry: LocalCollidableGeometry,
    idx: usize,
}

#[derive(Clone, Copy, Debug)]
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

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct CollidableID(u32);

#[derive(Clone, Debug)]
pub struct ContactSet {
    contacts: TinyVec<[Contact; 4]>,
}

#[derive(Clone, Debug)]
pub struct Contact {
    position: Position,
    surface_normal: UnitVector3<fph>,
    penetration_depth: fph,
}

impl CollisionWorld {
    pub fn new() -> Self {
        Self {
            collidable_descriptors: HashMap::new(),
            collidables: [Vec::new(), Vec::new(), Vec::new()],
        }
    }

    pub fn collidables_of_kind(&self, kind: CollidableKind) -> &[Collidable] {
        &self.collidables[kind as usize]
    }

    pub fn get_collidable_descriptor(
        &self,
        collidable_id: CollidableID,
    ) -> Option<&CollidableDescriptor> {
        self.collidable_descriptors.get(&collidable_id)
    }

    pub fn collidable_descriptor(&self, collidable_id: CollidableID) -> &CollidableDescriptor {
        self.get_collidable_descriptor(collidable_id)
            .expect("Missing descriptor for collidable")
    }

    pub fn get_collidable(&self, collidable_id: CollidableID) -> Option<&Collidable> {
        self.get_collidable_descriptor(collidable_id)
            .and_then(|descriptor| self.get_collidable_with_descriptor(descriptor))
    }

    pub fn collidable(&self, collidable_id: CollidableID) -> &Collidable {
        self.collidable_with_descriptor(self.collidable_descriptor(collidable_id))
    }

    fn get_collidable_with_descriptor(
        &self,
        descriptor: &CollidableDescriptor,
    ) -> Option<&Collidable> {
        self.collidables_of_kind(descriptor.kind)
            .get(descriptor.idx)
    }

    pub fn collidable_with_descriptor(&self, descriptor: &CollidableDescriptor) -> &Collidable {
        self.get_collidable_with_descriptor(descriptor)
            .expect("Missing collidable for descriptor")
    }

    pub fn for_each_collision_of_collidable_with_collidables_of_kind(
        &self,
        collidable_id: CollidableID,
        kind: CollidableKind,
        f: &mut impl FnMut(&Entity, &ContactSet),
    ) {
        let collidable_a = self.collidable(collidable_id);

        let mut contact_set = ContactSet::new();

        for collidable_b in self.collidables_of_kind(kind) {
            if collidable_b.id == collidable_a.id {
                continue;
            }

            collidable_a.determine_contact_set(collidable_b, &mut contact_set);

            if !contact_set.is_empty() {
                let descriptor_b = self.collidable_descriptor(collidable_b.id);

                f(descriptor_b.entity(), &contact_set);

                contact_set.clear();
            }
        }
    }

    pub fn clear_spatial_state(&mut self) {
        for collidables_of_kind in &mut self.collidables {
            collidables_of_kind.clear();
        }
        for descriptor in self.collidable_descriptors.values_mut() {
            descriptor.idx = usize::MAX;
        }
    }

    pub fn synchronize_spatial_state_for_collidable(
        &mut self,
        collidable_id: CollidableID,
        transform_to_world_space: Similarity3<fph>,
    ) {
        let descriptor = self
            .collidable_descriptors
            .get_mut(&collidable_id)
            .expect("Missing descriptor for collidable");

        let collidable = Collidable::new(
            collidable_id,
            WorldCollidableGeometry::from_local(&descriptor.geometry, transform_to_world_space),
        );

        let collidables_of_kind = &mut self.collidables[descriptor.kind as usize];
        descriptor.idx = collidables_of_kind.len();
        collidables_of_kind.push(collidable);
    }
}

impl CollidableDescriptor {
    pub fn entity(&self) -> &Entity {
        &self.entity
    }

    pub fn kind(&self) -> CollidableKind {
        self.kind
    }
}

impl Collidable {
    fn new(id: CollidableID, geometry: WorldCollidableGeometry) -> Self {
        Self { id, geometry }
    }

    fn determine_contact_set(&self, other: &Self, contact_set: &mut ContactSet) {
        self.geometry
            .determine_contact_set(&other.geometry, contact_set);
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

    fn determine_contact_set(&self, other: &Self, contact_set: &mut ContactSet) {
        match (self, other) {
            (Self::Sphere(this), Self::Sphere(other)) => {
                this.determine_contact_set_with_other(other, contact_set);
            }
            (Self::Sphere(sphere), Self::Plane(plane)) => {
                sphere.determine_contact_set_with_plane(plane, contact_set);
            }
            (Self::Sphere(sphere), Self::VoxelObject(voxel_object)) => {
                sphere.determine_contact_set_with_voxel_object(voxel_object, contact_set);
            }
            (Self::Plane(plane), Self::Sphere(sphere)) => {
                plane.determine_contact_set_with_sphere(sphere, contact_set);
            }
            (Self::Plane(this), Self::Plane(other)) => {
                this.determine_contact_set_with_other(other, contact_set);
            }
            (Self::Plane(plane), Self::VoxelObject(voxel_object)) => {
                plane.determine_contact_set_with_voxel_object(voxel_object, contact_set);
            }
            (Self::VoxelObject(voxel_object), Self::Sphere(sphere)) => {
                voxel_object.determine_contact_set_with_sphere(sphere, contact_set);
            }
            (Self::VoxelObject(voxel_object), Self::Plane(plane)) => {
                voxel_object.determine_contact_set_with_plane(plane, contact_set);
            }
            (Self::VoxelObject(this), Self::VoxelObject(other)) => {
                this.determine_contact_set_with_other(other, contact_set);
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
        let center_displacement = other.sphere.center() - self.sphere.center();
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
        todo!()
    }

    fn determine_contact_set_with_voxel_object(
        &self,
        voxel_object: &VoxelObjectCollidableGeometry,
        contact_set: &mut ContactSet,
    ) {
        todo!()
    }
}

impl PlaneCollidableGeometry {
    fn to_world(&self, transform_to_world_space: &Similarity3<fph>) -> Self {
        Self {
            plane: self.plane.transformed(transform_to_world_space),
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

    fn determine_contact_set_with_voxel_object(
        &self,
        voxel_object: &VoxelObjectCollidableGeometry,
        contact_set: &mut ContactSet,
    ) {
        todo!()
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

impl Default for Contact {
    fn default() -> Self {
        Self {
            position: Point3::origin(),
            surface_normal: Vector3::z_axis(),
            penetration_depth: 0.0,
        }
    }
}

impl ContactSet {
    pub fn new() -> Self {
        Self {
            contacts: TinyVec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }

    pub fn contacts(&self) -> &[Contact] {
        self.contacts.as_slice()
    }

    pub fn clear(&mut self) {
        self.contacts.clear();
    }

    pub fn add_contact(&mut self, contact: Contact) {
        self.contacts.push(contact);
    }
}

impl Contact {
    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn surface_normal(&self) -> &UnitVector3<fph> {
        &self.surface_normal
    }

    pub fn penetration_depth(&self) -> fph {
        self.penetration_depth
    }
}
