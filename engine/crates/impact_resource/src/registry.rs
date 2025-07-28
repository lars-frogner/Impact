//! Resource registry for managing resources with change tracking.

use crate::{Resource, ResourceDirtyMask};
use impact_containers::SlotMap;
use std::{
    ops::{Deref, DerefMut},
    vec::Drain,
};

/// A registry for storing and managing resources with change tracking.
#[derive(Debug)]
pub struct ResourceRegistry<R: Resource> {
    resources: SlotMap<R>,
    changelog: Vec<ResourceChange<R>>,
    /// How many times the registry has changed since it was created.
    revision: u64,
}

/// A change that occurred to a resource in a [`ResourceRegistry`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceChange<R: Resource> {
    handle: R::Handle,
    kind: ResourceChangeKind<R>,
    /// The revision number of the registry when the change was made.
    revision: u64,
}

/// The type of change that occurred to a resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceChangeKind<R: Resource> {
    /// The resource was added to the registry.
    Inserted,
    /// The resource was removed from the registry.
    Removed,
    /// The resource was modified with the given dirty mask.
    Modified(R::DirtyMask),
}

/// A mutable reference to a resource that tracks modifications.
///
/// When this reference is dropped, it will automatically record a change in the
/// registry's changelog if [`Self::set_dirty_mask`] was called with a non-empty
/// mask.
#[derive(Debug)]
pub struct ResourceMutRef<'a, R: Resource> {
    handle: R::Handle,
    resource: &'a mut R,
    dirty_mask: R::DirtyMask,
    changelog: &'a mut Vec<ResourceChange<R>>,
    revision: &'a mut u64,
}

impl<R: Resource> ResourceRegistry<R> {
    /// Creates a new empty resource registry.
    pub fn new() -> Self {
        Self {
            resources: SlotMap::new(),
            changelog: Vec::new(),
            revision: 0,
        }
    }

    /// Returns the number of resources in the registry.
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Whether the registry contains no resources.
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Returns a reference to the resource with the given handle.
    pub fn get(&self, handle: R::Handle) -> Option<&R> {
        self.resources.get_value(handle.into())
    }

    /// Gets a mutable reference to the resource with the given handle.
    ///
    /// When the returned [`ResourceMutRef`] drops, it will add an entry to the
    /// changelog and increment the revision number if
    /// [`ResourceMutRef::set_dirty_mask`] has been called with a non-empty
    /// mask.
    pub fn get_mut(&mut self, handle: R::Handle) -> Option<ResourceMutRef<'_, R>> {
        self.resources
            .get_value_mut(handle.into())
            .map(|resource| ResourceMutRef {
                handle,
                resource,
                dirty_mask: R::DirtyMask::empty(),
                changelog: &mut self.changelog,
                revision: &mut self.revision,
            })
    }

    /// Whether the registry contains a resource with the given handle.
    pub fn contains(&self, handle: R::Handle) -> bool {
        self.resources.contains(handle.into())
    }

    /// Returns an iterator over all resource handles and their resources.
    pub fn iter(&self) -> impl Iterator<Item = (R::Handle, &R)> {
        self.resources
            .iter()
            .map(|(key, resource)| (key.into(), resource))
    }

    /// Inserts the given resource into the registry.
    ///
    /// # Returns
    /// A new handle to the inserted resource.
    pub fn insert(&mut self, resource: R) -> R::Handle {
        let handle = self.resources.insert(resource).into();

        self.changelog.push(ResourceChange {
            handle,
            kind: ResourceChangeKind::Inserted,
            revision: self.revision,
        });
        self.revision += 1;

        handle
    }

    /// Removes the resource with the given handle from the registry.
    ///
    /// # Returns
    /// `true` if the resource existed in the registry.
    pub fn remove(&mut self, handle: R::Handle) -> bool {
        let existed = self.resources.remove(handle.into());

        if !existed {
            return false;
        }

        self.changelog.push(ResourceChange {
            handle,
            kind: ResourceChangeKind::Removed,
            revision: self.revision,
        });
        self.revision += 1;

        true
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
    pub fn changes_since(&self, revision: u64) -> &[ResourceChange<R>] {
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
    pub fn drain_changes_since(&mut self, revision: u64) -> Drain<'_, ResourceChange<R>> {
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

impl<R: Resource> Default for ResourceRegistry<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Resource> ResourceChange<R> {
    /// Returns the handle of the changed resource.
    pub fn handle(&self) -> R::Handle {
        self.handle
    }

    /// Returns the type of change that occurred.
    pub fn kind(&self) -> &ResourceChangeKind<R> {
        &self.kind
    }

    /// Returns the revision when this change occurred.
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

impl<'a, R: Resource> ResourceMutRef<'a, R> {
    /// Sets the dirty mask for this resource.
    ///
    /// If the mask is non-empty when this reference is dropped, a change
    /// will be recorded in the registry's changelog.
    pub fn set_dirty_mask(&mut self, dirty_mask: R::DirtyMask) {
        self.dirty_mask = dirty_mask;
    }
}

impl<'a, R: Resource> Deref for ResourceMutRef<'a, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.resource
    }
}

impl<'a, R: Resource> DerefMut for ResourceMutRef<'a, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.resource
    }
}

