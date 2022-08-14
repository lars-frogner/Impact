//! Render buffers for geometrical data.

mod tasks;

pub use tasks::SyncRenderBuffers;

use crate::{
    geometry::{
        Camera, CameraID, CameraRepository, MeshID, MeshRepository, ModelID, ModelInstanceBuffer,
        ModelInstancePool, TriangleMesh,
    },
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
    /// Creates all render buffers required for representing the
    /// given geometrical data.
    pub fn from_geometry(
        core_system: &CoreRenderingSystem,
        camera_repository: &CameraRepository<f32>,
        mesh_repository: &MeshRepository<f32>,
        model_instance_pool: &ModelInstancePool<f32>,
    ) -> Self {
        Self {
            synchronized_buffers: Some(SynchronizedRenderBuffers::from_geometry(
                core_system,
                camera_repository,
                mesh_repository,
                model_instance_pool,
            )),
            desynchronized_buffers: None,
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

    fn from_geometry(
        core_system: &CoreRenderingSystem,
        camera_repository: &CameraRepository<f32>,
        mesh_repository: &MeshRepository<f32>,
        model_instance_pool: &ModelInstancePool<f32>,
    ) -> Self {
        let perspective_camera_buffers = Self::create_camera_render_buffers(
            core_system,
            camera_repository.perspective_cameras(),
        );

        let color_mesh_buffers =
            Self::create_mesh_render_buffers(core_system, mesh_repository.color_meshes());
        let texture_mesh_buffers =
            Self::create_mesh_render_buffers(core_system, mesh_repository.texture_meshes());

        let model_instance_buffers = Self::create_model_instance_render_buffers(
            core_system,
            &model_instance_pool.model_instance_buffers,
        );

        Self {
            perspective_camera_buffers: Box::new(perspective_camera_buffers),
            color_mesh_buffers: Box::new(color_mesh_buffers),
            texture_mesh_buffers: Box::new(texture_mesh_buffers),
            model_instance_buffers: Box::new(model_instance_buffers),
        }
    }

    fn create_camera_render_buffers(
        core_system: &CoreRenderingSystem,
        cameras: &HashMap<CameraID, impl Camera<f32>>,
    ) -> CameraRenderBufferMap {
        cameras
            .iter()
            .map(|(&id, camera)| {
                (
                    id,
                    CameraRenderBufferManager::for_camera(core_system, camera, &id.to_string()),
                )
            })
            .collect()
    }

    fn create_mesh_render_buffers(
        core_system: &CoreRenderingSystem,
        meshes: &HashMap<MeshID, TriangleMesh<impl BufferableVertex>>,
    ) -> MeshRenderBufferMap {
        meshes
            .iter()
            .map(|(&id, mesh)| {
                (
                    id,
                    MeshRenderBufferManager::for_mesh(core_system, mesh, id.to_string()),
                )
            })
            .collect()
    }

    fn create_model_instance_render_buffers(
        core_system: &CoreRenderingSystem,
        model_instance_buffers: &HashMap<ModelID, ModelInstanceBuffer<f32>>,
    ) -> ModelInstanceRenderBufferMap {
        model_instance_buffers
            .iter()
            .map(|(&id, model_instance_buffer)| {
                (
                    id,
                    ModelInstanceRenderBufferManager::new(
                        core_system,
                        model_instance_buffer,
                        id.to_string(),
                    ),
                )
            })
            .collect()
    }
}

impl DesynchronizedRenderBuffers {
    /// Performs any required updates for keeping the given camera render
    /// buffers in sync with the given cameras.
    ///
    /// # Note
    /// Render buffers whose source geometry no longer
    /// exists will be removed.
    fn sync_camera_buffers_with_geometry(
        core_system: &CoreRenderingSystem,
        camera_render_buffers: &mut CameraRenderBufferMap,
        cameras: &HashMap<CameraID, impl Camera<f32>>,
    ) {
        Self::remove_unmatched_render_buffers(camera_render_buffers, cameras);
        camera_render_buffers
            .iter_mut()
            .for_each(|(label, camera_buffer)| {
                camera_buffer.sync_with_camera(core_system, cameras.get(label).unwrap())
            });
    }

    /// Performs any required updates for keeping the given mesh render
    /// buffers in sync with the given meshes.
    ///
    /// # Note
    /// Render buffers whose source geometry no longer
    /// exists will be removed.
    fn sync_mesh_buffers_with_geometry(
        core_system: &CoreRenderingSystem,
        mesh_render_buffers: &mut MeshRenderBufferMap,
        meshes: &HashMap<MeshID, TriangleMesh<impl BufferableVertex>>,
    ) {
        Self::remove_unmatched_render_buffers(mesh_render_buffers, meshes);
        mesh_render_buffers
            .iter_mut()
            .for_each(|(label, mesh_buffers)| {
                mesh_buffers.sync_with_mesh(core_system, meshes.get(label).unwrap())
            });
    }

    /// Performs any required updates for keeping the given model
    /// instance render buffers in sync with the given model instances.
    ///
    /// # Note
    /// Render buffers whose source geometry no longer
    /// exists will be removed.
    fn sync_model_instance_buffers_with_geometry(
        core_system: &CoreRenderingSystem,
        model_instance_render_buffers: &mut ModelInstanceRenderBufferMap,
        model_instance_buffers: &HashMap<ModelID, ModelInstanceBuffer<f32>>,
    ) {
        Self::remove_unmatched_render_buffers(
            model_instance_render_buffers,
            model_instance_buffers,
        );
        model_instance_render_buffers
            .iter_mut()
            .for_each(|(label, instance_render_buffer)| {
                instance_render_buffer.transfer_model_instances_to_render_buffer(
                    core_system,
                    model_instance_buffers.get(label).unwrap(),
                )
            });
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

    /// Removes render buffers whose source geometry is no longer present.
    fn remove_unmatched_render_buffers<K, T, U>(
        render_buffers: &mut HashMap<K, T>,
        geometrical_data: &HashMap<K, U>,
    ) where
        K: Eq + Hash,
    {
        render_buffers.retain(|label, _| geometrical_data.contains_key(label));
    }
}
