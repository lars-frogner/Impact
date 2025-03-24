//! Collision detection and resolution.

pub mod components;
pub mod entity;
pub mod systems;

use crate::{
    geometry::{Plane, Sphere},
    physics::{
        constraint::contact::{Contact, ContactGeometry, ContactID, ContactManifold},
        fph,
    },
    voxel::{VoxelObjectID, VoxelObjectManager},
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
    pub contact_manifold: &'a ContactManifold,
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
        voxel_object_manager: &VoxelObjectManager,
        collidable_id: CollidableID,
        f: &mut impl FnMut(Collision<'_>),
    ) {
        let descriptor_a = self.collidable_descriptor(collidable_id);
        let collidable_a = self.collidable_with_descriptor(descriptor_a);

        let mut contact_manifold = ContactManifold::new();

        for collidables_of_kind in &self.collidables {
            for collidable_b in collidables_of_kind {
                if collidable_b.id == collidable_a.id {
                    continue;
                }

                let order = generate_contact_manifold(
                    voxel_object_manager,
                    collidable_a,
                    collidable_b,
                    &mut contact_manifold,
                );

                if !contact_manifold.is_empty() {
                    let descriptor_b = self.collidable_descriptor(collidable_b.id);

                    let (collider_a, collider_b) =
                        order.swap_if_required(descriptor_a, descriptor_b);

                    f(Collision {
                        collider_a,
                        collider_b,
                        contact_manifold: &contact_manifold,
                    });

                    contact_manifold.clear();
                }
            }
        }
    }

    pub fn for_each_non_phantom_collision_involving_dynamic_collidable(
        &self,
        voxel_object_manager: &VoxelObjectManager,
        f: &mut impl FnMut(Collision<'_>),
    ) {
        let dynamic_collidables = self.collidables(CollidableKind::Dynamic);
        let static_collidables = self.collidables(CollidableKind::Static);

        let mut contact_manifold = ContactManifold::new();

        for (idx, collidable_a) in dynamic_collidables.iter().enumerate() {
            let descriptor_a = self.collidable_descriptor(collidable_a.id);

            for collidable_b in dynamic_collidables[idx + 1..]
                .iter()
                .chain(static_collidables)
            {
                let order = generate_contact_manifold(
                    voxel_object_manager,
                    collidable_a,
                    collidable_b,
                    &mut contact_manifold,
                );

                if !contact_manifold.is_empty() {
                    let descriptor_b = self.collidable_descriptor(collidable_b.id);

                    let (collider_a, collider_b) =
                        order.swap_if_required(descriptor_a, descriptor_b);

                    f(Collision {
                        collider_a,
                        collider_b,
                        contact_manifold: &contact_manifold,
                    });

                    contact_manifold.clear();
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
}

impl SphereCollidableGeometry {
    pub fn new(sphere: Sphere<fph>) -> Self {
        Self { sphere }
    }

    fn to_world(&self, transform_to_world_space: &Similarity3<fph>) -> Self {
        Self {
            sphere: self.sphere.transformed(transform_to_world_space),
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
}

impl CollidableOrder {
    fn swap_if_required<T>(self, a: T, b: T) -> (T, T) {
        match self {
            Self::Original => (a, b),
            Self::Swapped => (b, a),
        }
    }
}

fn generate_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    collidable_a: &Collidable,
    collidable_b: &Collidable,
    contact_manifold: &mut ContactManifold,
) -> CollidableOrder {
    use WorldCollidableGeometry::{Plane, Sphere, VoxelObject};

    match (&collidable_a.geometry, &collidable_b.geometry) {
        (VoxelObject(voxel_object_a), VoxelObject(voxel_object_b)) => {
            generate_voxel_object_voxel_object_contact_manifold(
                voxel_object_manager,
                voxel_object_a,
                voxel_object_b,
                collidable_a.id,
                collidable_b.id,
                contact_manifold,
            );
            CollidableOrder::Original
        }
        (VoxelObject(voxel_object), Sphere(sphere)) => {
            generate_voxel_object_sphere_contact_manifold(
                voxel_object_manager,
                voxel_object,
                sphere,
                collidable_a.id,
                collidable_b.id,
                contact_manifold,
            );
            CollidableOrder::Original
        }
        (VoxelObject(voxel_object), Plane(plane)) => {
            generate_voxel_object_plane_contact_manifold(
                voxel_object_manager,
                voxel_object,
                plane,
                collidable_a.id,
                collidable_b.id,
                contact_manifold,
            );
            CollidableOrder::Original
        }
        (Sphere(sphere_a), Sphere(sphere_b)) => {
            generate_sphere_sphere_contact_manifold(
                sphere_a,
                sphere_b,
                collidable_a.id,
                collidable_b.id,
                contact_manifold,
            );
            CollidableOrder::Original
        }
        (Sphere(sphere), VoxelObject(voxel_object)) => {
            generate_voxel_object_sphere_contact_manifold(
                voxel_object_manager,
                voxel_object,
                sphere,
                collidable_b.id,
                collidable_a.id,
                contact_manifold,
            );
            CollidableOrder::Swapped
        }
        (Sphere(sphere), Plane(plane)) => {
            generate_sphere_plane_contact_manifold(
                sphere,
                plane,
                collidable_a.id,
                collidable_b.id,
                contact_manifold,
            );
            CollidableOrder::Original
        }
        (Plane(_), Plane(_)) => {
            // Not useful
            CollidableOrder::Original
        }
        (Plane(plane), Sphere(sphere)) => {
            generate_sphere_plane_contact_manifold(
                sphere,
                plane,
                collidable_b.id,
                collidable_a.id,
                contact_manifold,
            );
            CollidableOrder::Swapped
        }
        (Plane(plane), VoxelObject(voxel_object)) => {
            generate_voxel_object_plane_contact_manifold(
                voxel_object_manager,
                voxel_object,
                plane,
                collidable_b.id,
                collidable_a.id,
                contact_manifold,
            );
            CollidableOrder::Swapped
        }
    }
}

fn generate_voxel_object_voxel_object_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    voxel_object_a: &VoxelObjectCollidableGeometry,
    voxel_object_b: &VoxelObjectCollidableGeometry,
    voxel_object_a_collidable_id: CollidableID,
    voxel_object_b_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    todo!()
}

fn generate_voxel_object_sphere_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    voxel_object: &VoxelObjectCollidableGeometry,
    sphere: &SphereCollidableGeometry,
    voxel_object_collidable_id: CollidableID,
    sphere_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    todo!()
}

fn generate_voxel_object_plane_contact_manifold(
    voxel_object_manager: &VoxelObjectManager,
    voxel_object: &VoxelObjectCollidableGeometry,
    plane: &PlaneCollidableGeometry,
    voxel_object_collidable_id: CollidableID,
    plane_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    todo!()
}

fn generate_sphere_sphere_contact_manifold(
    sphere_a: &SphereCollidableGeometry,
    sphere_b: &SphereCollidableGeometry,
    sphere_a_collidable_id: CollidableID,
    sphere_b_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) = determine_sphere_sphere_contact(&sphere_a.sphere, &sphere_b.sphere) {
        let id = contact_id_from_collidable_ids(sphere_a_collidable_id, sphere_b_collidable_id);
        contact_manifold.add_contact(Contact { id, geometry });
    }
}

fn generate_sphere_plane_contact_manifold(
    sphere: &SphereCollidableGeometry,
    plane: &PlaneCollidableGeometry,
    sphere_collidable_id: CollidableID,
    plane_collidable_id: CollidableID,
    contact_manifold: &mut ContactManifold,
) {
    if let Some(geometry) = determine_sphere_plane_contact(&sphere.sphere, &plane.plane) {
        let id = contact_id_from_collidable_ids(sphere_collidable_id, plane_collidable_id);
        contact_manifold.add_contact(Contact { id, geometry });
    }
}

fn contact_id_from_collidable_ids(a: CollidableID, b: CollidableID) -> ContactID {
    ContactID::from_two_u32(a.0, b.0)
}

pub fn determine_sphere_sphere_contact(
    sphere_a: &Sphere<fph>,
    sphere_b: &Sphere<fph>,
) -> Option<ContactGeometry> {
    let center_displacement = sphere_a.center() - sphere_b.center();
    let squared_center_distance = center_displacement.norm_squared();

    if squared_center_distance
        > sphere_a.radius_squared()
            + sphere_b.radius_squared()
            + 2.0 * sphere_a.radius() * sphere_b.radius()
    {
        return None;
    }

    let center_distance = squared_center_distance.sqrt();

    let surface_normal = if center_distance > 1e-8 {
        UnitVector3::new_unchecked(center_displacement.unscale(center_distance))
    } else {
        Vector3::z_axis()
    };

    let position = sphere_b.center() + surface_normal.scale(sphere_b.radius());

    let penetration_depth = fph::max(
        0.0,
        (sphere_a.radius() + sphere_b.radius()) - center_distance,
    );

    Some(ContactGeometry {
        position,
        surface_normal,
        penetration_depth,
    })
}

pub fn determine_sphere_plane_contact(
    sphere: &Sphere<fph>,
    plane: &Plane<fph>,
) -> Option<ContactGeometry> {
    let signed_distance = plane.compute_signed_distance(sphere.center());
    let penetration_depth = sphere.radius() - signed_distance;

    if penetration_depth < 0.0 {
        return None;
    }

    let surface_normal = *plane.unit_normal();
    let position = sphere.center() - surface_normal.scale(signed_distance);

    Some(ContactGeometry {
        position,
        surface_normal,
        penetration_depth,
    })
}
