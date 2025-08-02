//! Change tracking registries for storing resources.

use crate::{MutableResource, Resource, ResourceDirtyMask};
use impact_containers::RandomState;
use std::{
    collections::{HashMap, hash_map::Entry},
    fmt,
    hash::BuildHasher,
    ops::{Deref, DerefMut},
    vec::Drain,
};

/// A change tracking registry for storing immutable resources.
pub type ImmutableResourceRegistry<R, S = RandomState> = ResourceRegistry<R, (), S>;

/// A change tracking registry for storing mutable resources.
pub type MutableResourceRegistry<R, S = RandomState> =
    ResourceRegistry<R, <R as MutableResource>::DirtyMask, S>;

/// A change tracking registry for storing resources.
pub struct ResourceRegistry<R: Resource, D, S = RandomState> {
    resources: HashMap<R::ID, R, S>,
    changelog: Vec<ResourceChange<R, D>>,
    /// How many times the registry has changed since it was created.
    revision: u64,
}

/// A change that occurred to a resource in a [`ResourceRegistry`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceChange<R: Resource, D> {
    id: R::ID,
    kind: ResourceChangeKind<D>,
    /// The revision number of the registry when the change was made.
    revision: u64,
}

/// The type of change that occurred to a resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceChangeKind<D> {
    /// The resource was added to the registry.
    Inserted,
    /// The resource was removed from the registry.
    Removed,
    /// The resource was completely replaced.
    Replaced,
    /// The resource was modified with the given dirty mask.
    Modified(D),
}

/// A mutable reference to a resource that tracks modifications.
///
/// When this reference is dropped, it will automatically record a change in the
/// registry's changelog if [`Self::set_dirty_mask`] was called with a non-empty
/// mask.
#[derive(Debug)]
pub struct ResourceMutRef<'a, R: MutableResource> {
    id: R::ID,
    resource: &'a mut R,
    dirty_mask: R::DirtyMask,
    changelog: &'a mut Vec<ResourceChange<R, R::DirtyMask>>,
    revision: &'a mut u64,
}

impl<R, D, S> ResourceRegistry<R, D, S>
where
    R: Resource,
    S: Default,
{
    /// Creates a new empty resource registry.
    pub fn new() -> Self {
        Self {
            resources: HashMap::default(),
            changelog: Vec::new(),
            revision: 0,
        }
    }
}

