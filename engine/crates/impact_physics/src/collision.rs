//! Collision detection and resolution.

pub mod collidable;
pub mod setup;

use crate::{
    constraint::contact::ContactManifold,
    rigid_body::{RigidBodyManager, TypedRigidBodyID},
};
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_containers::{NoHashMap, hash_map::Entry};
use impact_id::define_entity_id_newtype;
use impact_intersection::IntersectionManager;
use impact_math::transform::Isometry3;
use roc_integration::roc;
use std::fmt;

pub trait Collidable: Sized + fmt::Debug {
    type Local: fmt::Debug;
    type Context;

    fn from_descriptor(
        descriptor: &CollidableDescriptor<Self>,
        transform_to_world_space: &Isometry3,
    ) -> Self;

    fn generate_contact_manifold(
        context: &Self::Context,
        collidable_a: &CollidableWithId<Self>,
        collidable_b: &CollidableWithId<Self>,
        contact_manifold: &mut ContactManifold,
    ) -> CollidableOrder;
}

#[derive(Debug)]
pub struct CollisionWorld<C: Collidable> {
    collidable_descriptors: NoHashMap<CollidableID, CollidableDescriptor<C>>,
    collidables: [Vec<CollidableWithId<C>>; 3],
}

#[derive(Clone, Debug)]
pub struct CollidableDescriptor<C: Collidable> {
    kind: CollidableKind,
    local_collidable: C::Local,
    rigid_body_id: TypedRigidBodyID,
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
pub struct CollidableWithId<C> {
    id: CollidableID,
    collidable: C,
}

define_entity_id_newtype! {
    /// Identifier for a collidable in a [`CollisionWorld`].
    [pub] CollidableID
}

define_component_type! {
    /// Marks that an entity has a collidable identified by a [`CollidableID`].
    ///
    /// Use [`CollidableID::from_entity_id`] to obtain the collidable ID from
    /// the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HasCollidable;
}

#[derive(Clone, Debug)]
pub struct Collision<'a, C: Collidable> {
    pub collider_a: &'a CollidableDescriptor<C>,
    pub collider_b: &'a CollidableDescriptor<C>,
    pub contact_manifold: &'a ContactManifold,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CollidableOrder {
    Original,
    Swapped,
}

impl<C: Collidable> CollisionWorld<C> {
    pub fn new() -> Self {
        Self {
            collidable_descriptors: NoHashMap::default(),
            collidables: [Vec::new(), Vec::new(), Vec::new()],
        }
    }

    pub fn get_collidable_descriptor(
        &self,
        collidable_id: CollidableID,
    ) -> Option<&CollidableDescriptor<C>> {
        self.collidable_descriptors.get(&collidable_id)
    }

    pub fn get_collidable_with_descriptor(
        &self,
        descriptor: &CollidableDescriptor<C>,
    ) -> Option<&CollidableWithId<C>> {
        self.collidables(descriptor.kind).get(descriptor.idx)
    }

    pub fn get_local_collidable(&self, collidable_id: CollidableID) -> Option<&C::Local> {
        let descriptor = self.collidable_descriptors.get(&collidable_id)?;
        Some(&descriptor.local_collidable)
    }

    pub fn get_local_collidable_mut(
        &mut self,
        collidable_id: CollidableID,
    ) -> Option<&mut C::Local> {
        let descriptor = self.collidable_descriptors.get_mut(&collidable_id)?;
        Some(&mut descriptor.local_collidable)
    }

    pub fn add_collidable(
        &mut self,
        collidable_id: CollidableID,
        rigid_body_id: TypedRigidBodyID,
        kind: CollidableKind,
        local_collidable: C::Local,
    ) -> Result<()> {
        let descriptor = CollidableDescriptor::new(rigid_body_id, kind, local_collidable);

        match self.collidable_descriptors.entry(collidable_id) {
            Entry::Vacant(entry) => {
                entry.insert(descriptor);
            }
            Entry::Occupied(_) => bail!("A collidable with ID {collidable_id} already exists"),
        }

        Ok(())
    }

    pub fn synchronize_collidables_with_rigid_bodies(
        &mut self,
        rigid_body_manager: &RigidBodyManager,
    ) {
        self.clear_spatial_state();

        for (&collidable_id, descriptor) in &mut self.collidable_descriptors {
            let (position, orientation) = match descriptor.rigid_body_id {
                TypedRigidBodyID::Dynamic(id) => {
                    let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(id) else {
                        continue;
                    };
                    (rigid_body.position(), rigid_body.orientation())
                }
                TypedRigidBodyID::Kinematic(id) => {
                    let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body(id) else {
                        continue;
                    };
                    (rigid_body.position(), rigid_body.orientation())
                }
            };

            let transform_to_world_space =
                Isometry3::from_parts(position.as_vector().aligned(), orientation.aligned());

            let collidable = CollidableWithId::new(
                collidable_id,
                C::from_descriptor(descriptor, &transform_to_world_space),
            );

            let collidables_of_kind = &mut self.collidables[descriptor.kind as usize];
            descriptor.idx = collidables_of_kind.len();
            collidables_of_kind.push(collidable);
        }
    }

