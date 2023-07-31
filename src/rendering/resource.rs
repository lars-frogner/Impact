//! Synchronization of render buffers with geometrical data.

mod tasks;

pub use tasks::SyncRenderResources;

use crate::{
    geometry::TriangleMesh,
    rendering::{
        camera::CameraRenderBufferManager, fre, instance::InstanceFeatureRenderBufferManager,
        light::LightRenderBufferManager, mesh::MeshRenderBufferManager, Assets,
        CoreRenderingSystem, MaterialPropertyTextureManager, MaterialRenderResourceManager,
        RenderingConfig,
    },
    scene::{
        InstanceFeatureManager, LightStorage, MaterialID, MaterialLibrary,
        MaterialPropertyTextureSetID, MeshID, ModelID, SceneCamera,
    },
};
use anyhow::Result;
use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
    sync::Mutex,
};

/// Manager and owner of render resources representing world data.
///
/// The render buffers will at any one time be in one of two states;
/// either in sync or out of sync with the source data. When in sync,
/// the [`synchronized`](Self::synchronized) method can be called to
/// obtain a [`SynchronizedRenderResources`] that enables lock free read
/// access to the render resources. When the source data changes,
/// the render resources should be marked as out of sync by calling the
/// [`declare_desynchronized`](Self::declare_desynchronized) method.
/// Access to the render resources is now only provided by the private
/// [`desynchronized`](Self::desynchronized) method, which returns
/// a [`DesynchronizedRenderResources`] that [`Mutex`]-wraps the resources
/// and provides methods for re-synchronizing the render resources with
/// the source data. When this is done, the
/// [`declare_synchronized`](Self::declare_synchronized) method can
/// be called to enable the `synchronized` method again.
#[derive(Debug)]
pub struct RenderResourceManager {
    synchronized_resources: Option<SynchronizedRenderResources>,
    desynchronized_resources: Option<DesynchronizedRenderResources>,
}

/// Wrapper providing lock free access to render resources that
/// are assumed to be in sync with the source data.
#[derive(Debug)]
pub struct SynchronizedRenderResources {
    camera_buffer_manager: Box<Option<CameraRenderBufferManager>>,
    mesh_buffer_managers: Box<MeshRenderBufferManagerMap>,
    light_buffer_manager: Box<Option<LightRenderBufferManager>>,
    material_resource_managers: Box<MaterialResourceManagerMap>,
    material_property_texture_managers: Box<MaterialPropertyTextureManagerMap>,
    instance_feature_buffer_managers: Box<InstanceFeatureRenderBufferManagerMap>,
}

/// Wrapper for render resources that are assumed to be out of sync
/// with the source data. The resources are protected by locks,
/// enabling concurrent re-synchronization of the resources.
#[derive(Debug)]
struct DesynchronizedRenderResources {
    camera_buffer_manager: Mutex<Box<Option<CameraRenderBufferManager>>>,
    mesh_buffer_managers: Mutex<Box<MeshRenderBufferManagerMap>>,
    light_buffer_manager: Mutex<Box<Option<LightRenderBufferManager>>>,
    material_resource_managers: Mutex<Box<MaterialResourceManagerMap>>,
    material_property_texture_managers: Mutex<Box<MaterialPropertyTextureManagerMap>>,
    instance_feature_buffer_managers: Mutex<Box<InstanceFeatureRenderBufferManagerMap>>,
}

type MeshRenderBufferManagerMap = HashMap<MeshID, MeshRenderBufferManager>;
type MaterialResourceManagerMap = HashMap<MaterialID, MaterialRenderResourceManager>;
type MaterialPropertyTextureManagerMap =
    HashMap<MaterialPropertyTextureSetID, MaterialPropertyTextureManager>;
