//! Collision detection and resolution.
#![allow(clippy::all, dead_code)]

pub mod components;
pub mod systems;

use crate::{
    geometry::{Plane, Sphere},
    physics::fph,
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
    position: Point3<fph>,
    surface_normal: UnitVector3<fph>,
    penetration_depth: fph,
}

#[derive(Clone, Debug)]
pub struct Collision<'a> {
    pub collider_a: &'a CollidableDescriptor,
    pub collider_b: &'a CollidableDescriptor,
    pub contact_set: &'a ContactSet,
}

impl CollisionWorld {
    pub fn new() -> Self {
        Self {
            collidable_descriptors: HashMap::new(),
            collidables: [Vec::new(), Vec::new(), Vec::new()],
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

                collidable_a.determine_contact_set(collidable_b, &mut contact_set);

                if !contact_set.is_empty() {
                    let descriptor_b = self.collidable_descriptor(collidable_b.id);

                    f(Collision {
                        collider_a: descriptor_a,
                        collider_b: descriptor_b,
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

    fn collidables_mut(&mut self, kind: CollidableKind) -> &mut Vec<Collidable> {
        &mut self.collidables[kind as usize]
    }

    fn collidable_descriptor(&self, collidable_id: CollidableID) -> &CollidableDescriptor {
        self.collidable_descriptors
            .get(&collidable_id)
            .expect("Missing descriptor for collidable")
    }

    fn collidable_with_descriptor(&self, descriptor: &CollidableDescriptor) -> &Collidable {
        &self.collidables(descriptor.kind)[descriptor.idx]
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
        todo!()
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
