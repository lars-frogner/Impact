//! Synchronization of GPU buffers with geometrical data.

pub mod tasks;

use crate::{
    assets::Assets,
    camera::{SceneCamera, buffer::CameraGPUBufferManager},
    gpu::{GraphicsDevice, rendering::ShadowMappingConfig},
    light::{LightStorage, buffer::LightGPUBufferManager},
    mesh::{
        MeshID, buffer::MeshGPUBufferManager, line_segment::LineSegmentMesh, triangle::TriangleMesh,
    },
    model::{InstanceFeatureManager, ModelID, buffer::InstanceFeatureGPUBufferManager},
    skybox::{Skybox, resource::SkyboxGPUResourceManager},
    voxel::{
        VoxelManager, VoxelObjectID,
        resource::{VoxelMaterialGPUResourceManager, VoxelObjectGPUBufferManager},
    },
};
use anyhow::Result;
use impact_containers::HashMap;
use std::{
    borrow::Cow,
    collections::hash_map::Entry,
    hash::Hash,
    sync::{Mutex, RwLock},
};

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
    camera_buffer_manager: Box<Option<CameraGPUBufferManager>>,
    skybox_resource_manager: Box<Option<SkyboxGPUResourceManager>>,
    triangle_mesh_buffer_managers: Box<MeshGPUBufferManagerMap>,
    line_segment_mesh_buffer_managers: Box<MeshGPUBufferManagerMap>,
    voxel_resource_managers: Box<(
        Option<VoxelMaterialGPUResourceManager>,
        VoxelObjectGPUBufferManagerMap,
    )>,
    light_buffer_manager: Box<Option<LightGPUBufferManager>>,
    instance_feature_buffer_managers: Box<InstanceFeatureGPUBufferManagerMap>,
}

/// Wrapper for render resources that are assumed to be out of sync
/// with the source data. The resources are protected by locks,
/// enabling concurrent re-synchronization of the resources.
#[derive(Debug)]
struct DesynchronizedRenderResources {
    camera_buffer_manager: Mutex<Box<Option<CameraGPUBufferManager>>>,
    skybox_resource_manager: Mutex<Box<Option<SkyboxGPUResourceManager>>>,
    triangle_mesh_buffer_managers: Mutex<Box<MeshGPUBufferManagerMap>>,
    line_segment_mesh_buffer_managers: Mutex<Box<MeshGPUBufferManagerMap>>,
    voxel_resource_managers: Mutex<
        Box<(
            Option<VoxelMaterialGPUResourceManager>,
            VoxelObjectGPUBufferManagerMap,
        )>,
    >,
    light_buffer_manager: Mutex<Box<Option<LightGPUBufferManager>>>,
    instance_feature_buffer_managers: Mutex<Box<InstanceFeatureGPUBufferManagerMap>>,
}

