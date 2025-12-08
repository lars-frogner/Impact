//! GPU resource management and synchronization.

use crate::{
    MutableResource, Resource,
    alloc::ResourceOperationArenas,
    registry::{
        ImmutableResourceRegistry, MutableResourceRegistry, ResourceChange, ResourceChangeKind,
        ResourceRegistry,
    },
};
use anyhow::Result;
use impact_alloc::Allocator;
use impact_containers::RandomState;
use std::{
    collections::{HashMap, hash_map::Entry},
    fmt,
    hash::BuildHasher,
};

/// Manages GPU resources corresponding to CPU resources.
pub trait GPUResources<'a, R: Resource> {
    type GPUContext;

    /// Ensures a GPU resource exists for the given CPU resource.
    fn ensure(&mut self, gpu_context: &Self::GPUContext, id: R::ID, resource: &R) -> Result<()>;

    /// Removes the GPU resource with the given ID.
    fn evict(&mut self, gpu_context: &Self::GPUContext, id: R::ID) -> Result<()>;

    /// Returns the last registry revision that was synchronized.
    fn last_synced_revision(&self) -> u64;

    /// Sets the last registry revision that was synchronized.
    fn set_last_synced_revision(&mut self, revision: u64);
}

/// Manages GPU resources corresponding to mutable CPU resources.
pub trait MutableGPUResources<'a, R: MutableResource>: GPUResources<'a, R> {
    /// Updates the GPU resource with changes from the CPU resource.
    fn update(
        &mut self,
        gpu_context: &Self::GPUContext,
        id: R::ID,
        resource: &R,
        dirty_mask: R::DirtyMask,
    ) -> Result<()>;
}

/// A GPU resource that corresponds to a CPU resource.
pub trait GPUResource<'a, R: Resource>: Sized {
    type GPUContext;

    /// Creates a new GPU resource from the given CPU resource. Returns [`None`]
    /// if there is no GPU resource to create.
    ///
    /// # Warning
    /// The passed allocator must only be used for allocations that don't
    /// outlive this method call.
    fn create<A>(
        scratch: A,
        gpu_context: &Self::GPUContext,
        id: R::ID,
        resource: &R,
    ) -> Result<Option<Self>>
    where
        A: Copy + Allocator;

    /// Performs cleanup for the GPU resource.
    fn cleanup(self, gpu_context: &Self::GPUContext, id: R::ID) -> Result<()>;
}

/// A GPU resource that corresponds to a mutable CPU resource.
pub trait MutableGPUResource<'a, R: MutableResource>: GPUResource<'a, R> {
    /// Updates this GPU resource with changes from the CPU resource.
    fn update(
        &mut self,
        gpu_context: &Self::GPUContext,
        resource: &R,
        dirty_mask: R::DirtyMask,
    ) -> Result<()>;
}

/// A map of GPU resources indexed by resource IDs.
pub struct GPUResourceMap<R: Resource, GR, S = RandomState> {
    gpu_resources: HashMap<R::ID, GR, S>,
    last_synced_revision: u64,
}

impl<R, GR, S> GPUResourceMap<R, GR, S>
where
    R: Resource,
    S: Default,
{
    /// Creates a new empty GPU resource map.
    pub fn new() -> Self {
        Self {
            gpu_resources: HashMap::default(),
            last_synced_revision: 0,
        }
    }
}

impl<R, GR, S> GPUResourceMap<R, GR, S>
where
    R: Resource,
    S: BuildHasher,
{
    /// Returns the GPU resource with the given ID.
    pub fn get(&self, id: R::ID) -> Option<&GR> {
        self.gpu_resources.get(&id)
    }

    /// Whether the map contains the GPU resource with the given key.
    pub fn contains(&self, id: R::ID) -> bool {
        self.gpu_resources.contains_key(&id)
    }
}

impl<R: Resource, GR, S> fmt::Debug for GPUResourceMap<R, GR, S>
where
    R::ID: fmt::Debug,
    GR: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GPUResourceMap")
            .field("gpu_resources", &self.gpu_resources)
            .field("last_synced_revision", &self.last_synced_revision)
            .finish()
    }
}

impl<R, GR, S> Default for GPUResourceMap<R, GR, S>
where
    R: Resource,
    S: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, R, GR, S> GPUResources<'a, R> for GPUResourceMap<R, GR, S>
