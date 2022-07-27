//! Render data associated with geometrical objects.

use crate::geometry::{Camera, GeomIdent, GeometricalData, GeometryMap, Mesh, MeshInstanceGroup};
use crate::rendering::{
    buffer::BufferableVertex,
    camera::CameraRenderDataManager,
    mesh::{MeshInstanceRenderDataManager, MeshRenderDataManager},
    CoreRenderingSystem,
};

/// Collection for all render data for which there is associated
/// geometrical data.
#[derive(Debug)]
pub struct RenderData {
    color_mesh_data: GeometryMap<MeshRenderDataManager>,
    texture_mesh_data: GeometryMap<MeshRenderDataManager>,
    mesh_instance_group_data: GeometryMap<MeshInstanceRenderDataManager>,
    perspective_camera_data: GeometryMap<CameraRenderDataManager>,
}

impl RenderData {
    /// Creates all render data required for representing the
    /// given geometrical data.
    pub fn from_geometrical_data(
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
            color_mesh_data,
            texture_mesh_data,
            mesh_instance_group_data,
            perspective_camera_data,
        }
    }

    /// Returns the render data manager for the given mesh identifier
    /// if the mesh exists, otherwise returns [`None`].
    pub fn get_mesh_data(&self, ident: &GeomIdent) -> Option<&MeshRenderDataManager> {
        self.color_mesh_data
            .get(ident)
            .or_else(|| self.texture_mesh_data.get(ident))
    }

    /// Returns the render data manager for the given mesh instance
    /// group if the group exists, otherwise returns [`None`].
    pub fn get_mesh_instance_data(
        &self,
        ident: &GeomIdent,
    ) -> Option<&MeshInstanceRenderDataManager> {
        self.mesh_instance_group_data.get(ident)
    }

    /// Returns the render data manager for the given camera identifier
    /// if the camera exists, otherwise returns [`None`].
    pub fn get_camera_data(&self, ident: &GeomIdent) -> Option<&CameraRenderDataManager> {
        self.perspective_camera_data.get(ident)
    }

    /// Performs any required updates for keeping the render data in
    /// sync with the geometrical data.
    ///
    /// # Notes
    /// - Render data entries for which the associated geometrical data no
    /// longer exists will be removed.
    /// - Mutable access to the geometrical data is required in order to reset
    /// all change trackers.
    pub fn sync_with_geometry(
        &mut self,
        core_system: &CoreRenderingSystem,
        geometrical_data: &mut GeometricalData,
    ) {
        Self::sync_mesh_data_with_geometry(
            core_system,
            &mut self.color_mesh_data,
            &mut geometrical_data.color_meshes,
        );
        Self::sync_mesh_data_with_geometry(
            core_system,
            &mut self.texture_mesh_data,
            &mut geometrical_data.texture_meshes,
        );
        Self::sync_mesh_instance_group_data_with_geometry(
            core_system,
            &mut self.mesh_instance_group_data,
            &mut geometrical_data.mesh_instance_groups,
        );
        Self::sync_camera_data_with_geometry(
            core_system,
            &mut self.perspective_camera_data,
            &mut geometrical_data.perspective_cameras,
        );
    }

    fn create_mesh_render_data(
        core_system: &CoreRenderingSystem,
        meshes: &GeometryMap<Mesh<impl BufferableVertex>>,
    ) -> GeometryMap<MeshRenderDataManager> {
        meshes
            .iter()
            .map(|(label, mesh)| {
                (
                    label.clone(),
                    MeshRenderDataManager::for_mesh(core_system, mesh, label.clone()),
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
            .map(|(label, mesh_instance_group)| {
                (
                    label.clone(),
                    MeshInstanceRenderDataManager::for_mesh_instance_group(
                        core_system,
                        mesh_instance_group,
                        label.clone(),
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
            .map(|(label, camera)| {
                (
                    label.clone(),
                    CameraRenderDataManager::for_camera(core_system, camera, label),
                )
            })
            .collect()
    }

    fn sync_mesh_data_with_geometry(
        core_system: &CoreRenderingSystem,
        mesh_render_data: &mut GeometryMap<MeshRenderDataManager>,
        meshes: &mut GeometryMap<Mesh<impl BufferableVertex>>,
    ) {
        Self::remove_unmatched_render_data(mesh_render_data, meshes);
        mesh_render_data.iter_mut().for_each(|(label, mesh_data)| {
            mesh_data.sync_with_mesh(core_system, meshes.get_mut(label).unwrap())
        });
    }

    fn sync_mesh_instance_group_data_with_geometry(
        core_system: &CoreRenderingSystem,
        mesh_instance_group_render_data: &mut GeometryMap<MeshInstanceRenderDataManager>,
        mesh_instance_groups: &mut GeometryMap<MeshInstanceGroup<f32>>,
    ) {
        Self::remove_unmatched_render_data(mesh_instance_group_render_data, mesh_instance_groups);
        mesh_instance_group_render_data
            .iter_mut()
            .for_each(|(label, mesh_instance_group_data)| {
                mesh_instance_group_data.sync_with_mesh_instance_group(
                    core_system,
                    mesh_instance_groups.get_mut(label).unwrap(),
                )
            });
    }

    fn sync_camera_data_with_geometry(
        core_system: &CoreRenderingSystem,
        camera_render_data: &mut GeometryMap<CameraRenderDataManager>,
        cameras: &mut GeometryMap<impl Camera<f32>>,
    ) {
        Self::remove_unmatched_render_data(camera_render_data, cameras);
        camera_render_data
            .iter_mut()
            .for_each(|(label, camera_data)| {
                camera_data.sync_with_camera(core_system, cameras.get_mut(label).unwrap())
            });
    }

    /// Removes render data whose geometrical counterpart is no longer present.
    fn remove_unmatched_render_data<T, U>(
        render_data: &mut GeometryMap<T>,
        geometrical_data: &GeometryMap<U>,
    ) {
        render_data.retain(|label, _| geometrical_data.contains_key(label));
    }
}
