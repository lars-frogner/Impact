//! GPU resource management and synchronization.

use crate::{
    Resource, ResourceLabelProvider, ResourcePID,
    index::ResourceIndex,
    indexed_registry::IndexedResourceRegistry,
    registry::{ResourceChange, ResourceChangeKind, ResourceRegistry},
};
use impact_containers::HashMap;

/// Manages GPU resources corresponding to CPU resources.
pub trait GPUResources<R: Resource> {
    type GraphicsDevice;

    /// Ensures a GPU resource exists for the given CPU resource.
    fn ensure(
        &mut self,
        graphics_device: &Self::GraphicsDevice,
        handle: R::Handle,
        resource: &R,
        label_provider: &impl ResourceLabelProvider<R::Handle>,
    );

    /// Updates the GPU resource with changes from the CPU resource.
    fn update(
        &mut self,
        graphics_device: &Self::GraphicsDevice,
        handle: R::Handle,
        resource: &R,
        dirty_mask: R::DirtyMask,
        label_provider: &impl ResourceLabelProvider<R::Handle>,
    );

    /// Removes the GPU resource for the given handle.
    fn evict(&mut self, handle: R::Handle);

    /// Returns the last registry revision that was synchronized.
    fn last_synced_revision(&self) -> u64;

    /// Sets the last registry revision that was synchronized.
    fn set_last_synced_revision(&mut self, revision: u64);
}

/// A GPU resource that corresponds to a CPU resource.
pub trait GPUResource<R: Resource> {
    type GraphicsDevice;

    /// Creates a new GPU resource from the given CPU resource.
    fn create(graphics_device: &Self::GraphicsDevice, resource: &R, label: String) -> Self;

    /// Updates this GPU resource with changes from the CPU resource.
    fn update(
        &mut self,
        graphics_device: &Self::GraphicsDevice,
        resource: &R,
        dirty_mask: R::DirtyMask,
    );
}

/// A map of GPU resources indexed by resource handles.
#[derive(Debug)]
pub struct GPUResourceMap<R: Resource, GR> {
    gpu_resources: HashMap<R::Handle, GR>,
    last_synced_revision: u64,
}

impl<R: Resource, GR> GPUResourceMap<R, GR> {
    /// Creates a new empty GPU resource map.
    pub fn new() -> Self {
        Self {
            gpu_resources: HashMap::default(),
            last_synced_revision: 0,
        }
    }

    /// Returns the GPU resource for the given handle.
    pub fn get(&self, handle: R::Handle) -> Option<&GR> {
        self.gpu_resources.get(&handle)
    }

    /// Returns the GPU resource for the given persistent ID.
    pub fn get_by_pid<PID: ResourcePID>(
        &self,
        index: &ResourceIndex<PID, R::Handle>,
        pid: PID,
    ) -> Option<&GR> {
        index.get_handle(pid).and_then(|handle| self.get(handle))
    }
}

impl<R: Resource, GR> Default for GPUResourceMap<R, GR> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, GR> GPUResources<R> for GPUResourceMap<R, GR>
where
    R: Resource,
    GR: GPUResource<R>,
{
    type GraphicsDevice = GR::GraphicsDevice;

    fn ensure(
        &mut self,
        graphics_device: &Self::GraphicsDevice,
        handle: R::Handle,
        resource: &R,
        label_provider: &impl ResourceLabelProvider<R::Handle>,
    ) {
        self.gpu_resources.entry(handle).or_insert_with(|| {
            GR::create(
                graphics_device,
                resource,
                label_provider.create_label(handle),
            )
        });
    }

    fn update(
        &mut self,
        graphics_device: &Self::GraphicsDevice,
        handle: R::Handle,
        resource: &R,
        dirty_mask: R::DirtyMask,
        label_provider: &impl ResourceLabelProvider<R::Handle>,
    ) {
        self.gpu_resources
            .entry(handle)
            .and_modify(|gpu_resource| gpu_resource.update(graphics_device, resource, dirty_mask))
            .or_insert_with(|| {
                GR::create(
                    graphics_device,
                    resource,
                    label_provider.create_label(handle),
                )
            });
    }

    fn evict(&mut self, handle: R::Handle) {
        self.gpu_resources.remove(&handle);
    }

    fn last_synced_revision(&self) -> u64 {
        self.last_synced_revision
    }

    fn set_last_synced_revision(&mut self, revision: u64) {
        self.last_synced_revision = revision;
    }
}

