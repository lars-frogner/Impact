//! Synchronization of render buffers with geometrical data.

mod tasks;

pub use tasks::SyncRenderBuffers;

use crate::{
    geometry::{Camera, CameraID, MeshID, ModelID, ModelInstanceBuffer, TriangleMesh},
    rendering::{
        buffer::BufferableVertex, camera::CameraRenderBufferManager, mesh::MeshRenderBufferManager,
        model::ModelInstanceRenderBufferManager, CoreRenderingSystem,
    },
};
use std::{collections::HashMap, hash::Hash, sync::Mutex};

/// Manager and owner of render buffers for geometrical data.
///
/// The render buffers will at any one time be in one of two states;
/// either in sync or out of sync with the source geometry. When in
/// sync, the [`synchronized`](Self::synchronized) method can be called
/// to obtain a [`SynchronizedRenderBuffers`] that enables lock free read
/// access to the render buffers. When the source geometry changes,
/// the render buffers should be marked as out of sync by calling the
/// [`declare_desynchronized`](Self::declare_desynchronized) method.
/// Access to the render buffers is now only provided by the private
/// [`desynchronized`](Self::desynchronized) method, which returns
/// a [`DesynchronizedRenderBuffers`] that [`Mutex`]-wraps the buffers
/// and provides methods for re-synchronizing the render buffers with
/// the source geometry. When this is done, the
/// [`declare_synchronized`](Self::declare_synchronized) method can
/// be called to enable the `synchronized` method again.
#[derive(Debug)]
pub struct RenderBufferManager {
    synchronized_buffers: Option<SynchronizedRenderBuffers>,
    desynchronized_buffers: Option<DesynchronizedRenderBuffers>,
}

/// Wrapper providing lock free access to render buffers that
/// are assumed to be in sync with the source geometry.
#[derive(Debug)]
pub struct SynchronizedRenderBuffers {
    perspective_camera_buffers: Box<CameraRenderBufferMap>,
    color_mesh_buffers: Box<MeshRenderBufferMap>,
    texture_mesh_buffers: Box<MeshRenderBufferMap>,
    model_instance_buffers: Box<ModelInstanceRenderBufferMap>,
}

/// Wrapper for render buffers that are assumed to be out of sync
/// with the source geometry. The buffers are protected by locks,
/// enabling concurrent re-synchronization of the buffers.
#[derive(Debug)]
struct DesynchronizedRenderBuffers {
    perspective_camera_buffers: Mutex<Box<CameraRenderBufferMap>>,
    color_mesh_buffers: Mutex<Box<MeshRenderBufferMap>>,
    texture_mesh_buffers: Mutex<Box<MeshRenderBufferMap>>,
    model_instance_buffers: Mutex<Box<ModelInstanceRenderBufferMap>>,
}

type CameraRenderBufferMap = HashMap<CameraID, CameraRenderBufferManager>;
type MeshRenderBufferMap = HashMap<MeshID, MeshRenderBufferManager>;
type ModelInstanceRenderBufferMap = HashMap<ModelID, ModelInstanceRenderBufferManager>;

impl RenderBufferManager {
    /// Creates a new render buffer manager with buffers that
    /// are not synchronized with any geometry.
    pub fn new() -> Self {
        Self {
            synchronized_buffers: None,
            desynchronized_buffers: Some(DesynchronizedRenderBuffers::new()),
        }
    }

    /// Returns a reference to the render buffers wrapped in
    /// a [`SynchronizedRenderBuffers`], providing lock free
    /// read access to the buffers.
    ///
    /// # Panics
    /// If the render buffers are not assumed to be synchronized
    /// (as a result of calling
    /// [`declare_desynchronized`](Self::declare_desynchronized)).
    pub fn synchronized(&self) -> &SynchronizedRenderBuffers {
        self.synchronized_buffers
            .as_ref()
            .expect("Attempted to access synchronized render buffers when out of sync")
    }

    /// Marks the render buffers as being out of sync with the
    /// source geometry.
    pub fn declare_desynchronized(&mut self) {
        if self.desynchronized_buffers.is_none() {
            self.desynchronized_buffers = Some(DesynchronizedRenderBuffers::from_synchronized(
                self.synchronized_buffers.take().unwrap(),
            ));
        }
    }

