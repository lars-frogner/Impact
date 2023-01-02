//! Synchronization of render buffers with geometrical data.

mod tasks;

pub use tasks::SyncRenderResources;

use crate::{
    geometry::{Camera, TriangleMesh},
    rendering::{
        buffer::BufferableVertex, camera::CameraRenderBufferManager, fre,
        mesh::MeshRenderBufferManager, model::ModelInstanceRenderBufferManager, Assets,
        CoreRenderingSystem, MaterialRenderResourceManager,
    },
    scene::{CameraID, MaterialID, MaterialSpecification, MeshID, ModelID, ModelInstancePool},
};
use anyhow::Result;
use std::{
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
    perspective_camera_buffers: Box<CameraRenderBufferMap>,
    color_mesh_buffers: Box<MeshRenderBufferMap>,
    texture_mesh_buffers: Box<MeshRenderBufferMap>,
    material_resources: Box<MaterialResourceMap>,
    model_instance_buffers: Box<ModelInstanceRenderBufferMap>,
}

/// Wrapper for render resources that are assumed to be out of sync
/// with the source data. The resources are protected by locks,
/// enabling concurrent re-synchronization of the resources.
#[derive(Debug)]
struct DesynchronizedRenderResources {
    perspective_camera_buffers: Mutex<Box<CameraRenderBufferMap>>,
    color_mesh_buffers: Mutex<Box<MeshRenderBufferMap>>,
    texture_mesh_buffers: Mutex<Box<MeshRenderBufferMap>>,
    material_resources: Mutex<Box<MaterialResourceMap>>,
    model_instance_buffers: Mutex<Box<ModelInstanceRenderBufferMap>>,
}

type CameraRenderBufferMap = HashMap<CameraID, CameraRenderBufferManager>;
type MeshRenderBufferMap = HashMap<MeshID, MeshRenderBufferManager>;
type MaterialResourceMap = HashMap<MaterialID, MaterialRenderResourceManager>;
type ModelInstanceRenderBufferMap = HashMap<ModelID, ModelInstanceRenderBufferManager>;

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
    /// Returns the render buffer manager for the given camera identifier
    /// if the camera exists, otherwise returns [`None`].
    pub fn get_camera_buffer(&self, camera_id: CameraID) -> Option<&CameraRenderBufferManager> {
        self.perspective_camera_buffers.get(&camera_id)
    }

    /// Returns the render buffer manager for the given mesh identifier
    /// if the mesh exists, otherwise returns [`None`].
    pub fn get_mesh_buffer(&self, mesh_id: MeshID) -> Option<&MeshRenderBufferManager> {
        self.color_mesh_buffers
            .get(&mesh_id)
            .or_else(|| self.texture_mesh_buffers.get(&mesh_id))
    }

    /// Returns the render resource manager for the given material identifier
    /// if the material exists, otherwise returns [`None`].
    pub fn get_material_resources(
        &self,
        material_id: MaterialID,
    ) -> Option<&MaterialRenderResourceManager> {
        self.material_resources.get(&material_id)
    }

    /// Returns the render buffer manager for the given model instance
    /// buffer if the model exists, otherwise returns [`None`].
    pub fn get_model_instance_buffer(
        &self,
        model_instance_buffer_id: ModelID,
    ) -> Option<&ModelInstanceRenderBufferManager> {
        self.model_instance_buffers.get(&model_instance_buffer_id)
    }

    /// Returns a reference to the map of model instance render buffers.
    pub fn model_instance_buffers(&self) -> &ModelInstanceRenderBufferMap {
        self.model_instance_buffers.as_ref()
    }
}

impl DesynchronizedRenderResources {
    fn new() -> Self {
        Self {
            perspective_camera_buffers: Mutex::new(Box::new(HashMap::new())),
            color_mesh_buffers: Mutex::new(Box::new(HashMap::new())),
            texture_mesh_buffers: Mutex::new(Box::new(HashMap::new())),
            material_resources: Mutex::new(Box::new(HashMap::new())),
            model_instance_buffers: Mutex::new(Box::new(HashMap::new())),
        }
    }

    fn from_synchronized(render_resources: SynchronizedRenderResources) -> Self {
        let SynchronizedRenderResources {
            perspective_camera_buffers,
            color_mesh_buffers,
            texture_mesh_buffers,
            material_resources,
            model_instance_buffers,
        } = render_resources;
        Self {
            perspective_camera_buffers: Mutex::new(perspective_camera_buffers),
            color_mesh_buffers: Mutex::new(color_mesh_buffers),
            texture_mesh_buffers: Mutex::new(texture_mesh_buffers),
            material_resources: Mutex::new(material_resources),
            model_instance_buffers: Mutex::new(model_instance_buffers),
        }
    }

