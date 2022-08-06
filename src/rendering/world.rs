//! Render data associated with geometrical objects.

mod tasks;

pub use tasks::SyncRenderData;

use crate::{
    geometry::{Camera, GeometricalData, GeometryID, GeometryMap, Mesh, MeshInstanceGroup},
    rendering::{
        buffer::BufferableVertex,
        camera::CameraRenderDataManager,
        mesh::{MeshInstanceRenderDataManager, MeshRenderDataManager},
        CoreRenderingSystem,
    },
};
use std::sync::Mutex;

/// Container for all render data for which there is associated
/// [`GeometricalData`].
///
/// The render data container will at any one time be in one of
/// two states; either in sync or out of sync with the associated
/// geometrical data. When in sync, the [`synchronized`](Self::synchronized)
/// method can be called to obtain a [`SynchronizedRenderData`] that
/// enables lock free read access to the render data. This can be used
/// for rendering. When the geometrical data changes, the render data
/// should be marked as out of sync by calling the
/// [`declare_desynchronized`](Self::declare_desynchronized) method.
/// Access to the render data is now only provided by the private
/// [`desynchronized`](Self::desynchronized) method, which returns
/// a [`DesynchronizedRenderData`] that [`Mutex`]-wraps the data
/// and provides methods for re-synchronizing the data with the
/// geometrical data. When this is done, the
/// [`declare_synchronized`](Self::declare_synchronized) method can
/// be called to enable the `synchronized` method again.
#[derive(Debug)]
pub struct RenderData {
    synchronized: Option<SynchronizedRenderData>,
    desynchronized: Option<DesynchronizedRenderData>,
}

/// Wrapper providing lock free access to render data that
/// is assumed to be in sync with the corresponding [`GeometricalData`].
#[derive(Debug)]
pub struct SynchronizedRenderData {
    color_mesh_data: RenderDataMap<MeshRenderDataManager>,
    texture_mesh_data: RenderDataMap<MeshRenderDataManager>,
    mesh_instance_group_data: RenderDataMap<MeshInstanceRenderDataManager>,
    perspective_camera_data: RenderDataMap<CameraRenderDataManager>,
}

/// Wrapper for render data that is assumed to be out of sync
/// with the corresponding [`GeometricalData`]. The data is
/// protected by locks, enabling concurrent re-synchronization
/// of the data.
#[derive(Debug)]
struct DesynchronizedRenderData {
    color_mesh_data: GuardedRenderDataMap<MeshRenderDataManager>,
    texture_mesh_data: GuardedRenderDataMap<MeshRenderDataManager>,
    mesh_instance_group_data: GuardedRenderDataMap<MeshInstanceRenderDataManager>,
    perspective_camera_data: GuardedRenderDataMap<CameraRenderDataManager>,
}

type RenderDataMap<T> = Box<GeometryMap<T>>;

type GuardedRenderDataMap<T> = Mutex<RenderDataMap<T>>;

impl RenderData {
    /// Creates all render data required for representing the
    /// given geometrical data.
    pub fn from_geometrical_data(
        core_system: &CoreRenderingSystem,
        geometrical_data: &GeometricalData,
    ) -> Self {
        Self {
            synchronized: Some(SynchronizedRenderData::from_geometrical_data(
                core_system,
                geometrical_data,
            )),
            desynchronized: None,
        }
    }

    /// Returns a reference to the render data wrapped in
    /// a [`SynchronizedRenderData`], providing lock free
    /// read access to the data.
    ///
    /// # Panics
    /// If the render data is not assumed to be synchronized
    /// (as a result of calling
    /// [`declare_desynchronized`](Self::declare_desynchronized)).
    pub fn synchronized(&self) -> &SynchronizedRenderData {
        self.synchronized
            .as_ref()
            .expect("Attempted to access synchronized render data when out of sync")
    }

    /// Marks the render data as being out of sync with the
    /// corresponding geometrical data.
    pub fn declare_desynchronized(&mut self) {
        if self.desynchronized.is_none() {
            self.desynchronized = Some(DesynchronizedRenderData::from_synchronized(
                self.synchronized.take().unwrap(),
            ));
        }
    }