where
    R: Resource,
    GR: GPUResource<'a, R>,
    S: BuildHasher,
{
    type GPUContext = GR::GPUContext;

    fn ensure(&mut self, gpu_context: &Self::GPUContext, id: R::ID, resource: &R) -> Result<()> {
        if let Entry::Vacant(entry) = self.gpu_resources.entry(id)
            && let Some(gpu_resource) =
                ResourceOperationArenas::with(|arena| GR::create(arena, gpu_context, id, resource))?
        {
            entry.insert(gpu_resource);
        }

        Ok(())
    }

    fn evict(&mut self, gpu_context: &Self::GPUContext, id: R::ID) -> Result<()> {
        if let Some(gpu_resource) = self.gpu_resources.remove(&id) {
            gpu_resource.cleanup(gpu_context, id)?;
        }
        Ok(())
    }

    fn last_synced_revision(&self) -> u64 {
        self.last_synced_revision
    }

    fn set_last_synced_revision(&mut self, revision: u64) {
        self.last_synced_revision = revision;
    }
}

impl<'a, R, GR, S> MutableGPUResources<'a, R> for GPUResourceMap<R, GR, S>
where
    R: MutableResource,
    GR: MutableGPUResource<'a, R>,
    S: BuildHasher,
{
    fn update(
        &mut self,
        gpu_context: &Self::GPUContext,
        id: R::ID,
        resource: &R,
        dirty_mask: R::DirtyMask,
    ) -> Result<()> {
        match self.gpu_resources.entry(id) {
            Entry::Vacant(entry) => {
                if let Some(gpu_resource) = ResourceOperationArenas::with(|arena| {
                    GR::create(arena, gpu_context, id, resource)
                })? {
                    entry.insert(gpu_resource);
                }
            }
            Entry::Occupied(mut entry) => {
                entry.get_mut().update(gpu_context, resource, dirty_mask)?;
            }
        }
        Ok(())
    }
}

/// Synchronizes immutable GPU resources with a resource registry.
///
/// Applies all changes that occurred since the last synchronization.
pub fn sync_immutable_gpu_resources<'a, R, GR, S>(
    gpu_context: &GR::GPUContext,
    registry: &ImmutableResourceRegistry<R, S>,
    gpu_resources: &mut GR,
) -> Result<()>
where
    R: Resource,
    GR: GPUResources<'a, R>,
    S: BuildHasher,
{
    for change in registry.changes_since(gpu_resources.last_synced_revision()) {
        sync_immutable_gpu_resource(gpu_context, registry, gpu_resources, change)?;
    }
    gpu_resources.set_last_synced_revision(registry.revision());
    Ok(())
}

/// Synchronizes mutable GPU resources with a resource registry.
///
/// Applies all changes that occurred since the last synchronization.
pub fn sync_mutable_gpu_resources<'a, R, GR, S>(
    gpu_context: &GR::GPUContext,
    registry: &MutableResourceRegistry<R, S>,
    gpu_resources: &mut GR,
) -> Result<()>
where
    R: MutableResource,
    GR: MutableGPUResources<'a, R>,
    S: BuildHasher,
{
    for change in registry.changes_since(gpu_resources.last_synced_revision()) {
        sync_mutable_gpu_resource(gpu_context, registry, gpu_resources, change)?;
    }
    gpu_resources.set_last_synced_revision(registry.revision());
    Ok(())
}

fn sync_immutable_gpu_resource<'a, R, GR, S>(
    gpu_context: &GR::GPUContext,
    registry: &ImmutableResourceRegistry<R, S>,
    gpu_resources: &mut GR,
    change: &ResourceChange<R, ()>,
) -> Result<()>
where
    R: Resource,
    GR: GPUResources<'a, R>,
    S: BuildHasher,
{
    match change.kind() {
        ResourceChangeKind::Inserted => {
            ensure_resource(gpu_context, registry, change.id(), gpu_resources)?;
        }
        ResourceChangeKind::Removed => {
            evict_resource(gpu_context, gpu_resources, change.id())?;
        }
        ResourceChangeKind::Replaced => {
            replace_resource(gpu_context, registry, change.id(), gpu_resources)?;
        }
        ResourceChangeKind::Modified(_) => unreachable!(),
    }
    Ok(())
}