impl<R, D, S> ResourceRegistry<R, D, S>
where
    R: Resource,
    S: BuildHasher,
{
    /// Returns the number of resources in the registry.
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Whether the registry contains no resources.
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Returns a reference to the resource with the given ID.
    pub fn get(&self, id: R::ID) -> Option<&R> {
        self.resources.get(&id)
    }

    /// Whether the registry contains a resource with the given ID.
    pub fn contains(&self, id: R::ID) -> bool {
        self.resources.contains_key(&id)
    }

    /// Returns an iterator over all resource IDs and their resources.
    pub fn iter(&self) -> impl Iterator<Item = (R::ID, &R)> {
        self.resources.iter().map(|(id, resource)| (*id, resource))
    }

    /// Inserts the given resource into the registry under the given ID. If a
    /// resource with the same ID already existed, it is replaced, and the
    /// existing resource is returned. Otherwise, [`None`] is returned.
    pub fn insert(&mut self, id: R::ID, resource: R) -> Option<R> {
        let existing_resource = self.resources.insert(id, resource);

        self.changelog.push(ResourceChange {
            id,
            kind: if existing_resource.is_some() {
                ResourceChangeKind::Replaced
            } else {
                ResourceChangeKind::Inserted
            },
            revision: self.revision,
        });
        self.revision += 1;

        existing_resource
    }

    /// If on resource exists under the given ID, creates a resource using the
    /// given closure and inserts it under that ID.
    pub fn insert_with_if_absent(&mut self, id: R::ID, get_resource: impl FnOnce() -> R) {
        if let Entry::Vacant(entry) = self.resources.entry(id) {
            entry.insert(get_resource());

            self.changelog.push(ResourceChange {
                id,
                kind: ResourceChangeKind::Inserted,
                revision: self.revision,
            });
            self.revision += 1;
        }
    }

    /// Removes the resource with the given ID from the registry, returning the
    /// removed resource if it existed. If the resource was not present,
    /// [`None`] is returned.
    pub fn remove(&mut self, id: R::ID) -> Option<R> {
        let removed_resource = self.resources.remove(&id);

        if removed_resource.is_some() {
            self.changelog.push(ResourceChange {
                id,
                kind: ResourceChangeKind::Removed,
                revision: self.revision,
            });
            self.revision += 1;
        }

        removed_resource
    }

    /// Returns the current revision number.
    ///
    /// The revision increments each time the registry changes.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns all changes that occurred after the given revision.
    ///
    /// The first entry in the returned slice will be the first change after the
    /// registry had the given revision number.
    pub fn changes_since(&self, revision: u64) -> &[ResourceChange<R, D>] {
        if let Some(start_idx) = self.idx_of_first_change_since_revision(revision) {
            &self.changelog[start_idx..]
        } else {
            &[]
        }
    }

    /// Removes and returns all changes that occurred after the given revision.
    ///
    /// The first drained entry will be the first change after the registry had
    /// the given revision number.
    pub fn drain_changes_since(&mut self, revision: u64) -> Drain<'_, ResourceChange<R, D>> {
        let start_idx = self
            .idx_of_first_change_since_revision(revision)
            .unwrap_or(self.changelog.len());

        self.changelog.drain(start_idx..)
    }

    fn idx_of_first_change_since_revision(&self, revision: u64) -> Option<usize> {
        let idx = self
            .changelog
            .partition_point(|change| change.revision < revision);
        (idx < self.changelog.len()).then_some(idx)
    }
}

impl<R: Resource, D, S> fmt::Debug for ResourceRegistry<R, D, S>
where
    R: fmt::Debug,
    R::ID: fmt::Debug,
    D: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceRegistry")
            .field("resources", &self.resources)
            .field("changelog", &self.changelog)
            .field("revision", &self.revision)
            .finish()
    }
}

impl<R, D, S> Default for ResourceRegistry<R, D, S>
where
    R: Resource,
    S: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R, S> MutableResourceRegistry<R, S>
where
    R: MutableResource,
    S: BuildHasher,
{
    /// Gets a mutable reference to the resource with the given id.
    ///
    /// When the returned [`ResourceMutRef`] drops, it will add an entry to the
    /// changelog and increment the revision number if
    /// [`ResourceMutRef::set_dirty_mask`] has been called with a non-empty
    /// mask.
    pub fn get_mut(&mut self, id: R::ID) -> Option<ResourceMutRef<'_, R>> {
        self.resources.get_mut(&id).map(|resource| ResourceMutRef {
            id,
            resource,
            dirty_mask: R::DirtyMask::empty(),
            changelog: &mut self.changelog,
            revision: &mut self.revision,
        })
    }
}

impl<R: Resource, D> ResourceChange<R, D> {
    /// Returns the id of the changed resource.
    pub fn id(&self) -> R::ID {
        self.id
    }

    /// Returns the type of change that occurred.
    pub fn kind(&self) -> &ResourceChangeKind<D> {
        &self.kind
    }

    /// Returns the revision when this change occurred.
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

impl<'a, R: MutableResource> ResourceMutRef<'a, R> {
    /// Sets the dirty mask for this resource.
    ///
    /// If the mask is non-empty when this reference is dropped, a change
    /// will be recorded in the registry's changelog.
    pub fn set_dirty_mask(&mut self, dirty_mask: R::DirtyMask) {
        self.dirty_mask = dirty_mask;
    }
}

impl<'a, R: MutableResource> Deref for ResourceMutRef<'a, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.resource
    }
}