type InstanceFeatureRenderBufferManagerMap =
    HashMap<ModelID, Vec<InstanceFeatureRenderBufferManager>>;

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
    /// Returns the render buffer manager for camera data, or [`None`] if it has
    /// not been created.
    pub fn get_camera_buffer_manager(&self) -> Option<&CameraRenderBufferManager> {
        self.camera_buffer_manager.as_ref().as_ref()
    }

    /// Returns the render buffer manager for the given mesh identifier if the
    /// mesh exists, otherwise returns [`None`].
    pub fn get_mesh_buffer_manager(&self, mesh_id: MeshID) -> Option<&MeshRenderBufferManager> {
        self.mesh_buffer_managers.get(&mesh_id)
    }

    /// Returns the render buffer manager for light data, or [`None`] if it has
    /// not been created.
    pub fn get_light_buffer_manager(&self) -> Option<&LightRenderBufferManager> {
        self.light_buffer_manager.as_ref().as_ref()
    }

    /// Returns the render resource manager for the given material identifier
    /// if the material exists, otherwise returns [`None`].
    pub fn get_material_resource_manager(
        &self,
        material_id: MaterialID,
    ) -> Option<&MaterialRenderResourceManager> {
        self.material_resource_managers.get(&material_id)
    }

    /// Returns the manager of the material property texture set with the given
    /// identifier if it exists, otherwise returns [`None`].
    pub fn get_material_property_texture_manager(
        &self,
        material_property_texture_set_id: MaterialPropertyTextureSetID,
    ) -> Option<&MaterialPropertyTextureManager> {
        self.material_property_texture_managers
            .get(&material_property_texture_set_id)
    }

    /// Returns the instance feature render buffer managers for the given model
    /// identifier if the model exists, otherwise returns [`None`].
    pub fn get_instance_feature_buffer_managers(
        &self,
        model_id: ModelID,
    ) -> Option<&Vec<InstanceFeatureRenderBufferManager>> {
        self.instance_feature_buffer_managers.get(&model_id)
    }

    /// Returns a reference to the map of instance feature render buffer managers.
    pub fn instance_feature_buffer_managers(&self) -> &InstanceFeatureRenderBufferManagerMap {
        self.instance_feature_buffer_managers.as_ref()
    }
}

impl DesynchronizedRenderResources {
    fn new() -> Self {
        Self {
            camera_buffer_manager: Mutex::new(Box::new(None)),
            mesh_buffer_managers: Mutex::new(Box::default()),
            material_resource_managers: Mutex::new(Box::default()),
            material_property_texture_managers: Mutex::new(Box::default()),
            light_buffer_manager: Mutex::new(Box::new(None)),
            instance_feature_buffer_managers: Mutex::new(Box::default()),
        }
    }

    fn from_synchronized(render_resources: SynchronizedRenderResources) -> Self {
        let SynchronizedRenderResources {
            camera_buffer_manager,
            mesh_buffer_managers,
            light_buffer_manager,
            material_resource_managers,
            material_property_texture_managers,
            instance_feature_buffer_managers,
        } = render_resources;
        Self {
            camera_buffer_manager: Mutex::new(camera_buffer_manager),
            mesh_buffer_managers: Mutex::new(mesh_buffer_managers),
            light_buffer_manager: Mutex::new(light_buffer_manager),
            material_resource_managers: Mutex::new(material_resource_managers),
            material_property_texture_managers: Mutex::new(material_property_texture_managers),
            instance_feature_buffer_managers: Mutex::new(instance_feature_buffer_managers),
        }
    }

    fn into_synchronized(self) -> SynchronizedRenderResources {
        let DesynchronizedRenderResources {
            camera_buffer_manager,
            mesh_buffer_managers,
            light_buffer_manager,
            material_resource_managers,
            material_property_texture_managers,
            instance_feature_buffer_managers,
        } = self;
        SynchronizedRenderResources {
            camera_buffer_manager: camera_buffer_manager.into_inner().unwrap(),
            mesh_buffer_managers: mesh_buffer_managers.into_inner().unwrap(),
            light_buffer_manager: light_buffer_manager.into_inner().unwrap(),
            material_resource_managers: material_resource_managers.into_inner().unwrap(),
            material_property_texture_managers: material_property_texture_managers
                .into_inner()
                .unwrap(),
            instance_feature_buffer_managers: instance_feature_buffer_managers
                .into_inner()
                .unwrap(),
        }
    }

    /// Performs any required updates for keeping the camera data in the given
    /// render buffer manager in sync with the given scene camera.
    fn sync_camera_buffer_with_scene_camera(
        core_system: &CoreRenderingSystem,
        camera_buffer_manager: &mut Option<CameraRenderBufferManager>,
        scene_camera: &SceneCamera<fre>,
    ) {
        if let Some(camera_buffer_manager) = camera_buffer_manager {
            camera_buffer_manager.sync_with_camera(core_system, scene_camera.camera());
        } else {
            // We initialize the camera render buffer manager the first time this
            // method is called
            *camera_buffer_manager = Some(CameraRenderBufferManager::for_camera(
                core_system,
                scene_camera.camera(),
            ));
        }
    }

    /// Performs any required updates for keeping the given map
    /// of mesh render buffers in sync with the given map of
    /// meshes.
    ///
    /// Render buffers whose source data no longer
    /// exists will be removed, and missing render buffers
    /// for new source data will be created.
    fn sync_mesh_buffers_with_meshes(
        core_system: &CoreRenderingSystem,
        mesh_render_buffers: &mut MeshRenderBufferManagerMap,
        meshes: &HashMap<MeshID, TriangleMesh<fre>>,
    ) {
        for (&mesh_id, mesh) in meshes {
            mesh_render_buffers
                .entry(mesh_id)
                .and_modify(|mesh_buffers| mesh_buffers.sync_with_mesh(core_system, mesh))
                .or_insert_with(|| MeshRenderBufferManager::for_mesh(core_system, mesh_id, mesh));
        }
        Self::remove_unmatched_render_resources(mesh_render_buffers, meshes);
    }

