//! ECS systems for scenes.

use crate::{
    camera::SceneCamera,
    physics::motion::components::ReferenceFrameComp,
    scene::{
        SceneGraph,
        components::{
            SceneEntityFlagsComp, SceneGraphCameraNodeComp, SceneGraphGroupNodeComp,
            SceneGraphModelInstanceNodeComp, SceneGraphParentNodeComp,
        },
    },
};
use impact_camera::buffer::BufferableCamera;
use impact_ecs::{query, world::World as ECSWorld};
use impact_light::{
    LightStorage,
    components::{
        AmbientEmissionComp, AmbientLightComp, OmnidirectionalEmissionComp,
        OmnidirectionalLightComp, ShadowableOmnidirectionalEmissionComp,
        ShadowableOmnidirectionalLightComp, ShadowableUnidirectionalEmissionComp,
        ShadowableUnidirectionalLightComp, UnidirectionalEmissionComp, UnidirectionalLightComp,
    },
};
use nalgebra::{Similarity3, UnitVector3};

/// Updates the model transform of each [`SceneGraph`] node representing an
/// entity that also has the
/// [`crate::physics::motion::components::ReferenceFrameComp`] component so that
/// the translational, rotational and scaling parts match the origin offset,
/// position, orientation and scaling. Also updates any flags for the node to
/// match the entity's [`crate::scene::SceneEntityFlags`].
pub fn sync_scene_object_transforms_and_flags(ecs_world: &ECSWorld, scene_graph: &mut SceneGraph) {
    query!(
        ecs_world,
        |node: &SceneGraphGroupNodeComp, frame: &ReferenceFrameComp| {
            scene_graph
                .set_group_to_parent_transform(node.id, frame.create_transform_to_parent_space());
        }
    );
    query!(
        ecs_world,
        |node: &SceneGraphModelInstanceNodeComp,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlagsComp| {
            scene_graph.set_model_to_parent_transform_and_flags(
                node.id,
                frame.create_transform_to_parent_space(),
                flags.0.into(),
            );
        }
    );
    query!(
        ecs_world,
        |node: &SceneGraphCameraNodeComp, frame: &ReferenceFrameComp| {
            scene_graph
                .set_camera_to_parent_transform(node.id, frame.create_transform_to_parent_space());
        }
    );
}

/// Updates the properties (position, direction, emission, extent and flags) of
/// every light source in the [`LightStorage`].
pub fn sync_lights_in_storage(
    ecs_world: &ECSWorld,
    scene_graph: &SceneGraph,
    scene_camera: Option<&SceneCamera>,
    light_storage: &mut LightStorage,
) {
    let view_transform = scene_camera.map_or_else(Similarity3::identity, |scene_camera| {
        *scene_camera.view_transform()
    });

    query!(
        ecs_world,
        |ambient_light: &AmbientLightComp, ambient_emission: &AmbientEmissionComp| {
            light_storage
                .set_ambient_light_illuminance(ambient_light.id, ambient_emission.illuminance);
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light: &OmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmissionComp,
         flags: &SceneEntityFlagsComp| {
            let light_id = omnidirectional_light.id;
            let light = light_storage.omnidirectional_light_mut(light_id);
            light.set_camera_space_position(view_transform.transform_point(&frame.position.cast()));
            light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
            light.set_emissive_extent(omnidirectional_emission.source_extent);
            light.set_flags(flags.0.into());
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |omnidirectional_light: &OmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &OmnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            let light_id = omnidirectional_light.id;
            let light = light_storage.omnidirectional_light_mut(light_id);
            light.set_camera_space_position(view_transform.transform_point(&frame.position.cast()));
            light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
            light.set_emissive_extent(omnidirectional_emission.source_extent);
            light.set_flags(flags.0.into());
        }
    );

    query!(
        ecs_world,
        |omnidirectional_light: &ShadowableOmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &ShadowableOmnidirectionalEmissionComp,
         flags: &SceneEntityFlagsComp| {
            let light_id = omnidirectional_light.id;
            let light = light_storage.shadowable_omnidirectional_light_mut(light_id);
            light.set_camera_space_position(view_transform.transform_point(&frame.position.cast()));
            light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
            light.set_emissive_extent(omnidirectional_emission.source_extent);
            light.set_flags(flags.0.into());
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |omnidirectional_light: &ShadowableOmnidirectionalLightComp,
         frame: &ReferenceFrameComp,
         omnidirectional_emission: &ShadowableOmnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            let light_id = omnidirectional_light.id;
            let light = light_storage.shadowable_omnidirectional_light_mut(light_id);
            light.set_camera_space_position(view_transform.transform_point(&frame.position.cast()));
            light.set_luminous_intensity(omnidirectional_emission.luminous_intensity);
            light.set_emissive_extent(omnidirectional_emission.source_extent);
            light.set_flags(flags.0.into());
        }
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         flags: &SceneEntityFlagsComp| {
            let light_id = unidirectional_light.id;
            let light = light_storage.unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&unidirectional_emission.direction),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
            light.set_flags(flags.0.into());
        },
        ![SceneGraphParentNodeComp, ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            let light_id = unidirectional_light.id;
            let light = light_storage.unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&unidirectional_emission.direction),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
            light.set_flags(flags.0.into());
        },
        ![ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlagsComp| {
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
            light.set_flags(flags.0.into());
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &UnidirectionalLightComp,
         unidirectional_emission: &UnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
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
            light.set_flags(flags.0.into());
        }
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightComp,
         unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         flags: &SceneEntityFlagsComp| {
            let light_id = unidirectional_light.id;
            let light = light_storage.shadowable_unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&unidirectional_emission.direction),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
            light.set_flags(flags.0.into());
        },
        ![SceneGraphParentNodeComp, ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightComp,
         unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();

            let light_id = unidirectional_light.id;
            let light = light_storage.shadowable_unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&unidirectional_emission.direction),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
            light.set_flags(flags.0.into());
        },
        ![ReferenceFrameComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightComp,
         unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         flags: &SceneEntityFlagsComp| {
            let world_direction = frame
                .orientation
                .transform_vector(&unidirectional_emission.direction.cast());

            let light_id = unidirectional_light.id;
            let light = light_storage.shadowable_unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&world_direction.cast()),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
            light.set_flags(flags.0.into());
        },
        ![SceneGraphParentNodeComp]
    );

    query!(
        ecs_world,
        |unidirectional_light: &ShadowableUnidirectionalLightComp,
         unidirectional_emission: &ShadowableUnidirectionalEmissionComp,
         frame: &ReferenceFrameComp,
         parent: &SceneGraphParentNodeComp,
         flags: &SceneEntityFlagsComp| {
            let parent_group_node = scene_graph.group_nodes().node(parent.id);

            let view_transform = view_transform * parent_group_node.group_to_root_transform();
            let world_direction = frame
                .orientation
                .transform_vector(&unidirectional_emission.direction.cast());

            let light_id = unidirectional_light.id;
            let light = light_storage.shadowable_unidirectional_light_mut(light_id);
            light.set_camera_space_direction(UnitVector3::new_unchecked(
                view_transform.transform_vector(&world_direction.cast()),
            ));
            light.set_perpendicular_illuminance(unidirectional_emission.perpendicular_illuminance);
            light.set_angular_extent(unidirectional_emission.angular_source_extent);
            light.set_flags(flags.0.into());
        }
    );
}