impl<'a, R: MutableResource> DerefMut for ResourceMutRef<'a, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.resource
    }
}

impl<'a, R: MutableResource> Drop for ResourceMutRef<'a, R> {
    fn drop(&mut self) {
        if self.dirty_mask == R::DirtyMask::empty() {
            return;
        }
        self.changelog.push(ResourceChange {
            id: self.id,
            kind: ResourceChangeKind::Modified(self.dirty_mask),
            revision: *self.revision,
        });
        *self.revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryDirtyMask, ResourceID};

    // Test resource types
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct TestResourceID(u32);

    impl ResourceID for TestResourceID {}

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestResource {
        data: String,
    }

    impl Resource for TestResource {
        type ID = TestResourceID;
    }

    #[derive(Debug, PartialEq, Eq)]
    struct TestMutableResource {
        value: i32,
        name: String,
    }

    impl Resource for TestMutableResource {
        type ID = TestResourceID;
    }

    impl MutableResource for TestMutableResource {
        type DirtyMask = BinaryDirtyMask;
    }

    // Test constants
    const ID_1: TestResourceID = TestResourceID(1);
    const ID_2: TestResourceID = TestResourceID(2);
    const ID_3: TestResourceID = TestResourceID(3);

    #[test]
    fn creating_new_registry_gives_empty_registry() {
        let registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert_eq!(registry.revision(), 0);
    }

    #[test]
    fn inserting_new_resource_adds_to_registry_and_tracks_change() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource = TestResource {
            data: "test".to_string(),
        };

        let result = registry.insert(ID_1, resource);