    /// Returns a reference to the render data wrapped in
    /// a [`DesynchronizedRenderData`], providing lock guarded
    /// access to the data.
    ///
    /// # Panics
    /// If the render data is not assumed to be desynchronized
    /// (as a result of calling
    /// [`declare_synchronized`](Self::declare_synchronized)).
    fn desynchronized(&self) -> &DesynchronizedRenderData {
        self.desynchronized
            .as_ref()
            .expect("Attempted to access desynchronized render data when in sync")
    }

    /// Marks all the render data as being in sync with the
    /// corresponding geometrical data.
    fn declare_synchronized(&mut self) {
        if self.synchronized.is_none() {
            self.synchronized = Some(self.desynchronized.take().unwrap().into_synchronized());
        }
    }
}

impl SynchronizedRenderData {
    /// Returns the render data manager for the given mesh identifier
    /// if the mesh exists, otherwise returns [`None`].
    pub fn get_mesh_data(&self, mesh_id: GeometryID) -> Option<&MeshRenderDataManager> {
        self.color_mesh_data
            .get(&mesh_id)
            .or_else(|| self.texture_mesh_data.get(&mesh_id))
    }

    /// Returns the render data manager for the given mesh instance
    /// group if the group exists, otherwise returns [`None`].
    pub fn get_mesh_instance_data(
        &self,
        mesh_instance_group_id: GeometryID,
    ) -> Option<&MeshInstanceRenderDataManager> {
        self.mesh_instance_group_data.get(&mesh_instance_group_id)
    }

    /// Returns the render data manager for the given camera identifier
    /// if the camera exists, otherwise returns [`None`].
    pub fn get_camera_data(&self, camera_id: GeometryID) -> Option<&CameraRenderDataManager> {
        self.perspective_camera_data.get(&camera_id)
    }

    fn from_geometrical_data(
        core_system: &CoreRenderingSystem,
        geometrical_data: &GeometricalData,
    ) -> Self {
        let color_mesh_data =
            Self::create_mesh_render_data(core_system, &geometrical_data.color_meshes);
        let texture_mesh_data =
            Self::create_mesh_render_data(core_system, &geometrical_data.texture_meshes);

        let mesh_instance_group_data = Self::create_mesh_instance_group_render_data(
            core_system,
            &geometrical_data.mesh_instance_groups,
        );

        let perspective_camera_data =
            Self::create_camera_render_data(core_system, &geometrical_data.perspective_cameras);

        Self {
            color_mesh_data: Box::new(color_mesh_data),
            texture_mesh_data: Box::new(texture_mesh_data),
            mesh_instance_group_data: Box::new(mesh_instance_group_data),
            perspective_camera_data: Box::new(perspective_camera_data),
        }
    }

    fn create_mesh_render_data(
        core_system: &CoreRenderingSystem,
        meshes: &GeometryMap<Mesh<impl BufferableVertex>>,
    ) -> GeometryMap<MeshRenderDataManager> {
        meshes
            .iter()
            .map(|(&id, mesh)| {
                (
                    id,
                    MeshRenderDataManager::for_mesh(core_system, mesh, id.to_string()),
                )
            })
            .collect()
    }

    fn create_mesh_instance_group_render_data(
        core_system: &CoreRenderingSystem,
        mesh_instance_groups: &GeometryMap<MeshInstanceGroup<f32>>,
    ) -> GeometryMap<MeshInstanceRenderDataManager> {
        mesh_instance_groups
            .iter()
            .map(|(&id, mesh_instance_group)| {
                (
                    id,
                    MeshInstanceRenderDataManager::for_mesh_instance_group(
                        core_system,
                        mesh_instance_group,
                        id.to_string(),
                    ),
                )
            })
            .collect()
    }

