//! Indexed resource registry combining resource management with persistent ID
//! mapping.

use crate::{
    Resource, ResourceDirtyMask, ResourceLabelProvider, ResourcePID, index::ResourceIndex,
    registry::ResourceRegistry,
};

/// A resource registry combined with an index for mapping persistent IDs to
/// handles.
#[derive(Debug)]
pub struct IndexedResourceRegistry<PID, R: Resource> {
    /// The underlying resource registry.
    pub registry: ResourceRegistry<R>,
    /// The index mapping persistent IDs to resource handles.
    pub index: ResourceIndex<PID, R::Handle>,
}

impl<PID, R> IndexedResourceRegistry<PID, R>
where
    PID: ResourcePID,
    R: Resource,
{
    /// Creates a new empty indexed resource registry.
    pub fn new() -> Self {
        Self {
            registry: ResourceRegistry::new(),
            index: ResourceIndex::new(),
        }
    }

    /// Inserts a resource with the given persistent ID.
    ///
    /// If a resource with the same PID already exists, that resource will be
    /// replaced while preserving the existing handle.
    ///
    /// # Returns
    /// The handle to the inserted resource.
    pub fn insert_resource_with_pid(&mut self, pid: PID, resource: R) -> R::Handle {
        if let Some(handle) = self.index.get_handle(pid) {
            if let Some(mut existing_resource) = self.registry.get_mut(handle) {
                // Replace the existing resource so that we don't invalidate
                // existing handles
                existing_resource.set_dirty_mask(R::DirtyMask::full());
                *existing_resource = resource;
                return handle;
            }
        }
        let handle = self.registry.insert(resource);
        self.index.bind(pid, handle);
        handle
    }

    /// Returns the handle for the resource with the given persistent ID, or
    /// [`None`] if no resource with the given PID exists.
    pub fn get_handle_to_resource_with_pid(&self, pid: PID) -> Option<R::Handle> {
        let handle = self.index.get_handle(pid)?;
        self.registry.contains(handle).then_some(handle)
    }

    /// Whether a resource with the given persistent ID exists.
    pub fn contains_resource_with_pid(&self, pid: PID) -> bool {
        self.index
            .get_handle(pid)
            .is_some_and(|handle| self.registry.contains(handle))
    }

    /// Creates a human-readable label for the given resource handle.
    ///
    /// Uses the persistent ID if available, otherwise falls back to the handle.
    pub fn label(&self, handle: R::Handle) -> String {
        self.index.create_label(handle)
    }
}