type MeshGPUBufferManagerMap = HashMap<MeshID, MeshGPUBufferManager>;
type VoxelObjectGPUBufferManagerMap = HashMap<VoxelObjectID, VoxelObjectGPUBufferManager>;
type InstanceFeatureGPUBufferManagerMap = HashMap<ModelID, Vec<InstanceFeatureGPUBufferManager>>;

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
    fn desynchronized(&self) -> &DesynchronizedRenderResources {
        self.desynchronized_resources
            .as_ref()
            .expect("Attempted to access desynchronized render resources when in sync")
    }

    /// Marks all the render resources as being in sync with the
    /// source data.
    fn declare_synchronized(&mut self) {
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
    /// Returns the GPU buffer manager for camera data, or [`None`] if it has
    /// not been created.
    pub fn get_camera_buffer_manager(&self) -> Option<&CameraGPUBufferManager> {
        self.camera_buffer_manager.as_ref().as_ref()
    }

    /// Returns the GPU resource manager for skybox data, or [`None`] if it has
    /// not been created.
    pub fn get_skybox_resource_manager(&self) -> Option<&SkyboxGPUResourceManager> {
        self.skybox_resource_manager.as_ref().as_ref()
    }

    /// Returns the GPU buffer manager for the given triangle mesh identifier if
    /// the triangle mesh exists, otherwise returns [`None`].
    pub fn get_triangle_mesh_buffer_manager(
        &self,
        mesh_id: MeshID,
    ) -> Option<&MeshGPUBufferManager> {
        self.triangle_mesh_buffer_managers.get(&mesh_id)
    }

    /// Returns the GPU buffer manager for the given line segment mesh
    /// identifier if the line segment mesh exists, otherwise returns [`None`].
    pub fn get_line_segment_mesh_buffer_manager(
        &self,
        mesh_id: MeshID,
    ) -> Option<&MeshGPUBufferManager> {
        self.line_segment_mesh_buffer_managers.get(&mesh_id)
    }

    /// Returns the GPU resource manager for voxel materials, or [`None`] if it
    /// has not been initialized.
    pub fn get_voxel_material_resource_manager(&self) -> Option<&VoxelMaterialGPUResourceManager> {
        self.voxel_resource_managers.0.as_ref()
    }

    /// Returns the GPU buffer manager for the given voxel object identifier if
    /// the voxel object exists, otherwise returns [`None`].
    pub fn get_voxel_object_buffer_manager(
        &self,
        voxel_object_id: VoxelObjectID,
    ) -> Option<&VoxelObjectGPUBufferManager> {
        self.voxel_resource_managers.1.get(&voxel_object_id)
    }

    /// Returns the GPU buffer manager for light data, or [`None`] if it has
    /// not been created.
    pub fn get_light_buffer_manager(&self) -> Option<&LightGPUBufferManager> {
        self.light_buffer_manager.as_ref().as_ref()
    }

    /// Returns the instance feature GPU buffer managers for the given model
    /// identifier if the model exists, otherwise returns [`None`].
    pub fn get_instance_feature_buffer_managers(
        &self,
        model_id: &ModelID,
    ) -> Option<&Vec<InstanceFeatureGPUBufferManager>> {
        self.instance_feature_buffer_managers.get(model_id)
    }

    /// Returns a reference to the map of instance feature GPU buffer managers.
    pub fn instance_feature_buffer_managers(&self) -> &InstanceFeatureGPUBufferManagerMap {
        self.instance_feature_buffer_managers.as_ref()
    }

    /// Returns a reference to the map of voxel object GPU buffer managers.
    pub fn voxel_object_buffer_managers(&self) -> &VoxelObjectGPUBufferManagerMap {
        &self.voxel_resource_managers.1
    }
}

impl DesynchronizedRenderResources {
    fn new() -> Self {
        Self {
            camera_buffer_manager: Mutex::new(Box::new(None)),
            skybox_resource_manager: Mutex::new(Box::new(None)),
            triangle_mesh_buffer_managers: Mutex::new(Box::default()),
            line_segment_mesh_buffer_managers: Mutex::new(Box::default()),
            voxel_resource_managers: Mutex::new(Box::default()),
            light_buffer_manager: Mutex::new(Box::new(None)),
            instance_feature_buffer_managers: Mutex::new(Box::default()),
        }
    }

    fn from_synchronized(render_resources: SynchronizedRenderResources) -> Self {
        let SynchronizedRenderResources {
            camera_buffer_manager,
            skybox_resource_manager,
            triangle_mesh_buffer_managers,
            line_segment_mesh_buffer_managers,
            voxel_resource_managers,
            light_buffer_manager,
            instance_feature_buffer_managers,
        } = render_resources;
        Self {
            camera_buffer_manager: Mutex::new(camera_buffer_manager),
            skybox_resource_manager: Mutex::new(skybox_resource_manager),
            triangle_mesh_buffer_managers: Mutex::new(triangle_mesh_buffer_managers),
            line_segment_mesh_buffer_managers: Mutex::new(line_segment_mesh_buffer_managers),
            voxel_resource_managers: Mutex::new(voxel_resource_managers),
            light_buffer_manager: Mutex::new(light_buffer_manager),
            instance_feature_buffer_managers: Mutex::new(instance_feature_buffer_managers),
        }
    }