/// Synchronizes GPU resources with an indexed resource registry.
pub fn sync_indexed_gpu_resources<PID, R, GR>(
    graphics_device: &GR::GraphicsDevice,
    registry: &IndexedResourceRegistry<PID, R>,
    gpu_resources: &mut GR,
) where
    PID: ResourcePID,
    R: Resource,
    GR: GPUResources<R>,
{
    sync_gpu_resources(
        graphics_device,
        &registry.registry,
        gpu_resources,
        &registry.index,
    );
}

/// Synchronizes GPU resources with a resource registry.
///
/// Applies all changes that occurred since the last synchronization.
pub fn sync_gpu_resources<R, GR>(
    graphics_device: &GR::GraphicsDevice,
    registry: &ResourceRegistry<R>,
    gpu_resources: &mut GR,
    label_provider: &impl ResourceLabelProvider<R::Handle>,
) where
    R: Resource,
    GR: GPUResources<R>,
{
    for change in registry.changes_since(gpu_resources.last_synced_revision()) {
        sync_gpu_resource(
            graphics_device,
            registry,
            gpu_resources,
            change,
            label_provider,
        );
    }
    gpu_resources.set_last_synced_revision(registry.revision());
}

fn sync_gpu_resource<R, GR>(
    graphics_device: &GR::GraphicsDevice,
    registry: &ResourceRegistry<R>,
    gpu_resources: &mut GR,
    change: &ResourceChange<R>,
    label_provider: &impl ResourceLabelProvider<R::Handle>,
) where
    R: Resource,
    GR: GPUResources<R>,
{
    match change.kind() {
        ResourceChangeKind::Inserted => {
            // Resource might have been removed after insertion, so only ensure if it still exists
            if let Some(resource) = registry.get(change.handle()) {
                gpu_resources.ensure(graphics_device, change.handle(), resource, label_provider);
            }
        }
        ResourceChangeKind::Modified(dirty_mask) => {
            // Resource might have been removed after modification, so only update if it still exists
            if let Some(resource) = registry.get(change.handle()) {
                gpu_resources.update(
                    graphics_device,
                    change.handle(),
                    resource,
                    *dirty_mask,
                    label_provider,
                );
            }
        }
        ResourceChangeKind::Removed => {
            gpu_resources.evict(change.handle());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryDirtyMask, HandleLabelProvider};
    use impact_containers::SlotKey;
    use std::fmt;

    // Test types
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestPID(u32);

    impl fmt::Display for TestPID {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestPID({})", self.0)
        }
    }

    impl ResourcePID for TestPID {}

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestHandle(SlotKey);

    impl_ResourceHandle_for_newtype!(TestHandle);

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestResource {
        data: String,
        value: i32,
    }

    impl Resource for TestResource {
        type Handle = TestHandle;
        type DirtyMask = BinaryDirtyMask;
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestGPUResource {
        data: String,
        value: i32,
        label: String,
    }

    struct TestGraphicsDevice;

    impl GPUResource<TestResource> for TestGPUResource {
        type GraphicsDevice = TestGraphicsDevice;

        fn create(
            _graphics_device: &Self::GraphicsDevice,
            resource: &TestResource,
            label: String,
        ) -> Self {
            Self {
                data: resource.data.clone(),
                value: resource.value,
                label,
            }
        }

        fn update(
            &mut self,
            _graphics_device: &Self::GraphicsDevice,
            resource: &TestResource,
            _dirty_mask: BinaryDirtyMask,
        ) {
            self.data = resource.data.clone();
            self.value = resource.value;
        }
    }

    fn test_resource(data: &str, value: i32) -> TestResource {
        TestResource {
            data: data.to_string(),
            value,
        }
    }

    fn test_handle(id: u32) -> TestHandle {
        use bytemuck::Zeroable;
        let mut key = SlotKey::zeroed();
        let bytes = bytemuck::bytes_of_mut(&mut key);
        bytes[0] = id as u8;
        TestHandle(key)
    }

    #[test]
    fn creating_new_gpu_resource_map_is_empty() {
        let map = GPUResourceMap::<TestResource, TestGPUResource>::new();

        assert!(map.get(test_handle(1)).is_none());
        assert_eq!(map.last_synced_revision(), 0);
    }

    #[test]
    fn ensuring_gpu_resource_creates_new_resource() {
        let mut map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let handle = test_handle(1);
        let resource = test_resource("test", 42);
        let label_provider = HandleLabelProvider;

        map.ensure(&graphics_device, handle, &resource, &label_provider);

        let gpu_resource = map.get(handle).unwrap();
        assert_eq!(gpu_resource.data, "test");
        assert_eq!(gpu_resource.value, 42);
        assert_eq!(gpu_resource.label, handle.to_string());
    }

    #[test]
    fn ensuring_existing_gpu_resource_does_not_recreate() {
        let mut map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let handle = test_handle(1);
        let resource = test_resource("test", 42);
        let label_provider = HandleLabelProvider;

        map.ensure(&graphics_device, handle, &resource, &label_provider);
        let original_resource = map.get(handle).unwrap().clone();

        let different_resource = test_resource("different", 99);
        map.ensure(
            &graphics_device,
            handle,
            &different_resource,
            &label_provider,
        );

        let gpu_resource = map.get(handle).unwrap();
        assert_eq!(*gpu_resource, original_resource);
    }

    #[test]
    fn updating_gpu_resource_creates_if_not_exists() {
        let mut map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let handle = test_handle(1);
        let resource = test_resource("test", 42);
        let label_provider = HandleLabelProvider;

        map.update(
            &graphics_device,
            handle,
            &resource,
            BinaryDirtyMask::ALL,
            &label_provider,
        );

        let gpu_resource = map.get(handle).unwrap();
        assert_eq!(gpu_resource.data, "test");
        assert_eq!(gpu_resource.value, 42);
    }

    #[test]
    fn updating_existing_gpu_resource_modifies_it() {
        let mut map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let handle = test_handle(1);
        let resource = test_resource("original", 42);
        let label_provider = HandleLabelProvider;

        map.ensure(&graphics_device, handle, &resource, &label_provider);

        let updated_resource = test_resource("updated", 99);
        map.update(
            &graphics_device,
            handle,
            &updated_resource,
            BinaryDirtyMask::ALL,
            &label_provider,
        );

        let gpu_resource = map.get(handle).unwrap();
        assert_eq!(gpu_resource.data, "updated");
        assert_eq!(gpu_resource.value, 99);
    }

    #[test]
    fn evicting_gpu_resource_removes_it() {
        let mut map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let handle = test_handle(1);
        let resource = test_resource("test", 42);
        let label_provider = HandleLabelProvider;

        map.ensure(&graphics_device, handle, &resource, &label_provider);
        assert!(map.get(handle).is_some());

        map.evict(handle);
        assert!(map.get(handle).is_none());
    }

    #[test]
    fn evicting_nonexistent_gpu_resource_does_nothing() {
        let mut map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let handle = test_handle(1);

        map.evict(handle); // Should not panic
        assert!(map.get(handle).is_none());
    }

    #[test]
    fn get_by_pid_returns_correct_resource() {
        let mut map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let mut index = ResourceIndex::new();
        let graphics_device = TestGraphicsDevice;
        let handle = test_handle(1);
        let pid = TestPID(42);
        let resource = test_resource("test", 42);
        let label_provider = HandleLabelProvider;

        map.ensure(&graphics_device, handle, &resource, &label_provider);
        index.bind(pid, handle);

        let gpu_resource = map.get_by_pid(&index, pid).unwrap();
        assert_eq!(gpu_resource.data, "test");
        assert_eq!(gpu_resource.value, 42);
    }

    #[test]
    fn get_by_pid_returns_none_for_nonexistent_pid() {
        let map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let index = ResourceIndex::new();
        let pid = TestPID(99);

        assert!(map.get_by_pid(&index, pid).is_none());
    }

    #[test]
    fn get_by_pid_returns_none_for_unbound_pid() {
        let map = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let mut index = ResourceIndex::new();
        let pid = TestPID(42);
        let handle = test_handle(1);

        // Bind PID to handle but don't create GPU resource
        index.bind(pid, handle);

        assert!(map.get_by_pid(&index, pid).is_none());
    }

    #[test]
    fn sync_gpu_resources_with_insertions_creates_resources() {
        let mut registry = ResourceRegistry::new();
        let mut gpu_resources = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let label_provider = HandleLabelProvider;

        let resource1 = test_resource("first", 1);
        let resource2 = test_resource("second", 2);
        let handle1 = registry.insert(resource1.clone());
        let handle2 = registry.insert(resource2.clone());

        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        let gpu_resource1 = gpu_resources.get(handle1).unwrap();
        let gpu_resource2 = gpu_resources.get(handle2).unwrap();
        assert_eq!(gpu_resource1.data, "first");
        assert_eq!(gpu_resource1.value, 1);
        assert_eq!(gpu_resource2.data, "second");
        assert_eq!(gpu_resource2.value, 2);
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_gpu_resources_with_modifications_updates_resources() {
        let mut registry = ResourceRegistry::new();
        let mut gpu_resources = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let label_provider = HandleLabelProvider;

        let resource = test_resource("original", 42);
        let handle = registry.insert(resource);

        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        // Modify the resource
        if let Some(mut resource_ref) = registry.get_mut(handle) {
            resource_ref.data = "modified".to_string();
            resource_ref.value = 99;
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
        }

        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        let gpu_resource = gpu_resources.get(handle).unwrap();
        assert_eq!(gpu_resource.data, "modified");
        assert_eq!(gpu_resource.value, 99);
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_gpu_resources_with_removals_evicts_resources() {
        let mut registry = ResourceRegistry::new();
        let mut gpu_resources = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let label_provider = HandleLabelProvider;

        let resource = test_resource("test", 42);
        let handle = registry.insert(resource);

        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );
        assert!(gpu_resources.get(handle).is_some());

        registry.remove(handle);
        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        assert!(gpu_resources.get(handle).is_none());
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_gpu_resources_handles_mixed_operations() {
        let mut registry = ResourceRegistry::new();
        let mut gpu_resources = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let label_provider = HandleLabelProvider;

        // Initial sync
        let resource1 = test_resource("first", 1);
        let resource2 = test_resource("second", 2);
        let handle1 = registry.insert(resource1);
        let handle2 = registry.insert(resource2);

        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        // Mixed operations: modify, remove, insert
        if let Some(mut resource_ref) = registry.get_mut(handle1) {
            resource_ref.data = "modified".to_string();
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
        }
        registry.remove(handle2);
        let resource3 = test_resource("third", 3);
        let handle3 = registry.insert(resource3);

        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        let gpu_resource1 = gpu_resources.get(handle1).unwrap();
        assert_eq!(gpu_resource1.data, "modified");
        assert!(gpu_resources.get(handle2).is_none());
        let gpu_resource3 = gpu_resources.get(handle3).unwrap();
        assert_eq!(gpu_resource3.data, "third");
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_gpu_resources_skips_resources_removed_after_insertion() {
        let mut registry = ResourceRegistry::new();
        let mut gpu_resources = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let label_provider = HandleLabelProvider;

        // Insert and immediately remove a resource
        let resource = test_resource("temporary", 42);
        let handle = registry.insert(resource);
        registry.remove(handle);

        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        // GPU resource should not be created since CPU resource was removed
        assert!(gpu_resources.get(handle).is_none());
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_gpu_resources_skips_resources_removed_after_modification() {
        let mut registry = ResourceRegistry::new();
        let mut gpu_resources = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let label_provider = HandleLabelProvider;

        // Insert resource and sync
        let resource = test_resource("test", 42);
        let handle = registry.insert(resource);
        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        // Modify and remove
        if let Some(mut resource_ref) = registry.get_mut(handle) {
            resource_ref.data = "modified".to_string();
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
        }
        registry.remove(handle);

        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        // GPU resource should be removed
        assert!(gpu_resources.get(handle).is_none());
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_gpu_resources_only_processes_new_changes() {
        let mut registry = ResourceRegistry::new();
        let mut gpu_resources = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;
        let label_provider = HandleLabelProvider;

        // First batch of changes
        let resource1 = test_resource("first", 1);
        let handle1 = registry.insert(resource1);
        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );
        let first_revision = registry.revision();

        // Second batch of changes
        let resource2 = test_resource("second", 2);
        let handle2 = registry.insert(resource2);

        // Manually set last synced revision to ensure only new changes are processed
        gpu_resources.set_last_synced_revision(first_revision);
        sync_gpu_resources(
            &graphics_device,
            &registry,
            &mut gpu_resources,
            &label_provider,
        );

        // Both resources should exist
        assert!(gpu_resources.get(handle1).is_some());
        assert!(gpu_resources.get(handle2).is_some());
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_indexed_gpu_resources_works_correctly() {
        let mut indexed_registry = crate::indexed_registry::IndexedResourceRegistry::new();
        let mut gpu_resources = GPUResourceMap::<TestResource, TestGPUResource>::new();
        let graphics_device = TestGraphicsDevice;

        let resource = test_resource("test", 42);
        let pid = TestPID(1);
        let handle = indexed_registry.insert_resource_with_pid(pid, resource.clone());

        sync_indexed_gpu_resources(&graphics_device, &indexed_registry, &mut gpu_resources);

        let gpu_resource = gpu_resources.get(handle).unwrap();
        assert_eq!(gpu_resource.data, "test");
        assert_eq!(gpu_resource.value, 42);
        assert_eq!(gpu_resource.label, pid.to_string());
        assert_eq!(
            gpu_resources.last_synced_revision(),
            indexed_registry.registry.revision()
        );
    }
}
