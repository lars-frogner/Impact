//! Collision detection and resolution.

pub mod components;
pub mod entity;
pub mod geometry;
pub mod systems;

use crate::physics::{constraint::contact::ContactManifold, fph};
use bytemuck::{Pod, Zeroable};
use impact_containers::HashMap;
use impact_ecs::world::EntityID;
use nalgebra::Similarity3;
use roc_integration::roc;
use std::fmt;

pub trait CollidableGeometry: Sized {
    type Local: fmt::Debug;
    type Context;

    fn from_local(local_geometry: &Self::Local, transform_to_world_space: Similarity3<fph>)
    -> Self;

    fn generate_contact_manifold(
        context: &Self::Context,
        collidable_a: &Collidable<Self>,
        collidable_b: &Collidable<Self>,
        contact_manifold: &mut ContactManifold,
    ) -> CollidableOrder;
}

#[derive(Debug)]
pub struct CollisionWorld<G: CollidableGeometry> {
    collidable_descriptors: HashMap<CollidableID, CollidableDescriptor<G>>,
    collidables: [Vec<Collidable<G>>; 3],
    collidable_id_counter: u32,
}

#[derive(Clone, Debug)]
pub struct CollidableDescriptor<G: CollidableGeometry> {
    kind: CollidableKind,
    geometry: G::Local,
    entity_id: EntityID,
    idx: usize,
}

#[roc(parents = "Physics")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollidableKind {
    Dynamic = 0,
    Static = 1,
    Phantom = 2,
}

#[derive(Clone, Debug)]
pub struct Collidable<G> {
    id: CollidableID,
    geometry: G,
}

/// Identifier for a collidable in a [`CollisionWorld`].
#[roc(parents = "Physics")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct CollidableID(u32);

#[derive(Clone, Debug)]
pub struct Collision<'a, G: CollidableGeometry> {
    pub collider_a: &'a CollidableDescriptor<G>,
    pub collider_b: &'a CollidableDescriptor<G>,
    pub contact_manifold: &'a ContactManifold,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CollidableOrder {
    Original,
    Swapped,
}

impl<G: CollidableGeometry> CollisionWorld<G> {
    pub fn new() -> Self {
        Self {
            collidable_descriptors: HashMap::default(),
            collidables: [Vec::new(), Vec::new(), Vec::new()],
            collidable_id_counter: 0,
        }
    }

    pub fn get_collidable_descriptor(
        &self,
        collidable_id: CollidableID,
    ) -> Option<&CollidableDescriptor<G>> {
        self.collidable_descriptors.get(&collidable_id)
    }

    pub fn get_collidable_with_descriptor(
        &self,
        descriptor: &CollidableDescriptor<G>,
    ) -> Option<&Collidable<G>> {
        self.collidables(descriptor.kind).get(descriptor.idx)
    }

    pub fn add_collidable(&mut self, kind: CollidableKind, geometry: G::Local) -> CollidableID {
        let descriptor = CollidableDescriptor::new(kind, geometry);
        let collidable_id = self.create_new_collidable_id();
        self.collidable_descriptors
            .insert(collidable_id, descriptor);
        collidable_id
    }

    pub fn synchronize_collidable(
        &mut self,
        collidable_id: CollidableID,
        entity_id: EntityID,
        transform_to_world_space: Similarity3<fph>,
    ) {
        let descriptor = self
            .collidable_descriptors
            .get_mut(&collidable_id)
            .expect("Missing descriptor for collidable");

        descriptor.entity_id = entity_id;

        let collidable = Collidable::new(
            collidable_id,
            G::from_local(&descriptor.geometry, transform_to_world_space),
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
        context: &G::Context,
        collidable_id: CollidableID,
        f: &mut impl FnMut(Collision<'_, G>),
    ) {
        let descriptor_a = self.collidable_descriptor(collidable_id);
        let collidable_a = self.collidable_with_descriptor(descriptor_a);

        let mut contact_manifold = ContactManifold::new();

        for collidables_of_kind in &self.collidables {
            for collidable_b in collidables_of_kind {
                if collidable_b.id == collidable_a.id {
                    continue;
                }

                let order = G::generate_contact_manifold(
                    context,
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
        context: &G::Context,
        f: &mut impl FnMut(Collision<'_, G>),
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
                let order = G::generate_contact_manifold(
                    context,
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

    fn collidables(&self, kind: CollidableKind) -> &[Collidable<G>] {
        &self.collidables[kind as usize]
    }

    fn collidable_descriptor(&self, collidable_id: CollidableID) -> &CollidableDescriptor<G> {
        self.get_collidable_descriptor(collidable_id)
            .expect("Missing descriptor for collidable")
    }

    fn collidable_with_descriptor(&self, descriptor: &CollidableDescriptor<G>) -> &Collidable<G> {
        self.get_collidable_with_descriptor(descriptor)
            .expect("Missing collidable for collidable descriptor")
    }

    fn create_new_collidable_id(&mut self) -> CollidableID {
        let collidable_id = CollidableID(self.collidable_id_counter);
        self.collidable_id_counter = self.collidable_id_counter.checked_add(1).unwrap();
        collidable_id
    }
}

impl<G: CollidableGeometry> Default for CollisionWorld<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: CollidableGeometry> CollidableDescriptor<G> {
    fn new(kind: CollidableKind, geometry: G::Local) -> Self {
        Self {
            kind,
            geometry,
            entity_id: EntityID::zeroed(),
            idx: usize::MAX,
        }
    }

    pub fn entity_id(&self) -> EntityID {
        self.entity_id
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

impl<G> Collidable<G> {
    fn new(id: CollidableID, geometry: G) -> Self {
        Self { id, geometry }
    }

    pub fn geometry(&self) -> &G {
        &self.geometry
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