    /// Returns a reference to the render buffers wrapped in
    /// a [`DesynchronizedRenderBuffers`], providing lock guarded
    /// access to the buffers.
    ///
    /// # Panics
    /// If the render buffers are not assumed to be desynchronized
    /// (as a result of calling
    /// [`declare_synchronized`](Self::declare_synchronized)).
    fn desynchronized(&self) -> &DesynchronizedRenderBuffers {
        self.desynchronized_buffers
            .as_ref()
            .expect("Attempted to access desynchronized render buffers when in sync")
    }

    /// Marks all the render buffers as being in sync with the
    /// source geometry.
    fn declare_synchronized(&mut self) {
        if self.synchronized_buffers.is_none() {
            self.synchronized_buffers = Some(
                self.desynchronized_buffers
                    .take()
                    .unwrap()
                    .into_synchronized(),
            );
        }
    }
}

impl Default for RenderBufferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SynchronizedRenderBuffers {
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

impl DesynchronizedRenderBuffers {
    fn new() -> Self {
        Self {
            perspective_camera_buffers: Mutex::new(Box::new(HashMap::new())),
            color_mesh_buffers: Mutex::new(Box::new(HashMap::new())),
            texture_mesh_buffers: Mutex::new(Box::new(HashMap::new())),
            model_instance_buffers: Mutex::new(Box::new(HashMap::new())),
        }
    }

    fn from_synchronized(render_buffers: SynchronizedRenderBuffers) -> Self {
        let SynchronizedRenderBuffers {
            perspective_camera_buffers,
            color_mesh_buffers,
            texture_mesh_buffers,
            model_instance_buffers,
        } = render_buffers;
        Self {
            perspective_camera_buffers: Mutex::new(perspective_camera_buffers),
            color_mesh_buffers: Mutex::new(color_mesh_buffers),
            texture_mesh_buffers: Mutex::new(texture_mesh_buffers),
            model_instance_buffers: Mutex::new(model_instance_buffers),
        }
    }

    fn into_synchronized(self) -> SynchronizedRenderBuffers {
        let DesynchronizedRenderBuffers {
            perspective_camera_buffers,
            color_mesh_buffers,
            texture_mesh_buffers,
            model_instance_buffers,
        } = self;
        SynchronizedRenderBuffers {
            perspective_camera_buffers: perspective_camera_buffers.into_inner().unwrap(),
            color_mesh_buffers: color_mesh_buffers.into_inner().unwrap(),
            texture_mesh_buffers: texture_mesh_buffers.into_inner().unwrap(),
            model_instance_buffers: model_instance_buffers.into_inner().unwrap(),
        }
    }

    /// Performs any required updates for keeping the given map
    /// of camera render buffers in sync with the given map of
    /// cameras.
    ///
    /// Render buffers whose source geometry no longer
    /// exists will be removed, and missing render buffers
    /// for new geometry will be created.
    fn sync_camera_buffers_with_geometry(
        core_system: &CoreRenderingSystem,
        camera_render_buffers: &mut CameraRenderBufferMap,
        cameras: &HashMap<CameraID, impl Camera<f32>>,
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
        Self::remove_unmatched_render_buffers(camera_render_buffers, cameras);
    }

    /// Performs any required updates for keeping the given map
    /// of mesh render buffers in sync with the given map of
    /// meshes.
    ///
    /// Render buffers whose source geometry no longer
    /// exists will be removed, and missing render buffers
    /// for new geometry will be created.
    fn sync_mesh_buffers_with_geometry(
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
        Self::remove_unmatched_render_buffers(mesh_render_buffers, meshes);
    }

    /// Performs any required updates for keeping the given map
    /// of model instance render buffers in sync with the given
    /// map of model instances.
    ///
    /// Render buffers whose source geometry no longer
    /// exists will be removed, and missing render buffers
    /// for new geometry will be created.
    fn sync_model_instance_buffers_with_geometry(
        core_system: &CoreRenderingSystem,
        model_instance_render_buffers: &mut ModelInstanceRenderBufferMap,
        model_instance_buffers: &HashMap<ModelID, ModelInstanceBuffer<f32>>,
    ) {
        for (&model_id, model_instance_buffer) in model_instance_buffers {
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
        Self::remove_unmatched_render_buffers(
            model_instance_render_buffers,
            model_instance_buffers,
        );
    }

    /// Removes render buffers whose source geometry is no longer present.
    fn remove_unmatched_render_buffers<K, T, U>(
        render_buffers: &mut HashMap<K, T>,
        geometrical_data: &HashMap<K, U>,
    ) where
        K: Eq + Hash,
    {
        render_buffers.retain(|id, _| geometrical_data.contains_key(id));
    }
}