    fn into_synchronized(self) -> SynchronizedRenderResources {
        let DesynchronizedRenderResources {
            camera_buffer_manager,
            skybox_resource_manager,
            triangle_mesh_buffer_managers,
            line_segment_mesh_buffer_managers,
            voxel_resource_managers,
            light_buffer_manager,
            instance_feature_buffer_managers,
        } = self;
        SynchronizedRenderResources {
            camera_buffer_manager: camera_buffer_manager.into_inner().unwrap(),
            skybox_resource_manager: skybox_resource_manager.into_inner().unwrap(),
            triangle_mesh_buffer_managers: triangle_mesh_buffer_managers.into_inner().unwrap(),
            line_segment_mesh_buffer_managers: line_segment_mesh_buffer_managers
                .into_inner()
                .unwrap(),
            voxel_resource_managers: voxel_resource_managers.into_inner().unwrap(),
            light_buffer_manager: light_buffer_manager.into_inner().unwrap(),
            instance_feature_buffer_managers: instance_feature_buffer_managers
                .into_inner()
                .unwrap(),
        }
    }

    /// Performs any required updates for keeping the camera data in the given
    /// GPU buffer manager in sync with the given scene camera.
    fn sync_camera_buffer_with_scene_camera(
        graphics_device: &GraphicsDevice,
        camera_buffer_manager: &mut Option<CameraGPUBufferManager>,
        scene_camera: Option<&SceneCamera<f32>>,
    ) {
        if let Some(scene_camera) = scene_camera {
            if let Some(camera_buffer_manager) = camera_buffer_manager {
                camera_buffer_manager.sync_with_camera(graphics_device, scene_camera);
            } else {
                // We initialize the camera GPU buffer manager the first time this
                // method is called
                *camera_buffer_manager = Some(CameraGPUBufferManager::for_camera(
                    graphics_device,
                    scene_camera,
                ));
            }
        } else {
            camera_buffer_manager.take();
        }
    }

    /// Performs any required updates for keeping the skybox data in the given
    /// GPU resource manager in sync with the given scene skybox.
    fn sync_skybox_resources_with_scene_skybox(
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        skybox_resource_manager: &mut Option<SkyboxGPUResourceManager>,
        skybox: Option<&Skybox>,
    ) -> Result<()> {
        if let Some(&skybox) = skybox {
            if let Some(skybox_resource_manager) = skybox_resource_manager {
                skybox_resource_manager.sync_with_skybox(graphics_device, assets, skybox)?;
            } else {
                // We initialize the skybox resource manager the first time this
                // method is called
                *skybox_resource_manager = Some(SkyboxGPUResourceManager::for_skybox(
                    graphics_device,
                    assets,
                    skybox,
                )?);
            }
        } else {
            skybox_resource_manager.take();
        }
        Ok(())
    }

    /// Performs any required updates for keeping the given map of triangle mesh
    /// GPU buffers in sync with the given map of triangle meshes.
    ///
    /// GPU buffers whose source data no longer exists will be removed, and
    /// missing GPU buffers for new source data will be created.
    fn sync_triangle_mesh_buffers_with_triangle_meshes(
        graphics_device: &GraphicsDevice,
        triangle_mesh_gpu_buffers: &mut MeshGPUBufferManagerMap,
        triangle_meshes: &HashMap<MeshID, TriangleMesh<f32>>,
    ) {
        for (&mesh_id, mesh) in triangle_meshes {
            triangle_mesh_gpu_buffers
                .entry(mesh_id)
                .and_modify(|mesh_buffers| {
                    mesh_buffers.sync_with_triangle_mesh(graphics_device, mesh);
                })
                .or_insert_with(|| {
                    MeshGPUBufferManager::for_triangle_mesh(graphics_device, mesh_id, mesh)
                });
        }
        Self::remove_unmatched_render_resources(triangle_mesh_gpu_buffers, triangle_meshes);
    }

    /// Performs any required updates for keeping the given map of line segment
    /// mesh GPU buffers in sync with the given map of line segment meshes.
    ///
    /// GPU buffers whose source data no longer exists will be removed, and
    /// missing GPU buffers for new source data will be created.
    fn sync_line_segment_mesh_buffers_with_line_segment_meshes(
        graphics_device: &GraphicsDevice,
        line_segment_mesh_gpu_buffers: &mut MeshGPUBufferManagerMap,
        line_segment_meshes: &HashMap<MeshID, LineSegmentMesh<f32>>,
    ) {
        for (&mesh_id, mesh) in line_segment_meshes {
            line_segment_mesh_gpu_buffers
                .entry(mesh_id)
                .and_modify(|mesh_buffers| {
                    mesh_buffers.sync_with_line_segment_mesh(graphics_device, mesh);
                })
                .or_insert_with(|| {
                    MeshGPUBufferManager::for_line_segment_mesh(graphics_device, mesh_id, mesh)
                });
        }
        Self::remove_unmatched_render_resources(line_segment_mesh_gpu_buffers, line_segment_meshes);
    }

