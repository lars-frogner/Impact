//! Anchor points for constraints and forces on rigid bodies.
//!
//! Anchors provide a layer of indirection between rigid bodies and constraints
//! and forces that act on fixed points on the bodies. All anchors are stored in
//! a central registry where they can be looked up for constraint and force
//! calculations. When anchor points on a rigid body have to change, this can be
//! done through the registry without knowledge of the constraints and forces
//! that rely on those anchors.

pub mod setup;

use crate::{
    quantities::Position,
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID, TypedRigidBodyID},
};
use impact_containers::HashMap;
use std::{collections::hash_map::Entry, fmt, hash::Hash};
use tinyvec::TinyVec;

/// Identifier for a [`DynamicRigidBodyAnchor`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DynamicRigidBodyAnchorID(u64);

/// Identifier for a [`KinematicRigidBodyAnchor`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct KinematicRigidBodyAnchorID(u64);

/// Identifier for a [`DynamicRigidBodyAnchor`] or [`KinematicRigidBodyAnchor`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypedRigidBodyAnchorID {
    Dynamic(DynamicRigidBodyAnchorID),
    Kinematic(KinematicRigidBodyAnchorID),
}

/// An anchor point on a dynamic rigid body.
#[derive(Copy, Clone, Debug)]
pub struct DynamicRigidBodyAnchor {
    /// The dynamic rigid body the anchor is attached to.
    pub rigid_body_id: DynamicRigidBodyID,
    /// The point where the anchor is attached, in the body-fixed frame.
    pub point: Position,
}

/// An anchor point on a kinematic rigid body.
#[derive(Copy, Clone, Debug)]
pub struct KinematicRigidBodyAnchor {
    /// The kinematic rigid body the anchor is attached to.
    pub rigid_body_id: KinematicRigidBodyID,
    /// The point where the anchor is attached, in the body-fixed frame.
    pub point: Position,
}

