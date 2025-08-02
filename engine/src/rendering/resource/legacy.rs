//! Synchronization of GPU buffers with geometrical data.

use anyhow::Result;
use impact_containers::HashMap;
use impact_gpu::{bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice};
use impact_texture::gpu_resource::{SamplerMap, TextureMap};
use impact_voxel::{
    VoxelObjectID, VoxelObjectManager,
    resource::{VoxelMaterialGPUResourceManager, VoxelObjectGPUBufferManager},
    voxel_types::VoxelTypeRegistry,
};
use parking_lot::Mutex;
use std::hash::Hash;

/// Manager and owner of render resources representing world data.
///
/// The GPU buffers will at any one time be in one of two states;
/// either in sync or out of sync with the source data. When in sync,
/// the [`synchronized`](Self::synchronized) method can be called to
/// obtain a [`SynchronizedRenderResources`] that enables lock free read
/// access to the render resources. When the source data changes,
/// the render resources should be marked as out of sync by calling the
/// [`declare_desynchronized`](Self::declare_desynchronized) method.
/// Access to the render resources is then provided through internal
/// mechanisms that wrap the resources and provide methods for
/// re-synchronizing the render resources with the source data.
/// When synchronization is complete, the `synchronized` method
/// becomes available again.
#[derive(Debug)]
pub struct RenderResourceManager {
    synchronized_resources: Option<SynchronizedRenderResources>,
    desynchronized_resources: Option<DesynchronizedRenderResources>,
}

/// Wrapper providing lock free access to render resources that
/// are assumed to be in sync with the source data.
#[derive(Debug)]
pub struct SynchronizedRenderResources {
    voxel_resource_managers: Box<(
        Option<VoxelMaterialGPUResourceManager>,
        VoxelObjectGPUBufferManagerMap,
    )>,
}

/// Wrapper for render resources that are assumed to be out of sync
/// with the source data. The resources are protected by locks,
/// enabling concurrent re-synchronization of the resources.
#[derive(Debug)]
pub struct DesynchronizedRenderResources {
    pub voxel_resource_managers: Mutex<
        Box<(
            Option<VoxelMaterialGPUResourceManager>,
            VoxelObjectGPUBufferManagerMap,
        )>,
    >,
}

type VoxelObjectGPUBufferManagerMap = HashMap<VoxelObjectID, VoxelObjectGPUBufferManager>;

impl RenderResourceManager {
    /// Creates a new render resource manager with resources that
    /// are not synchronized with any world data.
    pub fn new() -> Self {
        Self {
            synchronized_resources: None,
            desynchronized_resources: Some(DesynchronizedRenderResources::new()),
        }
    }

    /// Whether the render resources are marked as being out of sync
    /// with the source data.
    pub fn is_desynchronized(&self) -> bool {
        self.desynchronized_resources.is_some()
    }

    /// Returns a reference to the render resources wrapped in
    /// a [`SynchronizedRenderResources`], providing lock free
    /// read access to the resources.
    ///
    /// # Panics
    /// If the render resources are not assumed to be synchronized
    /// (as a result of calling
    /// [`declare_desynchronized`](Self::declare_desynchronized)).
    pub fn synchronized(&self) -> &SynchronizedRenderResources {
        self.synchronized_resources
            .as_ref()
            .expect("Attempted to access synchronized render resources when out of sync")
    }

    /// Marks the render resources as being out of sync with the
    /// source data.
    pub fn declare_desynchronized(&mut self) {
        if self.desynchronized_resources.is_none() {
            self.desynchronized_resources = Some(DesynchronizedRenderResources::from_synchronized(
                self.synchronized_resources.take().unwrap(),
            ));
        }
    }

    /// Returns a reference to the render resources wrapped in
    /// a [`DesynchronizedRenderResources`], providing lock guarded
    /// access to the resources.
    ///
    /// # Panics
    /// If the render resources are not assumed to be desynchronized
    /// (as a result of calling
    /// [`declare_synchronized`](Self::declare_synchronized)).
    pub fn desynchronized(&self) -> &DesynchronizedRenderResources {
        self.desynchronized_resources
            .as_ref()
            .expect("Attempted to access desynchronized render resources when in sync")
    }