impl<'a, R: Resource> Drop for ResourceMutRef<'a, R> {
    fn drop(&mut self) {
        if self.dirty_mask == R::DirtyMask::empty() {
            return;
        }
        self.changelog.push(ResourceChange {
            handle: self.handle,
            kind: ResourceChangeKind::Modified(self.dirty_mask),
            revision: *self.revision,
        });
        *self.revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BinaryDirtyMask;
    use impact_containers::SlotKey;

    // Test resource type
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestResource {
        data: String,
        value: i32,
    }

    // Test handle type
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestHandle(SlotKey);

    impl_ResourceHandle_for_newtype!(TestHandle);

    impl Resource for TestResource {
        type Handle = TestHandle;
        type DirtyMask = BinaryDirtyMask;
    }

    // Helper functions for creating TestResource instances
    fn test_resource(data: &str, value: i32) -> TestResource {
        TestResource {
            data: data.to_string(),
            value,
        }
    }

    fn test_resource_default() -> TestResource {
        TestResource {
            data: "test".to_string(),
            value: 42,
        }
    }

    #[test]
    fn creating_new_registry_gives_empty_registry() {
        let registry = ResourceRegistry::<TestResource>::new();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert_eq!(registry.revision(), 0);
    }

    #[test]
    fn inserting_resource_returns_valid_handle_and_increments_revision() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();

        let handle = registry.insert(resource.clone());

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert_eq!(registry.revision(), 1);
        assert!(registry.contains(handle));
        assert_eq!(registry.get(handle), Some(&resource));
    }