        assert_eq!(result, None);
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.contains(ID_1));
        assert_eq!(registry.revision(), 1);

        let changes = registry.changes_since(0);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].id(), ID_1);
        assert_eq!(changes[0].kind(), &ResourceChangeKind::Inserted);
        assert_eq!(changes[0].revision(), 0);
    }

    #[test]
    fn inserting_resource_with_existing_id_replaces_and_returns_old() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource1 = TestResource {
            data: "first".to_string(),
        };
        let resource2 = TestResource {
            data: "second".to_string(),
        };

        registry.insert(ID_1, resource1.clone());
        let result = registry.insert(ID_1, resource2.clone());

        assert_eq!(result, Some(resource1));
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(ID_1), Some(&resource2));
        assert_eq!(registry.revision(), 2);

        let changes = registry.changes_since(0);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[1].kind(), &ResourceChangeKind::Replaced);
    }

    #[test]
    fn getting_existing_resource_works() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource = TestResource {
            data: "test".to_string(),
        };

        registry.insert(ID_1, resource.clone());
        let retrieved = registry.get(ID_1);

        assert_eq!(retrieved, Some(&resource));
    }

    #[test]
    fn getting_nonexistent_resource_returns_none() {
        let registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();

        let result = registry.get(ID_1);

        assert_eq!(result, None);
    }

    #[test]
    fn removing_existing_resource_returns_resource_and_tracks_change() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource = TestResource {
            data: "test".to_string(),
        };

        registry.insert(ID_1, resource.clone());
        let result = registry.remove(ID_1);

        assert_eq!(result, Some(resource));
        assert_eq!(registry.len(), 0);
        assert!(!registry.contains(ID_1));
        assert_eq!(registry.revision(), 2);

        let changes = registry.changes_since(0);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[1].kind(), &ResourceChangeKind::Removed);
    }

    #[test]
    fn removing_nonexistent_resource_returns_none_and_no_change() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();

        let result = registry.remove(ID_1);

        assert_eq!(result, None);
        assert_eq!(registry.revision(), 0);
        assert_eq!(registry.changes_since(0).len(), 0);
    }

    #[test]
    fn iterating_over_registry_yields_all_resources() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource1 = TestResource {
            data: "first".to_string(),
        };
        let resource2 = TestResource {
            data: "second".to_string(),
        };

        registry.insert(ID_1, resource1.clone());
        registry.insert(ID_2, resource2.clone());

        let items: Vec<_> = registry.iter().collect();
        assert_eq!(items.len(), 2);
        assert!(items.contains(&(ID_1, &resource1)));
        assert!(items.contains(&(ID_2, &resource2)));
    }

    #[test]
    fn changes_since_returns_changes_after_given_revision() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource1 = TestResource {
            data: "first".to_string(),
        };
        let resource2 = TestResource {
            data: "second".to_string(),
        };

        registry.insert(ID_1, resource1); // revision 0 -> 1
        let checkpoint_revision = registry.revision();
        registry.insert(ID_2, resource2); // revision 1 -> 2
        registry.remove(ID_1); // revision 2 -> 3

        let changes = registry.changes_since(checkpoint_revision);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].kind(), &ResourceChangeKind::Inserted);
        assert_eq!(changes[0].id(), ID_2);
        assert_eq!(changes[1].kind(), &ResourceChangeKind::Removed);
        assert_eq!(changes[1].id(), ID_1);
    }

    #[test]
    fn changes_since_future_revision_returns_empty_slice() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource = TestResource {
            data: "test".to_string(),
        };

        registry.insert(ID_1, resource);
        let changes = registry.changes_since(999);

        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn draining_changes_since_removes_and_returns_changes() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource1 = TestResource {
            data: "first".to_string(),
        };
        let resource2 = TestResource {
            data: "second".to_string(),
        };

        registry.insert(ID_1, resource1); // revision 0 -> 1
        let checkpoint_revision = registry.revision();
        registry.insert(ID_2, resource2); // revision 1 -> 2
        registry.remove(ID_1); // revision 2 -> 3

        assert_eq!(registry.drain_changes_since(checkpoint_revision).count(), 2);

        // Verify changes were removed from registry
        let remaining_changes = registry.changes_since(0);
        assert_eq!(remaining_changes.len(), 1);
        assert_eq!(remaining_changes[0].id(), ID_1);
    }

    #[test]
    fn getting_mutable_reference_for_existing_resource_works() {
        let mut registry: MutableResourceRegistry<TestMutableResource> = ResourceRegistry::new();
        let resource = TestMutableResource {
            value: 42,
            name: "test".to_string(),
        };

        registry.insert(ID_1, resource);
        let mut_ref = registry.get_mut(ID_1);

        assert!(mut_ref.is_some());
        let resource_ref = mut_ref.unwrap();
        assert_eq!(resource_ref.value, 42);
        assert_eq!(resource_ref.name, "test");
    }

    #[test]
    fn getting_mutable_reference_for_nonexistent_resource_returns_none() {
        let mut registry: MutableResourceRegistry<TestMutableResource> = ResourceRegistry::new();

        let result = registry.get_mut(ID_1);

        assert!(result.is_none());
    }

    #[test]
    fn modifying_resource_without_dirty_mask_creates_no_change() {
        let mut registry: MutableResourceRegistry<TestMutableResource> = ResourceRegistry::new();
        let resource = TestMutableResource {
            value: 42,
            name: "test".to_string(),
        };

        registry.insert(ID_1, resource);
        let initial_revision = registry.revision();

        {
            let mut resource_ref = registry.get_mut(ID_1).unwrap();
            resource_ref.value = 100; // Modify but don't set dirty mask
        } // ResourceMutRef drops here

        assert_eq!(registry.revision(), initial_revision);
        assert_eq!(registry.changes_since(initial_revision).len(), 0);
    }

    #[test]
    fn modifying_resource_with_dirty_mask_creates_change() {
        let mut registry: MutableResourceRegistry<TestMutableResource> = ResourceRegistry::new();
        let resource = TestMutableResource {
            value: 42,
            name: "test".to_string(),
        };

        registry.insert(ID_1, resource);
        let initial_revision = registry.revision();

        {
            let mut resource_ref = registry.get_mut(ID_1).unwrap();
            resource_ref.value = 100;
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
        } // ResourceMutRef drops here and records change

        assert_eq!(registry.revision(), initial_revision + 1);
        let changes = registry.changes_since(initial_revision);
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].kind(),
            &ResourceChangeKind::Modified(BinaryDirtyMask::ALL)
        );
        assert_eq!(changes[0].id(), ID_1);
    }

    #[test]
    fn setting_empty_dirty_mask_creates_no_change() {
        let mut registry: MutableResourceRegistry<TestMutableResource> = ResourceRegistry::new();
        let resource = TestMutableResource {
            value: 42,
            name: "test".to_string(),
        };

        registry.insert(ID_1, resource);
        let initial_revision = registry.revision();

        {
            let mut resource_ref = registry.get_mut(ID_1).unwrap();
            resource_ref.value = 100;
            resource_ref.set_dirty_mask(BinaryDirtyMask::empty());
        }

        assert_eq!(registry.revision(), initial_revision);
        assert_eq!(registry.changes_since(initial_revision).len(), 0);
    }

    #[test]
    fn multiple_overlapping_changes_are_tracked_correctly() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource1 = TestResource {
            data: "first".to_string(),
        };
        let resource2 = TestResource {
            data: "second".to_string(),
        };
        let resource3 = TestResource {
            data: "third".to_string(),
        };

        registry.insert(ID_1, resource1); // rev 0->1
        registry.insert(ID_2, resource2); // rev 1->2
        registry.insert(ID_3, resource3); // rev 2->3
        registry.remove(ID_2); // rev 3->4

        let all_changes = registry.changes_since(0);
        assert_eq!(all_changes.len(), 4);

        let changes_since_2 = registry.changes_since(2);
        assert_eq!(changes_since_2.len(), 2);
        assert_eq!(changes_since_2[0].id(), ID_3);
        assert_eq!(changes_since_2[0].kind(), &ResourceChangeKind::Inserted);
        assert_eq!(changes_since_2[1].id(), ID_2);
        assert_eq!(changes_since_2[1].kind(), &ResourceChangeKind::Removed);
    }

    #[test]
    fn inserting_with_if_absent_with_new_id_adds_resource_and_tracks_change() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let resource = TestResource {
            data: "test".to_string(),
        };

        registry.insert_with_if_absent(ID_1, || resource.clone());

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(ID_1));
        assert_eq!(registry.get(ID_1), Some(&resource));
        assert_eq!(registry.revision(), 1);

        let changes = registry.changes_since(0);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].id(), ID_1);
        assert_eq!(changes[0].kind(), &ResourceChangeKind::Inserted);
        assert_eq!(changes[0].revision(), 0);
    }

    #[test]
    fn inserting_with_if_absent_with_existing_id_does_nothing() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let original_resource = TestResource {
            data: "original".to_string(),
        };

        registry.insert(ID_1, original_resource.clone());
        let initial_revision = registry.revision();

        registry.insert_with_if_absent(ID_1, || TestResource {
            data: "new".to_string(),
        });

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(ID_1), Some(&original_resource));
        assert_eq!(registry.revision(), initial_revision);

        let changes = registry.changes_since(initial_revision);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn inserting_with_if_absent_only_calls_closure_when_inserting() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let original_resource = TestResource {
            data: "original".to_string(),
        };

        registry.insert(ID_1, original_resource.clone());

        let mut closure_called = false;
        registry.insert_with_if_absent(ID_1, || {
            closure_called = true;
            TestResource {
                data: "should not be created".to_string(),
            }
        });

        assert!(!closure_called);
        assert_eq!(registry.get(ID_1), Some(&original_resource));

        // Test that closure is called for new ID
        let mut closure_called_for_new = false;
        registry.insert_with_if_absent(ID_2, || {
            closure_called_for_new = true;
            TestResource {
                data: "new resource".to_string(),
            }
        });

        assert!(closure_called_for_new);
        assert!(registry.contains(ID_2));
    }
}