fn sync_mutable_gpu_resource<'a, R, GR, S>(
    gpu_context: &GR::GPUContext,
    registry: &MutableResourceRegistry<R, S>,
    gpu_resources: &mut GR,
    change: &ResourceChange<R, R::DirtyMask>,
) -> Result<()>
where
    R: MutableResource,
    GR: MutableGPUResources<'a, R>,
    S: BuildHasher,
{
    match change.kind() {
        ResourceChangeKind::Inserted => {
            ensure_resource(gpu_context, registry, change.id(), gpu_resources)?;
        }
        ResourceChangeKind::Removed => {
            evict_resource(gpu_context, gpu_resources, change.id())?;
        }
        ResourceChangeKind::Replaced => {
            replace_resource(gpu_context, registry, change.id(), gpu_resources)?;
        }
        ResourceChangeKind::Modified(dirty_mask) => {
            modify_resource(
                gpu_context,
                registry,
                change.id(),
                *dirty_mask,
                gpu_resources,
            )?;
        }
    }
    Ok(())
}

fn ensure_resource<'a, R, GR, D, S>(
    gpu_context: &GR::GPUContext,
    registry: &ResourceRegistry<R, D, S>,
    id: R::ID,
    gpu_resources: &mut GR,
) -> Result<()>
where
    R: Resource,
    GR: GPUResources<'a, R>,
    S: BuildHasher,
{
    // Resource might have been removed after insertion, so only ensure if it still exists
    if let Some(resource) = registry.get(id) {
        gpu_resources.ensure(gpu_context, id, resource)?;
    }
    Ok(())
}

fn evict_resource<'a, R, GR>(
    gpu_context: &GR::GPUContext,
    gpu_resources: &mut GR,
    id: R::ID,
) -> Result<()>
where
    R: Resource,
    GR: GPUResources<'a, R>,
{
    gpu_resources.evict(gpu_context, id)
}

fn replace_resource<'a, R, GR, D, S>(
    gpu_context: &GR::GPUContext,
    registry: &ResourceRegistry<R, D, S>,
    id: R::ID,
    gpu_resources: &mut GR,
) -> Result<()>
where
    R: Resource,
    GR: GPUResources<'a, R>,
    S: BuildHasher,
{
    evict_resource(gpu_context, gpu_resources, id)?;
    ensure_resource(gpu_context, registry, id, gpu_resources)
}