    #[test]
    fn inserting_multiple_resources_gives_different_handles() {
        let mut registry = ResourceRegistry::new();
        let resource1 = test_resource("first", 1);
        let resource2 = test_resource("second", 2);

        let handle1 = registry.insert(resource1.clone());
        let handle2 = registry.insert(resource2.clone());

        assert_ne!(handle1, handle2);
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.revision(), 2);
        assert_eq!(registry.get(handle1), Some(&resource1));
        assert_eq!(registry.get(handle2), Some(&resource2));
    }

    #[test]
    fn getting_nonexistent_resource_returns_none() {
        let mut registry = ResourceRegistry::<TestResource>::new();
        let resource = test_resource("temp", 0);
        let handle = registry.insert(resource);
        registry.remove(handle);

        assert_eq!(registry.get(handle), None);
        assert!(!registry.contains(handle));
    }

    #[test]
    fn removing_existing_resource_returns_true_and_increments_revision() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();
        let handle = registry.insert(resource);

        let removed = registry.remove(handle);

        assert!(removed);
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert_eq!(registry.revision(), 2); // Insert + remove
        assert!(!registry.contains(handle));
        assert_eq!(registry.get(handle), None);
    }

    #[test]
    fn removing_nonexistent_resource_returns_false_and_does_not_increment_revision() {
        let mut registry = ResourceRegistry::<TestResource>::new();
        let resource = test_resource("temp", 0);
        let handle = registry.insert(resource);
        registry.remove(handle);
        let initial_revision = registry.revision();

        let removed = registry.remove(handle);

        assert!(!removed);
        assert_eq!(registry.revision(), initial_revision);
    }

    #[test]
    fn getting_mutable_reference_allows_modification() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource("original", 10);
        let handle = registry.insert(resource);

        {
            let mut resource_ref = registry.get_mut(handle).unwrap();
            resource_ref.data = "modified".to_string();
            resource_ref.value = 20;
        }

        let retrieved = registry.get(handle).unwrap();
        assert_eq!(retrieved.data, "modified");
        assert_eq!(retrieved.value, 20);
    }

    #[test]
    fn getting_mutable_reference_for_nonexistent_resource_returns_none() {
        let mut registry = ResourceRegistry::<TestResource>::new();
        let resource = test_resource("temp", 0);
        let handle = registry.insert(resource);
        registry.remove(handle);

        assert!(registry.get_mut(handle).is_none());
    }

    #[test]
    fn setting_dirty_mask_on_mut_ref_records_modification_change() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();
        let handle = registry.insert(resource);
        let initial_revision = registry.revision();

        {
            let mut resource_ref = registry.get_mut(handle).unwrap();
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
            resource_ref.data = "modified".to_string();
        }

        assert_eq!(registry.revision(), initial_revision + 1);
        let changes = registry.changes_since(initial_revision);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].handle(), handle);
        assert_eq!(
            changes[0].kind(),
            &ResourceChangeKind::Modified(BinaryDirtyMask::ALL)
        );
        assert_eq!(changes[0].revision(), initial_revision);
    }

    #[test]
    fn not_setting_dirty_mask_on_mut_ref_does_not_record_change() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();
        let handle = registry.insert(resource);
        let initial_revision = registry.revision();

        {
            let mut resource_ref = registry.get_mut(handle).unwrap();
            resource_ref.data = "modified".to_string();
            // Not calling set_dirty_mask
        }

        assert_eq!(registry.revision(), initial_revision);
        let changes = registry.changes_since(initial_revision);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn setting_empty_dirty_mask_does_not_record_change() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();
        let handle = registry.insert(resource);
        let initial_revision = registry.revision();

        {
            let mut resource_ref = registry.get_mut(handle).unwrap();
            resource_ref.set_dirty_mask(BinaryDirtyMask::empty());
            resource_ref.data = "modified".to_string();
        }

        assert_eq!(registry.revision(), initial_revision);
        let changes = registry.changes_since(initial_revision);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn iter_returns_all_resources_with_handles() {
        let mut registry = ResourceRegistry::new();
        let resource1 = test_resource("first", 1);
        let resource2 = test_resource("second", 2);

        let handle1 = registry.insert(resource1.clone());
        let handle2 = registry.insert(resource2.clone());

        let mut collected: Vec<_> = registry.iter().collect();
        collected.sort_by_key(|(_, resource)| resource.value);

        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], (handle1, &resource1));
        assert_eq!(collected[1], (handle2, &resource2));
    }

    #[test]
    fn insert_records_inserted_change() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();

        let handle = registry.insert(resource);

        let changes = registry.changes_since(0);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].handle(), handle);
        assert_eq!(changes[0].kind(), &ResourceChangeKind::Inserted);
        assert_eq!(changes[0].revision(), 0);
    }

    #[test]
    fn remove_records_removed_change() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();
        let handle = registry.insert(resource);

        registry.remove(handle);

        let changes = registry.changes_since(0);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].kind(), &ResourceChangeKind::Inserted);
        assert_eq!(changes[1].handle(), handle);
        assert_eq!(changes[1].kind(), &ResourceChangeKind::Removed);
        assert_eq!(changes[1].revision(), 1);
    }

    #[test]
    fn changes_since_returns_changes_after_given_revision() {
        let mut registry = ResourceRegistry::new();
        let resource1 = test_resource("first", 1);
        let resource2 = test_resource("second", 2);

        let _handle1 = registry.insert(resource1);
        let revision_after_first = registry.revision();
        let handle2 = registry.insert(resource2);

        let changes = registry.changes_since(revision_after_first);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].handle(), handle2);
        assert_eq!(changes[0].kind(), &ResourceChangeKind::Inserted);
    }

    #[test]
    fn changes_since_returns_empty_slice_for_current_revision() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();

        registry.insert(resource);
        let current_revision = registry.revision();

        let changes = registry.changes_since(current_revision);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn changes_since_returns_empty_slice_for_future_revision() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();

        registry.insert(resource);

        let changes = registry.changes_since(1000);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn drain_changes_since_removes_and_returns_changes_after_given_revision() {
        let mut registry = ResourceRegistry::new();
        let resource1 = test_resource("first", 1);
        let resource2 = test_resource("second", 2);

        let handle1 = registry.insert(resource1);
        let revision_after_first = registry.revision();
        let handle2 = registry.insert(resource2);

        let drained: Vec<_> = registry.drain_changes_since(revision_after_first).collect();

        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].handle(), handle2);
        assert_eq!(drained[0].kind(), &ResourceChangeKind::Inserted);

        // Changes should be removed from registry
        let remaining_changes = registry.changes_since(0);
        assert_eq!(remaining_changes.len(), 1);
        assert_eq!(remaining_changes[0].handle(), handle1);
    }

    #[test]
    fn drain_changes_since_returns_empty_for_current_revision() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();

        registry.insert(resource);
        let current_revision = registry.revision();

        assert_eq!(registry.drain_changes_since(current_revision).count(), 0);
    }

    #[test]
    fn multiple_modifications_create_separate_change_entries() {
        let mut registry = ResourceRegistry::new();
        let resource = test_resource_default();
        let handle = registry.insert(resource);
        let initial_revision = registry.revision();

        // First modification
        {
            let mut resource_ref = registry.get_mut(handle).unwrap();
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
            resource_ref.data = "modified1".to_string();
        }

        // Second modification
        {
            let mut resource_ref = registry.get_mut(handle).unwrap();
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
            resource_ref.data = "modified2".to_string();
        }

        let changes = registry.changes_since(initial_revision);
        assert_eq!(changes.len(), 2);
        assert_eq!(
            changes[0].kind(),
            &ResourceChangeKind::Modified(BinaryDirtyMask::ALL)
        );
        assert_eq!(
            changes[1].kind(),
            &ResourceChangeKind::Modified(BinaryDirtyMask::ALL)
        );
        assert_eq!(registry.revision(), initial_revision + 2);
    }

    #[test]
    fn complex_scenario_with_mixed_operations_maintains_correct_state() {
        let mut registry = ResourceRegistry::new();

        // Insert resources
        let resource1 = test_resource("first", 1);
        let resource2 = test_resource("second", 2);
        let resource3 = test_resource("third", 3);

        let handle1 = registry.insert(resource1);
        let handle2 = registry.insert(resource2);
        let handle3 = registry.insert(resource3);

        // Modify resource1
        {
            let mut resource_ref = registry.get_mut(handle1).unwrap();
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
            resource_ref.data = "first_modified".to_string();
        }

        // Remove resource2
        registry.remove(handle2);

        // Modify resource3
        {
            let mut resource_ref = registry.get_mut(handle3).unwrap();
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
            resource_ref.value = 30;
        }

        // Verify final state
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.revision(), 6); // 3 inserts + 1 modify + 1 remove + 1 modify
        assert!(registry.contains(handle1));
        assert!(!registry.contains(handle2));
        assert!(registry.contains(handle3));

        // Verify changes
        let all_changes = registry.changes_since(0);
        assert_eq!(all_changes.len(), 6);

        let change_kinds: Vec<_> = all_changes.iter().map(|c| c.kind()).collect();
        assert_eq!(change_kinds[0], &ResourceChangeKind::Inserted); // resource1
        assert_eq!(change_kinds[1], &ResourceChangeKind::Inserted); // resource2
        assert_eq!(change_kinds[2], &ResourceChangeKind::Inserted); // resource3
        assert_eq!(
            change_kinds[3],
            &ResourceChangeKind::Modified(BinaryDirtyMask::ALL)
        ); // resource1
        assert_eq!(change_kinds[4], &ResourceChangeKind::Removed); // resource2
        assert_eq!(
            change_kinds[5],
            &ResourceChangeKind::Modified(BinaryDirtyMask::ALL)
        ); // resource3
    }
}
