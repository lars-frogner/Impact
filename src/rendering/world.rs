//! Render data associated with geometrical objects.

use crate::geometry::{Camera, Mesh, MeshInstanceGroup, WorldData, WorldIdent, WorldObjMap};
use crate::rendering::{
    buffer::BufferableVertex,
    camera::CameraRenderDataManager,
    mesh::{MeshInstanceRenderDataManager, MeshRenderDataManager},
    CoreRenderingSystem,
};

/// Collection for all render data for which there is associated
/// geometrical world data.
pub struct RenderData {
    color_mesh_data: WorldObjMap<MeshRenderDataManager>,
    texture_mesh_data: WorldObjMap<MeshRenderDataManager>,
    mesh_instance_group_data: WorldObjMap<MeshInstanceRenderDataManager>,
    perspective_camera_data: WorldObjMap<CameraRenderDataManager>,
}

impl RenderData {
    /// Creates all render data required for representing the
    /// geometrical data in the given world.
    pub fn from_world_data(core_system: &CoreRenderingSystem, world_data: &WorldData) -> Self {
        let color_mesh_data = Self::create_mesh_render_data(core_system, &world_data.color_meshes);
        let texture_mesh_data =
            Self::create_mesh_render_data(core_system, &world_data.texture_meshes);

        let mesh_instance_group_data = Self::create_mesh_instance_group_render_data(
            core_system,
            &world_data.mesh_instance_groups,
        );

        let perspective_camera_data =
            Self::create_camera_render_data(core_system, &world_data.perspective_cameras);

        Self {
            color_mesh_data,
            texture_mesh_data,
            mesh_instance_group_data,
            perspective_camera_data,
        }
    }

    /// Returns the render data manager for the given mesh identifier
    /// if the mesh exists, otherwise returns `None`.
    pub fn get_mesh_data(&self, ident: &WorldIdent) -> Option<&MeshRenderDataManager> {
        self.color_mesh_data
            .get(ident)
            .or_else(|| self.texture_mesh_data.get(ident))
    }

    /// Returns the render data manager for the given mesh instance
    /// group if the group exists, otherwise returns `None`.
    pub fn get_mesh_instance_data(
        &self,
        ident: &WorldIdent,
    ) -> Option<&MeshInstanceRenderDataManager> {
        self.mesh_instance_group_data.get(ident)
    }

    /// Returns the render data manager for the given camera identifier
    /// if the camera exists, otherwise returns `None`.
    pub fn get_camera_data(&self, ident: &WorldIdent) -> Option<&CameraRenderDataManager> {
        self.perspective_camera_data.get(ident)
    }

    /// Performs any required updates for keeping the render data in
    /// sync with the world data.
    ///
    /// # Notes
    /// - Render data entries for which the associated world data no
    /// longer exists will be removed.
    /// - Mutable access to the world data is required in order to reset
    /// all change trackers.
    pub fn sync_with_world(
        &mut self,
        core_system: &CoreRenderingSystem,
        world_data: &mut WorldData,
    ) {
        Self::sync_mesh_data_with_world(
            core_system,
            &mut self.color_mesh_data,
            &mut world_data.color_meshes,
        );
        Self::sync_mesh_data_with_world(
            core_system,
            &mut self.texture_mesh_data,
            &mut world_data.texture_meshes,
        );
        Self::sync_mesh_instance_group_data_with_world(
            core_system,
            &mut self.mesh_instance_group_data,
            &mut world_data.mesh_instance_groups,
        );
        Self::sync_camera_data_with_world(
            core_system,
            &mut self.perspective_camera_data,
            &mut world_data.perspective_cameras,
        );
    }

    fn create_mesh_render_data<V>(
        core_system: &CoreRenderingSystem,
        meshes: &WorldObjMap<Mesh<V>>,
    ) -> WorldObjMap<MeshRenderDataManager>
    where
        V: BufferableVertex,
    {
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
        mesh_instance_groups: &WorldObjMap<MeshInstanceGroup<f32>>,
    ) -> WorldObjMap<MeshInstanceRenderDataManager> {
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

    fn create_camera_render_data<C>(
        core_system: &CoreRenderingSystem,
        cameras: &WorldObjMap<C>,
    ) -> WorldObjMap<CameraRenderDataManager>
    where
        C: Camera<f32>,
    {
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

    fn sync_mesh_data_with_world<V>(
        core_system: &CoreRenderingSystem,
        mesh_render_data: &mut WorldObjMap<MeshRenderDataManager>,
        meshes: &mut WorldObjMap<Mesh<V>>,
    ) where
        V: BufferableVertex,
    {
        Self::remove_unmatched_render_data(mesh_render_data, meshes);
        mesh_render_data.iter_mut().for_each(|(label, mesh_data)| {
            mesh_data.sync_with_mesh(core_system, meshes.get_mut(label).unwrap())
        });
    }

    fn sync_mesh_instance_group_data_with_world(
        core_system: &CoreRenderingSystem,
        mesh_instance_group_render_data: &mut WorldObjMap<MeshInstanceRenderDataManager>,
        mesh_instance_groups: &mut WorldObjMap<MeshInstanceGroup<f32>>,
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

    fn sync_camera_data_with_world<C>(
        core_system: &CoreRenderingSystem,
        camera_render_data: &mut WorldObjMap<CameraRenderDataManager>,
        cameras: &mut WorldObjMap<C>,
    ) where
        C: Camera<f32>,
    {
        Self::remove_unmatched_render_data(camera_render_data, cameras);
        camera_render_data
            .iter_mut()
            .for_each(|(label, camera_data)| {
                camera_data.sync_with_camera(core_system, cameras.get_mut(label).unwrap())
            });
    }

    /// Removes render data whose world data counterpart is no longer present.
    fn remove_unmatched_render_data<T, U>(
        render_data: &mut WorldObjMap<T>,
        world_data: &WorldObjMap<U>,
    ) {
        render_data.retain(|label, _| world_data.contains_key(label));
    }
}
