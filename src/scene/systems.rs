//! Tasks representing ECS systems related to scenes.

use crate::{
    define_task,
    physics::{fmo, Position},
    rendering::RenderingTag,
    scene::{
        graph::SceneGraphNode, CameraNodeID, GroupNodeID, ModelInstanceNodeID, NodeStorage,
        Renderable,
    },
    world::World,
};
use impact_ecs::query;
use nalgebra::{Point3, Translation3};

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the model transform
    /// of each [`SceneGraph`](crate::scene::SceneGraph) node representing
    /// an entity that also has the [`Position`] component so that the
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
                ecs_world, |renderable: &Renderable<GroupNodeID>, position: &Position| {
                    set_node_translation_to_position(scene_graph.group_nodes_mut(), renderable, position);
                }
            );
            query!(
                ecs_world, |renderable: &Renderable<ModelInstanceNodeID>, position: &Position| {
                    set_node_translation_to_position(scene_graph.model_instance_nodes_mut(), renderable, position);
                }
            );
            query!(
                ecs_world, |renderable: &Renderable<CameraNodeID>, position: &Position| {
                    set_node_translation_to_position(scene_graph.camera_nodes_mut(), renderable, position);
                }
            );
            Ok(())
        })
    }
);

fn set_node_translation_to_position<N>(
    nodes: &mut NodeStorage<N>,
    renderable: &Renderable<N::ID>,
    position: &Position,
) where
    N: SceneGraphNode,
    fmo: simba::scalar::SubsetOf<<N as SceneGraphNode>::F>,
{
    let point: Point3<<N as SceneGraphNode>::F> = position.point.cast();
    nodes.set_node_translation(renderable.node_id, Translation3::from(point));
}