    fn create_camera_render_data(
        core_system: &CoreRenderingSystem,
        cameras: &GeometryMap<impl Camera<f32>>,
    ) -> GeometryMap<CameraRenderDataManager> {
        cameras
            .iter()
            .map(|(&id, camera)| {
                (
                    id,
                    CameraRenderDataManager::for_camera(core_system, camera, &id.to_string()),
                )
            })
            .collect()
    }
}

impl DesynchronizedRenderData {
    /// Performs any required updates for keeping the given mesh render data in
    /// sync with the given geometrical data.
    ///
    /// # Note
    /// Render data entries for which the associated geometrical data no
    /// longer exists will be removed.
    fn sync_mesh_data_with_geometry(
        core_system: &CoreRenderingSystem,
        mesh_render_data: &mut GeometryMap<MeshRenderDataManager>,
        meshes: &GeometryMap<Mesh<impl BufferableVertex>>,
    ) {
        Self::remove_unmatched_render_data(mesh_render_data, meshes);
        mesh_render_data.iter_mut().for_each(|(label, mesh_data)| {
            mesh_data.sync_with_mesh(core_system, meshes.get(label).unwrap())
        });
    }

    /// Performs any required updates for keeping the given mesh instance group
    /// render data in sync with the given geometrical data.
    ///
    /// # Note
    /// Render data entries for which the associated geometrical data no
    /// longer exists will be removed.
    fn sync_mesh_instance_group_data_with_geometry(
        core_system: &CoreRenderingSystem,
        mesh_instance_group_render_data: &mut GeometryMap<MeshInstanceRenderDataManager>,
        mesh_instance_groups: &GeometryMap<MeshInstanceGroup<f32>>,
    ) {
        Self::remove_unmatched_render_data(mesh_instance_group_render_data, mesh_instance_groups);
        mesh_instance_group_render_data
            .iter_mut()
            .for_each(|(label, mesh_instance_group_data)| {
                mesh_instance_group_data.sync_with_mesh_instance_group(
                    core_system,
                    mesh_instance_groups.get(label).unwrap(),
                )
            });
    }

    /// Performs any required updates for keeping the given camera render
    /// data in sync with the given geometrical data.
    ///
    /// # Note
    /// Render data entries for which the associated geometrical data no
    /// longer exists will be removed.
    fn sync_camera_data_with_geometry(
        core_system: &CoreRenderingSystem,
        camera_render_data: &mut GeometryMap<CameraRenderDataManager>,
        cameras: &GeometryMap<impl Camera<f32>>,
    ) {
        Self::remove_unmatched_render_data(camera_render_data, cameras);
        camera_render_data
            .iter_mut()
            .for_each(|(label, camera_data)| {
                camera_data.sync_with_camera(core_system, cameras.get(label).unwrap())
            });
    }

    fn from_synchronized(render_data: SynchronizedRenderData) -> Self {
        let SynchronizedRenderData {
            color_mesh_data,
            texture_mesh_data,
            mesh_instance_group_data,
            perspective_camera_data,
        } = render_data;
        Self {
            color_mesh_data: Mutex::new(color_mesh_data),
            texture_mesh_data: Mutex::new(texture_mesh_data),
            mesh_instance_group_data: Mutex::new(mesh_instance_group_data),
            perspective_camera_data: Mutex::new(perspective_camera_data),
        }
    }

    fn into_synchronized(self) -> SynchronizedRenderData {
        let DesynchronizedRenderData {
            color_mesh_data,
            texture_mesh_data,
            mesh_instance_group_data,
            perspective_camera_data,
        } = self;
        SynchronizedRenderData {
            color_mesh_data: color_mesh_data.into_inner().unwrap(),
            texture_mesh_data: texture_mesh_data.into_inner().unwrap(),
            mesh_instance_group_data: mesh_instance_group_data.into_inner().unwrap(),
            perspective_camera_data: perspective_camera_data.into_inner().unwrap(),
        }
    }

    /// Removes render data whose geometrical counterpart is no longer present.
    fn remove_unmatched_render_data<T, U>(
        render_data: &mut GeometryMap<T>,
        geometrical_data: &GeometryMap<U>,
    ) {
        render_data.retain(|label, _| geometrical_data.contains_key(label));
    }
}
