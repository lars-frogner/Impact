//! ECS systems for scenes.

use crate::{
    camera::SceneCamera,
    gpu::rendering::fre,
    light::{
        components::{
            AmbientEmissionComp, AmbientLightComp, OmnidirectionalEmissionComp,
            OmnidirectionalLightComp, UnidirectionalEmissionComp, UnidirectionalLightComp,
        },
        LightStorage,
    },
    physics::motion::components::ReferenceFrameComp,
    scene::{
        components::{SceneGraphNodeComp, SceneGraphParentNodeComp},
        CameraNodeID, GroupNodeID, ModelInstanceNodeID, SceneGraph,
    },
    voxel::components::VoxelTreeNodeComp,
};
use impact_ecs::{query, world::World as ECSWorld};
use nalgebra::{Similarity3, UnitVector3};

/// Updates the model transform of each [`SceneGraph`](crate::scene::SceneGraph)
/// node representing an entity that also has the [`ReferenceFrameComp`]
/// component so that the translational, rotational and scaling parts match the
/// origin offset, position, orientation and scaling.
pub fn sync_scene_object_transforms(ecs_world: &ECSWorld, scene_graph: &mut SceneGraph<fre>) {
    query!(
        ecs_world,
        |node: &SceneGraphNodeComp<GroupNodeID>, frame: &ReferenceFrameComp| {
            scene_graph
                .set_group_to_parent_transform(node.id, frame.create_transform_to_parent_space());
        }
    );
    query!(
        ecs_world,
        |node: &SceneGraphNodeComp<ModelInstanceNodeID>, frame: &ReferenceFrameComp| {
            scene_graph
                .set_model_to_parent_transform(node.id, frame.create_transform_to_parent_space());
        }
    );
    query!(
        ecs_world,
        |node: &SceneGraphNodeComp<CameraNodeID>, frame: &ReferenceFrameComp| {
            scene_graph
                .set_camera_to_parent_transform(node.id, frame.create_transform_to_parent_space());
        }
    );
    query!(
        ecs_world,
        |voxel_tree_node: &VoxelTreeNodeComp, frame: &ReferenceFrameComp| {
            scene_graph.set_group_to_parent_transform(
                voxel_tree_node.group_node_id,
                frame.create_transform_to_parent_space(),
            );
        }
    );
}

/// Updates the properties (position, direction, emission and extent) of every
/// light source in the [`LightStorage`](crate::light::LightStorage).
pub fn sync_lights_in_storage(
    ecs_world: &ECSWorld,
    scene_graph: &SceneGraph<fre>,
    scene_camera: Option<&SceneCamera<fre>>,
    light_storage: &mut LightStorage,
) {
    let view_transform = scene_camera.map_or_else(Similarity3::identity, |scene_camera| {
        *scene_camera.view_transform()
    });

    query!(
        ecs_world,
        |ambient_light: &AmbientLightComp, ambient_emission: &AmbientEmissionComp| {
            light_storage
                .ambient_light_mut(ambient_light.id)
                .set_illuminance(ambient_emission.illuminance);
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light: &OmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmissionComp| {
            let light_id = omnidirectional_light.id;
            let light = light_storage.omnidirectional_light_mut(light_id);
            light.set_camera_space_position(view_transform.transform_point(&frame.position.cast()));
            light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
            light.set_emission_extent(omnidirectional_emission.source_extent);
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |omnidirectional_light: &OmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            let light_id = omnidirectional_light.id;
            let light = light_storage.omnidirectional_light_mut(light_id);
            light.set_camera_space_position(view_transform.transform_point(&frame.position.cast()));
            light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
            light.set_emission_extent(omnidirectional_emission.source_extent);
        }
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp| {
            let light_id = unidirectional_light.id;
            let light = light_storage.unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&unidirectional_emission.direction),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
        },
        ![SceneGraphParentNodeComp, ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            let light_id = unidirectional_light.id;
            let light = light_storage.unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&unidirectional_emission.direction),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
        },
        ![ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         frame: &ReferenceFrameComp| {
            let world_direction = frame
                .orientation
                .transform_vector(&unidirectional_emission.direction.cast());

            let light_id = unidirectional_light.id;
            let light = light_storage.unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&world_direction.cast()),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         parent: &SceneGraphParentNodeComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();
            let world_direction = frame
                .orientation
                .transform_vector(&unidirectional_emission.direction.cast());

            let light_id = unidirectional_light.id;
            let light = light_storage.unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&world_direction.cast()),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
        }
    );
}