    fn into_synchronized(self) -> SynchronizedRenderResources {
        let DesynchronizedRenderResources {
            perspective_camera_buffers,
            color_mesh_buffers,
            texture_mesh_buffers,
            material_resources,
            model_instance_buffers,
        } = self;
        SynchronizedRenderResources {
            perspective_camera_buffers: perspective_camera_buffers.into_inner().unwrap(),
            color_mesh_buffers: color_mesh_buffers.into_inner().unwrap(),
            texture_mesh_buffers: texture_mesh_buffers.into_inner().unwrap(),
            material_resources: material_resources.into_inner().unwrap(),
            model_instance_buffers: model_instance_buffers.into_inner().unwrap(),
        }
    }

    /// Performs any required updates for keeping the given map
    /// of camera render buffers in sync with the given map of
    /// cameras.
    ///
    /// Render buffers whose source data no longer
    /// exists will be removed, and missing render buffers
    /// for new source data will be created.
    fn sync_camera_buffers_with_cameras(
        core_system: &CoreRenderingSystem,
        camera_render_buffers: &mut CameraRenderBufferMap,
        cameras: &HashMap<CameraID, impl Camera<fre>>,
    ) {
        for (&camera_id, camera) in cameras {
            camera_render_buffers
                .entry(camera_id)
                .and_modify(|camera_buffer| camera_buffer.sync_with_camera(core_system, camera))
                .or_insert_with(|| {
                    CameraRenderBufferManager::for_camera(
                        core_system,
                        camera,
                        &camera_id.to_string(),
                    )
                });
        }
        Self::remove_unmatched_render_resources(camera_render_buffers, cameras);
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
        mesh_render_buffers: &mut MeshRenderBufferMap,
        meshes: &HashMap<MeshID, TriangleMesh<impl BufferableVertex>>,
    ) {
        for (&mesh_id, mesh) in meshes {
            mesh_render_buffers
                .entry(mesh_id)
                .and_modify(|mesh_buffers| mesh_buffers.sync_with_mesh(core_system, mesh))
                .or_insert_with(|| {
                    MeshRenderBufferManager::for_mesh(core_system, mesh, mesh_id.to_string())
                });
        }
        Self::remove_unmatched_render_resources(mesh_render_buffers, meshes);
    }

    /// Performs any required updates for keeping the given map
    /// of material render resources in sync with the given map
    /// of material specifications.
    ///
    /// Render resources whose source data no longer
    /// exists will be removed, and missing render resources
    /// for new source data will be created.
    fn sync_material_resources_with_material_specifications(
        core_system: &CoreRenderingSystem,
        assets: &Assets,
        material_resources: &mut MaterialResourceMap,
        material_specifications: &HashMap<MaterialID, MaterialSpecification>,
    ) -> Result<()> {
        for (&material_id, material_specification) in material_specifications {
            match material_resources.entry(material_id) {
                Entry::Occupied(entry) => entry.get_mut().sync_with_material_specification(
                    core_system,
                    assets,
                    material_specification,
                )?,
                Entry::Vacant(entry) => {
                    entry.insert(MaterialRenderResourceManager::for_material_specification(
                        core_system,
                        assets,
                        material_specification,
                        material_id.to_string(),
                    )?);
                }
            }
        }
        Self::remove_unmatched_render_resources(material_resources, material_specifications);
        Ok(())
    }

    /// Performs any required updates for keeping the given map
    /// of model instance render buffers in sync with the given
    /// pool of model instances buffers.
    ///
    /// Render buffers whose source data no longer
    /// exists will be removed, and missing render buffers
    /// for new source data will be created.
    fn sync_model_instance_buffers_with_instance_pool(
        core_system: &CoreRenderingSystem,
        model_instance_render_buffers: &mut ModelInstanceRenderBufferMap,
        model_instance_pool: &ModelInstancePool<fre>,
    ) {
        for (model_id, model_instance_buffer) in model_instance_pool.models_and_buffers() {
            model_instance_render_buffers
                .entry(model_id)
                .and_modify(|instance_render_buffer| {
                    instance_render_buffer.transfer_model_instances_to_render_buffer(
                        core_system,
                        model_instance_buffer,
                    )
                })
                .or_insert_with(|| {
                    ModelInstanceRenderBufferManager::new(
                        core_system,
                        model_instance_buffer,
                        model_id.to_string(),
                    )
                });
        }
        model_instance_render_buffers
            .retain(|model_id, _| model_instance_pool.has_buffer_for_model(*model_id));
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