    /// Performs any required updates for keeping the given voxel GPU resources
    /// in sync with the given voxel manager.
    ///
    /// GPU buffers whose source data no longer exists will be removed, and
    /// missing GPU buffers for new source data will be created.
    fn sync_voxel_resources_with_voxel_manager(
        graphics_device: &GraphicsDevice,
        assets: &RwLock<Assets>,
        (voxel_material_resource_manager, voxel_object_buffer_managers): &mut (
            Option<VoxelMaterialGPUResourceManager>,
            VoxelObjectGPUBufferManagerMap,
        ),
        voxel_manager: &mut VoxelManager,
    ) -> Result<()> {
        let voxel_object_manager = &mut voxel_manager.object_manager;

        if !voxel_object_manager.voxel_objects().is_empty()
            && voxel_material_resource_manager.is_none()
        {
            *voxel_material_resource_manager =
                Some(VoxelMaterialGPUResourceManager::for_voxel_type_registry(
                    graphics_device,
                    &mut assets.write().unwrap(),
                    &voxel_manager.type_registry,
                )?);
        }

        for (voxel_object_id, voxel_object) in voxel_object_manager.voxel_objects_mut() {
            voxel_object_buffer_managers
                .entry(*voxel_object_id)
                .and_modify(|manager| manager.sync_with_voxel_object(graphics_device, voxel_object))
                .or_insert_with(|| {
                    VoxelObjectGPUBufferManager::for_voxel_object(
                        graphics_device,
                        *voxel_object_id,
                        voxel_object,
                    )
                });
        }
        Self::remove_unmatched_render_resources(
            voxel_object_buffer_managers,
            voxel_object_manager.voxel_objects(),
        );

        Ok(())
    }

    /// Performs any required updates for keeping the lights in the given render
    /// buffer manager in sync with the lights in the given light storage.
    fn sync_light_buffers_with_light_storage(
        graphics_device: &GraphicsDevice,
        light_buffer_manager: &mut Option<LightGPUBufferManager>,
        light_storage: &LightStorage,
        shadow_mapping_config: &ShadowMappingConfig,
    ) {
        if let Some(light_buffer_manager) = light_buffer_manager {
            light_buffer_manager.sync_with_light_storage(graphics_device, light_storage);
        } else {
            // We initialize the light GPU buffer manager the first time this
            // method is called
            *light_buffer_manager = Some(LightGPUBufferManager::for_light_storage(
                graphics_device,
                light_storage,
                shadow_mapping_config,
            ));
        }
    }

    /// Performs any required updates for keeping the given map of
    /// model instance feature GPU buffers in sync with the data
    /// of the given instance feature manager.
    ///
    /// GPU buffers whose source data no longer
    /// exists will be removed, and missing GPU buffers
    /// for new source data will be created.
    fn sync_instance_feature_buffers_with_manager(
        graphics_device: &GraphicsDevice,
        feature_gpu_buffer_managers: &mut InstanceFeatureGPUBufferManagerMap,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        for (model_id, model_instance_buffer) in
            instance_feature_manager.model_ids_and_mutable_instance_buffers()
        {
            match feature_gpu_buffer_managers.entry(*model_id) {
                Entry::Occupied(mut occupied_entry) => {
                    let feature_gpu_buffer_managers = occupied_entry.get_mut();

                    model_instance_buffer.copy_buffered_instance_features_to_gpu_buffers(
                        graphics_device,
                        feature_gpu_buffer_managers,
                    );
                }
                Entry::Vacant(vacant_entry) => {
                    let feature_gpu_buffer_managers = model_instance_buffer
                        .copy_buffered_instance_features_to_new_gpu_buffers(
                            graphics_device,
                            Cow::Owned(model_id.to_string()),
                        );

                    vacant_entry.insert(feature_gpu_buffer_managers);
                }
            }
        }
        feature_gpu_buffer_managers
            .retain(|model_id, _| instance_feature_manager.has_model_id(model_id));
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
