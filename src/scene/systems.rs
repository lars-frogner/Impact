//! Tasks representing ECS systems related to scenes.

use crate::{
    define_task,
    physics::PositionComp,
    rendering::RenderingTag,
    scene::{CameraNodeID, GroupNodeID, ModelInstanceNodeID, SceneGraphNodeComp},
    world::World,
};
use impact_ecs::query;
use nalgebra::Translation3;

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the model transform
    /// of each [`SceneGraph`](crate::scene::SceneGraph) node representing
    /// an entity that also has the [`PositionComp`] component so that the
    /// translational part matches the position.
    [pub] SyncSceneObjectTransformsWithPositions,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing scene graph node transforms with positions"; {
            let ecs_world = world.ecs_world().read().unwrap();
            let scene = world.scene().read().unwrap();
            let mut scene_graph = scene.scene_graph().write().unwrap();
            query!(
                ecs_world, |node: &SceneGraphNodeComp<GroupNodeID>, position: &PositionComp| {
                    scene_graph.set_group_node_translation(node.id, Translation3::from(position.point.cast()));
                }
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<ModelInstanceNodeID>, position: &PositionComp| {
                    scene_graph.set_model_instance_node_translation(node.id, Translation3::from(position.point.cast()));
                }
            );
            query!(
                ecs_world, |node: &SceneGraphNodeComp<CameraNodeID>, position: &PositionComp| {
                    scene_graph.set_camera_node_translation(node.id, Translation3::from(position.point.cast()));
                }
            );
            Ok(())
        })
    }
);