/// Reference to an anchor point on a dynamic or kinematic rigid body.
#[derive(Copy, Clone, Debug)]
pub enum TypedRigidBodyAnchorRef<'a> {
    Dynamic(&'a DynamicRigidBodyAnchor),
    Kinematic(&'a KinematicRigidBodyAnchor),
}

/// Manager for anchor points on dynamic and kinematic rigid bodies.
#[derive(Debug)]
pub struct AnchorManager {
    dynamic: SpecificAnchorManager<DynamicRigidBodyAnchor>,
    kinematic: SpecificAnchorManager<KinematicRigidBodyAnchor>,
}

/// Manager for anchor points on a specific type of rigid body.
#[derive(Debug)]
pub struct SpecificAnchorManager<A: Anchor> {
    anchors: HashMap<A::ID, A>,
    anchor_ids_by_body: HashMap<A::RigidBodyID, TinyVec<[A::ID; 4]>>,
    id_counter: u64,
}

/// Trait for anchor points on a specific type of rigid body.
pub trait Anchor {
    type RigidBodyID: Eq + Hash + fmt::Debug;
    type ID: Copy + Default + Eq + Hash + From<u64>;
    type Point;

    fn rigid_body_id(&self) -> Self::RigidBodyID;

    fn point(&self) -> &Self::Point;

    fn point_mut(&mut self) -> &mut Self::Point;
}

impl From<u64> for DynamicRigidBodyAnchorID {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<u64> for KinematicRigidBodyAnchorID {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl Anchor for DynamicRigidBodyAnchor {
    type ID = DynamicRigidBodyAnchorID;
    type RigidBodyID = DynamicRigidBodyID;
    type Point = Position;

    fn rigid_body_id(&self) -> Self::RigidBodyID {
        self.rigid_body_id
    }

    fn point(&self) -> &Self::Point {
        &self.point
    }

    fn point_mut(&mut self) -> &mut Self::Point {
        &mut self.point
    }
}

impl Anchor for KinematicRigidBodyAnchor {
    type ID = KinematicRigidBodyAnchorID;
    type RigidBodyID = KinematicRigidBodyID;
    type Point = Position;

    fn rigid_body_id(&self) -> Self::RigidBodyID {
        self.rigid_body_id
    }

    fn point(&self) -> &Self::Point {
        &self.point
    }

    fn point_mut(&mut self) -> &mut Self::Point {
        &mut self.point
    }
}

impl<'a> TypedRigidBodyAnchorRef<'a> {
    /// Returns the ID of the rigid body associated with this anchor.
    pub fn rigid_body_id(&self) -> TypedRigidBodyID {
        match self {
            Self::Dynamic(anchor) => TypedRigidBodyID::Dynamic(anchor.rigid_body_id),
            Self::Kinematic(anchor) => TypedRigidBodyID::Kinematic(anchor.rigid_body_id),
        }
    }

    /// Returns the position of the anchor in the body-fixed frame.
    pub fn point(&self) -> &Position {
        match self {
            Self::Dynamic(anchor) => &anchor.point,
            Self::Kinematic(anchor) => &anchor.point,
        }
    }
}

impl AnchorManager {
    /// Creates a new empty anchor manager.
    pub fn new() -> Self {
        Self {
            dynamic: SpecificAnchorManager::new(),
            kinematic: SpecificAnchorManager::new(),
        }
    }

    /// Returns a reference to the anchor with the given ID, or [`None`] if it
    /// does not exist.
    pub fn get(&self, id: &TypedRigidBodyAnchorID) -> Option<TypedRigidBodyAnchorRef<'_>> {
        match id {
            TypedRigidBodyAnchorID::Dynamic(id) => {
                self.dynamic.get(*id).map(TypedRigidBodyAnchorRef::Dynamic)
            }
            TypedRigidBodyAnchorID::Kinematic(id) => self
                .kinematic
                .get(*id)
                .map(TypedRigidBodyAnchorRef::Kinematic),
        }
    }

    /// Returns a reference to the manager for anchors on dynamic rigid bodies.
    pub fn dynamic(&self) -> &SpecificAnchorManager<DynamicRigidBodyAnchor> {
        &self.dynamic
    }

    /// Returns a mutable reference to the manager for anchors on dynamic rigid
    /// bodies.
    pub fn dynamic_mut(&mut self) -> &mut SpecificAnchorManager<DynamicRigidBodyAnchor> {
        &mut self.dynamic
    }

    /// Returns a reference to the manager for anchors on kinematic rigid
    /// bodies.
    pub fn kinematic(&self) -> &SpecificAnchorManager<KinematicRigidBodyAnchor> {
        &self.kinematic
    }

    /// Returns a mutable reference to the manager for anchors on kinematic
    /// rigid bodies.
    pub fn kinematic_mut(&mut self) -> &mut SpecificAnchorManager<KinematicRigidBodyAnchor> {
        &mut self.kinematic
    }
}

impl Default for AnchorManager {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: Anchor> SpecificAnchorManager<A> {
    fn new() -> Self {
        Self {
            anchors: HashMap::default(),
            anchor_ids_by_body: HashMap::default(),
            id_counter: 0,
        }
    }

    /// Returns a reference to an anchor with the given ID, or [`None`] if it
    /// does not exist.
    pub fn get(&self, anchor_id: A::ID) -> Option<&A> {
        self.anchors.get(&anchor_id)
    }

    /// Returns an iterator over the ID and point of each anchor on the rigid
    /// body with the given ID.
    pub fn anchors_for_body(
        &self,
        rigid_body_id: A::RigidBodyID,
    ) -> impl Iterator<Item = (A::ID, &A::Point)> {
        self.anchor_ids_by_body
            .get(&rigid_body_id)
            .into_iter()
            .flat_map(|anchor_ids| anchor_ids.iter().map(|id| (*id, self.anchors[id].point())))
    }

    /// Calls the given closure with the anchor ID and a mutable reference to
    /// the point for each anchor on the rigid body with the given ID.
    pub fn for_each_body_anchor_mut(
        &mut self,
        rigid_body_id: A::RigidBodyID,
        f: &mut impl FnMut(A::ID, &mut A::Point),
    ) {
        let Some(anchor_ids) = self.anchor_ids_by_body.get(&rigid_body_id) else {
            return;
        };
        for id in anchor_ids {
            let anchor = self.anchors.get_mut(id).unwrap();
            f(*id, anchor.point_mut());
        }
    }

    /// Inserts the given anchor into the manager and returns a new ID for it.
    pub fn insert(&mut self, anchor: A) -> A::ID {
        let anchor_id = self.create_new_id();
        self.insert_with_id(anchor_id, anchor);
        anchor_id
    }

    /// Removes the anchor with the given ID if it exists.
    pub fn remove(&mut self, anchor_id: A::ID) {
        if let Some(anchor) = self.anchors.remove(&anchor_id)
            && let Entry::Occupied(mut entry) =
                self.anchor_ids_by_body.entry(anchor.rigid_body_id())
            {
                let body_anchor_ids = entry.get_mut();

                if let Some(anchor_idx) = body_anchor_ids.iter().position(|id| *id == anchor_id) {
                    body_anchor_ids.swap_remove(anchor_idx);

                    if body_anchor_ids.is_empty() {
                        entry.remove();
                    }
                }
            }
    }

    /// Inserts the given anchor under the given ID, replacing the existing
    /// anchor if it exists.
    pub fn replace(&mut self, anchor_id: A::ID, new_anchor: A) {
        // We need to explicitly remove before inserting to make sure
        // `anchor_ids_by_body` is correctly updated
        self.remove(anchor_id);
        self.insert_with_id(anchor_id, new_anchor);
    }

    /// Removes all anchors on the rigid body with the given ID.
    pub fn remove_all_anchors_for_body(&mut self, rigid_body_id: A::RigidBodyID) {
        if let Some(anchor_ids) = self.anchor_ids_by_body.remove(&rigid_body_id) {
            for id in anchor_ids {
                self.anchors.remove(&id);
            }
        }
    }

    fn insert_with_id(&mut self, anchor_id: A::ID, anchor: A) {
        self.anchor_ids_by_body
            .entry(anchor.rigid_body_id())
            .or_default()
            .push(anchor_id);

        self.anchors.insert(anchor_id, anchor);
    }

    fn create_new_id(&mut self) -> A::ID {
        let id = A::ID::from(self.id_counter);
        self.id_counter = self.id_counter.checked_add(1).unwrap();
        id
    }
}

impl<A: Anchor> Default for SpecificAnchorManager<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestBodyID(u64);

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    struct TestAnchorID(u64);

    #[derive(Copy, Clone, Debug)]
    struct TestAnchor {
        body_id: TestBodyID,
        value: f64,
    }

    impl From<u64> for TestAnchorID {
        fn from(id: u64) -> Self {
            Self(id)
        }
    }

    impl Anchor for TestAnchor {
        type RigidBodyID = TestBodyID;
        type ID = TestAnchorID;
        type Point = f64;

        fn rigid_body_id(&self) -> Self::RigidBodyID {
            self.body_id
        }

        fn point(&self) -> &Self::Point {
            &self.value
        }

        fn point_mut(&mut self) -> &mut Self::Point {
            &mut self.value
        }
    }

    #[test]
    fn insert_anchor_returns_unique_ids() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let anchor = TestAnchor {
            body_id,
            value: 1.0,
        };

        let id1 = manager.insert(anchor);
        let id2 = manager.insert(anchor);

        assert_eq!(id1, TestAnchorID(0));
        assert_eq!(id2, TestAnchorID(1));
    }

    #[test]
    fn insert_anchor_allows_retrieval_by_id() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let anchor = TestAnchor {
            body_id,
            value: 2.5,
        };

        let id = manager.insert(anchor);
        let retrieved = manager.get(id).unwrap();

        assert_eq!(retrieved.body_id, body_id);
        assert_eq!(retrieved.value, 2.5);
    }

    #[test]
    fn anchors_for_body_returns_empty_for_nonexistent_body() {
        let manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(999);

        assert!(manager.anchors_for_body(body_id).next().is_none());
    }

    #[test]
    fn anchors_for_body_returns_all_anchors_for_body() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let other_body_id = TestBodyID(2);

        let anchor1 = TestAnchor {
            body_id,
            value: 1.0,
        };
        let anchor2 = TestAnchor {
            body_id,
            value: 2.0,
        };
        let other_anchor = TestAnchor {
            body_id: other_body_id,
            value: 3.0,
        };

        manager.insert(anchor1);
        manager.insert(anchor2);
        manager.insert(other_anchor);

        let anchors: Vec<_> = manager.anchors_for_body(body_id).collect();
        assert_eq!(anchors.len(), 2);

        let points: Vec<_> = anchors.iter().map(|(_, pt)| **pt).collect();
        assert!(points.contains(&1.0));
        assert!(points.contains(&2.0));
    }

    #[test]
    fn remove_anchors_for_body_removes_all_body_anchors() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let other_body_id = TestBodyID(2);

        let anchor1 = TestAnchor {
            body_id,
            value: 1.0,
        };
        let anchor2 = TestAnchor {
            body_id,
            value: 2.0,
        };
        let other_anchor = TestAnchor {
            body_id: other_body_id,
            value: 3.0,
        };

        let id1 = manager.insert(anchor1);
        let id2 = manager.insert(anchor2);
        let other_id = manager.insert(other_anchor);

        manager.remove_all_anchors_for_body(body_id);

        assert!(manager.get(id1).is_none());
        assert!(manager.get(id2).is_none());
        assert!(manager.get(other_id).is_some());

        assert!(manager.anchors_for_body(body_id).next().is_none());
    }

    #[test]
    fn remove_anchors_for_nonexistent_body_does_nothing() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let nonexistent_body_id = TestBodyID(999);

        let anchor = TestAnchor {
            body_id,
            value: 1.0,
        };
        let id = manager.insert(anchor);

        manager.remove_all_anchors_for_body(nonexistent_body_id);

        assert!(manager.get(id).is_some());
    }

    #[test]
    fn remove_existing_anchor_removes_anchor() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let anchor = TestAnchor {
            body_id,
            value: 1.0,
        };

        let id = manager.insert(anchor);
        assert!(manager.get(id).is_some());

        manager.remove(id);

        assert!(manager.get(id).is_none());
        assert!(manager.anchors_for_body(body_id).next().is_none());
    }

    #[test]
    fn remove_nonexistent_anchor_does_nothing() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let nonexistent_id = TestAnchorID(999);

        let anchor = TestAnchor {
            body_id,
            value: 1.0,
        };
        let existing_id = manager.insert(anchor);

        manager.remove(nonexistent_id);

        assert!(manager.get(existing_id).is_some());
    }

    #[test]
    fn remove_one_anchor_leaves_others_on_same_body() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);

        let anchor1 = TestAnchor {
            body_id,
            value: 1.0,
        };
        let anchor2 = TestAnchor {
            body_id,
            value: 2.0,
        };

        let id1 = manager.insert(anchor1);
        let id2 = manager.insert(anchor2);

        manager.remove(id1);

        assert!(manager.get(id1).is_none());
        assert!(manager.get(id2).is_some());

        let remaining_anchors: Vec<_> = manager.anchors_for_body(body_id).collect();
        assert_eq!(remaining_anchors.len(), 1);
        assert_eq!(*remaining_anchors[0].1, 2.0);
    }

    #[test]
    fn remove_last_anchor_from_body_cleans_up_body_mapping() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let other_body_id = TestBodyID(2);

        let anchor1 = TestAnchor {
            body_id,
            value: 1.0,
        };
        let anchor2 = TestAnchor {
            body_id: other_body_id,
            value: 2.0,
        };

        let id1 = manager.insert(anchor1);
        let id2 = manager.insert(anchor2);

        manager.remove(id1);

        assert!(manager.get(id1).is_none());
        assert!(manager.get(id2).is_some());
        assert!(manager.anchors_for_body(body_id).next().is_none());
        assert_eq!(manager.anchors_for_body(other_body_id).count(), 1);
    }

    #[test]
    fn replace_existing_anchor_updates_anchor_data() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let original_anchor = TestAnchor {
            body_id,
            value: 1.0,
        };

        let id = manager.insert(original_anchor);

        let new_anchor = TestAnchor {
            body_id,
            value: 2.5,
        };

        manager.replace(id, new_anchor);

        let retrieved = manager.get(id).unwrap();
        assert_eq!(retrieved.value, 2.5);
        assert_eq!(retrieved.body_id, body_id);
    }

    #[test]
    fn replace_anchor_with_different_body_updates_body_mapping() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let original_body_id = TestBodyID(1);
        let new_body_id = TestBodyID(2);

        let original_anchor = TestAnchor {
            body_id: original_body_id,
            value: 1.0,
        };

        let id = manager.insert(original_anchor);

        let new_anchor = TestAnchor {
            body_id: new_body_id,
            value: 2.0,
        };

        manager.replace(id, new_anchor);

        assert!(manager.anchors_for_body(original_body_id).next().is_none());

        let new_body_anchors: Vec<_> = manager.anchors_for_body(new_body_id).collect();
        assert_eq!(new_body_anchors.len(), 1);
        assert_eq!(*new_body_anchors[0].1, 2.0);
    }

    #[test]
    fn replace_nonexistent_anchor_creates_new_anchor() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);
        let nonexistent_id = TestAnchorID(999);

        let anchor = TestAnchor {
            body_id,
            value: 1.5,
        };

        manager.replace(nonexistent_id, anchor);

        let retrieved = manager.get(nonexistent_id).unwrap();
        assert_eq!(retrieved.value, 1.5);
        assert_eq!(retrieved.body_id, body_id);

        assert_eq!(manager.anchors_for_body(body_id).count(), 1);
    }

    #[test]
    fn replace_preserves_other_anchors_on_same_body() {
        let mut manager = SpecificAnchorManager::<TestAnchor>::new();
        let body_id = TestBodyID(1);

        let anchor1 = TestAnchor {
            body_id,
            value: 1.0,
        };
        let anchor2 = TestAnchor {
            body_id,
            value: 2.0,
        };

        let id1 = manager.insert(anchor1);
        let id2 = manager.insert(anchor2);

        let new_anchor1 = TestAnchor {
            body_id,
            value: 10.0,
        };

        manager.replace(id1, new_anchor1);

        let retrieved1 = manager.get(id1).unwrap();
        let retrieved2 = manager.get(id2).unwrap();

        assert_eq!(retrieved1.value, 10.0);
        assert_eq!(retrieved2.value, 2.0);

        assert_eq!(manager.anchors_for_body(body_id).count(), 2);
    }
}