    /// Performs any required updates for keeping the lights in the given render
    /// buffer manager in sync with the lights in the given light storage.
    fn sync_light_buffers_with_light_storage(
        core_system: &CoreRenderingSystem,
        light_buffer_manager: &mut Option<LightRenderBufferManager>,
        light_storage: &LightStorage,
        config: &RenderingConfig,
    ) {
        if let Some(light_buffer_manager) = light_buffer_manager {
            light_buffer_manager.sync_with_light_storage(core_system, light_storage);
        } else {
            // We initialize the light render buffer manager the first time this
            // method is called
            *light_buffer_manager = Some(LightRenderBufferManager::for_light_storage(
                core_system,
                light_storage,
                config,
            ));
        }
    }

    /// Performs any required updates for keeping the given map of material
    /// render resource managers in sync with the material specifications in the
    /// given material library.
    ///
    /// Render resources whose source data no longer exists will be removed, and
    /// missing render resources for new source data will be created.
    fn sync_material_resources_with_material_library(
        core_system: &CoreRenderingSystem,
        material_resource_managers: &mut MaterialResourceManagerMap,
        material_library: &MaterialLibrary,
    ) {
        for (&material_id, material_specification) in material_library.material_specifications() {
            if let Entry::Vacant(entry) = material_resource_managers.entry(material_id) {
                entry.insert(MaterialRenderResourceManager::for_material_specification(
                    core_system,
                    material_specification,
                    material_id.to_string(),
                ));
            };
        }
        Self::remove_unmatched_render_resources(
            material_resource_managers,
            material_library.material_specifications(),
        );
    }

    /// Performs any required updates for keeping the given map of material
    /// property texture managers in sync with the material property texture
    /// sets in the given material library.
    ///
    /// Render resources whose source data no longer exists will be removed, and
    /// missing render resources for new source data will be created.
    fn sync_material_property_textures_with_material_library(
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        material_property_texture_managers: &mut MaterialPropertyTextureManagerMap,
        material_library: &MaterialLibrary,
    ) -> Result<()> {
        for (&texture_set_id, texture_set) in material_library.material_property_texture_sets() {
            if let Entry::Vacant(entry) = material_property_texture_managers.entry(texture_set_id) {
                entry.insert(MaterialPropertyTextureManager::for_texture_set(
                    core_system,
                    assets,
                    texture_set,
                    texture_set_id.to_string(),
                )?);
            };
        }
        Self::remove_unmatched_render_resources(
            material_property_texture_managers,
            material_library.material_property_texture_sets(),
        );
        Ok(())
    }

    /// Performs any required updates for keeping the given map of
    /// model instance feature render buffers in sync with the data
    /// of the given instance feature manager.
    ///
    /// Render buffers whose source data no longer
    /// exists will be removed, and missing render buffers
    /// for new source data will be created.
    fn sync_instance_feature_buffers_with_manager(
        core_system: &CoreRenderingSystem,
        feature_render_buffer_managers: &mut InstanceFeatureRenderBufferManagerMap,
        instance_feature_manager: &InstanceFeatureManager,
    ) {
        for (model_id, instance_feature_buffers) in instance_feature_manager.model_ids_and_buffers()
        {
            match feature_render_buffer_managers.entry(model_id) {
                Entry::Occupied(mut occupied_entry) => {
                    let feature_render_buffer_managers = occupied_entry.get_mut();

                    for (feature_buffer, render_buffer_manager) in instance_feature_buffers
                        .iter()
                        .zip(feature_render_buffer_managers.iter_mut())
                    {
                        render_buffer_manager
                            .copy_instance_features_to_render_buffer(core_system, feature_buffer);
                        feature_buffer.clear();
                    }
                }
                Entry::Vacant(vacant_entry) => {
                    let feature_render_buffer_managers = instance_feature_buffers
                        .iter()
                        .map(|feature_buffer| {
                            let render_buffer_manager = InstanceFeatureRenderBufferManager::new(
                                core_system,
                                feature_buffer,
                                Cow::Owned(model_id.to_string()),
                            );
                            feature_buffer.clear();
                            render_buffer_manager
                        })
                        .collect();

                    vacant_entry.insert(feature_render_buffer_managers);
                }
            }
        }
        feature_render_buffer_managers
            .retain(|model_id, _| instance_feature_manager.has_model_id(*model_id));
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
