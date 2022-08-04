use super::{DesynchronizedRenderData, RenderData};
use crate::{
    define_task,
    rendering::RenderingTag,
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) performs any required
    /// updates for keeping the [`World`]s render data in sync with
    /// its geometrical data.
    ///
    /// # Note
    /// Render data entries for which the associated geometrical data
    /// no longer exists are removed.
    [pub] SyncRenderData,
    depends_on = [
        SyncColorMeshData,
        SyncTextureMeshData,
        SyncMeshInstanceGroupData,
        SyncPerspectiveCameraData
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Completing synchronization of render data"; {
            let renderer = world.renderer().read().unwrap();
            let mut render_data = renderer.render_data().write().unwrap();
            render_data.declare_synchronized();
            Ok(())
        })
    }
);

impl RenderData {
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncColorMeshData)?;
        task_scheduler.register_task(SyncTextureMeshData)?;
        task_scheduler.register_task(SyncMeshInstanceGroupData)?;
        task_scheduler.register_task(SyncPerspectiveCameraData)?;
        task_scheduler.register_task(SyncRenderData)
    }
}

define_task!(
    SyncColorMeshData,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing color mesh data"; {
            let renderer = world.renderer().read().unwrap();
            let render_data = renderer.render_data().read().unwrap();
            DesynchronizedRenderData::sync_mesh_data_with_geometry(
                renderer.core_system(),
                render_data
                    .desynchronized()
                    .color_mesh_data
                    .lock()
                    .unwrap()
                    .as_mut(),
                &world.geometrical_data().read().unwrap().color_meshes,
            );
            Ok(())
        })
    }
);

define_task!(
    SyncTextureMeshData,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing texture mesh data"; {
            let renderer = world.renderer().read().unwrap();
            let render_data = renderer.render_data().read().unwrap();
            DesynchronizedRenderData::sync_mesh_data_with_geometry(
                renderer.core_system(),
                render_data
                    .desynchronized()
                    .texture_mesh_data
                    .lock()
                    .unwrap()
                    .as_mut(),
                &world.geometrical_data().read().unwrap().texture_meshes,
            );
            Ok(())
        })
    }
);

define_task!(
    SyncMeshInstanceGroupData,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing mesh instance group data"; {
            let renderer = world.renderer().read().unwrap();
            let render_data = renderer.render_data().read().unwrap();
            DesynchronizedRenderData::sync_mesh_instance_group_data_with_geometry(
                renderer.core_system(),
                render_data
                    .desynchronized()
                    .mesh_instance_group_data
                    .lock()
                    .unwrap()
                    .as_mut(),
                &world
                    .geometrical_data()
                    .read()
                    .unwrap()
                    .mesh_instance_groups,
            );
            Ok(())
        })
    }
);

define_task!(
    SyncPerspectiveCameraData,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing perspective camera data"; {
            let renderer = world.renderer().read().unwrap();
            let render_data = renderer.render_data().read().unwrap();
            DesynchronizedRenderData::sync_camera_data_with_geometry(
                renderer.core_system(),
                render_data
                    .desynchronized()
                    .perspective_camera_data
                    .lock()
                    .unwrap()
                    .as_mut(),
                &world.geometrical_data().read().unwrap().perspective_cameras,
            );
            Ok(())
        })
    }
);