impl<PID, R> Default for IndexedResourceRegistry<PID, R>
where
    PID: ResourcePID,
    R: Resource,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BinaryDirtyMask;
    use impact_containers::SlotKey;
    use std::fmt;

    // Test PID type
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestPID(u32);

    impl fmt::Display for TestPID {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestPID({})", self.0)
        }
    }

    impl ResourcePID for TestPID {}

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

    // Helper functions
    fn test_resource(data: &str, value: i32) -> TestResource {
        TestResource {
            data: data.to_string(),
            value,
        }
    }

    fn test_pid(id: u32) -> TestPID {
        TestPID(id)
    }

    #[test]
    fn inserting_resource_with_pid_stores_resource_and_returns_handle() {
        let mut registry = IndexedResourceRegistry::new();
        let resource = test_resource("test", 42);
        let pid = test_pid(1);

        let handle = registry.insert_resource_with_pid(pid, resource.clone());

        assert_eq!(registry.registry.len(), 1);
        assert!(registry.contains_resource_with_pid(pid));
        assert_eq!(registry.registry.get(handle), Some(&resource));
    }

    #[test]
    fn inserting_multiple_resources_with_different_pids_stores_all() {
        let mut registry = IndexedResourceRegistry::new();
        let resource1 = test_resource("first", 1);
        let resource2 = test_resource("second", 2);
        let pid1 = test_pid(1);
        let pid2 = test_pid(2);

        let handle1 = registry.insert_resource_with_pid(pid1, resource1.clone());
        let handle2 = registry.insert_resource_with_pid(pid2, resource2.clone());

        assert_eq!(registry.registry.len(), 2);
        assert!(registry.contains_resource_with_pid(pid1));
        assert!(registry.contains_resource_with_pid(pid2));
        assert_eq!(registry.registry.get(handle1), Some(&resource1));
        assert_eq!(registry.registry.get(handle2), Some(&resource2));
        assert_ne!(handle1, handle2);
    }

    #[test]
    fn inserting_resource_with_existing_pid_replaces_resource_preserves_handle() {
        let mut registry = IndexedResourceRegistry::new();
        let original_resource = test_resource("original", 42);
        let replacement_resource = test_resource("replacement", 99);
        let pid = test_pid(1);

        let original_handle = registry.insert_resource_with_pid(pid, original_resource);
        let replacement_handle =
            registry.insert_resource_with_pid(pid, replacement_resource.clone());

        assert_eq!(original_handle, replacement_handle);
        assert_eq!(registry.registry.len(), 1);
        assert!(registry.contains_resource_with_pid(pid));
        assert_eq!(
            registry.registry.get(replacement_handle),
            Some(&replacement_resource)
        );
    }

    #[test]
    fn getting_handle_to_resource_with_existing_pid_returns_handle() {
        let mut registry = IndexedResourceRegistry::new();
        let resource = test_resource("test", 42);
        let pid = test_pid(1);

        let inserted_handle = registry.insert_resource_with_pid(pid, resource);
        let retrieved_handle = registry.get_handle_to_resource_with_pid(pid);

        assert_eq!(retrieved_handle, Some(inserted_handle));
    }

    #[test]
    fn getting_handle_to_resource_with_nonexistent_pid_returns_none() {
        let registry: IndexedResourceRegistry<TestPID, TestResource> =
            IndexedResourceRegistry::new();

        let handle = registry.get_handle_to_resource_with_pid(test_pid(999));

        assert_eq!(handle, None);
    }

    #[test]
    fn contains_resource_with_existing_pid_returns_true() {
        let mut registry = IndexedResourceRegistry::new();
        let resource = test_resource("test", 42);
        let pid = test_pid(1);

        registry.insert_resource_with_pid(pid, resource);

        assert!(registry.contains_resource_with_pid(pid));
    }

    #[test]
    fn contains_resource_with_nonexistent_pid_returns_false() {
        let registry: IndexedResourceRegistry<TestPID, TestResource> =
            IndexedResourceRegistry::new();

        assert!(!registry.contains_resource_with_pid(test_pid(999)));
    }

    #[test]
    fn replacing_resource_sets_dirty_mask_to_full() {
        let mut registry = IndexedResourceRegistry::new();
        let original_resource = test_resource("original", 42);
        let replacement_resource = test_resource("replacement", 99);
        let pid = test_pid(1);

        let handle = registry.insert_resource_with_pid(pid, original_resource);
        registry.insert_resource_with_pid(pid, replacement_resource);

        // Check that the resource was marked as dirty by looking at changes
        let changes = registry.registry.changes_since(0);
        let modification_changes: Vec<_> = changes
            .iter()
            .filter(|change| {
                matches!(
                    change.kind(),
                    crate::registry::ResourceChangeKind::Modified(_)
                )
            })
            .collect();

        assert_eq!(modification_changes.len(), 1);
        assert_eq!(modification_changes[0].handle(), handle);
    }

    #[test]
    fn inserting_resource_after_removing_from_registry_fails_gracefully() {
        let mut registry = IndexedResourceRegistry::new();
        let resource = test_resource("test", 42);
        let pid = test_pid(1);

        let handle = registry.insert_resource_with_pid(pid, resource.clone());

        // Remove from registry but not from index (simulating an inconsistent state)
        registry.registry.remove(handle);

        // Inserting with same PID should create new entry since old handle is invalid
        let new_handle = registry.insert_resource_with_pid(pid, resource.clone());

        assert_ne!(handle, new_handle);
        assert!(registry.contains_resource_with_pid(pid));
        assert_eq!(registry.registry.get(new_handle), Some(&resource));
    }

    #[test]
    fn getting_handle_after_resource_removed_from_registry_returns_none() {
        let mut registry = IndexedResourceRegistry::new();
        let resource = test_resource("test", 42);
        let pid = test_pid(1);

        let handle = registry.insert_resource_with_pid(pid, resource);

        // Remove from registry but not from index
        registry.registry.remove(handle);

        // Should return None since the handle is no longer valid in registry
        assert_eq!(registry.get_handle_to_resource_with_pid(pid), None);
        assert!(!registry.contains_resource_with_pid(pid));
    }
}