    /// Marks all the render resources as being in sync with the
    /// source data.
    pub fn declare_synchronized(&mut self) {
        if self.synchronized_resources.is_none() {
            self.synchronized_resources = Some(
                self.desynchronized_resources
                    .take()
                    .unwrap()
                    .into_synchronized(),
            );
        }
    }
}

impl Default for RenderResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SynchronizedRenderResources {
    pub fn get_voxel_material_resource_manager(&self) -> Option<&VoxelMaterialGPUResourceManager> {
        self.voxel_resource_managers.0.as_ref()
    }

    pub fn voxel_object_buffer_managers(&self) -> &VoxelObjectGPUBufferManagerMap {
        &self.voxel_resource_managers.1
    }
}

impl DesynchronizedRenderResources {
    fn new() -> Self {
        Self {
            voxel_resource_managers: Mutex::new(Box::default()),
        }
    }

    fn from_synchronized(render_resources: SynchronizedRenderResources) -> Self {
        let SynchronizedRenderResources {
            voxel_resource_managers,
        } = render_resources;
        Self {
            voxel_resource_managers: Mutex::new(voxel_resource_managers),
        }
    }

    fn into_synchronized(self) -> SynchronizedRenderResources {
        let DesynchronizedRenderResources {
            voxel_resource_managers,
        } = self;
        SynchronizedRenderResources {
            voxel_resource_managers: voxel_resource_managers.into_inner(),
        }
    }

    /// Performs any required updates for keeping the given voxel GPU resources
    /// in sync with the given voxel type registry and voxel object manager.
    ///
    /// GPU buffers whose source data no longer exists will be removed, and
    /// missing GPU buffers for new source data will be created.
    pub fn sync_voxel_resources(
        graphics_device: &GraphicsDevice,
        textures: &TextureMap,
        samplers: &SamplerMap,
        voxel_type_registry: &VoxelTypeRegistry,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        (voxel_material_resource_manager, voxel_object_buffer_managers): &mut (
            Option<VoxelMaterialGPUResourceManager>,
            VoxelObjectGPUBufferManagerMap,
        ),
        voxel_object_manager: &mut VoxelObjectManager,
    ) -> Result<()> {
        if !voxel_object_manager.voxel_objects().is_empty()
            && voxel_type_registry.n_voxel_types() > 0
            && voxel_material_resource_manager.is_none()
        {
            *voxel_material_resource_manager =
                Some(VoxelMaterialGPUResourceManager::for_voxel_type_registry(
                    graphics_device,
                    textures,
                    samplers,
                    voxel_type_registry,
                    bind_group_layout_registry,
                )?);
        }

        for (voxel_object_id, voxel_object) in voxel_object_manager.voxel_objects_mut() {
            voxel_object_buffer_managers
                .entry(*voxel_object_id)
                .and_modify(|manager| {
                    manager.sync_with_voxel_object(
                        graphics_device,
                        voxel_object,
                        bind_group_layout_registry,
                    );
                })
                .or_insert_with(|| {
                    VoxelObjectGPUBufferManager::for_voxel_object(
                        graphics_device,
                        *voxel_object_id,
                        voxel_object,
                        bind_group_layout_registry,
                    )
                });
        }
        Self::remove_unmatched_render_resources(
            voxel_object_buffer_managers,
            voxel_object_manager.voxel_objects(),
        );

        Ok(())
    }

    /// Removes render resources whose source data is no longer present.
    fn remove_unmatched_render_resources<K, T, U>(
        render_resources: &mut HashMap<K, T>,
        source_data: &HashMap<K, U>,
    ) where
        K: Eq + Hash,
    {
        render_resources.retain(|id, _| source_data.contains_key(id));
    }
}