fn modify_resource<'a, R, GR, S>(
    gpu_context: &GR::GPUContext,
    registry: &MutableResourceRegistry<R, S>,
    id: R::ID,
    dirty_mask: R::DirtyMask,
    gpu_resources: &mut GR,
) -> Result<()>
where
    R: MutableResource,
    GR: MutableGPUResources<'a, R>,
    S: BuildHasher,
{
    // Resource might have been removed after modification, so only update if it
    // still exists
    if let Some(resource) = registry.get(id) {
        gpu_resources.update(gpu_context, id, resource, dirty_mask)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryDirtyMask, MutableResource, Resource, ResourceID};
    use anyhow::Result;

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

    // Test GPU context
    #[derive(Debug, Default)]
    struct TestGPUContext {
        create_calls: std::cell::RefCell<Vec<TestResourceID>>,
        update_calls: std::cell::RefCell<Vec<(TestResourceID, BinaryDirtyMask)>>,
        cleanup_calls: std::cell::RefCell<Vec<TestResourceID>>,
    }

    impl TestGPUContext {
        fn reset(&self) {
            self.create_calls.borrow_mut().clear();
            self.update_calls.borrow_mut().clear();
            self.cleanup_calls.borrow_mut().clear();
        }

        fn create_calls(&self) -> Vec<TestResourceID> {
            self.create_calls.borrow().clone()
        }

        fn update_calls(&self) -> Vec<(TestResourceID, BinaryDirtyMask)> {
            self.update_calls.borrow().clone()
        }

        fn cleanup_calls(&self) -> Vec<TestResourceID> {
            self.cleanup_calls.borrow().clone()
        }
    }

    // Test GPU resource implementations
    #[derive(Debug, PartialEq, Eq)]
    struct TestGPUResource {
        id: TestResourceID,
        gpu_data: String,
    }

    impl<'a> GPUResource<'a, TestResource> for TestGPUResource {
        type GPUContext = TestGPUContext;

        fn create<A>(
            _scratch: A,
            gpu_context: &Self::GPUContext,
            id: TestResourceID,
            resource: &TestResource,
        ) -> Result<Option<Self>>
        where
            A: Copy + Allocator,
        {
            gpu_context.create_calls.borrow_mut().push(id);
            Ok(Some(Self {
                id,
                gpu_data: format!("GPU_{}", resource.data),
            }))
        }

        fn cleanup(self, gpu_context: &Self::GPUContext, id: TestResourceID) -> Result<()> {
            gpu_context.cleanup_calls.borrow_mut().push(id);
            Ok(())
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct TestMutableGPUResource {
        id: TestResourceID,
        gpu_value: i32,
        gpu_name: String,
    }

    impl<'a> GPUResource<'a, TestMutableResource> for TestMutableGPUResource {
        type GPUContext = TestGPUContext;

        fn create<A>(
            _scratch: A,
            gpu_context: &Self::GPUContext,
            id: TestResourceID,
            resource: &TestMutableResource,
        ) -> Result<Option<Self>>
        where
            A: Copy + Allocator,
        {
            gpu_context.create_calls.borrow_mut().push(id);
            Ok(Some(Self {
                id,
                gpu_value: resource.value,
                gpu_name: resource.name.clone(),
            }))
        }

        fn cleanup(self, gpu_context: &Self::GPUContext, id: TestResourceID) -> Result<()> {
            gpu_context.cleanup_calls.borrow_mut().push(id);
            Ok(())
        }
    }

    impl<'a> MutableGPUResource<'a, TestMutableResource> for TestMutableGPUResource {
        fn update(
            &mut self,
            gpu_context: &Self::GPUContext,
            resource: &TestMutableResource,
            dirty_mask: BinaryDirtyMask,
        ) -> Result<()> {
            gpu_context
                .update_calls
                .borrow_mut()
                .push((self.id, dirty_mask));
            if dirty_mask.contains(BinaryDirtyMask::ALL) {
                self.gpu_value = resource.value;
                self.gpu_name = resource.name.clone();
            }
            Ok(())
        }
    }

    // Test constants
    const ID_1: TestResourceID = TestResourceID(1);
    const ID_2: TestResourceID = TestResourceID(2);

    #[test]
    fn creating_new_gpu_resource_map_gives_empty_map() {
        let map: GPUResourceMap<TestResource, TestGPUResource> = GPUResourceMap::new();

        assert_eq!(map.get(ID_1), None);
        assert_eq!(map.last_synced_revision(), 0);
    }

    #[test]
    fn ensuring_resource_creates_gpu_resource_when_missing() {
        let mut map: GPUResourceMap<TestResource, TestGPUResource> = GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();
        let resource = TestResource {
            data: "test".to_string(),
        };

        let result = map.ensure(&gpu_context, ID_1, &resource);

        assert!(result.is_ok());
        let gpu_resource = map.get(ID_1).unwrap();
        assert_eq!(gpu_resource.id, ID_1);
        assert_eq!(gpu_resource.gpu_data, "GPU_test");
        assert_eq!(gpu_context.create_calls(), vec![ID_1]);
    }

    #[test]
    fn ensuring_resource_does_not_recreate_existing_gpu_resource() {
        let mut map: GPUResourceMap<TestResource, TestGPUResource> = GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();
        let resource = TestResource {
            data: "test".to_string(),
        };

        // First ensure
        map.ensure(&gpu_context, ID_1, &resource).unwrap();
        gpu_context.reset();

        // Second ensure
        let result = map.ensure(&gpu_context, ID_1, &resource);

        assert!(result.is_ok());
        assert_eq!(gpu_context.create_calls(), Vec::<TestResourceID>::new());
    }

    #[test]
    fn evicting_resource_removes_gpu_resource() {
        let mut map: GPUResourceMap<TestResource, TestGPUResource> = GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();
        let resource = TestResource {
            data: "test".to_string(),
        };

        map.ensure(&gpu_context, ID_1, &resource).unwrap();
        assert!(map.get(ID_1).is_some());

        map.evict(&gpu_context, ID_1).unwrap();

        assert_eq!(map.get(ID_1), None);
    }

    #[test]
    fn updating_mutable_resource_creates_gpu_resource_when_missing() {
        let mut map: GPUResourceMap<TestMutableResource, TestMutableGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();
        let resource = TestMutableResource {
            value: 42,
            name: "test".to_string(),
        };

        let result = map.update(&gpu_context, ID_1, &resource, BinaryDirtyMask::ALL);

        assert!(result.is_ok());
        let gpu_resource = map.get(ID_1).unwrap();
        assert_eq!(gpu_resource.id, ID_1);
        assert_eq!(gpu_resource.gpu_value, 42);
        assert_eq!(gpu_resource.gpu_name, "test");
        assert_eq!(gpu_context.create_calls(), vec![ID_1]);
    }

    #[test]
    fn updating_mutable_resource_updates_existing_gpu_resource() {
        let mut map: GPUResourceMap<TestMutableResource, TestMutableGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();
        let initial_resource = TestMutableResource {
            value: 42,
            name: "initial".to_string(),
        };
        let updated_resource = TestMutableResource {
            value: 100,
            name: "updated".to_string(),
        };

        // Create initial GPU resource
        map.update(&gpu_context, ID_1, &initial_resource, BinaryDirtyMask::ALL)
            .unwrap();
        gpu_context.reset();

        // Update existing GPU resource
        let result = map.update(&gpu_context, ID_1, &updated_resource, BinaryDirtyMask::ALL);

        assert!(result.is_ok());
        let gpu_resource = map.get(ID_1).unwrap();
        assert_eq!(gpu_resource.gpu_value, 100);
        assert_eq!(gpu_resource.gpu_name, "updated");
        assert_eq!(gpu_context.create_calls(), Vec::<TestResourceID>::new());
        assert_eq!(
            gpu_context.update_calls(),
            vec![(ID_1, BinaryDirtyMask::ALL)]
        );
    }

    #[test]
    fn sync_immutable_gpu_resources_processes_insertions() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestResource, TestGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        let resource = TestResource {
            data: "test".to_string(),
        };
        registry.insert(ID_1, resource);

        let result = sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        assert!(gpu_resources.get(ID_1).is_some());
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
        assert_eq!(gpu_context.create_calls(), vec![ID_1]);
    }

    #[test]
    fn sync_immutable_gpu_resources_processes_removals() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestResource, TestGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        // Insert and sync
        let resource = TestResource {
            data: "test".to_string(),
        };
        registry.insert(ID_1, resource);
        sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources).unwrap();
        assert!(gpu_resources.get(ID_1).is_some());

        // Remove and sync
        registry.remove(ID_1);
        let result = sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        assert_eq!(gpu_resources.get(ID_1), None);
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_immutable_gpu_resources_processes_replacements() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestResource, TestGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        // Insert and sync
        let resource1 = TestResource {
            data: "first".to_string(),
        };
        registry.insert(ID_1, resource1);
        sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources).unwrap();
        let first_gpu_data = gpu_resources.get(ID_1).unwrap().gpu_data.clone();
        gpu_context.reset();

        // Replace and sync
        let resource2 = TestResource {
            data: "second".to_string(),
        };
        registry.insert(ID_1, resource2);
        let result = sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        let gpu_resource = gpu_resources.get(ID_1).unwrap();
        assert_ne!(gpu_resource.gpu_data, first_gpu_data);
        assert_eq!(gpu_resource.gpu_data, "GPU_second");
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
        assert_eq!(gpu_context.create_calls(), vec![ID_1]);
        assert_eq!(gpu_context.cleanup_calls(), vec![ID_1]);
    }

    #[test]
    fn sync_immutable_gpu_resources_handles_removed_after_insertion() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestResource, TestGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        // Insert then immediately remove before sync
        let resource = TestResource {
            data: "test".to_string(),
        };
        registry.insert(ID_1, resource);
        registry.remove(ID_1);

        let result = sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        assert_eq!(gpu_resources.get(ID_1), None);
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
        assert_eq!(gpu_context.create_calls(), Vec::<TestResourceID>::new());
    }

    #[test]
    fn sync_mutable_gpu_resources_processes_insertions() {
        let mut registry: MutableResourceRegistry<TestMutableResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestMutableResource, TestMutableGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        let resource = TestMutableResource {
            value: 42,
            name: "test".to_string(),
        };
        registry.insert(ID_1, resource);

        let result = sync_mutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        assert!(gpu_resources.get(ID_1).is_some());
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
        assert_eq!(gpu_context.create_calls(), vec![ID_1]);
        assert_eq!(gpu_context.cleanup_calls(), vec![]);
    }

    #[test]
    fn sync_mutable_gpu_resources_processes_modifications() {
        let mut registry: MutableResourceRegistry<TestMutableResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestMutableResource, TestMutableGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        // Insert and sync
        let resource = TestMutableResource {
            value: 42,
            name: "test".to_string(),
        };
        registry.insert(ID_1, resource);
        sync_mutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources).unwrap();
        gpu_context.reset();

        // Modify and sync
        {
            let mut resource_ref = registry.get_mut(ID_1).unwrap();
            resource_ref.value = 100;
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
        }
        let result = sync_mutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        let gpu_resource = gpu_resources.get(ID_1).unwrap();
        assert_eq!(gpu_resource.gpu_value, 100);
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
        assert_eq!(gpu_context.create_calls(), Vec::<TestResourceID>::new());
        assert_eq!(
            gpu_context.update_calls(),
            vec![(ID_1, BinaryDirtyMask::ALL)]
        );
    }

    #[test]
    fn sync_mutable_gpu_resources_handles_removed_after_modification() {
        let mut registry: MutableResourceRegistry<TestMutableResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestMutableResource, TestMutableGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        // Insert, modify, then remove before sync
        let resource = TestMutableResource {
            value: 42,
            name: "test".to_string(),
        };
        registry.insert(ID_1, resource);
        {
            let mut resource_ref = registry.get_mut(ID_1).unwrap();
            resource_ref.value = 100;
            resource_ref.set_dirty_mask(BinaryDirtyMask::ALL);
        }
        registry.remove(ID_1);

        let result = sync_mutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        assert_eq!(gpu_resources.get(ID_1), None);
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
        assert_eq!(gpu_context.create_calls(), Vec::<TestResourceID>::new());
        assert_eq!(gpu_context.update_calls(), Vec::new());
    }

    #[test]
    fn sync_immutable_gpu_resources_processes_multiple_changes() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestResource, TestGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        // Add multiple resources
        let resource1 = TestResource {
            data: "first".to_string(),
        };
        let resource2 = TestResource {
            data: "second".to_string(),
        };
        registry.insert(ID_1, resource1);
        registry.insert(ID_2, resource2);
        registry.remove(ID_1);

        let result = sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        assert_eq!(gpu_resources.get(ID_1), None);
        assert!(gpu_resources.get(ID_2).is_some());
        assert_eq!(gpu_resources.last_synced_revision(), registry.revision());
    }

    #[test]
    fn sync_only_processes_changes_since_last_revision() {
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();
        let mut gpu_resources: GPUResourceMap<TestResource, TestGPUResource> =
            GPUResourceMap::new();
        let gpu_context = TestGPUContext::default();

        // First sync
        let resource1 = TestResource {
            data: "first".to_string(),
        };
        registry.insert(ID_1, resource1);
        sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources).unwrap();
        gpu_context.reset();

        // Second sync with additional changes
        let resource2 = TestResource {
            data: "second".to_string(),
        };
        registry.insert(ID_2, resource2);
        let result = sync_immutable_gpu_resources(&gpu_context, &registry, &mut gpu_resources);

        assert!(result.is_ok());
        assert!(gpu_resources.get(ID_1).is_some());
        assert!(gpu_resources.get(ID_2).is_some());
        // Only ID_2 should have been created in the second sync
        assert_eq!(gpu_context.create_calls(), vec![ID_2]);
        assert_eq!(gpu_context.cleanup_calls(), vec![]);
    }

    #[test]
    fn evicting_resource_calls_cleanup() {
        let gpu_context = TestGPUContext::default();
        let mut gpu_resources: GPUResourceMap<TestResource, TestGPUResource> =
            GPUResourceMap::new();
        let mut registry: ImmutableResourceRegistry<TestResource> = ResourceRegistry::new();

        registry.insert(
            ID_1,
            TestResource {
                data: "test".into(),
            },
        );

        let resource = registry.get(ID_1).unwrap();

        // Create the GPU resource
        gpu_resources.ensure(&gpu_context, ID_1, resource).unwrap();
        assert_eq!(gpu_context.create_calls(), vec![ID_1]);
        assert!(gpu_context.cleanup_calls().is_empty());

        // Evict the resource
        gpu_resources.evict(&gpu_context, ID_1).unwrap();

        // Verify cleanup was called
        assert_eq!(gpu_context.cleanup_calls(), vec![ID_1]);
        assert!(gpu_resources.get(ID_1).is_none());
    }

    #[test]
    fn evicting_nonexistent_resource_does_not_call_cleanup() {
        let gpu_context = TestGPUContext::default();
        let mut gpu_resources: GPUResourceMap<TestResource, TestGPUResource> =
            GPUResourceMap::new();

        // Evict a resource that doesn't exist
        gpu_resources.evict(&gpu_context, ID_1).unwrap();

        // Verify cleanup was not called
        assert!(gpu_context.cleanup_calls().is_empty());
    }
}