    pub fn remove_collidable(&mut self, collidable_id: CollidableID) {
        self.collidable_descriptors.remove(&collidable_id);
    }

    pub fn for_each_non_phantom_collision_involving_dynamic_collidable(
        &self,
        context: &C::Context,
        intersection_manager: &IntersectionManager,
        f: &mut impl FnMut(Collision<'_, C>),
    ) {
        let mut contact_manifold = ContactManifold::new();

        intersection_manager.for_each_intersecting_bounding_volume_pair(|id_a, id_b| {
            let collidable_a_id = CollidableID::from_entity_id(id_a.as_entity_id());

            let Some(descriptor_a) = self.get_collidable_descriptor(collidable_a_id) else {
                return;
            };
            if descriptor_a.kind == CollidableKind::Phantom {
                return;
            }

            let collidable_b_id = CollidableID::from_entity_id(id_b.as_entity_id());

            let Some(descriptor_b) = self.get_collidable_descriptor(collidable_b_id) else {
                return;
            };
            if descriptor_b.kind == CollidableKind::Phantom {
                return;
            }

            if descriptor_a.kind != CollidableKind::Dynamic
                && descriptor_b.kind != CollidableKind::Dynamic
            {
                return;
            }

            let collidable_a = self.collidable_with_descriptor(descriptor_a);
            let collidable_b = self.collidable_with_descriptor(descriptor_b);

            let order = C::generate_contact_manifold(
                context,
                collidable_a,
                collidable_b,
                &mut contact_manifold,
            );

            if !contact_manifold.is_empty() {
                let descriptor_b = self.collidable_descriptor(collidable_b.id);

                let (collider_a, collider_b) = order.swap_if_required(descriptor_a, descriptor_b);

                f(Collision {
                    collider_a,
                    collider_b,
                    contact_manifold: &contact_manifold,
                });

                contact_manifold.clear();
            }
        });
    }

    /// Removes all stored collision state.
    pub fn clear(&mut self) {
        self.collidable_descriptors.clear();
        self.clear_spatial_state();
    }

    fn collidables(&self, kind: CollidableKind) -> &[CollidableWithId<C>] {
        &self.collidables[kind as usize]
    }

    fn collidable_descriptor(&self, collidable_id: CollidableID) -> &CollidableDescriptor<C> {
        self.get_collidable_descriptor(collidable_id)
            .expect("Missing descriptor for collidable")
    }

    fn collidable_with_descriptor(
        &self,
        descriptor: &CollidableDescriptor<C>,
    ) -> &CollidableWithId<C> {
        self.get_collidable_with_descriptor(descriptor)
            .expect("Missing collidable for collidable descriptor")
    }

    fn clear_spatial_state(&mut self) {
        for collidables_of_kind in &mut self.collidables {
            collidables_of_kind.clear();
        }
        for descriptor in self.collidable_descriptors.values_mut() {
            descriptor.idx = usize::MAX;
        }
    }
}

impl<C: Collidable> Default for CollisionWorld<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Collidable> CollidableDescriptor<C> {
    fn new(
        rigid_body_id: TypedRigidBodyID,
        kind: CollidableKind,
        local_collidable: C::Local,
    ) -> Self {
        Self {
            kind,
            local_collidable,
            rigid_body_id,
            idx: usize::MAX,
        }
    }

    pub fn local_collidable(&self) -> &C::Local {
        &self.local_collidable
    }

    pub fn rigid_body_id(&self) -> TypedRigidBodyID {
        self.rigid_body_id
    }

    pub fn kind(&self) -> CollidableKind {
        self.kind
    }
}

impl CollidableKind {
    pub fn to_u32(self) -> u32 {
        self as u32
    }

    pub fn from_u32(number: u32) -> Option<Self> {
        match number {
            0 => Some(Self::Dynamic),
            1 => Some(Self::Static),
            2 => Some(Self::Phantom),
            _ => None,
        }
    }
}

impl<C> CollidableWithId<C> {
    fn new(id: CollidableID, collidable: C) -> Self {
        Self { id, collidable }
    }

    pub fn collidable(&self) -> &C {
        &self.collidable
    }

    pub fn id(&self) -> CollidableID {
        self.id
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
